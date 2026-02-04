pub mod directory;
pub mod hex;
pub mod image;
pub mod metadata;
pub mod text;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Instant;

use ratatui::text::Line;
use ratatui_image::picker::Picker;
use ratatui_image::protocol::StatefulProtocol;

use self::metadata::{FileMetadata, ImageMetadata, get_file_metadata, get_file_metadata_with_lines, get_image_metadata, get_git_commits};
use self::text::SyntaxHighlighter;
use crate::git::{GitCommit, GitRepo};

const MAX_TEXT_BYTES: u64 = 1024 * 1024; // 1MB
const MAX_TEXT_LINES: usize = 1000;
const MAX_HEX_BYTES: usize = 4096;
const CACHE_SIZE: usize = 10;
const DEBOUNCE_MS: u128 = 80;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PreviewType {
  Text,
  Image,
  Binary,
  Directory,
  Empty,
  TooLarge,
  Error(String),
}

pub struct PreviewContent {
  pub lines: Vec<Line<'static>>,
  pub preview_type: PreviewType,
  pub line_count: usize,
  pub file_size: u64,
  pub extension: String,
  pub metadata: Option<FileMetadata>,
  pub image_metadata: Option<ImageMetadata>,
  pub git_commits: Vec<GitCommit>,
}

pub struct PreviewState {
  pub scroll_offset: usize,
  pub current_path: Option<PathBuf>,
  pub content: Option<PreviewContent>,
  pub image_protocol: Option<StatefulProtocol>,
  pub image_rx: Option<mpsc::Receiver<self::image::ImageLoadResult>>,
  highlighter: SyntaxHighlighter,
  cache: HashMap<PathBuf, PreviewContent>,
  cache_order: Vec<PathBuf>,
  last_request: Option<(PathBuf, Instant)>,
}

impl PreviewState {
  pub fn new() -> Self {
    Self {
      scroll_offset: 0,
      current_path: None,
      content: None,
      image_protocol: None,
      image_rx: None,
      highlighter: SyntaxHighlighter::new(),
      cache: HashMap::new(),
      cache_order: Vec::new(),
      last_request: None,
    }
  }

  pub fn request_preview(&mut self, path: &Path, picker: Option<&Picker>, git_repo: Option<&GitRepo>) {
    // Debounce: only load if enough time has passed since last request
    if let Some((ref last_path, last_time)) = self.last_request
      && last_path == path && last_time.elapsed().as_millis() < DEBOUNCE_MS {
        return;
      }
    self.last_request = Some((path.to_path_buf(), Instant::now()));

    if Some(path.to_path_buf()) == self.current_path {
      return;
    }

    self.scroll_offset = 0;
    self.image_protocol = None;
    self.image_rx = None;
    self.current_path = Some(path.to_path_buf());

    // Check cache
    if self.cache.contains_key(path) {
      // Move to front of cache order
      self.cache_order.retain(|p| p != path);
      self.cache_order.push(path.to_path_buf());
      return;
    }

    self.load_preview(path, picker, git_repo);
  }

  fn load_preview(&mut self, path: &Path, picker: Option<&Picker>, git_repo: Option<&GitRepo>) {
    let preview_type = detect_preview_type(path);
    let git_commits = get_git_commits(git_repo, path, 3);
    let content = match preview_type {
      PreviewType::Text => self.load_text(path, git_repo),
      PreviewType::Image => {
        if let Some(picker) = picker {
          self.image_rx = Some(self::image::load_image_async(path, picker));
        }
        let file_size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
        let metadata = get_file_metadata(path);
        let image_metadata = get_image_metadata(path);
        Some(PreviewContent {
          lines: vec![Line::from(" Loading image...")],
          preview_type: PreviewType::Image,
          line_count: 0,
          file_size,
          extension: get_extension(path),
          metadata,
          image_metadata,
          git_commits,
        })
      }
      PreviewType::Binary => self.load_hex(path, git_repo),
      PreviewType::Directory => self.load_directory(path),
      PreviewType::TooLarge => {
        let file_size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
        Some(PreviewContent {
          lines: vec![Line::from(" File too large to preview")],
          preview_type: PreviewType::TooLarge,
          line_count: 0,
          file_size,
          extension: get_extension(path),
          metadata: get_file_metadata(path),
          image_metadata: None,
          git_commits,
        })
      }
      PreviewType::Empty => Some(PreviewContent {
        lines: vec![Line::from(" Empty file")],
        preview_type: PreviewType::Empty,
        line_count: 0,
        file_size: 0,
        extension: String::new(),
        metadata: get_file_metadata(path),
        image_metadata: None,
        git_commits,
      }),
      PreviewType::Error(ref msg) => Some(PreviewContent {
        lines: vec![Line::from(format!(" Error: {msg}"))],
        preview_type: preview_type.clone(),
        line_count: 0,
        file_size: 0,
        extension: String::new(),
        metadata: None,
        image_metadata: None,
        git_commits: Vec::new(),
      }),
    };

    if let Some(content) = content {
      self.insert_cache(path.to_path_buf(), content);
    }
  }

