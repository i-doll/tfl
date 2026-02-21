use std::path::PathBuf;

use ratatui::style::Color;

use crate::theme::Theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GitFileStatus {
  Modified,
  Added,
  Deleted,
  Renamed,
  Untracked,
  Conflicted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct GitStatus {
  pub staged: Option<GitFileStatus>,
  pub unstaged: Option<GitFileStatus>,
}

impl GitStatus {
  pub fn is_clean(&self) -> bool {
    self.staged.is_none() && self.unstaged.is_none()
  }

  pub fn display_color(&self, theme: &Theme) -> Option<Color> {
    // Conflicted
    if self.staged == Some(GitFileStatus::Conflicted)
      || self.unstaged == Some(GitFileStatus::Conflicted)
    {
      return Some(theme.git_conflicted);
    }
    // Untracked
    if self.staged == Some(GitFileStatus::Untracked)
      || self.unstaged == Some(GitFileStatus::Untracked)
    {
      return Some(theme.git_untracked);
    }
    // Unstaged changes
    if self.unstaged.is_some() {
      return Some(theme.git_modified);
    }
    // Staged only
    if self.staged.is_some() {
      return Some(theme.git_staged);
    }
    None
  }

  /// Merge another status into this one, keeping the highest severity.
  pub fn merge(&mut self, other: &GitStatus) {
    if self.staged.is_none() || severity(other.staged) > severity(self.staged) {
      self.staged = other.staged;
    }
    if self.unstaged.is_none() || severity(other.unstaged) > severity(self.unstaged) {
      self.unstaged = other.unstaged;
    }
  }
}

fn severity(status: Option<GitFileStatus>) -> u8 {
  match status {
    None => 0,
    Some(GitFileStatus::Added) | Some(GitFileStatus::Renamed) => 1,
    Some(GitFileStatus::Deleted) => 2,
    Some(GitFileStatus::Modified) => 3,
    Some(GitFileStatus::Untracked) => 4,
    Some(GitFileStatus::Conflicted) => 5,
  }
}

#[derive(Debug, Clone)]
pub struct FileEntry {
  pub path: PathBuf,
  pub name: String,
  pub depth: usize,
  pub is_dir: bool,
  pub is_symlink: bool,
  pub symlink_target: Option<String>,
  pub expanded: bool,
  pub size: u64,
  pub is_git_ignored: bool,
  pub git_status: GitStatus,
}

impl FileEntry {
  #[allow(dead_code)] // Used in tests
  pub fn from_path(path: PathBuf, depth: usize) -> Self {
    let metadata = path.symlink_metadata();
    let is_symlink = metadata.as_ref().is_ok_and(|m| m.is_symlink());
    let symlink_target = if is_symlink {
      std::fs::read_link(&path)
        .ok()
        .map(|t| t.to_string_lossy().to_string())
    } else {
      None
    };
    let metadata = path.metadata();
    let is_dir = metadata.as_ref().is_ok_and(|m| m.is_dir());
    let size = metadata.as_ref().map_or(0, |m| m.len());
    let name = path
      .file_name()
      .map(|n| n.to_string_lossy().to_string())
      .unwrap_or_default();

    Self {
      path,
      name,
      depth,
      is_dir,
      is_symlink,
      symlink_target,
      expanded: false,
      size,
      is_git_ignored: false,
      git_status: GitStatus::default(),
    }
  }

  pub fn from_dir_entry(dir_entry: std::fs::DirEntry, depth: usize) -> Self {
    let path = dir_entry.path();
    let name = dir_entry.file_name().to_string_lossy().to_string();
    let file_type = dir_entry.file_type().ok();
    let is_symlink = file_type.as_ref().is_some_and(|ft| ft.is_symlink());
    let symlink_target = if is_symlink {
      std::fs::read_link(&path)
        .ok()
        .map(|t| t.to_string_lossy().to_string())
    } else {
      None
    };
    let (is_dir, size) = if is_symlink {
      let meta = path.metadata().ok();
      (
        meta.as_ref().is_some_and(|m| m.is_dir()),
        meta.as_ref().map_or(0, |m| m.len()),
      )
    } else {
      let is_dir = file_type.as_ref().is_some_and(|ft| ft.is_dir());
      let size = if is_dir {
        0
      } else {
        dir_entry.metadata().ok().map_or(0, |m| m.len())
      };
      (is_dir, size)
    };

    Self {
      path,
      name,
      depth,
      is_dir,
      is_symlink,
      symlink_target,
      expanded: false,
      size,
      is_git_ignored: false,
      git_status: GitStatus::default(),
    }
  }

  pub fn is_hidden(&self) -> bool {
    self.name.starts_with('.')
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::fs;

  #[test]
  fn test_from_path_file() {
    let dir = std::env::temp_dir().join("tui_explorer_test_entry");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let file = dir.join("test.txt");
    fs::write(&file, "hello").unwrap();

    let entry = FileEntry::from_path(file.clone(), 0);
    assert_eq!(entry.name, "test.txt");
    assert_eq!(entry.depth, 0);
    assert!(!entry.is_dir);
    assert!(!entry.is_symlink);
    assert!(!entry.expanded);
    assert_eq!(entry.size, 5);
    assert_eq!(entry.path, file);

    let _ = fs::remove_dir_all(&dir);
  }

  #[test]
  fn test_from_path_dir() {
    let dir = std::env::temp_dir().join("tui_explorer_test_entry_dir");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();

    let entry = FileEntry::from_path(dir.clone(), 2);
    assert!(entry.is_dir);
    assert_eq!(entry.depth, 2);

    let _ = fs::remove_dir_all(&dir);
  }

  #[test]
  fn test_is_hidden() {
    let entry = FileEntry {
      path: PathBuf::from(".gitignore"),
      name: ".gitignore".to_string(),
      depth: 0,
      is_dir: false,
      is_symlink: false,
      symlink_target: None,
      expanded: false,
      size: 0,
      is_git_ignored: false,
      git_status: GitStatus::default(),
    };
    assert!(entry.is_hidden());

    let entry = FileEntry {
      path: PathBuf::from("README.md"),
      name: "README.md".to_string(),
      depth: 0,
      is_dir: false,
      is_symlink: false,
      symlink_target: None,
      expanded: false,
      size: 0,
      is_git_ignored: false,
      git_status: GitStatus::default(),
    };
    assert!(!entry.is_hidden());
  }

  #[test]
  fn test_symlink_target_resolved() {
    let dir = std::env::temp_dir().join("tfl_test_symlink_target");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let file = dir.join("real.txt");
    fs::write(&file, "content").unwrap();
    let link = dir.join("link.txt");
    std::os::unix::fs::symlink(&file, &link).unwrap();

    let entry = FileEntry::from_path(link.clone(), 0);
    assert!(entry.is_symlink);
    assert_eq!(entry.symlink_target, Some(file.to_string_lossy().to_string()));

    let _ = fs::remove_dir_all(&dir);
  }

  #[test]
  fn test_non_symlink_has_no_target() {
    let dir = std::env::temp_dir().join("tfl_test_no_symlink_target");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let file = dir.join("regular.txt");
    fs::write(&file, "content").unwrap();

    let entry = FileEntry::from_path(file.clone(), 0);
    assert!(!entry.is_symlink);
    assert_eq!(entry.symlink_target, None);

    let _ = fs::remove_dir_all(&dir);
  }

  #[test]
  fn test_entry_default_not_git_ignored() {
    let dir = std::env::temp_dir().join("tui_explorer_test_not_ignored");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let file = dir.join("test.txt");
    fs::write(&file, "hello").unwrap();

    let entry = FileEntry::from_path(file, 0);
    assert!(!entry.is_git_ignored);

    let _ = fs::remove_dir_all(&dir);
  }

  #[test]
  fn test_from_nonexistent_path() {
    let entry = FileEntry::from_path(PathBuf::from("/nonexistent/file.txt"), 0);
    assert_eq!(entry.name, "file.txt");
    assert!(!entry.is_dir);
    assert!(!entry.is_symlink);
    assert_eq!(entry.size, 0);
  }

  #[test]
  fn test_git_status_default_is_clean() {
    let theme = Theme::dark();
    let status = GitStatus::default();
    assert!(status.is_clean());
    assert_eq!(status.display_color(&theme), None);
  }

  #[test]
  fn test_git_status_staged_not_clean() {
    let status = GitStatus {
      staged: Some(GitFileStatus::Added),
      unstaged: None,
    };
    assert!(!status.is_clean());
  }

  #[test]
  fn test_git_status_unstaged_not_clean() {
    let status = GitStatus {
      staged: None,
      unstaged: Some(GitFileStatus::Modified),
    };
    assert!(!status.is_clean());
  }

  #[test]
  fn test_display_color_staged_only() {
    let theme = Theme::dark();
    let status = GitStatus {
      staged: Some(GitFileStatus::Added),
      unstaged: None,
    };
    assert_eq!(status.display_color(&theme), Some(theme.git_staged));
  }

  #[test]
  fn test_display_color_unstaged() {
    let theme = Theme::dark();
    let status = GitStatus {
      staged: None,
      unstaged: Some(GitFileStatus::Modified),
    };
    assert_eq!(status.display_color(&theme), Some(theme.git_modified));
  }

  #[test]
  fn test_display_color_untracked() {
    let theme = Theme::dark();
    let status = GitStatus {
      staged: None,
      unstaged: Some(GitFileStatus::Untracked),
    };
    assert_eq!(status.display_color(&theme), Some(theme.git_untracked));
  }

  #[test]
  fn test_display_color_conflicted() {
    let theme = Theme::dark();
    let status = GitStatus {
      staged: Some(GitFileStatus::Conflicted),
      unstaged: None,
    };
    assert_eq!(status.display_color(&theme), Some(theme.git_conflicted));
  }

  #[test]
  fn test_display_color_mixed_is_modified() {
    let theme = Theme::dark();
    let status = GitStatus {
      staged: Some(GitFileStatus::Added),
      unstaged: Some(GitFileStatus::Modified),
    };
    assert_eq!(status.display_color(&theme), Some(theme.git_modified));
  }

  #[test]
  fn test_git_status_merge() {
    let mut parent = GitStatus::default();
    let child = GitStatus {
      staged: Some(GitFileStatus::Modified),
      unstaged: None,
    };
    parent.merge(&child);
    assert_eq!(parent.staged, Some(GitFileStatus::Modified));

    // Higher severity wins
    let child2 = GitStatus {
      staged: Some(GitFileStatus::Conflicted),
      unstaged: None,
    };
    parent.merge(&child2);
    assert_eq!(parent.staged, Some(GitFileStatus::Conflicted));
  }
}
