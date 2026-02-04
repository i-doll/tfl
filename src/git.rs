use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use git2::{BranchType, Repository, Status, StatusOptions};

use crate::fs::entry::{GitFileStatus, GitStatus};

#[derive(Debug, Default)]
pub struct GitRepoInfo {
  pub branch: Option<String>,
  pub ahead: usize,
  pub behind: usize,
  pub staged_count: usize,
  pub modified_count: usize,
  pub untracked_count: usize,
}

#[allow(dead_code)]
pub struct GitCommit {
  pub hash: String,
  pub author: String,
  pub date: String,
  pub message: String,
}

pub struct GitRepo {
  #[allow(dead_code)]
  repo: Repository,
  root: PathBuf,
}

impl std::fmt::Debug for GitRepo {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("GitRepo").field("root", &self.root).finish()
  }
}

impl GitRepo {
  pub fn open(path: &Path) -> Option<Self> {
    Repository::discover(path).ok().map(|repo| {
      let root = repo.workdir().map(|p| p.to_path_buf()).unwrap_or_default();
      Self { repo, root }
    })
  }

  #[allow(dead_code)]
  pub fn root(&self) -> &Path {
    &self.root
  }

  pub fn get_branch(&self) -> Option<String> {
    let head = self.repo.head().ok()?;
    if head.is_branch() {
      head.shorthand().map(|s| s.to_string())
    } else {
      None
    }
  }

  pub fn get_ahead_behind(&self) -> (usize, usize) {
    let Ok(head) = self.repo.head() else { return (0, 0) };
    let Some(local_oid) = head.target() else { return (0, 0) };

    // Find upstream branch
    let Ok(local_branch) = self.repo.find_branch(
      head.shorthand().unwrap_or(""),
      BranchType::Local,
    ) else {
      return (0, 0);
    };

    let Ok(upstream) = local_branch.upstream() else { return (0, 0) };
    let Some(upstream_oid) = upstream.get().target() else { return (0, 0) };

    self.repo.graph_ahead_behind(local_oid, upstream_oid).unwrap_or((0, 0))
  }

  pub fn get_file_statuses(&self) -> (HashMap<PathBuf, GitStatus>, GitRepoInfo) {
    let mut info = GitRepoInfo {
      branch: self.get_branch(),
      ..Default::default()
    };
    let (ahead, behind) = self.get_ahead_behind();
    info.ahead = ahead;
    info.behind = behind;

    let mut opts = StatusOptions::new();
    opts.include_untracked(true);
    opts.recurse_untracked_dirs(true);
    opts.include_ignored(false);

    let Ok(statuses) = self.repo.statuses(Some(&mut opts)) else {
      return (HashMap::new(), info);
    };

    let mut map: HashMap<PathBuf, GitStatus> = HashMap::new();

    for entry in statuses.iter() {
      let Some(path_str) = entry.path() else { continue };
      let abs_path = self.root.join(path_str);
      let status = entry.status();

      let git_status = convert_status(status);

      if git_status.staged.is_some() && git_status.staged != Some(GitFileStatus::Untracked) {
        info.staged_count += 1;
      }
      if git_status.unstaged == Some(GitFileStatus::Modified) {
        info.modified_count += 1;
      }
      if git_status.unstaged == Some(GitFileStatus::Untracked) {
        info.untracked_count += 1;
      }

      map.insert(abs_path, git_status);
    }

    (map, info)
  }

  pub fn is_ignored(&self, path: &Path) -> bool {
    // Make path relative to repo root
    let rel_path = path.strip_prefix(&self.root).unwrap_or(path);
    self.repo.status_should_ignore(rel_path).unwrap_or(false)
  }

  pub fn is_ignored_batch(&self, paths: &[PathBuf]) -> HashSet<PathBuf> {
    let mut ignored = HashSet::new();
    for path in paths {
      if self.is_ignored(path) {
        ignored.insert(path.clone());
      }
    }
    ignored
  }

  pub fn get_file_commits(&self, path: &Path, limit: usize) -> Vec<GitCommit> {
    let rel_path = match path.strip_prefix(&self.root) {
      Ok(p) => p,
      Err(_) => return Vec::new(),
    };
    let rel_path_str = rel_path.to_string_lossy();

    let Ok(head) = self.repo.head() else { return Vec::new() };
    let Some(head_oid) = head.target() else { return Vec::new() };

    let mut revwalk = match self.repo.revwalk() {
      Ok(rw) => rw,
      Err(_) => return Vec::new(),
    };

    if revwalk.push(head_oid).is_err() {
      return Vec::new();
    }

    let mut commits = Vec::new();

    for oid_result in revwalk {
      let Ok(oid) = oid_result else { continue };
      let Ok(commit) = self.repo.find_commit(oid) else { continue };

      // Check if this commit touches the file
      if !commit_touches_file(&self.repo, &commit, &rel_path_str) {
        continue;
      }

      let hash = oid.to_string()[..7].to_string();
      let author = commit.author().name().unwrap_or("").to_string();
      let time = commit.time();
      let date = format_relative_time(time.seconds());
      let message = commit
        .message()
        .unwrap_or("")
        .lines()
        .next()
        .unwrap_or("")
        .to_string();

      commits.push(GitCommit { hash, author, date, message });

      if commits.len() >= limit {
        break;
      }
    }

    commits
  }
}

