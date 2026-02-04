use std::path::{Path, PathBuf};

use anyhow::Result;

pub struct Favorites {
  path: PathBuf,
  entries: Vec<PathBuf>,
}

impl Favorites {
  pub fn load() -> Self {
    Self::load_from(Self::favorites_path())
  }

  pub fn load_from(path: PathBuf) -> Self {
    let entries = if path.exists() {
      std::fs::read_to_string(&path)
        .unwrap_or_default()
        .lines()
        .filter(|l| !l.is_empty())
        .map(PathBuf::from)
        .collect()
    } else {
      Vec::new()
    };
    Self { path, entries }
  }

  pub fn save(&self) -> Result<()> {
    if let Some(parent) = self.path.parent() {
      std::fs::create_dir_all(parent)?;
    }
    let content: String = self
      .entries
      .iter()
      .map(|p| p.to_string_lossy().to_string())
      .collect::<Vec<_>>()
      .join("\n");
    std::fs::write(&self.path, content)?;
    Ok(())
  }

  pub fn add(&mut self, path: PathBuf) {
    if !self.entries.contains(&path) {
      self.entries.push(path);
    }
  }

  pub fn remove(&mut self, index: usize) {
    if index < self.entries.len() {
      self.entries.remove(index);
    }
  }

  pub fn get(&self, index: usize) -> Option<&Path> {
    self.entries.get(index).map(|p| p.as_path())
  }

  pub fn list(&self) -> &[PathBuf] {
    &self.entries
  }

  pub fn len(&self) -> usize {
    self.entries.len()
  }

  pub fn contains(&self, path: &Path) -> bool {
    self.entries.iter().any(|p| p == path)
  }

  fn favorites_path() -> PathBuf {
    dirs::config_dir()
      .unwrap_or_else(|| PathBuf::from("."))
      .join("tfl")
      .join("favorites")
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::sync::atomic::{AtomicU32, Ordering};

  static COUNTER: AtomicU32 = AtomicU32::new(0);

  fn temp_path() -> PathBuf {
    let id = COUNTER.fetch_add(1, Ordering::SeqCst);
    std::env::temp_dir().join(format!("tfl_favorites_test_{id}_{}", std::process::id()))
  }

  #[test]
  fn test_empty_favorites() {
    let path = temp_path();
    let favs = Favorites::load_from(path.clone());
    assert!(favs.list().is_empty());
    assert_eq!(favs.len(), 0);
    let _ = std::fs::remove_file(&path);
  }

  #[test]
  fn test_add_and_list() {
    let path = temp_path();
    let mut favs = Favorites::load_from(path.clone());
    favs.add(PathBuf::from("/home/user/projects"));
    favs.add(PathBuf::from("/home/user/documents"));
    assert_eq!(favs.len(), 2);
    assert_eq!(favs.list()[0], PathBuf::from("/home/user/projects"));
    assert_eq!(favs.list()[1], PathBuf::from("/home/user/documents"));
    let _ = std::fs::remove_file(&path);
  }

  #[test]
  fn test_add_dedup() {
    let path = temp_path();
    let mut favs = Favorites::load_from(path.clone());
    favs.add(PathBuf::from("/home/user/projects"));
    favs.add(PathBuf::from("/home/user/projects"));
    assert_eq!(favs.len(), 1);
    let _ = std::fs::remove_file(&path);
  }

  #[test]
  fn test_remove() {
    let path = temp_path();
    let mut favs = Favorites::load_from(path.clone());
    favs.add(PathBuf::from("/a"));
    favs.add(PathBuf::from("/b"));
    favs.add(PathBuf::from("/c"));
    favs.remove(1);
    assert_eq!(favs.len(), 2);
    assert_eq!(favs.list()[0], PathBuf::from("/a"));
    assert_eq!(favs.list()[1], PathBuf::from("/c"));
    let _ = std::fs::remove_file(&path);
  }

  #[test]
  fn test_contains() {
    let path = temp_path();
    let mut favs = Favorites::load_from(path.clone());
    favs.add(PathBuf::from("/home/user/projects"));
    assert!(favs.contains(Path::new("/home/user/projects")));
    assert!(!favs.contains(Path::new("/home/user/other")));
    let _ = std::fs::remove_file(&path);
  }

  #[test]
  fn test_save_and_load() {
    let path = temp_path();
    let mut favs = Favorites::load_from(path.clone());
    favs.add(PathBuf::from("/home/user/projects"));
    favs.add(PathBuf::from("/home/user/documents"));
    favs.save().unwrap();

    let loaded = Favorites::load_from(path.clone());
    assert_eq!(loaded.len(), 2);
    assert_eq!(loaded.list()[0], PathBuf::from("/home/user/projects"));
    assert_eq!(loaded.list()[1], PathBuf::from("/home/user/documents"));
    let _ = std::fs::remove_file(&path);
  }

  #[test]
  fn test_get_by_index() {
    let path = temp_path();
    let mut favs = Favorites::load_from(path.clone());
    favs.add(PathBuf::from("/a"));
    favs.add(PathBuf::from("/b"));
    assert_eq!(favs.get(0), Some(Path::new("/a")));
    assert_eq!(favs.get(1), Some(Path::new("/b")));
    assert_eq!(favs.get(2), None);
    let _ = std::fs::remove_file(&path);
  }
}
