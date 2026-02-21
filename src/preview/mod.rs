pub mod archive;
pub mod blame;
pub mod diff;
pub mod directory;
pub mod hex;
pub mod image;
pub mod markdown;
pub mod metadata;
pub mod structured;
pub mod text;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Instant;

use ratatui::text::Line;
use ratatui_image::picker::Picker;
use ratatui_image::protocol::StatefulProtocol;

use self::blame::BlameData;
use self::metadata::{FileMetadata, ImageMetadata, get_file_metadata, get_file_metadata_with_lines, get_image_metadata};
use self::text::SyntaxHighlighter;
use crate::git::{GitCommit, GitRepo};
use crate::theme::Theme;

const MAX_TEXT_BYTES: u64 = 1024 * 1024; // 1MB
const MAX_TEXT_LINES: usize = 1000;
const MAX_HEX_BYTES: usize = 4096;
const CACHE_SIZE: usize = 10;
const DEBOUNCE_MS: u128 = 80;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PreviewType {
  Text,
  Markdown,
  Image,
  Binary,
  Directory,
  Archive,
  Diff,
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
  pub blame_data: Option<BlameData>,
  /// Raw (unformatted) lines for structured data files, if formatting was applied.
  pub raw_lines: Option<Vec<Line<'static>>>,
  /// Whether this file is a structured data file (JSON/TOML).
  pub is_structured: bool,
  pub diff_hunks: Vec<usize>, // Indices of hunk headers for navigation
}

pub struct PreviewState {
  pub scroll_offset: usize,
  pub current_path: Option<PathBuf>,
  pub content: Option<PreviewContent>,
  pub image_protocol: Option<StatefulProtocol>,
  pub image_rx: Option<mpsc::Receiver<self::image::ImageLoadResult>>,
  pub git_commits_rx: Option<mpsc::Receiver<(PathBuf, Vec<GitCommit>)>>,
  pub blame_enabled: bool,
  pub markdown_rendered: bool,
  /// Whether to show formatted (pretty-printed) view for structured data.
  pub show_formatted: bool,
  highlighter: SyntaxHighlighter,
  pub theme: Theme,
  cache: HashMap<PathBuf, PreviewContent>,
  cache_order: Vec<PathBuf>,
  last_request: Option<(PathBuf, Instant)>,
  /// Cache for raw markdown content (when toggling between raw/rendered)
  markdown_raw_cache: HashMap<PathBuf, PreviewContent>,
}

impl PreviewState {
  pub fn new(syntax_theme: &str, theme: Theme) -> Self {
    Self {
      scroll_offset: 0,
      current_path: None,
      content: None,
      image_protocol: None,
      image_rx: None,
      git_commits_rx: None,
      blame_enabled: false,
      markdown_rendered: true,
      show_formatted: true,
      highlighter: SyntaxHighlighter::new(syntax_theme),
      theme,
      cache: HashMap::new(),
      cache_order: Vec::new(),
      last_request: None,
      markdown_raw_cache: HashMap::new(),
    }
  }

  pub fn set_syntax_theme(&mut self, name: &str) {
    self.highlighter.set_theme_name(name);
    self.invalidate();
  }

  pub fn set_theme(&mut self, theme: Theme) {
    self.theme = theme;
    self.invalidate();
  }

  pub fn toggle_blame(&mut self, git_repo: Option<&GitRepo>) {
    self.blame_enabled = !self.blame_enabled;
    self.scroll_offset = 0;

    // Lazily compute blame data on first toggle-on
    if self.blame_enabled
      && let Some(path) = self.current_path.clone()
      && let Some(content) = self.cache.get_mut(&path)
      && content.blame_data.is_none()
    {
      content.blame_data = git_repo.and_then(|repo| blame::get_blame(repo, &path));
    }
  }