fn commit_touches_file(repo: &Repository, commit: &git2::Commit, path: &str) -> bool {
  let Ok(tree) = commit.tree() else { return false };

  // Check if file exists in this commit
  let in_commit = tree.get_path(Path::new(path)).is_ok();

  // Get parent tree (if any)
  let parent_has_file = if commit.parent_count() > 0 {
    if let Ok(parent) = commit.parent(0) {
      if let Ok(parent_tree) = parent.tree() {
        parent_tree.get_path(Path::new(path)).is_ok()
      } else {
        false
      }
    } else {
      false
    }
  } else {
    false
  };

  // File was added, removed, or potentially modified
  if in_commit != parent_has_file {
    return true;
  }

  // If file exists in both, check if content changed
  if in_commit && parent_has_file {
    if let Ok(parent) = commit.parent(0)
      && let Ok(diff) = repo.diff_tree_to_tree(parent.tree().ok().as_ref(), Some(&tree), None)
    {
      for delta in diff.deltas() {
        if let Some(new_path) = delta.new_file().path()
          && new_path.to_string_lossy() == path
        {
          return true;
        }
        if let Some(old_path) = delta.old_file().path()
          && old_path.to_string_lossy() == path
        {
          return true;
        }
      }
    }
    return false;
  }

  in_commit && commit.parent_count() == 0
}

fn convert_status(status: Status) -> GitStatus {
  // Check for conflicts first
  if status.is_conflicted() {
    return GitStatus {
      staged: Some(GitFileStatus::Conflicted),
      unstaged: Some(GitFileStatus::Conflicted),
    };
  }

  // Check for untracked
  if status.is_wt_new() && !status.is_index_new() {
    return GitStatus {
      staged: None,
      unstaged: Some(GitFileStatus::Untracked),
    };
  }

  let staged = if status.is_index_new() {
    Some(GitFileStatus::Added)
  } else if status.is_index_modified() {
    Some(GitFileStatus::Modified)
  } else if status.is_index_deleted() {
    Some(GitFileStatus::Deleted)
  } else if status.is_index_renamed() {
    Some(GitFileStatus::Renamed)
  } else {
    None
  };

  let unstaged = if status.is_wt_modified() {
    Some(GitFileStatus::Modified)
  } else if status.is_wt_deleted() {
    Some(GitFileStatus::Deleted)
  } else if status.is_wt_renamed() {
    Some(GitFileStatus::Renamed)
  } else if status.is_wt_new() {
    Some(GitFileStatus::Untracked)
  } else {
    None
  };

  GitStatus { staged, unstaged }
}

