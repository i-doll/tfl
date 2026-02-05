use std::path::PathBuf;

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// A saved search with name, pattern, and optional filters
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedSearch {
  pub name: String,
  pub pattern: String,
  #[serde(default)]
  pub regex: bool,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub size: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub date: Option<String>,
}

impl SavedSearch {
  pub fn new(name: impl Into<String>, pattern: impl Into<String>) -> Self {
    Self {
      name: name.into(),
      pattern: pattern.into(),
      regex: false,
      size: None,
      date: None,
    }
  }

  #[allow(dead_code)]
  pub fn with_size(mut self, size: impl Into<String>) -> Self {
    self.size = Some(size.into());
    self
  }

  #[allow(dead_code)]
  pub fn with_date(mut self, date: impl Into<String>) -> Self {
    self.date = Some(date.into());
    self
  }

  #[allow(dead_code)]
  pub fn with_regex(mut self, regex: bool) -> Self {
    self.regex = regex;
    self
  }
}

/// Container for saved searches, handles persistence
pub struct SavedSearches {
  path: PathBuf,
  entries: Vec<SavedSearch>,
}

#[derive(Serialize, Deserialize, Default)]
struct SearchesFile {
  #[serde(default, rename = "search")]
  searches: Vec<SavedSearch>,
}

impl SavedSearches {
  pub fn load() -> Self {
    Self::load_from(Self::searches_path())
  }

  pub fn load_from(path: PathBuf) -> Self {
    let entries = if path.exists() {
      std::fs::read_to_string(&path)
        .ok()
        .and_then(|content| toml::from_str::<SearchesFile>(&content).ok())
        .map(|f| f.searches)
        .unwrap_or_default()
    } else {
      Vec::new()
    };
    Self { path, entries }
  }

  pub fn save(&self) -> Result<()> {
    if let Some(parent) = self.path.parent() {
      std::fs::create_dir_all(parent)?;
    }
    let file = SearchesFile { searches: self.entries.clone() };
    let content = toml::to_string_pretty(&file)?;
    std::fs::write(&self.path, content)?;
    Ok(())
  }

  pub fn add(&mut self, search: SavedSearch) {
    // Replace existing search with same name
    if let Some(pos) = self.entries.iter().position(|s| s.name == search.name) {
      self.entries[pos] = search;
    } else {
      self.entries.push(search);
    }
  }

  pub fn remove(&mut self, index: usize) {
    if index < self.entries.len() {
      self.entries.remove(index);
    }
  }

  pub fn get(&self, index: usize) -> Option<&SavedSearch> {
    self.entries.get(index)
  }

  #[allow(dead_code)]
  pub fn get_by_name(&self, name: &str) -> Option<&SavedSearch> {
    self.entries.iter().find(|s| s.name == name)
  }

  #[allow(dead_code)]
  pub fn list(&self) -> &[SavedSearch] {
    &self.entries
  }

  pub fn len(&self) -> usize {
    self.entries.len()
  }

  pub fn is_empty(&self) -> bool {
    self.entries.is_empty()
  }

  #[allow(dead_code)]
  pub fn contains(&self, name: &str) -> bool {
    self.entries.iter().any(|s| s.name == name)
  }

  fn searches_path() -> PathBuf {
    dirs::config_dir()
      .unwrap_or_else(|| PathBuf::from("."))
      .join("tfl")
      .join("searches.toml")
  }

  /// Export searches to a TOML string
  #[allow(dead_code)]
  pub fn export(&self) -> Result<String> {
    let file = SearchesFile { searches: self.entries.clone() };
    Ok(toml::to_string_pretty(&file)?)
  }