  /// Toggles between formatted and raw view for structured data files.
  /// Returns true if the current file is a structured data file.
  pub fn toggle_formatted(&mut self) -> bool {
    if let Some(content) = self.current_path.as_ref().and_then(|p| self.cache.get(p))
      && content.is_structured
    {
      self.show_formatted = !self.show_formatted;
      self.scroll_offset = 0;
      return true;
    }
    false
  }

  /// Returns the appropriate lines to display based on formatted/raw mode.
  pub fn get_display_lines(&self) -> Option<&Vec<Line<'static>>> {
    let content = self.get_content()?;
    if content.is_structured && !self.show_formatted {
      // Show raw lines if available and not in formatted mode
      content.raw_lines.as_ref().or(Some(&content.lines))
    } else {
      Some(&content.lines)
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
    self.git_commits_rx = None;
    self.current_path = Some(path.to_path_buf());

    // Check cache
    if let Some(cached) = self.cache.get(path) {
      // Move to front of cache order
      self.cache_order.retain(|p| p != path);
      self.cache_order.push(path.to_path_buf());

      // For images, re-trigger async load since we don't cache the protocol
      if cached.preview_type == PreviewType::Image
        && let Some(picker) = picker {
          self.image_rx = Some(self::image::load_image_async(path, picker));
        }
      return;
    }

    self.load_preview(path, picker, git_repo);
  }