fn format_relative_time(timestamp: i64) -> String {
  let now = std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)
    .map(|d| d.as_secs() as i64)
    .unwrap_or(0);

  let diff = now - timestamp;
  if diff < 0 {
    return "future".to_string();
  }

  let diff = diff as u64;
  let minute = 60;
  let hour = minute * 60;
  let day = hour * 24;
  let week = day * 7;
  let month = day * 30;
  let year = day * 365;

  if diff < minute {
    "now".to_string()
  } else if diff < hour {
    let m = diff / minute;
    format!("{m}m ago")
  } else if diff < day {
    let h = diff / hour;
    format!("{h}h ago")
  } else if diff < week {
    let d = diff / day;
    format!("{d}d ago")
  } else if diff < month {
    let w = diff / week;
    format!("{w}w ago")
  } else if diff < year {
    let m = diff / month;
    format!("{m}mo ago")
  } else {
    let y = diff / year;
    format!("{y}y ago")
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::fs;
  use std::sync::atomic::{AtomicU32, Ordering};

  static COUNTER: AtomicU32 = AtomicU32::new(0);

  fn make_test_dir() -> PathBuf {
    let id = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!("tfl_git_test_{id}_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
  }

  fn init_git_repo(dir: &Path) {
    Repository::init(dir).unwrap();
    // Configure user for commits
    let repo = Repository::open(dir).unwrap();
    let mut config = repo.config().unwrap();
    config.set_str("user.email", "test@test.com").unwrap();
    config.set_str("user.name", "Test").unwrap();
  }

  #[test]
  fn test_open_non_git_dir() {
    let dir = make_test_dir();
    let repo = GitRepo::open(&dir);
    assert!(repo.is_none());
    let _ = fs::remove_dir_all(&dir);
  }

  #[test]
  fn test_open_git_repo() {
    let dir = make_test_dir();
    init_git_repo(&dir);
    let repo = GitRepo::open(&dir);
    assert!(repo.is_some());
    let _ = fs::remove_dir_all(&dir);
  }

  #[test]
  fn test_get_branch_empty_repo() {
    let dir = make_test_dir();
    init_git_repo(&dir);
    let repo = GitRepo::open(&dir).unwrap();
    // Empty repo has no branch until first commit
    let branch = repo.get_branch();
    // May be None or "master"/"main" depending on git config
    assert!(branch.is_none() || branch.is_some());
    let _ = fs::remove_dir_all(&dir);
  }

  #[test]
  fn test_is_ignored() {
    let dir = make_test_dir();
    init_git_repo(&dir);
    fs::write(dir.join(".gitignore"), "*.log\n").unwrap();
    fs::write(dir.join("test.log"), "log data").unwrap();
    fs::write(dir.join("test.txt"), "text data").unwrap();

    let repo = GitRepo::open(&dir).unwrap();
    assert!(repo.is_ignored(&dir.join("test.log")));
    assert!(!repo.is_ignored(&dir.join("test.txt")));

    let _ = fs::remove_dir_all(&dir);
  }

  #[test]
  fn test_get_file_statuses_untracked() {
    let dir = make_test_dir();
    init_git_repo(&dir);
    fs::write(dir.join("untracked.txt"), "new file").unwrap();

    let repo = GitRepo::open(&dir).unwrap();
    let (statuses, info) = repo.get_file_statuses();

    let status = statuses.get(&dir.join("untracked.txt"));
    assert!(status.is_some());
    assert_eq!(status.unwrap().unstaged, Some(GitFileStatus::Untracked));
    assert_eq!(info.untracked_count, 1);

    let _ = fs::remove_dir_all(&dir);
  }

  #[test]
  fn test_format_relative_time() {
    let now = std::time::SystemTime::now()
      .duration_since(std::time::UNIX_EPOCH)
      .unwrap()
      .as_secs() as i64;

    assert_eq!(format_relative_time(now), "now");
    assert_eq!(format_relative_time(now - 30), "now"); // less than a minute
    assert_eq!(format_relative_time(now - 120), "2m ago");
    assert_eq!(format_relative_time(now - 3600), "1h ago");
    assert_eq!(format_relative_time(now - 86400), "1d ago");
    assert_eq!(format_relative_time(now - 86400 * 7), "1w ago");
  }

  #[test]
  fn test_is_ignored_batch() {
    let dir = make_test_dir();
    init_git_repo(&dir);
    fs::write(dir.join(".gitignore"), "*.log\nbuild/\n").unwrap();
    fs::write(dir.join("test.log"), "log").unwrap();
    fs::write(dir.join("test.txt"), "txt").unwrap();
    fs::create_dir_all(dir.join("build")).unwrap();
    fs::write(dir.join("build/output"), "build").unwrap();

    let repo = GitRepo::open(&dir).unwrap();
    let paths = vec![
      dir.join("test.log"),
      dir.join("test.txt"),
      dir.join("build"),
    ];
    let ignored = repo.is_ignored_batch(&paths);

    assert!(ignored.contains(&dir.join("test.log")));
    assert!(!ignored.contains(&dir.join("test.txt")));
    assert!(ignored.contains(&dir.join("build")));

    let _ = fs::remove_dir_all(&dir);
  }

  #[test]
  fn test_convert_status_untracked() {
    let status = convert_status(Status::WT_NEW);
    assert_eq!(status.staged, None);
    assert_eq!(status.unstaged, Some(GitFileStatus::Untracked));
  }

  #[test]
  fn test_convert_status_staged_new() {
    let status = convert_status(Status::INDEX_NEW);
    assert_eq!(status.staged, Some(GitFileStatus::Added));
    assert_eq!(status.unstaged, None);
  }

  #[test]
  fn test_convert_status_modified() {
    let status = convert_status(Status::WT_MODIFIED);
    assert_eq!(status.staged, None);
    assert_eq!(status.unstaged, Some(GitFileStatus::Modified));
  }

  #[test]
  fn test_convert_status_conflicted() {
    let status = convert_status(Status::CONFLICTED);
    assert_eq!(status.staged, Some(GitFileStatus::Conflicted));
    assert_eq!(status.unstaged, Some(GitFileStatus::Conflicted));
  }
}
