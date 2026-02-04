use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::Result;

use super::entry::{FileEntry, GitStatus};
use crate::git::{GitRepo, GitRepoInfo};

fn mark_git_status(statuses: &HashMap<PathBuf, GitStatus>, children: &mut [FileEntry]) {
  for child in children.iter_mut() {
    let status = statuses.get(&child.path).or_else(|| {
      child.path.canonicalize().ok().and_then(|p| statuses.get(&p))
    });
    if let Some(status) = status {
      child.git_status = *status;
    }
  }
}

fn propagate_git_status(entries: &mut [FileEntry]) {
  // Walk forward; for each non-clean entry, walk backward to set parent dirs
  let len = entries.len();
  for i in 0..len {
    if entries[i].git_status.is_clean() {
      continue;
    }
    let child_status = entries[i].git_status;
    let child_depth = entries[i].depth;
    // Walk backward to find parent directories
    if child_depth > 0 {
      for j in (0..i).rev() {
        if entries[j].is_dir && entries[j].depth < child_depth {
          entries[j].git_status.merge(&child_status);
          if entries[j].depth == 0 {
            break;
          }
        }
      }
    }
  }
}

fn mark_git_ignored(git_repo: Option<&GitRepo>, children: &mut [FileEntry]) {
  let Some(repo) = git_repo else { return };
  if children.is_empty() {
    return;
  }
  let paths: Vec<PathBuf> = children.iter().map(|c| c.path.clone()).collect();
  let ignored = repo.is_ignored_batch(&paths);
  for child in children.iter_mut() {
    child.is_git_ignored = ignored.contains(&child.path);
  }
}

#[derive(Debug)]
pub struct FileTree {
  pub root: PathBuf,
  pub entries: Vec<FileEntry>,
  pub show_hidden: bool,
  pub git_statuses: HashMap<PathBuf, GitStatus>,
  pub git_info: GitRepoInfo,
  git_repo: Option<GitRepo>,
}

impl FileTree {
  pub fn new(root: PathBuf) -> Result<Self> {
    let git_repo = GitRepo::open(&root);
    let (git_statuses, git_info) = git_repo
      .as_ref()
      .map(|r| r.get_file_statuses())
      .unwrap_or_default();
    let mut tree = Self {
      root: root.clone(),
      entries: Vec::new(),
      show_hidden: false,
      git_statuses,
      git_info,
      git_repo,
    };
    tree.load_dir(&root, 0)?;
    propagate_git_status(&mut tree.entries);
    Ok(tree)
  }

  pub fn git_repo(&self) -> Option<&GitRepo> {
    self.git_repo.as_ref()
  }

  pub fn load_dir(&mut self, path: &Path, depth: usize) -> Result<()> {
    let insert_pos = if depth == 0 {
      self.entries.clear();
      0
    } else {
      self
        .entries
        .iter()
        .position(|e| e.path == path)
        .map(|i| i + 1)
        .unwrap_or(self.entries.len())
    };

    let mut children = Vec::new();
    let read_dir = match std::fs::read_dir(path) {
      Ok(rd) => rd,
      Err(_) => return Ok(()),
    };

    for entry in read_dir.flatten() {
      let child = FileEntry::from_path(entry.path(), depth);
      if !self.show_hidden && child.is_hidden() {
        continue;
      }
      children.push(child);
    }

    // Sort: directories first, then case-insensitive alphabetical
    children.sort_by(|a, b| {
      b.is_dir
        .cmp(&a.is_dir)
        .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });

    mark_git_ignored(self.git_repo.as_ref(), &mut children);
    mark_git_status(&self.git_statuses, &mut children);

    // Insert children at the correct position
    for (i, child) in children.into_iter().enumerate() {
      self.entries.insert(insert_pos + i, child);
    }

    Ok(())
  }

  pub fn toggle_expand(&mut self, index: usize) -> Result<()> {
    if index >= self.entries.len() || !self.entries[index].is_dir {
      return Ok(());
    }

    if self.entries[index].expanded {
      self.collapse(index);
    } else {
      self.expand(index)?;
    }
    Ok(())
  }