  /// Import searches from a TOML string, merging with existing
  #[allow(dead_code)]
  pub fn import(&mut self, content: &str) -> Result<usize> {
    let file: SearchesFile = toml::from_str(content)?;
    let count = file.searches.len();
    for search in file.searches {
      self.add(search);
    }
    Ok(count)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::sync::atomic::{AtomicU32, Ordering};

  static COUNTER: AtomicU32 = AtomicU32::new(0);

  fn temp_path() -> PathBuf {
    let id = COUNTER.fetch_add(1, Ordering::SeqCst);
    std::env::temp_dir().join(format!("tfl_searches_test_{id}_{}", std::process::id()))
  }

  #[test]
  fn test_empty_searches() {
    let path = temp_path();
    let searches = SavedSearches::load_from(path.clone());
    assert!(searches.list().is_empty());
    assert_eq!(searches.len(), 0);
    assert!(searches.is_empty());
    let _ = std::fs::remove_file(&path);
  }

  #[test]
  fn test_add_and_list() {
    let path = temp_path();
    let mut searches = SavedSearches::load_from(path.clone());
    searches.add(SavedSearch::new("Large files", "*"));
    searches.add(SavedSearch::new("Rust code", "*.rs"));
    assert_eq!(searches.len(), 2);
    assert_eq!(searches.list()[0].name, "Large files");
    assert_eq!(searches.list()[1].name, "Rust code");
    let _ = std::fs::remove_file(&path);
  }

  #[test]
  fn test_add_replaces_duplicate_name() {
    let path = temp_path();
    let mut searches = SavedSearches::load_from(path.clone());
    searches.add(SavedSearch::new("Test", "old_pattern"));
    searches.add(SavedSearch::new("Test", "new_pattern"));
    assert_eq!(searches.len(), 1);
    assert_eq!(searches.list()[0].pattern, "new_pattern");
    let _ = std::fs::remove_file(&path);
  }

  #[test]
  fn test_remove() {
    let path = temp_path();
    let mut searches = SavedSearches::load_from(path.clone());
    searches.add(SavedSearch::new("A", "*"));
    searches.add(SavedSearch::new("B", "*"));
    searches.add(SavedSearch::new("C", "*"));
    searches.remove(1);
    assert_eq!(searches.len(), 2);
    assert_eq!(searches.list()[0].name, "A");
    assert_eq!(searches.list()[1].name, "C");
    let _ = std::fs::remove_file(&path);
  }

  #[test]
  fn test_contains() {
    let path = temp_path();
    let mut searches = SavedSearches::load_from(path.clone());
    searches.add(SavedSearch::new("Test", "*"));
    assert!(searches.contains("Test"));
    assert!(!searches.contains("Other"));
    let _ = std::fs::remove_file(&path);
  }

  #[test]
  fn test_get_by_name() {
    let path = temp_path();
    let mut searches = SavedSearches::load_from(path.clone());
    searches.add(SavedSearch::new("Large files", "*").with_size(">100M"));
    let search = searches.get_by_name("Large files");
    assert!(search.is_some());
    assert_eq!(search.unwrap().size, Some(">100M".to_string()));
    assert!(searches.get_by_name("Nonexistent").is_none());
    let _ = std::fs::remove_file(&path);
  }

  #[test]
  fn test_save_and_load() {
    let path = temp_path();
    let mut searches = SavedSearches::load_from(path.clone());
    searches.add(SavedSearch::new("Large files", "*").with_size(">100M"));
    searches.add(SavedSearch::new("Recent code", "*.rs").with_date("7d"));
    searches.save().unwrap();

    let loaded = SavedSearches::load_from(path.clone());
    assert_eq!(loaded.len(), 2);
    assert_eq!(loaded.list()[0].name, "Large files");
    assert_eq!(loaded.list()[0].size, Some(">100M".to_string()));
    assert_eq!(loaded.list()[1].name, "Recent code");
    assert_eq!(loaded.list()[1].date, Some("7d".to_string()));
    let _ = std::fs::remove_file(&path);
  }

  #[test]
  fn test_get_by_index() {
    let path = temp_path();
    let mut searches = SavedSearches::load_from(path.clone());
    searches.add(SavedSearch::new("A", "*"));
    searches.add(SavedSearch::new("B", "*"));
    assert_eq!(searches.get(0).map(|s| &s.name), Some(&"A".to_string()));
    assert_eq!(searches.get(1).map(|s| &s.name), Some(&"B".to_string()));
    assert!(searches.get(2).is_none());
    let _ = std::fs::remove_file(&path);
  }

  #[test]
  fn test_saved_search_builders() {
    let search = SavedSearch::new("Test", "*.rs")
      .with_regex(true)
      .with_size(">1M")
      .with_date("30d");

    assert_eq!(search.name, "Test");
    assert_eq!(search.pattern, "*.rs");
    assert!(search.regex);
    assert_eq!(search.size, Some(">1M".to_string()));
    assert_eq!(search.date, Some("30d".to_string()));
  }

  #[test]
  fn test_toml_format() {
    let path = temp_path();
    let mut searches = SavedSearches::load_from(path.clone());
    searches.add(SavedSearch::new("Large files", "*").with_size(">100M"));
    searches.save().unwrap();

    let content = std::fs::read_to_string(&path).unwrap();
    assert!(content.contains("[[search]]"));
    assert!(content.contains("name = \"Large files\""));
    assert!(content.contains("pattern = \"*\""));
    assert!(content.contains("size = \">100M\""));
    let _ = std::fs::remove_file(&path);
  }

  #[test]
  fn test_export() {
    let path = temp_path();
    let mut searches = SavedSearches::load_from(path.clone());
    searches.add(SavedSearch::new("Test", "*.rs"));
    let exported = searches.export().unwrap();
    assert!(exported.contains("[[search]]"));
    assert!(exported.contains("name = \"Test\""));
    let _ = std::fs::remove_file(&path);
  }

  #[test]
  fn test_import() {
    let path = temp_path();
    let mut searches = SavedSearches::load_from(path.clone());
    let content = r#"
[[search]]
name = "Imported"
pattern = "*.txt"
"#;
    let count = searches.import(content).unwrap();
    assert_eq!(count, 1);
    assert_eq!(searches.len(), 1);
    assert_eq!(searches.list()[0].name, "Imported");
    let _ = std::fs::remove_file(&path);
  }

  #[test]
  fn test_import_merges() {
    let path = temp_path();
    let mut searches = SavedSearches::load_from(path.clone());
    searches.add(SavedSearch::new("Existing", "*.rs"));
    let content = r#"
[[search]]
name = "New"
pattern = "*.txt"

[[search]]
name = "Existing"
pattern = "*.md"
"#;
    searches.import(content).unwrap();
    assert_eq!(searches.len(), 2);
    // Existing should be updated
    assert_eq!(searches.get_by_name("Existing").unwrap().pattern, "*.md");
    let _ = std::fs::remove_file(&path);
  }

  #[test]
  fn test_regex_flag_serialization() {
    let path = temp_path();
    let mut searches = SavedSearches::load_from(path.clone());
    searches.add(SavedSearch::new("Regex", ".*\\.rs$").with_regex(true));
    searches.add(SavedSearch::new("Glob", "*.rs"));
    searches.save().unwrap();

    let loaded = SavedSearches::load_from(path.clone());
    assert!(loaded.get_by_name("Regex").unwrap().regex);
    assert!(!loaded.get_by_name("Glob").unwrap().regex);
    let _ = std::fs::remove_file(&path);
  }
}