  fn load_text(&self, path: &Path, git_repo: Option<&GitRepo>) -> Option<PreviewContent> {
    let content = match std::fs::read_to_string(path) {
      Ok(c) => c,
      Err(e) => {
        return Some(PreviewContent {
          lines: vec![Line::from(format!(" Error reading file: {e}"))],
          preview_type: PreviewType::Error(e.to_string()),
          line_count: 0,
          file_size: 0,
          extension: String::new(),
          metadata: None,
          image_metadata: None,
          git_commits: Vec::new(),
        });
      }
    };

    let line_count = content.lines().count();
    let truncated: String = content.lines().take(MAX_TEXT_LINES).collect::<Vec<_>>().join("\n");
    let ext = get_extension(path);
    let lines = self.highlighter.highlight(&truncated, &ext);
    let file_size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    let metadata = get_file_metadata_with_lines(path, line_count);
    let git_commits = get_git_commits(git_repo, path, 3);

    Some(PreviewContent {
      lines,
      preview_type: PreviewType::Text,
      line_count,
      file_size,
      extension: ext,
      metadata,
      image_metadata: None,
      git_commits,
    })
  }

  fn load_hex(&self, path: &Path, git_repo: Option<&GitRepo>) -> Option<PreviewContent> {
    let data = match std::fs::read(path) {
      Ok(d) => d,
      Err(e) => {
        return Some(PreviewContent {
          lines: vec![Line::from(format!(" Error reading file: {e}"))],
          preview_type: PreviewType::Error(e.to_string()),
          line_count: 0,
          file_size: 0,
          extension: String::new(),
          metadata: None,
          image_metadata: None,
          git_commits: Vec::new(),
        });
      }
    };

    let file_size = data.len() as u64;
    let truncated = &data[..data.len().min(MAX_HEX_BYTES)];
    let lines = hex::hex_dump(truncated);
    let metadata = get_file_metadata(path);
    let git_commits = get_git_commits(git_repo, path, 3);

    Some(PreviewContent {
      lines,
      preview_type: PreviewType::Binary,
      line_count: data.len().div_ceil(16),
      file_size,
      extension: get_extension(path),
      metadata,
      image_metadata: None,
      git_commits,
    })
  }

  fn load_directory(&self, path: &Path) -> Option<PreviewContent> {
    let summary = directory::summarize_dir(path);
    let lines = directory::render_dir_summary(&summary);

    Some(PreviewContent {
      lines,
      preview_type: PreviewType::Directory,
      line_count: summary.file_count + summary.dir_count,
      file_size: summary.total_size,
      extension: String::new(),
      metadata: None,
      image_metadata: None,
      git_commits: Vec::new(),
    })
  }

  fn insert_cache(&mut self, path: PathBuf, content: PreviewContent) {
    if self.cache.len() >= CACHE_SIZE
      && let Some(oldest) = self.cache_order.first().cloned() {
        self.cache.remove(&oldest);
        self.cache_order.remove(0);
      }
    self.cache_order.push(path.clone());
    self.cache.insert(path, content);
  }

  pub fn get_content(&self) -> Option<&PreviewContent> {
    self.current_path.as_ref().and_then(|p| self.cache.get(p))
  }

  pub fn check_image_loaded(&mut self) {
    if let Some(ref rx) = self.image_rx
      && let Ok(result) = rx.try_recv() {
        match result {
          self::image::ImageLoadResult::Loaded(protocol) => {
            self.image_protocol = Some(protocol);
          }
          self::image::ImageLoadResult::Error(msg) => {
            if let Some(ref path) = self.current_path {
              let content = PreviewContent {
                lines: vec![Line::from(format!(" {msg}"))],
                preview_type: PreviewType::Error(msg),
                line_count: 0,
                file_size: 0,
                extension: String::new(),
                metadata: None,
                image_metadata: None,
                git_commits: Vec::new(),
              };
              self.insert_cache(path.clone(), content);
            }
          }
        }
        self.image_rx = None;
      }
  }

  pub fn scroll_up(&mut self, amount: usize) {
    self.scroll_offset = self.scroll_offset.saturating_sub(amount);
  }

  pub fn scroll_down(&mut self, amount: usize) {
    if let Some(content) = self.get_content() {
      let max = content.lines.len().saturating_sub(1);
      self.scroll_offset = (self.scroll_offset + amount).min(max);
    }
  }

  pub fn invalidate(&mut self) {
    self.cache.clear();
    self.cache_order.clear();
    self.current_path = None;
    self.content = None;
    self.image_protocol = None;
    self.image_rx = None;
  }
}