  fn load_preview(&mut self, path: &Path, picker: Option<&Picker>, git_repo: Option<&GitRepo>) {
    let preview_type = detect_preview_type(path);

    // Spawn async git commit loading
    if let Some(repo) = git_repo {
      self.git_commits_rx = Some(load_git_commits_async(repo.root(), path, 3));
    }
    let git_commits = Vec::new();
    let content = match preview_type {
      PreviewType::Text => self.load_text(path, &git_commits),
      PreviewType::Markdown => self.load_markdown(path, &git_commits),
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
          blame_data: None,
          raw_lines: None,
          is_structured: false,
          diff_hunks: Vec::new(),
        })
      }
      PreviewType::Binary => self.load_hex(path, &git_commits),
      PreviewType::Archive => self.load_archive(path, &git_commits),
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
          blame_data: None,
          raw_lines: None,
          is_structured: false,
          diff_hunks: Vec::new(),
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
        blame_data: None,
        raw_lines: None,
        is_structured: false,
        diff_hunks: Vec::new(),
      }),
      PreviewType::Diff => None, // Diff is handled separately via show_diff
      PreviewType::Error(ref msg) => Some(PreviewContent {
        lines: vec![Line::from(format!(" Error: {msg}"))],
        preview_type: preview_type.clone(),
        line_count: 0,
        file_size: 0,
        extension: String::new(),
        metadata: None,
        image_metadata: None,
        git_commits: Vec::new(),
        blame_data: None,
        raw_lines: None,
        is_structured: false,
        diff_hunks: Vec::new(),
      }),
    };

    if let Some(content) = content {
      self.insert_cache(path.to_path_buf(), content);
    }
  }

  fn load_text(&self, path: &Path, git_commits: &[GitCommit]) -> Option<PreviewContent> {
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
          blame_data: None,
          raw_lines: None,
          is_structured: false,
          diff_hunks: Vec::new(),
        });
      }
    };

    let line_count = content.lines().count();
    let truncated: String = content.lines().take(MAX_TEXT_LINES).collect::<Vec<_>>().join("\n");
    let ext = get_extension(path);
    let file_size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    let metadata = get_file_metadata_with_lines(path, line_count);

    // Check if this is a structured data file (JSON/TOML)
    let is_structured = structured::is_structured_data(&ext);

    let (lines, raw_lines) = if is_structured {
      // Try to format the content
      match structured::format_structured(&content, &ext) {
        Some(structured::FormatResult::Formatted { content: formatted, extension: fmt_ext }) => {
          // Truncate formatted content too
          let fmt_truncated: String = formatted.lines().take(MAX_TEXT_LINES).collect::<Vec<_>>().join("\n");
          let formatted_lines = self.highlighter.highlight(&fmt_truncated, &fmt_ext);
          let raw_highlighted = self.highlighter.highlight(&truncated, &ext);
          (formatted_lines, Some(raw_highlighted))
        }
        Some(structured::FormatResult::Error(_)) => {
          // Formatting failed, show raw content
          let raw_highlighted = self.highlighter.highlight(&truncated, &ext);
          (raw_highlighted, None)
        }
        None => {
          // Not a structured format (shouldn't happen given is_structured check)
          let raw_highlighted = self.highlighter.highlight(&truncated, &ext);
          (raw_highlighted, None)
        }
      }
    } else {
      // Regular text file
      let highlighted = self.highlighter.highlight(&truncated, &ext);
      (highlighted, None)
    };

    Some(PreviewContent {
      lines,
      preview_type: PreviewType::Text,
      line_count,
      file_size,
      extension: ext,
      metadata,
      image_metadata: None,
      git_commits: git_commits.to_vec(),
      blame_data: None,
      raw_lines,
      is_structured,
      diff_hunks: Vec::new(),
    })
  }

  fn load_markdown(&self, path: &Path, git_commits: &[GitCommit]) -> Option<PreviewContent> {
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
          blame_data: None,
          raw_lines: None,
          is_structured: false,
          diff_hunks: Vec::new(),
        });
      }
    };

    let line_count = content.lines().count();
    let truncated: String = content.lines().take(MAX_TEXT_LINES).collect::<Vec<_>>().join("\n");
    let ext = get_extension(path);
    let file_size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    let metadata = get_file_metadata_with_lines(path, line_count);

    // Render markdown if in rendered mode, otherwise show raw with syntax highlighting
    let lines = if self.markdown_rendered {
      markdown::render_markdown(&truncated, &self.highlighter, &self.theme)
    } else {
      self.highlighter.highlight(&truncated, &ext)
    };

    Some(PreviewContent {
      lines,
      preview_type: PreviewType::Markdown,
      line_count,
      file_size,
      extension: ext,
      metadata,
      image_metadata: None,
      git_commits: git_commits.to_vec(),
      blame_data: None,
      raw_lines: None,
      is_structured: false,
      diff_hunks: Vec::new(),
    })
  }

  fn load_hex(&self, path: &Path, git_commits: &[GitCommit]) -> Option<PreviewContent> {
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
          blame_data: None,
          raw_lines: None,
          is_structured: false,
          diff_hunks: Vec::new(),
        });
      }
    };

    let file_size = data.len() as u64;
    let truncated = &data[..data.len().min(MAX_HEX_BYTES)];
    let lines = hex::hex_dump(truncated, &self.theme);
    let metadata = get_file_metadata(path);

    Some(PreviewContent {
      lines,
      preview_type: PreviewType::Binary,
      line_count: data.len().div_ceil(16),
      file_size,
      extension: get_extension(path),
      metadata,
      image_metadata: None,
      git_commits: git_commits.to_vec(),
      blame_data: None,
      raw_lines: None,
      is_structured: false,
      diff_hunks: Vec::new(),
    })
  }

  fn load_directory(&self, path: &Path) -> Option<PreviewContent> {
    let summary = directory::summarize_dir(path);
    let lines = directory::render_dir_summary(&summary, &self.theme);

    Some(PreviewContent {
      lines,
      preview_type: PreviewType::Directory,
      line_count: summary.file_count + summary.dir_count,
      file_size: summary.total_size,
      extension: String::new(),
      metadata: None,
      image_metadata: None,
      git_commits: Vec::new(),
      blame_data: None,
      raw_lines: None,
      is_structured: false,
      diff_hunks: Vec::new(),
    })
  }

  fn load_archive(&self, path: &Path, git_commits: &[GitCommit]) -> Option<PreviewContent> {
    let file_size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    let metadata = get_file_metadata(path);
    let archive_type = archive::archive_type(path).unwrap_or("archive");
    let lines = archive::render_archive_summary(archive_type, file_size, &self.theme);

    Some(PreviewContent {
      lines,
      preview_type: PreviewType::Archive,
      line_count: 0,
      file_size,
      extension: get_extension(path),
      metadata,
      image_metadata: None,
      git_commits: git_commits.to_vec(),
      blame_data: None,
      raw_lines: None,
      is_structured: false,
      diff_hunks: Vec::new(),
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

  pub fn check_image_loaded(&mut self) -> bool {
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
                blame_data: None,
                raw_lines: None,
                is_structured: false,
                diff_hunks: Vec::new(),
              };
              self.insert_cache(path.clone(), content);
            }
          }
        }
        self.image_rx = None;
        return true;
      }
    false
  }

  pub fn check_git_commits_loaded(&mut self) -> bool {
    if let Some(ref rx) = self.git_commits_rx
      && let Ok((path, commits)) = rx.try_recv()
    {
      if let Some(content) = self.cache.get_mut(&path) {
        content.git_commits = commits;
      }
      self.git_commits_rx = None;
      return true;
    }
    false
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
    self.markdown_raw_cache.clear();
    self.current_path = None;
    self.content = None;
    self.image_protocol = None;
    self.image_rx = None;
    self.git_commits_rx = None;
  }

  /// Toggle between raw and rendered markdown mode
  /// Returns true if the current file is markdown and was toggled
  pub fn toggle_markdown_mode(&mut self) -> bool {
    let path = match self.current_path.clone() {
      Some(p) => p,
      None => return false,
    };

    let ext = get_extension(&path);
    if !MARKDOWN_EXTENSIONS.contains(&ext.to_lowercase().as_str()) {
      return false;
    }

    // Preserve scroll position
    let scroll = self.scroll_offset;

    // Toggle the mode
    self.markdown_rendered = !self.markdown_rendered;

    // Remove from cache to force reload
    self.cache.remove(&path);
    self.cache_order.retain(|p| p != &path);

    // Reload with new mode
    self.load_preview(&path, None, None);

    // Restore scroll position (clamped to new content length)
    if let Some(content) = self.get_content() {
      self.scroll_offset = scroll.min(content.lines.len().saturating_sub(1));
    }

    true
  }

  /// Show the git diff for the given file path
  pub fn show_diff(&mut self, path: &Path, git_repo: Option<&GitRepo>) -> bool {
    let repo_root = git_repo.map(|r| r.root());

    let (lines, diff_hunks) = if let Some(root) = repo_root
      && let Some(file_diff) = diff::generate_diff(root, path)
    {
      (diff::render_diff(&file_diff, &self.theme), file_diff.hunks)
    } else {
      (diff::render_no_diff_message(&self.theme), Vec::new())
    };

    let has_diff = !diff_hunks.is_empty();
    let file_size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);

    let content = PreviewContent {
      lines,
      preview_type: PreviewType::Diff,
      line_count: 0,
      file_size,
      extension: get_extension(path),
      metadata: metadata::get_file_metadata(path),
      image_metadata: None,
      git_commits: Vec::new(),
      blame_data: None,
      raw_lines: None,
      is_structured: false,
      diff_hunks,
    };

    self.scroll_offset = 0;
    self.image_protocol = None;
    self.image_rx = None;

    // Use a unique cache key for diff mode
    let cache_key = path.with_extension(format!(
      "{}.diff",
      path.extension().map(|e| e.to_string_lossy()).unwrap_or_default()
    ));
    self.current_path = Some(cache_key.clone());
    self.insert_cache(cache_key, content);

    has_diff
  }

  /// Navigate to the next diff hunk, returns true if moved
  pub fn next_hunk(&mut self) -> bool {
    if let Some(content) = self.get_content()
      && let Some(next) = content.diff_hunks.iter().find(|&&idx| idx > self.scroll_offset).copied()
    {
      self.scroll_offset = next;
      return true;
    }
    false
  }

  /// Navigate to the previous diff hunk, returns true if moved
  pub fn prev_hunk(&mut self) -> bool {
    if let Some(content) = self.get_content()
      && let Some(prev) = content.diff_hunks.iter().rev().find(|&&idx| idx < self.scroll_offset).copied()
    {
      self.scroll_offset = prev;
      return true;
    }
    false
  }
}

