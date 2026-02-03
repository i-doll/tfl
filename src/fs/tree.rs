use std::collections::HashSet;
use std::path::{Path, PathBuf};

use anyhow::Result;

use super::entry::FileEntry;

fn mark_git_ignored(dir: &Path, children: &mut [FileEntry]) {
  if children.is_empty() {
    return;
  }
  let output = std::process::Command::new("git")
    .arg("check-ignore")
    .args(children.iter().map(|c| &c.path))
    .current_dir(dir)
    .stdout(std::process::Stdio::piped())
    .stderr(std::process::Stdio::null())
    .output();
  let Ok(output) = output else { return };
  let ignored: HashSet<PathBuf> = String::from_utf8_lossy(&output.stdout)
    .lines()
    .map(PathBuf::from)
    .collect();
  for child in children.iter_mut() {
    child.is_git_ignored = ignored.contains(&child.path);
  }
}

#[derive(Debug)]
pub struct FileTree {
  pub root: PathBuf,
  pub entries: Vec<FileEntry>,
  pub show_hidden: bool,
}

impl FileTree {
  pub fn new(root: PathBuf) -> Result<Self> {
    let mut tree = Self {
      root: root.clone(),
      entries: Vec::new(),
      show_hidden: false,
    };
    tree.load_dir(&root, 0)?;
    Ok(tree)
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

    mark_git_ignored(path, &mut children);

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

    mark_git_ignored(&path, &mut children);

    for (i, child) in children.into_iter().enumerate() {
      self.entries.insert(index + 1 + i, child);
    }

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

    Ok(())
  }

  pub fn enter_dir(&mut self, index: usize) -> Result<()> {
    if index >= self.entries.len() || !self.entries[index].is_dir {
      return Ok(());
    }
    let path = self.entries[index].path.clone();
    self.root = path;
    let root = self.root.clone();
    self.entries.clear();
    self.load_dir(&root, 0)?;
    Ok(())
  }

  pub fn go_parent(&mut self) -> Result<Option<PathBuf>> {
    if let Some(parent) = self.root.parent().map(|p| p.to_path_buf()) {
      let old_root = self.root.clone();
      self.root = parent;
      let root = self.root.clone();
      self.entries.clear();
      self.load_dir(&root, 0)?;
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
  fn test_enter_dir() {
    let dir = setup_test_dir();
    let mut tree = FileTree::new(dir.clone()).unwrap();
    tree.enter_dir(0).unwrap();
    assert_eq!(tree.root, dir.join("alpha_dir"));
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
    std::process::Command::new("git")
      .args(["init"])
      .current_dir(&dir)
      .output()
      .unwrap();

    fs::write(dir.join(".gitignore"), "*.log\n").unwrap();
    fs::write(dir.join("foo.log"), "log data").unwrap();
    fs::write(dir.join("bar.txt"), "text data").unwrap();

    let mut children = vec![
      FileEntry::from_path(dir.join("foo.log"), 0),
      FileEntry::from_path(dir.join("bar.txt"), 0),
    ];

    mark_git_ignored(&dir, &mut children);

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

    mark_git_ignored(&dir, &mut children);

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

    std::process::Command::new("git")
      .args(["init"])
      .current_dir(&dir)
      .output()
      .unwrap();

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
}