fn get_extension(path: &Path) -> String {
  path
    .extension()
    .map(|e| e.to_string_lossy().to_string())
    .unwrap_or_default()
}

const IMAGE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "gif", "webp", "bmp", "tiff", "tif", "ico", "svg"];

pub fn detect_preview_type(path: &Path) -> PreviewType {
  if path.is_dir() {
    return PreviewType::Directory;
  }

  let metadata = match path.metadata() {
    Ok(m) => m,
    Err(e) => return PreviewType::Error(e.to_string()),
  };

  if metadata.len() == 0 {
    return PreviewType::Empty;
  }

  if metadata.len() > MAX_TEXT_BYTES {
    // Check if it's an image (images can be large)
    let ext = get_extension(path);
    if IMAGE_EXTENSIONS.contains(&ext.to_lowercase().as_str()) {
      return PreviewType::Image;
    }
    return PreviewType::TooLarge;
  }

  // Check extension for images
  let ext = get_extension(path);
  if IMAGE_EXTENSIONS.contains(&ext.to_lowercase().as_str()) {
    return PreviewType::Image;
  }

  // Try to detect if binary using infer
  if let Ok(data) = std::fs::read(path) {
    if let Some(kind) = infer::get(&data) {
      let mime = kind.mime_type();
      if mime.starts_with("image/") {
        return PreviewType::Image;
      }
      if !mime.starts_with("text/") {
        return PreviewType::Binary;
      }
    }

    // Check if content looks like text (no null bytes in first chunk)
    let check_len = data.len().min(8192);
    if data[..check_len].contains(&0) {
      return PreviewType::Binary;
    }
  }

  PreviewType::Text
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::fs;

  #[test]
  fn test_detect_directory() {
    let dir = std::env::temp_dir();
    assert_eq!(detect_preview_type(&dir), PreviewType::Directory);
  }

  #[test]
  fn test_detect_text_file() {
    let dir = std::env::temp_dir().join("tui_explorer_test_detect");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let file = dir.join("test.txt");
    fs::write(&file, "hello world").unwrap();
    assert_eq!(detect_preview_type(&file), PreviewType::Text);
    let _ = fs::remove_dir_all(&dir);
  }

  #[test]
  fn test_detect_empty_file() {
    let dir = std::env::temp_dir().join("tui_explorer_test_empty");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let file = dir.join("empty.txt");
    fs::write(&file, "").unwrap();
    assert_eq!(detect_preview_type(&file), PreviewType::Empty);
    let _ = fs::remove_dir_all(&dir);
  }

  #[test]
  fn test_detect_binary_file() {
    let dir = std::env::temp_dir().join("tui_explorer_test_binary");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let file = dir.join("binary.bin");
    fs::write(&file, &[0u8, 1, 2, 3, 0, 0, 0, 0]).unwrap();
    assert_eq!(detect_preview_type(&file), PreviewType::Binary);
    let _ = fs::remove_dir_all(&dir);
  }

  #[test]
  fn test_detect_image_by_extension() {
    let dir = std::env::temp_dir().join("tui_explorer_test_imgext");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let file = dir.join("photo.png");
    fs::write(&file, "fake png data").unwrap();
    assert_eq!(detect_preview_type(&file), PreviewType::Image);
    let _ = fs::remove_dir_all(&dir);
  }

  #[test]
  fn test_detect_nonexistent() {
    let result = detect_preview_type(Path::new("/nonexistent/file.txt"));
    assert!(matches!(result, PreviewType::Error(_)));
  }

  #[test]
  fn test_get_extension() {
    assert_eq!(get_extension(Path::new("foo.rs")), "rs");
    assert_eq!(get_extension(Path::new("foo")), "");
    assert_eq!(get_extension(Path::new("foo.tar.gz")), "gz");
  }

  #[test]
  fn test_preview_state_scroll() {
    let mut state = PreviewState::new();
    state.scroll_offset = 5;
    state.scroll_up(3);
    assert_eq!(state.scroll_offset, 2);
    state.scroll_up(10);
    assert_eq!(state.scroll_offset, 0);
  }

  #[test]
  fn test_cache_eviction() {
    let mut state = PreviewState::new();
    // Insert CACHE_SIZE + 1 items
    for i in 0..=CACHE_SIZE {
      let path = PathBuf::from(format!("/fake/path/{i}"));
      let content = PreviewContent {
        lines: vec![],
        preview_type: PreviewType::Text,
        line_count: 0,
        file_size: 0,
        extension: String::new(),
        metadata: None,
        image_metadata: None,
        git_commits: Vec::new(),
      };
      state.insert_cache(path, content);
    }
    assert_eq!(state.cache.len(), CACHE_SIZE);
    // First item should have been evicted
    assert!(!state.cache.contains_key(&PathBuf::from("/fake/path/0")));
  }
}
