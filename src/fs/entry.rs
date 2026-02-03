use std::path::PathBuf;

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
}

impl FileEntry {
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
  fn test_from_nonexistent_path() {
    let entry = FileEntry::from_path(PathBuf::from("/nonexistent/file.txt"), 0);
    assert_eq!(entry.name, "file.txt");
    assert!(!entry.is_dir);
    assert!(!entry.is_symlink);
    assert_eq!(entry.size, 0);
  }
}