  fn expand(&mut self, index: usize) -> Result<()> {
    let path = self.entries[index].path.clone();
    let depth = self.entries[index].depth + 1;
    self.entries[index].expanded = true;

    let mut children = Vec::new();
    let read_dir = match std::fs::read_dir(&path) {
      Ok(rd) => rd,
      Err(_) => return Ok(()),
    };

    for entry in read_dir.flatten() {
      let child = FileEntry::from_path(entry.path(), depth);
      if !self.show_hidden && child.is_hidden() {
        continue;
      }
      children.push(child);
    }

    children.sort_by(|a, b| {
      b.is_dir
        .cmp(&a.is_dir)
        .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });

    mark_git_ignored(self.git_repo.as_ref(), &mut children);
    mark_git_status(&self.git_statuses, &mut children);

    for (i, child) in children.into_iter().enumerate() {
      self.entries.insert(index + 1 + i, child);
    }

    propagate_git_status(&mut self.entries);

    Ok(())
  }

  fn collapse(&mut self, index: usize) {
    self.entries[index].expanded = false;
    let depth = self.entries[index].depth;

    // Remove all entries with depth > parent's depth that follow it
    let mut remove_count = 0;
    for entry in &self.entries[index + 1..] {
      if entry.depth > depth {
        remove_count += 1;
      } else {
        break;
      }
    }

    self.entries.drain(index + 1..index + 1 + remove_count);
  }

  pub fn toggle_hidden(&mut self) -> Result<()> {
    self.show_hidden = !self.show_hidden;
    self.reload()
  }

  pub fn reload(&mut self) -> Result<()> {
    // Re-query git status
    self.git_repo = GitRepo::open(&self.root);
    let (statuses, info) = self
      .git_repo
      .as_ref()
      .map(|r| r.get_file_statuses())
      .unwrap_or_default();
    self.git_statuses = statuses;
    self.git_info = info;

    // Remember expanded dirs
    let expanded: Vec<PathBuf> = self
      .entries
      .iter()
      .filter(|e| e.expanded)
      .map(|e| e.path.clone())
      .collect();

    let root = self.root.clone();
    self.load_dir(&root, 0)?;

    // Re-expand previously expanded dirs
    let mut i = 0;
    while i < self.entries.len() {
      if self.entries[i].is_dir && expanded.contains(&self.entries[i].path) {
        self.expand(i)?;
      }
      i += 1;
    }

    propagate_git_status(&mut self.entries);

    Ok(())
  }

  pub fn enter_dir(&mut self, index: usize) -> Result<()> {
    if index >= self.entries.len() || !self.entries[index].is_dir {
      return Ok(());
    }
    let path = self.entries[index].path.clone();
    self.root = path;
    self.git_repo = GitRepo::open(&self.root);
    let (statuses, info) = self
      .git_repo
      .as_ref()
      .map(|r| r.get_file_statuses())
      .unwrap_or_default();
    self.git_statuses = statuses;
    self.git_info = info;
    let root = self.root.clone();
    self.entries.clear();
    self.load_dir(&root, 0)?;
    propagate_git_status(&mut self.entries);
    Ok(())
  }

  pub fn navigate_to(&mut self, path: &Path) -> Result<()> {
    self.root = path.to_path_buf();
    self.git_repo = GitRepo::open(&self.root);
    let (statuses, info) = self
      .git_repo
      .as_ref()
      .map(|r| r.get_file_statuses())
      .unwrap_or_default();
    self.git_statuses = statuses;
    self.git_info = info;
    let root = self.root.clone();
    self.entries.clear();
    self.load_dir(&root, 0)?;
    propagate_git_status(&mut self.entries);
    Ok(())
  }

  /// Find the index of the parent directory for an entry at the given index.
  pub fn find_parent_index(&self, index: usize) -> Option<usize> {
    if index >= self.entries.len() {
      return None;
    }
    let target_depth = self.entries[index].depth;
    if target_depth == 0 {
      return None;
    }
    (0..index)
      .rev()
      .find(|&i| self.entries[i].is_dir && self.entries[i].depth == target_depth - 1)
  }