fn load_git_commits_async(
  repo_root: &Path,
  path: &Path,
  limit: usize,
) -> mpsc::Receiver<(PathBuf, Vec<GitCommit>)> {
  let (tx, rx) = mpsc::channel();
  let repo_root = repo_root.to_path_buf();
  let path = path.to_path_buf();

  std::thread::spawn(move || {
    let commits = GitRepo::open(&repo_root)
      .map(|r| r.get_file_commits(&path, limit))
      .unwrap_or_default();
    let _ = tx.send((path, commits));
  });

  rx
}

fn get_extension(path: &Path) -> String {
  path
    .extension()
    .map(|e| e.to_string_lossy().to_string())
    .unwrap_or_default()
}

const IMAGE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "gif", "webp", "bmp", "tiff", "tif", "ico", "svg"];
const MARKDOWN_EXTENSIONS: &[&str] = &["md", "markdown", "mdown", "mkd", "mkdn"];

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

  // Check for archive types early (they can be large)
  if archive::is_archive(path) {
    return PreviewType::Archive;
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

  // Check extension for markdown
  if MARKDOWN_EXTENSIONS.contains(&ext.to_lowercase().as_str()) {
    return PreviewType::Markdown;
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
    fs::write(&file, [0u8, 1, 2, 3, 0, 0, 0, 0]).unwrap();
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
  fn test_detect_archive_zip() {
    let dir = std::env::temp_dir().join("tui_explorer_test_archive_zip");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let file = dir.join("test.zip");
    // Create a minimal valid ZIP file
    {
      let f = fs::File::create(&file).unwrap();
      let mut zip = zip::ZipWriter::new(f);
      let options = zip::write::SimpleFileOptions::default();
      zip.start_file("test.txt", options).unwrap();
      std::io::Write::write_all(&mut zip, b"test").unwrap();
      zip.finish().unwrap();
    }
    assert_eq!(detect_preview_type(&file), PreviewType::Archive);
    let _ = fs::remove_dir_all(&dir);
  }

  #[test]
  fn test_detect_archive_tar_gz() {
    let dir = std::env::temp_dir().join("tui_explorer_test_archive_tgz");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let file = dir.join("test.tar.gz");
    // Just create a fake file - detection is by extension
    fs::write(&file, "fake tar.gz data").unwrap();
    assert_eq!(detect_preview_type(&file), PreviewType::Archive);
    let _ = fs::remove_dir_all(&dir);
  }

  #[test]
  fn test_detect_markdown_file() {
    let dir = std::env::temp_dir().join("tui_explorer_test_markdown");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let file = dir.join("README.md");
    fs::write(&file, "# Hello World").unwrap();
    assert_eq!(detect_preview_type(&file), PreviewType::Markdown);
    let _ = fs::remove_dir_all(&dir);
  }

  #[test]
  fn test_detect_markdown_extensions() {
    let dir = std::env::temp_dir().join("tui_explorer_test_md_ext");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();

    for ext in &["md", "markdown", "mdown", "mkd", "mkdn"] {
      let file = dir.join(format!("test.{ext}"));
      fs::write(&file, "# Test").unwrap();
      assert_eq!(detect_preview_type(&file), PreviewType::Markdown, "Failed for extension: {ext}");
    }

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
    let mut state = PreviewState::new("base16-ocean.dark", Theme::dark());
    state.scroll_offset = 5;
    state.scroll_up(3);
    assert_eq!(state.scroll_offset, 2);
    state.scroll_up(10);
    assert_eq!(state.scroll_offset, 0);
  }

  #[test]
  fn test_cache_eviction() {
    let mut state = PreviewState::new("base16-ocean.dark", Theme::dark());
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
        blame_data: None,
        raw_lines: None,
        is_structured: false,
        diff_hunks: Vec::new(),
      };
      state.insert_cache(path, content);
    }
    assert_eq!(state.cache.len(), CACHE_SIZE);
    // First item should have been evicted
    assert!(!state.cache.contains_key(&PathBuf::from("/fake/path/0")));
  }

  #[test]
  fn test_toggle_blame() {
    let mut state = PreviewState::new("base16-ocean.dark", Theme::dark());
    assert!(!state.blame_enabled);

    state.toggle_blame(None);
    assert!(state.blame_enabled);

    state.toggle_blame(None);
    assert!(!state.blame_enabled);
  }

  #[test]
  fn test_toggle_blame_resets_scroll() {
    let mut state = PreviewState::new("base16-ocean.dark", Theme::dark());
    state.scroll_offset = 15;

    state.toggle_blame(None);
    assert_eq!(state.scroll_offset, 0);
  }

  #[test]
  fn test_toggle_formatted_on_non_structured() {
    let mut state = PreviewState::new("base16-ocean.dark", Theme::dark());
    let path = PathBuf::from("/fake/path/test.rs");
    let content = PreviewContent {
      lines: vec![],
      preview_type: PreviewType::Text,
      line_count: 0,
      file_size: 0,
      extension: "rs".to_string(),
      metadata: None,
      image_metadata: None,
      git_commits: Vec::new(),
      blame_data: None,
      raw_lines: None,
      is_structured: false,
      diff_hunks: Vec::new(),
    };
    state.insert_cache(path.clone(), content);
    state.current_path = Some(path);

    // Should return false for non-structured files
    assert!(!state.toggle_formatted());
    // show_formatted should remain true
    assert!(state.show_formatted);
  }

  #[test]
  fn test_toggle_formatted_on_structured() {
    let mut state = PreviewState::new("base16-ocean.dark", Theme::dark());
    let path = PathBuf::from("/fake/path/test.json");
    let content = PreviewContent {
      lines: vec![Line::from("formatted")],
      preview_type: PreviewType::Text,
      line_count: 1,
      file_size: 10,
      extension: "json".to_string(),
      metadata: None,
      image_metadata: None,
      git_commits: Vec::new(),
      blame_data: None,
      raw_lines: Some(vec![Line::from("raw")]),
      is_structured: true,
      diff_hunks: Vec::new(),
    };
    state.insert_cache(path.clone(), content);
    state.current_path = Some(path);

    assert!(state.show_formatted);
    // Should return true for structured files
    assert!(state.toggle_formatted());
    assert!(!state.show_formatted);
    // Toggle again
    assert!(state.toggle_formatted());
    assert!(state.show_formatted);
  }

  #[test]
  fn test_get_display_lines_formatted() {
    let mut state = PreviewState::new("base16-ocean.dark", Theme::dark());
    let path = PathBuf::from("/fake/path/test.json");
    let content = PreviewContent {
      lines: vec![Line::from("formatted")],
      preview_type: PreviewType::Text,
      line_count: 1,
      file_size: 10,
      extension: "json".to_string(),
      metadata: None,
      image_metadata: None,
      git_commits: Vec::new(),
      blame_data: None,
      raw_lines: Some(vec![Line::from("raw")]),
      is_structured: true,
      diff_hunks: Vec::new(),
    };
    state.insert_cache(path.clone(), content);
    state.current_path = Some(path);

    // Should return formatted lines by default
    let lines = state.get_display_lines().unwrap();
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].spans[0].content, "formatted");

    // Toggle to raw
    state.toggle_formatted();
    let lines = state.get_display_lines().unwrap();
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].spans[0].content, "raw");
  }

  #[test]
  fn test_scroll_down_clamps_to_content() {
    let mut state = PreviewState::new("base16-ocean.dark", Theme::dark());
    let path = PathBuf::from("/fake/scroll_test");
    let content = PreviewContent {
      lines: vec![Line::from("line1"), Line::from("line2"), Line::from("line3")],
      preview_type: PreviewType::Text,
      line_count: 3,
      file_size: 0,
      extension: String::new(),
      metadata: None,
      image_metadata: None,
      git_commits: Vec::new(),
      blame_data: None,
      raw_lines: None,
      is_structured: false,
      diff_hunks: Vec::new(),
    };
    state.insert_cache(path.clone(), content);
    state.current_path = Some(path);

    // Scroll down by a large amount should clamp to lines.len()-1
    state.scroll_down(100);
    assert_eq!(state.scroll_offset, 2); // 3 lines, max index is 2
  }

  #[test]
  fn test_scroll_down_no_content_noop() {
    let mut state = PreviewState::new("base16-ocean.dark", Theme::dark());
    // No content set
    state.scroll_down(5);
    assert_eq!(state.scroll_offset, 0); // Should stay at 0, no panic
  }

  #[test]
  fn test_invalidate_clears_all() {
    let mut state = PreviewState::new("base16-ocean.dark", Theme::dark());
    let path = PathBuf::from("/fake/invalidate_test");
    let content = PreviewContent {
      lines: vec![Line::from("test")],
      preview_type: PreviewType::Text,
      line_count: 1,
      file_size: 0,
      extension: String::new(),
      metadata: None,
      image_metadata: None,
      git_commits: Vec::new(),
      blame_data: None,
      raw_lines: None,
      is_structured: false,
      diff_hunks: Vec::new(),
    };
    state.insert_cache(path.clone(), content);
    state.current_path = Some(path);

    state.invalidate();
    assert!(state.cache.is_empty());
    assert!(state.cache_order.is_empty());
    assert!(state.current_path.is_none());
    assert!(state.get_content().is_none());
  }

  #[test]
  fn test_request_preview_debounce() {
    let mut state = PreviewState::new("base16-ocean.dark", Theme::dark());
    let dir = std::env::temp_dir().join("tfl_preview_debounce");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let file = dir.join("test.txt");
    fs::write(&file, "hello").unwrap();

    // First request loads the file
    state.request_preview(&file, None, None);
    assert!(state.current_path.is_some());

    // Immediately invalidate and re-request - should be debounced
    state.current_path = None; // force current_path check to pass
    state.last_request = Some((file.clone(), std::time::Instant::now()));
    state.request_preview(&file, None, None);
    // Debounced: current_path should remain None
    assert!(state.current_path.is_none());

    let _ = fs::remove_dir_all(&dir);
  }

  #[test]
  fn test_request_preview_cache_hit() {
    let mut state = PreviewState::new("base16-ocean.dark", Theme::dark());
    let path = PathBuf::from("/fake/cache_hit");
    let content = PreviewContent {
      lines: vec![Line::from("cached")],
      preview_type: PreviewType::Text,
      line_count: 1,
      file_size: 0,
      extension: String::new(),
      metadata: None,
      image_metadata: None,
      git_commits: Vec::new(),
      blame_data: None,
      raw_lines: None,
      is_structured: false,
      diff_hunks: Vec::new(),
    };
    state.insert_cache(path.clone(), content);
    // Clear last_request to avoid debounce
    state.last_request = None;

    state.request_preview(&path, None, None);
    // Cache hit: current_path should be set
    assert_eq!(state.current_path, Some(path.clone()));
    // cache_order should have moved the path to the end
    assert_eq!(state.cache_order.last().unwrap(), &path);
  }

  #[test]
  fn test_load_text_success() {
    let state = PreviewState::new("base16-ocean.dark", Theme::dark());
    let dir = std::env::temp_dir().join("tfl_preview_load_text");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let file = dir.join("test.rs");
    fs::write(&file, "fn main() {\n  println!(\"hello\");\n}\n").unwrap();

    let result = state.load_text(&file, &[]);
    assert!(result.is_some());
    let content = result.unwrap();
    assert_eq!(content.preview_type, PreviewType::Text);
    assert_eq!(content.line_count, 3);
    assert!(!content.lines.is_empty());
    let _ = fs::remove_dir_all(&dir);
  }

  #[test]
  fn test_load_text_nonexistent() {
    let state = PreviewState::new("base16-ocean.dark", Theme::dark());
    let result = state.load_text(Path::new("/nonexistent/file.txt"), &[]);
    assert!(result.is_some());
    let content = result.unwrap();
    assert!(matches!(content.preview_type, PreviewType::Error(_)));
  }

  #[test]
  fn test_load_markdown_success() {
    let state = PreviewState::new("base16-ocean.dark", Theme::dark());
    let dir = std::env::temp_dir().join("tfl_preview_load_md");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let file = dir.join("test.md");
    fs::write(&file, "# Hello\n\nWorld\n").unwrap();

    let result = state.load_markdown(&file, &[]);
    assert!(result.is_some());
    let content = result.unwrap();
    assert_eq!(content.preview_type, PreviewType::Markdown);
    assert_eq!(content.line_count, 3);
    let _ = fs::remove_dir_all(&dir);
  }

  #[test]
  fn test_load_hex_success() {
    let state = PreviewState::new("base16-ocean.dark", Theme::dark());
    let dir = std::env::temp_dir().join("tfl_preview_load_hex");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let file = dir.join("test.bin");
    fs::write(&file, [0u8, 1, 2, 3, 0xFF, 0xFE]).unwrap();

    let result = state.load_hex(&file, &[]);
    assert!(result.is_some());
    let content = result.unwrap();
    assert_eq!(content.preview_type, PreviewType::Binary);
    assert!(!content.lines.is_empty());
    let _ = fs::remove_dir_all(&dir);
  }

  #[test]
  fn test_next_prev_hunk_empty() {
    let mut state = PreviewState::new("base16-ocean.dark", Theme::dark());
    let path = PathBuf::from("/fake/no_hunks");
    let content = PreviewContent {
      lines: vec![Line::from("no diff")],
      preview_type: PreviewType::Diff,
      line_count: 0,
      file_size: 0,
      extension: String::new(),
      metadata: None,
      image_metadata: None,
      git_commits: Vec::new(),
      blame_data: None,
      raw_lines: None,
      is_structured: false,
      diff_hunks: Vec::new(), // no hunks
    };
    state.insert_cache(path.clone(), content);
    state.current_path = Some(path);

    assert!(!state.next_hunk());
    assert!(!state.prev_hunk());
  }

  #[test]
  fn test_next_prev_hunk_navigation() {
    let mut state = PreviewState::new("base16-ocean.dark", Theme::dark());
    let path = PathBuf::from("/fake/with_hunks");
    let content = PreviewContent {
      lines: (0..30).map(|i| Line::from(format!("line {i}"))).collect(),
      preview_type: PreviewType::Diff,
      line_count: 0,
      file_size: 0,
      extension: String::new(),
      metadata: None,
      image_metadata: None,
      git_commits: Vec::new(),
      blame_data: None,
      raw_lines: None,
      is_structured: false,
      diff_hunks: vec![5, 15, 25],
    };
    state.insert_cache(path.clone(), content);
    state.current_path = Some(path);
    state.scroll_offset = 0;

    // Next hunk from 0 should go to 5
    assert!(state.next_hunk());
    assert_eq!(state.scroll_offset, 5);

    // Next hunk from 5 should go to 15
    assert!(state.next_hunk());
    assert_eq!(state.scroll_offset, 15);

    // Prev hunk from 15 should go to 5
    assert!(state.prev_hunk());
    assert_eq!(state.scroll_offset, 5);

    // Prev hunk from 5 should not find one (0 < 5, no hunk at 0)
    assert!(!state.prev_hunk());
    assert_eq!(state.scroll_offset, 5);
  }
}