  pub fn go_parent(&mut self) -> Result<Option<PathBuf>> {
    if let Some(parent) = self.root.parent().map(|p| p.to_path_buf()) {
      let old_root = self.root.clone();

      // Remember expanded dirs - they'll be re-expanded after we go up
      let mut expanded: Vec<PathBuf> = self
        .entries
        .iter()
        .filter(|e| e.expanded)
        .map(|e| e.path.clone())
        .collect();
      // The old root itself should be expanded when we go up
      expanded.push(old_root.clone());

      self.root = parent;
      self.git_repo = GitRepo::open(&self.root);
      let (statuses, info) = self
        .git_repo
        .as_ref()
        .map(|r| r.get_file_statuses())
        .unwrap_or_default();
      self.git_statuses = statuses;
      self.git_info = info;
      let root = self.root.clone();
      self.entries.clear();
      self.load_dir(&root, 0)?;

      // Re-expand old root and all previously expanded dirs
      let mut i = 0;
      while i < self.entries.len() {
        if self.entries[i].is_dir && expanded.contains(&self.entries[i].path) {
          self.expand(i)?;
        }
        i += 1;
      }

      propagate_git_status(&mut self.entries);
      Ok(Some(old_root))
    } else {
      Ok(None)
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::fs;
  use std::sync::atomic::{AtomicU32, Ordering};

  use crate::fs::entry::GitFileStatus;
  static COUNTER: AtomicU32 = AtomicU32::new(0);

  fn setup_test_dir() -> PathBuf {
    let id = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!("tui_tree_{id}_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("alpha_dir")).unwrap();
    fs::create_dir_all(dir.join("beta_dir")).unwrap();
    fs::write(dir.join("charlie.txt"), "hello").unwrap();
    fs::write(dir.join("delta.rs"), "fn main() {}").unwrap();
    fs::write(dir.join(".hidden_file"), "secret").unwrap();
    fs::write(dir.join("alpha_dir").join("inner.txt"), "nested").unwrap();
    dir
  }

  fn cleanup(dir: &PathBuf) {
    let _ = fs::remove_dir_all(dir);
  }

  #[test]
  fn test_new_tree_loads_entries() {
    let dir = setup_test_dir();
    let tree = FileTree::new(dir.clone()).unwrap();
    // Should have 4 entries (hidden excluded by default)
    assert_eq!(tree.entries.len(), 4);
    cleanup(&dir);
  }

  #[test]
  fn test_dirs_come_first() {
    let dir = setup_test_dir();
    let tree = FileTree::new(dir.clone()).unwrap();
    assert!(tree.entries[0].is_dir);
    assert!(tree.entries[1].is_dir);
    assert!(!tree.entries[2].is_dir);
    assert!(!tree.entries[3].is_dir);
    cleanup(&dir);
  }

  #[test]
  fn test_case_insensitive_sort() {
    let dir = setup_test_dir();
    let tree = FileTree::new(dir.clone()).unwrap();
    assert_eq!(tree.entries[0].name, "alpha_dir");
    assert_eq!(tree.entries[1].name, "beta_dir");
    assert_eq!(tree.entries[2].name, "charlie.txt");
    assert_eq!(tree.entries[3].name, "delta.rs");
    cleanup(&dir);
  }

  #[test]
  fn test_hidden_files_excluded_by_default() {
    let dir = setup_test_dir();
    let tree = FileTree::new(dir.clone()).unwrap();
    assert!(!tree.entries.iter().any(|e| e.name == ".hidden_file"));
    cleanup(&dir);
  }

  #[test]
  fn test_toggle_hidden_shows_hidden() {
    let dir = setup_test_dir();
    let mut tree = FileTree::new(dir.clone()).unwrap();
    tree.toggle_hidden().unwrap();
    assert!(tree.show_hidden);
    assert!(tree.entries.iter().any(|e| e.name == ".hidden_file"));
    cleanup(&dir);
  }

  #[test]
  fn test_expand_collapse() {
    let dir = setup_test_dir();
    let mut tree = FileTree::new(dir.clone()).unwrap();
    let initial_len = tree.entries.len();

    tree.toggle_expand(0).unwrap();
    assert!(tree.entries[0].expanded);
    assert!(tree.entries.len() > initial_len);
    assert_eq!(tree.entries[1].name, "inner.txt");
    assert_eq!(tree.entries[1].depth, 1);

    tree.toggle_expand(0).unwrap();
    assert!(!tree.entries[0].expanded);
    assert_eq!(tree.entries.len(), initial_len);
    cleanup(&dir);
  }

  #[test]
  fn test_toggle_expand_on_file_is_noop() {
    let dir = setup_test_dir();
    let mut tree = FileTree::new(dir.clone()).unwrap();
    let file_idx = tree.entries.iter().position(|e| !e.is_dir).unwrap();
    let len_before = tree.entries.len();
    tree.toggle_expand(file_idx).unwrap();
    assert_eq!(tree.entries.len(), len_before);
    cleanup(&dir);
  }

  #[test]
  fn test_go_parent() {
    let dir = setup_test_dir();
    let child = dir.join("alpha_dir");
    let mut tree = FileTree::new(child.clone()).unwrap();
    assert_eq!(tree.root, child);

    let old = tree.go_parent().unwrap();
    assert_eq!(old, Some(child));
    assert_eq!(tree.root, dir);
    cleanup(&dir);
  }

  #[test]
  fn test_go_parent_preserves_expanded() {
    let dir = setup_test_dir();
    // Start inside alpha_dir which has inner.txt
    let child = dir.join("alpha_dir");
    let mut tree = FileTree::new(child.clone()).unwrap();
    assert_eq!(tree.root, child);

    // Go up to parent
    tree.go_parent().unwrap();
    assert_eq!(tree.root, dir);

    // alpha_dir should now be expanded (since we came from there)
    let alpha = tree.entries.iter().find(|e| e.name == "alpha_dir").unwrap();
    assert!(alpha.expanded);

    // inner.txt should be visible (child of expanded alpha_dir)
    assert!(tree.entries.iter().any(|e| e.name == "inner.txt"));

    cleanup(&dir);
  }

  #[test]
  fn test_enter_dir() {
    let dir = setup_test_dir();
    let mut tree = FileTree::new(dir.clone()).unwrap();
    tree.enter_dir(0).unwrap();
    assert_eq!(tree.root, dir.join("alpha_dir"));
    assert!(tree.entries.iter().any(|e| e.name == "inner.txt"));
    cleanup(&dir);
  }

  #[test]
  fn test_navigate_to() {
    let dir = setup_test_dir();
    let target = dir.join("alpha_dir");
    let mut tree = FileTree::new(dir.clone()).unwrap();
    assert_eq!(tree.root, dir);

    tree.navigate_to(&target).unwrap();
    assert_eq!(tree.root, target);
    assert!(tree.entries.iter().any(|e| e.name == "inner.txt"));
    cleanup(&dir);
  }

  #[test]
  fn test_mark_git_ignored_marks_ignored_files() {
    let dir = std::env::temp_dir().join(format!(
      "tui_tree_gitignore_{}_{}", COUNTER.fetch_add(1, Ordering::SeqCst), std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();

    // Init a git repo
    git2::Repository::init(&dir).unwrap();

    fs::write(dir.join(".gitignore"), "*.log\n").unwrap();
    fs::write(dir.join("foo.log"), "log data").unwrap();
    fs::write(dir.join("bar.txt"), "text data").unwrap();

    let mut children = vec![
      FileEntry::from_path(dir.join("foo.log"), 0),
      FileEntry::from_path(dir.join("bar.txt"), 0),
    ];

    let repo = GitRepo::open(&dir);
    mark_git_ignored(repo.as_ref(), &mut children);

    let foo = children.iter().find(|e| e.name == "foo.log").unwrap();
    let bar = children.iter().find(|e| e.name == "bar.txt").unwrap();
    assert!(foo.is_git_ignored);
    assert!(!bar.is_git_ignored);

    let _ = fs::remove_dir_all(&dir);
  }

  #[test]
  fn test_mark_git_ignored_no_git_repo() {
    let dir = std::env::temp_dir().join(format!(
      "tui_tree_no_git_{}_{}", COUNTER.fetch_add(1, Ordering::SeqCst), std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();

    fs::write(dir.join("foo.log"), "data").unwrap();
    fs::write(dir.join("bar.txt"), "data").unwrap();

    let mut children = vec![
      FileEntry::from_path(dir.join("foo.log"), 0),
      FileEntry::from_path(dir.join("bar.txt"), 0),
    ];

    mark_git_ignored(None, &mut children);

    // No entries should be marked when not in a git repo
    assert!(!children.iter().any(|e| e.is_git_ignored));

    let _ = fs::remove_dir_all(&dir);
  }

  #[test]
  fn test_tree_loads_with_git_ignored_flag() {
    let dir = std::env::temp_dir().join(format!(
      "tui_tree_load_ignored_{}_{}", COUNTER.fetch_add(1, Ordering::SeqCst), std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();

    git2::Repository::init(&dir).unwrap();

    fs::write(dir.join(".gitignore"), "*.log\n").unwrap();
    fs::write(dir.join("ignored.log"), "log").unwrap();
    fs::write(dir.join("visible.txt"), "text").unwrap();

    let mut tree = FileTree::new(dir.clone()).unwrap();
    tree.show_hidden = true;
    tree.reload().unwrap();

    let ignored = tree.entries.iter().find(|e| e.name == "ignored.log").unwrap();
    let visible = tree.entries.iter().find(|e| e.name == "visible.txt").unwrap();
    assert!(ignored.is_git_ignored);
    assert!(!visible.is_git_ignored);

    let _ = fs::remove_dir_all(&dir);
  }

  #[test]
  fn test_reload_preserves_expanded() {
    let dir = setup_test_dir();
    let mut tree = FileTree::new(dir.clone()).unwrap();
    tree.toggle_expand(0).unwrap();
    assert!(tree.entries[0].expanded);

    tree.reload().unwrap();
    assert!(tree.entries[0].expanded);
    assert!(tree.entries.iter().any(|e| e.name == "inner.txt"));
    cleanup(&dir);
  }

  #[test]
  fn test_non_git_dir_all_clean() {
    let dir = std::env::temp_dir().join(format!(
      "tui_tree_nogit_clean_{}_{}", COUNTER.fetch_add(1, Ordering::SeqCst), std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("file.txt"), "data").unwrap();

    let tree = FileTree::new(dir.clone()).unwrap();
    assert!(tree.entries.iter().all(|e| e.git_status.is_clean()));
    assert!(tree.git_info.branch.is_none());

    let _ = fs::remove_dir_all(&dir);
  }

  fn init_git_repo_with_config(dir: &Path) -> git2::Repository {
    let repo = git2::Repository::init(dir).unwrap();
    let mut config = repo.config().unwrap();
    config.set_str("user.email", "test@test.com").unwrap();
    config.set_str("user.name", "Test").unwrap();
    repo
  }

  fn git_add_and_commit(repo: &git2::Repository, paths: &[&str], message: &str) {
    let mut index = repo.index().unwrap();
    for path in paths {
      index.add_path(Path::new(path)).unwrap();
    }
    index.write().unwrap();
    let tree_id = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    let sig = repo.signature().unwrap();

    if let Ok(head) = repo.head() {
      let parent = repo.find_commit(head.target().unwrap()).unwrap();
      repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &[&parent]).unwrap();
    } else {
      repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &[]).unwrap();
    }
  }

  fn git_stage(repo: &git2::Repository, paths: &[&str]) {
    let mut index = repo.index().unwrap();
    for path in paths {
      index.add_path(Path::new(path)).unwrap();
    }
    index.write().unwrap();
  }

  #[test]
  fn test_git_status_on_modified_files() {
    let dir = std::env::temp_dir().join(format!(
      "tui_tree_gitstatus_{}_{}", COUNTER.fetch_add(1, Ordering::SeqCst), std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();

    let repo = init_git_repo_with_config(&dir);

    // Create and commit a file
    fs::write(dir.join("tracked.txt"), "initial").unwrap();
    git_add_and_commit(&repo, &["tracked.txt"], "init");

    // Modify the tracked file
    fs::write(dir.join("tracked.txt"), "modified").unwrap();
    // Create an untracked file
    fs::write(dir.join("untracked.txt"), "new").unwrap();
    // Create a staged file
    fs::write(dir.join("staged.txt"), "staged").unwrap();
    git_stage(&repo, &["staged.txt"]);

    let tree = FileTree::new(dir.clone()).unwrap();

    let tracked = tree.entries.iter().find(|e| e.name == "tracked.txt").unwrap();
    assert!(!tracked.git_status.is_clean());
    assert_eq!(tracked.git_status.unstaged, Some(GitFileStatus::Modified));

    let untracked = tree.entries.iter().find(|e| e.name == "untracked.txt").unwrap();
    assert_eq!(untracked.git_status.unstaged, Some(GitFileStatus::Untracked));

    let staged = tree.entries.iter().find(|e| e.name == "staged.txt").unwrap();
    assert_eq!(staged.git_status.staged, Some(GitFileStatus::Added));

    // Branch should be set
    assert!(tree.git_info.branch.is_some());

    let _ = fs::remove_dir_all(&dir);
  }

  #[test]
  fn test_git_status_parent_propagation() {
    let dir = std::env::temp_dir().join(format!(
      "tui_tree_gitprop_{}_{}", COUNTER.fetch_add(1, Ordering::SeqCst), std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("subdir")).unwrap();

    git2::Repository::init(&dir).unwrap();

    // Create untracked file inside subdir
    fs::write(dir.join("subdir").join("new.txt"), "new").unwrap();

    let mut tree = FileTree::new(dir.clone()).unwrap();
    // Expand subdir
    let subdir_idx = tree.entries.iter().position(|e| e.name == "subdir").unwrap();
    tree.toggle_expand(subdir_idx).unwrap();

    // Parent dir should have propagated status
    let subdir_entry = &tree.entries[subdir_idx];
    assert!(!subdir_entry.git_status.is_clean());

    let _ = fs::remove_dir_all(&dir);
  }

  #[test]
  fn test_reload_refreshes_git_status() {
    let dir = std::env::temp_dir().join(format!(
      "tui_tree_gitreload_{}_{}", COUNTER.fetch_add(1, Ordering::SeqCst), std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();

    let repo = init_git_repo_with_config(&dir);

    // Create and commit
    fs::write(dir.join("file.txt"), "initial").unwrap();
    git_add_and_commit(&repo, &["file.txt"], "init");

    let mut tree = FileTree::new(dir.clone()).unwrap();
    let file = tree.entries.iter().find(|e| e.name == "file.txt").unwrap();
    assert!(file.git_status.is_clean());

    // Modify file and reload
    fs::write(dir.join("file.txt"), "changed").unwrap();
    tree.reload().unwrap();

    let file = tree.entries.iter().find(|e| e.name == "file.txt").unwrap();
    assert!(!file.git_status.is_clean());
    assert_eq!(file.git_status.unstaged, Some(GitFileStatus::Modified));

    let _ = fs::remove_dir_all(&dir);
  }

  #[test]
  fn test_find_parent_index() {
    let dir = setup_test_dir();
    let mut tree = FileTree::new(dir.clone()).unwrap();

    // Expand alpha_dir (index 0)
    tree.toggle_expand(0).unwrap();
    // Tree structure now:
    // 0: alpha_dir (depth 0)
    // 1: inner.txt (depth 1)
    // 2: beta_dir (depth 0)
    // 3: charlie.txt (depth 0)
    // 4: delta.rs (depth 0)

    // inner.txt at index 1 should have parent alpha_dir at index 0
    assert_eq!(tree.find_parent_index(1), Some(0));

    // Root-level items should return None
    assert_eq!(tree.find_parent_index(0), None);
    assert_eq!(tree.find_parent_index(2), None);
    assert_eq!(tree.find_parent_index(3), None);

    // Out of bounds should return None
    assert_eq!(tree.find_parent_index(100), None);

    cleanup(&dir);
  }
}
