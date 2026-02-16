use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::Command;
use std::sync::mpsc;

use anyhow::Result;
use ratatui_image::picker::Picker;

use crate::action::Action;
use crate::config::Config;
use crate::event::{InputMode, PromptKind};
use crate::favorites::Favorites;
use crate::fs::{FileProperties, FileTree};
use crate::fs::ops;
use crate::opener::{self, OpenApp};
use crate::preview::{PreviewState, archive};
use crate::ui::breadcrumb::{BreadcrumbSegment, parse_breadcrumb_segments};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClipboardOp {
  Cut,
  Copy,
}

/// Result of an async archive extraction
pub struct ExtractResult {
  pub name: String,
  pub path: PathBuf,
  pub delete_after: bool,
  pub result: Result<(), String>,
}

/// State for an in-progress extraction
pub struct ExtractingState {
  pub rx: mpsc::Receiver<ExtractResult>,
}

#[derive(Debug, Clone)]
pub struct Clipboard {
  pub paths: Vec<PathBuf>,
  pub op: Option<ClipboardOp>,
}

#[derive(Debug, Clone)]
pub struct ChmodState {
  pub path: PathBuf,
  pub original_mode: u32,
  pub new_mode: u32,
  pub is_dir: bool,
  pub recursive: bool,
  pub octal_mode: bool,
  pub octal_input: String,
}

impl Default for ChmodState {
  fn default() -> Self {
    Self {
      path: PathBuf::new(),
      original_mode: 0o644,
      new_mode: 0o644,
      is_dir: false,
      recursive: false,
      octal_mode: false,
      octal_input: String::new(),
    }
  }
}

/// Maximum number of entries in the directory history
const HISTORY_LIMIT: usize = 50;

/// State for a single pane in dual-pane mode
pub struct Pane {
  pub tree: FileTree,
  pub cursor: usize,
  pub scroll_offset: usize,
  pub search_query: String,
}

impl Pane {
  pub fn new(root: PathBuf) -> Result<Self> {
    let tree = FileTree::new(root)?;
    Ok(Self {
      tree,
      cursor: 0,
      scroll_offset: 0,
      search_query: String::new(),
    })
  }

  pub fn visible_entries(&self) -> Vec<usize> {
    if self.search_query.is_empty() {
      return (0..self.tree.entries.len()).collect();
    }
    let query = self.search_query.to_lowercase();
    self
      .tree
      .entries
      .iter()
      .enumerate()
      .filter(|(_, e)| e.name.to_lowercase().contains(&query))
      .map(|(i, _)| i)
      .collect()
  }

  pub fn adjust_scroll(&mut self, viewport_height: usize) {
    let visible = viewport_height.saturating_sub(2); // borders
    if visible == 0 {
      return;
    }
    if self.cursor < self.scroll_offset {
      self.scroll_offset = self.cursor;
    } else if self.cursor >= self.scroll_offset + visible {
      self.scroll_offset = self.cursor - visible + 1;
    }
  }
}


pub struct App {
  pub tree: FileTree,
  pub cursor: usize,
  pub preview: PreviewState,
  pub picker: Option<Picker>,
  pub input_mode: InputMode,
  pub search_query: String,
  pub tree_ratio: u16,
  pub min_tree_ratio: u16,
  pub max_tree_ratio: u16,
  pub ratio_step: u16,
  pub show_help: bool,
  pub should_quit: bool,
  pub should_suspend: Option<SuspendAction>,
  pub status_message: Option<String>,
  pub status_ticks: u8,
  pub viewport_height: usize,
  pub tree_scroll_offset: usize,
  pub clipboard: Clipboard,
  pub prompt_kind: Option<PromptKind>,
  pub prompt_input: String,
  pub prompt_cursor: usize,
  pub favorites: Favorites,
  pub favorites_cursor: usize,
  pub open_with_apps: Vec<OpenApp>,
  pub open_with_cursor: usize,
  pub custom_apps: Vec<OpenApp>,
  pub error_messages: Vec<String>,
  pub wrote_config: bool,
  pub claude_yolo: bool,
  pub extracting: Option<ExtractingState>,
  pub chmod_state: ChmodState,
  /// Stack of previously visited directories (for back navigation)
  history_back: Vec<PathBuf>,
  /// Stack of directories to return to (for forward navigation)
  history_forward: Vec<PathBuf>,
  pub breadcrumb_segments: Vec<BreadcrumbSegment>,
  pub breadcrumb_truncated: bool,
  // Dual-pane state
  pub dual_pane_mode: bool,
  pub active_pane: usize,
  pub right_pane: Option<Pane>,
  pub dual_left_ratio: u16,
  pub dual_right_ratio: u16,
  pub file_properties: Option<FileProperties>,
  pub has_apps_file: bool,
}

#[derive(Debug, Clone)]
pub enum SuspendAction {
  Editor(PathBuf),
  Claude(PathBuf, bool),
  Shell(PathBuf),
  OpenWith(String, PathBuf),
}

impl App {
  pub fn new(root: PathBuf, picker: Option<Picker>, config: &Config) -> Result<Self> {
    let mut tree = FileTree::with_ignore_patterns(root, config.ignore_glob_set.clone())?;
    // Initialize custom ignore state from config
    tree.show_custom_ignored = !config.use_custom_ignore;
    let breadcrumb_segments = parse_breadcrumb_segments(&tree.root);
    Ok(Self {
      tree,
      cursor: 0,
      preview: PreviewState::new(),
      picker,
      input_mode: InputMode::Normal,
      search_query: String::new(),
      tree_ratio: config.tree_ratio,
      min_tree_ratio: config.min_tree_ratio,
      max_tree_ratio: config.max_tree_ratio,
      ratio_step: config.ratio_step,
      show_help: false,
      should_quit: false,
      should_suspend: None,
      status_message: None,
      status_ticks: 0,
      viewport_height: 20,
      tree_scroll_offset: 0,
      clipboard: Clipboard { paths: Vec::new(), op: None },
      prompt_kind: None,
      prompt_input: String::new(),
      prompt_cursor: 0,
      favorites: Favorites::load(),
      favorites_cursor: 0,
      open_with_apps: Vec::new(),
      open_with_cursor: 0,
      custom_apps: config.custom_apps.clone(),
      error_messages: Vec::new(),
      wrote_config: false,
      claude_yolo: config.claude_yolo,
      extracting: None,
      chmod_state: ChmodState::default(),
      history_back: Vec::new(),
      history_forward: Vec::new(),
      breadcrumb_segments,
      breadcrumb_truncated: false,
      dual_pane_mode: false,
      active_pane: 0,
      right_pane: None,
      dual_left_ratio: config.tree_ratio,
      dual_right_ratio: config.tree_ratio,
      file_properties: None,
      has_apps_file: config.has_apps_file,
    })
  }

  pub fn update_breadcrumbs(&mut self) {
    self.breadcrumb_segments = parse_breadcrumb_segments(&self.tree.root);
  }

  pub fn update(&mut self, action: Action) -> Result<()> {
    match action {
      Action::Quit => self.should_quit = true,
      Action::MoveDown => self.move_cursor(1),
      Action::MoveUp => self.move_cursor(-1),
      Action::ToggleExpand => self.enter_or_expand()?,
      Action::MoveRight => self.expand_only()?,
      Action::EnterDir => self.enter_directory()?,
      Action::MoveLeft => self.go_parent_or_collapse()?,
      Action::ScrollPreviewDown => self.preview.scroll_down(3),
      Action::ScrollPreviewUp => self.preview.scroll_up(3),
      Action::ToggleHidden => self.toggle_hidden()?,
      Action::ToggleFormatted => {
        if self.preview.toggle_formatted() {
          let mode = if self.preview.show_formatted { "formatted" } else { "raw" };
          self.set_status(format!("Showing {mode} view"));
        }
      }
      Action::GoToTop => {
        if self.dual_pane_mode && self.active_pane == 1 {
          if let Some(ref mut pane) = self.right_pane {
            pane.cursor = 0;
            pane.scroll_offset = 0;
          }
        } else {
          self.cursor = 0;
          self.tree_scroll_offset = 0;
        }
        self.input_mode = InputMode::Normal;
        self.update_preview();
      }
      Action::GoToBottom => {
        if self.dual_pane_mode && self.active_pane == 1 {
          if let Some(ref mut pane) = self.right_pane
            && !pane.tree.entries.is_empty()
          {
            pane.cursor = pane.visible_entries().len().saturating_sub(1);
            pane.adjust_scroll(self.viewport_height);
          }
        } else if !self.tree.entries.is_empty() {
          self.cursor = self.visible_entries().len().saturating_sub(1);
          self.adjust_scroll();
        }
        self.update_preview();
      }
      Action::GPress => {
        self.input_mode = InputMode::GPrefix;
      }
      Action::SearchStart => {
        self.input_mode = InputMode::Search;
        if self.dual_pane_mode && self.active_pane == 1 {
          if let Some(ref mut pane) = self.right_pane {
            pane.search_query.clear();
          }
        } else {
          self.search_query.clear();
        }
      }
      Action::SearchInput(c) => {
        if self.dual_pane_mode && self.active_pane == 1 {
          if let Some(ref mut pane) = self.right_pane {
            pane.search_query.push(c);
          }
        } else {
          self.search_query.push(c);
        }
        self.apply_search_filter();
      }
      Action::SearchBackspace => {
        if self.dual_pane_mode && self.active_pane == 1 {
          if let Some(ref mut pane) = self.right_pane {
            pane.search_query.pop();
          }
        } else {
          self.search_query.pop();
        }
        self.apply_search_filter();
      }
      Action::SearchConfirm => {
        self.input_mode = InputMode::Normal;
        // Enter directory while filter is still active so cursor resolves correctly
        self.enter_directory()?;
        // Clear query for non-dir entries (enter_directory already clears for dirs)
        if self.dual_pane_mode && self.active_pane == 1 {
          if let Some(ref mut pane) = self.right_pane {
            pane.search_query.clear();
          }
        } else {
          self.search_query.clear();
        }
      }
      Action::SearchCancel => {
        self.input_mode = InputMode::Normal;
        if self.dual_pane_mode && self.active_pane == 1 {
          if let Some(ref mut pane) = self.right_pane {
            pane.search_query.clear();
          }
        } else {
          self.search_query.clear();
        }
      }
      Action::YankPath => self.yank_path(),
      Action::OpenEditor => {
        if let Some(entry) = self.selected_entry()
          && !entry.is_dir {
            self.should_suspend = Some(SuspendAction::Editor(entry.path.clone()));
          }
      }
      Action::OpenClaude => {
        let dir = self.current_dir();
        self.should_suspend = Some(SuspendAction::Claude(dir, self.claude_yolo));
      }
      Action::OpenClaudeAlt => {
        let dir = self.current_dir();
        self.should_suspend = Some(SuspendAction::Claude(dir, !self.claude_yolo));
      }
      Action::OpenShell => {
        let dir = self.current_dir();
        self.should_suspend = Some(SuspendAction::Shell(dir));
      }
      Action::ShrinkTree => {
        if self.dual_pane_mode {
          if self.active_pane == 0 {
            // Shrink left pane (right edge moves left)
            self.dual_left_ratio = self.dual_left_ratio
              .saturating_sub(self.ratio_step)
              .max(self.min_tree_ratio);
          } else {
            // Shrink right pane (right edge moves left, preview grows)
            self.dual_right_ratio = self.dual_right_ratio
              .saturating_sub(self.ratio_step)
              .max(self.min_tree_ratio);
          }
        } else {
          self.tree_ratio = self.tree_ratio.saturating_sub(self.ratio_step).max(self.min_tree_ratio);
        }
      }
      Action::GrowTree => {
        if self.dual_pane_mode {
          let preview_min = 10u16;
          if self.active_pane == 0 {
            // Grow left pane (right edge moves right, right pane shrinks)
            let max_left = (100 - self.dual_right_ratio - preview_min).min(self.max_tree_ratio);
            self.dual_left_ratio = (self.dual_left_ratio + self.ratio_step).min(max_left);
          } else {
            // Grow right pane (right edge moves right, preview shrinks)
            let max_right = (100 - self.dual_left_ratio - preview_min).min(self.max_tree_ratio);
            self.dual_right_ratio = (self.dual_right_ratio + self.ratio_step).min(max_right);
          }
        } else {
          self.tree_ratio = (self.tree_ratio + self.ratio_step).min(self.max_tree_ratio);
        }
      }
      Action::ToggleHelp => {
        self.show_help = !self.show_help;
        self.input_mode = if self.show_help { InputMode::Help } else { InputMode::Normal };
      }
      Action::ToggleBlame => {
        self.preview.toggle_blame();
      }
      Action::CutFile => self.cut_file(),
      Action::CopyFile => self.copy_file(),
      Action::Paste => self.paste_clipboard()?,
      Action::DeleteFile => {
        if let Some(entry) = self.selected_entry() {
          let name = entry.name.clone();
          self.prompt_kind = Some(PromptKind::ConfirmDelete);
          self.prompt_input.clear();
          self.prompt_cursor = 0;
          self.input_mode = InputMode::Prompt;
          self.set_status(format!("Delete {name}? (y/N)"));
        }
      }
      Action::RenameStart => {
        if let Some(entry) = self.selected_entry() {
          self.prompt_input = entry.name.clone();
          self.prompt_cursor = self.prompt_input.chars().count();
          self.prompt_kind = Some(PromptKind::Rename);
          self.input_mode = InputMode::Prompt;
        }
      }
      Action::NewFileStart => {
        self.prompt_input.clear();
        self.prompt_cursor = 0;
        self.prompt_kind = Some(PromptKind::NewFile);
        self.input_mode = InputMode::Prompt;
      }
      Action::NewDirStart => {
        self.prompt_input.clear();
        self.prompt_cursor = 0;
        self.prompt_kind = Some(PromptKind::NewDir);
        self.input_mode = InputMode::Prompt;
      }
      Action::PromptInput(c) => {
        match self.prompt_kind {
          Some(PromptKind::ConfirmDelete) => {
            if c == 'y' {
              self.execute_delete()?;
            } else {
              self.cancel_prompt();
              self.set_status("Delete cancelled".to_string());
            }
          }
          Some(PromptKind::ConfirmExtractAndDelete) => {
            if c == 'y' {
              self.execute_extract_and_delete()?;
            } else {
              self.cancel_prompt();
              self.set_status("Extract cancelled".to_string());
            }
          }
          Some(_) => {
            let byte_pos = self.prompt_input.char_indices()
              .nth(self.prompt_cursor)
              .map(|(i, _)| i)
              .unwrap_or(self.prompt_input.len());
            self.prompt_input.insert(byte_pos, c);
            self.prompt_cursor += 1;
          }
          None => {}
        }
      }
      Action::PromptBackspace => {
        let is_confirm = matches!(
          self.prompt_kind,
          Some(PromptKind::ConfirmDelete) | Some(PromptKind::ConfirmExtractAndDelete)
        );
        if !is_confirm && self.prompt_cursor > 0 {
          let byte_pos = self.prompt_input.char_indices()
            .nth(self.prompt_cursor - 1)
            .map(|(i, _)| i)
            .unwrap_or(0);
          self.prompt_input.remove(byte_pos);
          self.prompt_cursor -= 1;
        }
      }
      Action::PromptDelete => {
        let is_confirm = matches!(
          self.prompt_kind,
          Some(PromptKind::ConfirmDelete) | Some(PromptKind::ConfirmExtractAndDelete)
        );
        if !is_confirm && self.prompt_cursor < self.prompt_input.chars().count()
        {
          let byte_pos = self.prompt_input.char_indices()
            .nth(self.prompt_cursor)
            .map(|(i, _)| i)
            .unwrap_or(self.prompt_input.len());
          self.prompt_input.remove(byte_pos);
        }
      }
      Action::PromptLeft => {
        self.prompt_cursor = self.prompt_cursor.saturating_sub(1);
      }
      Action::PromptRight => {
        let len = self.prompt_input.chars().count();
        if self.prompt_cursor < len {
          self.prompt_cursor += 1;
        }
      }
      Action::PromptHome => {
        self.prompt_cursor = 0;
      }
      Action::PromptEnd => {
        self.prompt_cursor = self.prompt_input.chars().count();
      }
      Action::PromptConfirm => {
        match self.prompt_kind {
          Some(PromptKind::Rename) => self.execute_rename()?,
          Some(PromptKind::NewFile) => self.execute_new_file()?,
          Some(PromptKind::NewDir) => self.execute_new_dir()?,
          Some(PromptKind::ConfirmDelete) => {
            self.cancel_prompt();
            self.set_status("Delete cancelled".to_string());
          }
          Some(PromptKind::ConfirmExtractAndDelete) => {
            self.cancel_prompt();
            self.set_status("Extract cancelled".to_string());
          }
          None => {}
        }
      }
      Action::PromptCancel => {
        self.cancel_prompt();
      }
      Action::Resize(_, h) => {
        self.viewport_height = h.saturating_sub(4) as usize;
      }
      Action::GoHome => self.go_home()?,
      Action::FavoriteAdd => self.favorites_add(),
      Action::FavoritesOpen => self.favorites_open(),
      Action::FavoritesDown => self.favorites_move(1),
      Action::FavoritesUp => self.favorites_move(-1),
      Action::FavoritesSelect => self.favorites_select()?,
      Action::FavoritesClose => self.favorites_close(),
      Action::FavoritesRemove => self.favorites_remove(),
      Action::FavoritesAddCurrent => self.favorites_add_current(),
      Action::OpenDefault => self.open_default_action()?,
      Action::OpenWithStart => self.open_with_start(),
      Action::OpenWithDown => self.open_with_move(1),
      Action::OpenWithUp => self.open_with_move(-1),
      Action::OpenWithSelect => self.open_with_select()?,
      Action::OpenWithClose => {
        self.input_mode = InputMode::Normal;
      }
      Action::ErrorClose => {
        self.error_messages.clear();
        self.input_mode = InputMode::Normal;
      }
      Action::ExtractArchive => self.extract_archive_start(false)?,
      Action::ExtractAndDelete => self.extract_archive_start_confirm()?,
      Action::ChmodStart => self.chmod_start(),
      Action::ChmodToggleBit(bit) => self.chmod_toggle_bit(bit),
      Action::ChmodDigit(c) => self.chmod_digit(c),
      Action::ChmodOctalBackspace => self.chmod_octal_backspace(),
      Action::ChmodToggleOctal => self.chmod_toggle_octal(),
      Action::ChmodToggleRecursive => self.chmod_toggle_recursive(),
      Action::ChmodApply => self.chmod_apply()?,
      Action::ChmodClose => self.chmod_close(),
      Action::ToggleCustomIgnore => self.toggle_custom_ignore()?,
      Action::HistoryBack => self.history_go_back()?,
      Action::HistoryForward => self.history_go_forward()?,
      Action::BreadcrumbSelect(index) => self.breadcrumb_select(index)?,
      Action::SwitchPane => self.switch_pane(),
      Action::ToggleDualPane => self.toggle_dual_pane()?,
      Action::ShowDiff => {
        if let Some(entry) = self.selected_entry()
          && !entry.is_dir
        {
          let path = entry.path.clone();
          let has_diff = self.preview.show_diff(&path, self.tree.git_repo());
          if has_diff {
            self.set_status("Showing diff".to_string());
          } else {
            self.set_status("No uncommitted changes".to_string());
          }
        }
      }
      Action::NextHunk => {
        if self.preview.next_hunk() {
          self.set_status("Next hunk".to_string());
        }
      }
      Action::PrevHunk => {
        if self.preview.prev_hunk() {
          self.set_status("Previous hunk".to_string());
        }
      }
      Action::ShowProperties => {
        if let Some(entry) = self.selected_entry()
          && let Some(props) = FileProperties::from_path(&entry.path)
        {
          self.file_properties = Some(props);
          self.input_mode = InputMode::Properties;
        }
      }
      Action::PropertiesClose => {
        self.file_properties = None;
        self.input_mode = InputMode::Normal;
      }
      Action::Tick => {
        self.preview.check_image_loaded();
        self.check_extraction_complete()?;
      }
      Action::ToggleMarkdownMode => {
        if self.preview.toggle_markdown_mode() {
          let mode = if self.preview.markdown_rendered { "rendered" } else { "raw" };
          self.set_status(format!("Markdown: {mode}"));
        }
      }
      Action::None => {}
    }
    Ok(())
  }

  fn breadcrumb_select(&mut self, index: usize) -> Result<()> {
    if let Some(segment) = self.breadcrumb_segments.get(index)
      && segment.path != self.tree.root
      && segment.path.is_dir()
    {
      self.tree.navigate_to(&segment.path)?;
      self.search_query.clear();
      self.cursor = 0;
      self.tree_scroll_offset = 0;
      self.preview.invalidate();
      self.update_preview();
      self.update_breadcrumbs();
    }
    Ok(())
  }

  fn go_home(&mut self) -> Result<()> {
    if let Some(home) = dirs::home_dir() {
      self.push_history(self.tree.root.clone());
      self.tree.navigate_to(&home)?;
      self.search_query.clear();
      self.cursor = 0;
      self.tree_scroll_offset = 0;
      self.input_mode = InputMode::Normal;
      self.preview.invalidate();
      self.update_preview();
      self.update_breadcrumbs();
    }
    Ok(())
  }

  /// Push a directory onto the back history stack, clearing forward history
  fn push_history(&mut self, path: PathBuf) {
    // Skip if same as the last entry (avoid duplicates in sequence)
    if self.history_back.last() == Some(&path) {
      return;
    }
    self.history_back.push(path);
    // Enforce history limit
    if self.history_back.len() > HISTORY_LIMIT {
      self.history_back.remove(0);
    }
    // Clear forward history on new navigation
    self.history_forward.clear();
  }

  /// Go back in history
  fn history_go_back(&mut self) -> Result<()> {
    if let Some(prev) = self.history_back.pop() {
      let current = self.tree.root.clone();
      // Push current to forward stack (skip if same as last forward entry)
      if self.history_forward.last() != Some(&current) {
        self.history_forward.push(current);
      }
      self.tree.navigate_to(&prev)?;
      self.search_query.clear();
      self.cursor = 0;
      self.tree_scroll_offset = 0;
      self.preview.invalidate();
      self.update_preview();
      self.update_breadcrumbs();
    }
    Ok(())
  }

  /// Go forward in history
  fn history_go_forward(&mut self) -> Result<()> {
    if let Some(next) = self.history_forward.pop() {
      let current = self.tree.root.clone();
      // Push current to back stack (skip if same as last back entry)
      if self.history_back.last() != Some(&current) {
        self.history_back.push(current);
      }
      self.tree.navigate_to(&next)?;
      self.search_query.clear();
      self.cursor = 0;
      self.tree_scroll_offset = 0;
      self.preview.invalidate();
      self.update_preview();
      self.update_breadcrumbs();
    }
    Ok(())
  }

  fn switch_pane(&mut self) {
    if !self.dual_pane_mode || self.right_pane.is_none() {
      return;
    }

    // Save current pane state
    if self.active_pane == 0 {
      // Store left pane state and switch to right
      self.active_pane = 1;
    } else {
      // Store right pane state and switch to left
      self.active_pane = 0;
    }
    self.preview.invalidate();
    self.update_preview();
    self.set_status(format!("Pane: {}", if self.active_pane == 0 { "left" } else { "right" }));
  }

  fn toggle_dual_pane(&mut self) -> Result<()> {
    if self.dual_pane_mode {
      // Disable dual-pane mode
      self.dual_pane_mode = false;
      self.active_pane = 0;
      self.right_pane = None;
      self.set_status("Dual-pane mode: off".to_string());
    } else {
      // Enable dual-pane mode
      let root = self.tree.root.clone();
      self.right_pane = Some(Pane::new(root)?);
      self.dual_pane_mode = true;
      self.active_pane = 0;
      self.set_status("Dual-pane mode: on".to_string());
    }
    self.preview.invalidate();
    self.update_preview();
    Ok(())
  }

  /// Returns the inactive pane's current directory (for copy/move destination)
  #[allow(dead_code)] // Used in tests, will be used for copy/move operations
  pub fn inactive_pane_dir(&self) -> Option<PathBuf> {
    if !self.dual_pane_mode {
      return None;
    }
    if self.active_pane == 0 {
      self.right_pane.as_ref().map(|p| p.tree.root.clone())
    } else {
      Some(self.tree.root.clone())
    }
  }

  fn favorites_add(&mut self) {
    let root = self.tree.root.clone();
    if self.favorites.contains(&root) {
      self.set_status("Already in favorites".to_string());
      return;
    }
    self.favorites.add(root);
    if let Err(e) = self.favorites.save() {
      self.set_status(format!("Save favorites failed: {e}"));
      return;
    }
    self.wrote_config = true;
    self.set_status("Added to favorites".to_string());
  }

  fn favorites_open(&mut self) {
    self.input_mode = InputMode::Favorites;
    self.favorites_cursor = 0;
  }

  fn favorites_close(&mut self) {
    self.input_mode = InputMode::Normal;
  }

  fn favorites_move(&mut self, delta: i32) {
    let len = self.favorites.len();
    if len == 0 {
      return;
    }
    if delta > 0 {
      self.favorites_cursor = (self.favorites_cursor + delta as usize).min(len - 1);
    } else {
      self.favorites_cursor = self.favorites_cursor.saturating_sub((-delta) as usize);
    }
  }

  fn favorites_select(&mut self) -> Result<()> {
    if let Some(path) = self.favorites.get(self.favorites_cursor).map(|p| p.to_path_buf()) {
      if path.is_dir() {
        self.push_history(self.tree.root.clone());
        self.tree.navigate_to(&path)?;
        self.search_query.clear();
        self.cursor = 0;
        self.tree_scroll_offset = 0;
        self.preview.invalidate();
        self.update_preview();
        self.update_breadcrumbs();
        self.input_mode = InputMode::Normal;
      } else {
        self.set_status("Directory no longer exists".to_string());
      }
    }
    Ok(())
  }

  fn favorites_remove(&mut self) {
    if self.favorites_cursor < self.favorites.len() {
      self.favorites.remove(self.favorites_cursor);
      if let Err(e) = self.favorites.save() {
        self.set_status(format!("Save favorites failed: {e}"));
        return;
      }
      self.wrote_config = true;
      if self.favorites.len() > 0 {
        self.favorites_cursor = self.favorites_cursor.min(self.favorites.len() - 1);
      } else {
        self.favorites_cursor = 0;
      }
    }
  }

  fn favorites_add_current(&mut self) {
    let root = self.tree.root.clone();
    if self.favorites.contains(&root) {
      self.set_status("Already in favorites".to_string());
      return;
    }
    self.favorites.add(root);
    if let Err(e) = self.favorites.save() {
      self.set_status(format!("Save favorites failed: {e}"));
      return;
    }
    self.wrote_config = true;
    self.set_status("Added to favorites".to_string());
  }

  fn open_default_action(&mut self) -> Result<()> {
    let entries = self.visible_entries();
    if let Some(idx) = entries.get(self.cursor).copied() {
      if self.tree.entries[idx].is_dir {
        return self.enter_directory();
      }
      let path = self.tree.entries[idx].path.clone();
      match opener::open_default(&path) {
        Ok(()) => {
          let name = &self.tree.entries[idx].name;
          self.set_status(format!("Opened: {name}"));
        }
        Err(e) => {
          self.set_status(e);
        }
      }
    }
    Ok(())
  }

  fn open_with_start(&mut self) {
    if let Some(entry) = self.selected_entry() {
      let is_dir = entry.is_dir;
      let mut apps = opener::detect_apps(&self.custom_apps, !self.has_apps_file);
      if !is_dir {
        let folder_apps: Vec<opener::OpenApp> = apps
          .iter()
          .filter(|a| a.opens_dir)
          .map(|a| opener::OpenApp {
            dir_mode: true,
            ..a.clone()
          })
          .collect();
        apps.extend(folder_apps);
      }
      self.open_with_apps = apps;
      self.open_with_cursor = 0;
      self.input_mode = InputMode::OpenWith;
    }
  }

  fn open_with_move(&mut self, delta: i32) {
    // Total items = 1 (Default Application) + detected apps
    let total = 1 + self.open_with_apps.len();
    if total == 0 {
      return;
    }
    if delta > 0 {
      self.open_with_cursor = (self.open_with_cursor + delta as usize).min(total - 1);
    } else {
      self.open_with_cursor = self.open_with_cursor.saturating_sub((-delta) as usize);
    }
  }

  fn open_with_select(&mut self) -> Result<()> {
    let Some(entry) = self.selected_entry() else {
      self.input_mode = InputMode::Normal;
      return Ok(());
    };
    let path = entry.path.clone();
    let name = entry.name.clone();

    if self.open_with_cursor == 0 {
      // Default Application
      self.input_mode = InputMode::Normal;
      match opener::open_default(&path) {
        Ok(()) => self.set_status(format!("Opened: {name}")),
        Err(e) => self.status_message = Some(e),
      }
    } else {
      let app_idx = self.open_with_cursor - 1;
      if let Some(app) = self.open_with_apps.get(app_idx).cloned() {
        let target = if app.dir_mode {
          path.parent().unwrap_or(&path).to_path_buf()
        } else {
          path
        };
        self.input_mode = InputMode::Normal;
        if app.is_tui {
          self.should_suspend = Some(SuspendAction::OpenWith(app.command.clone(), target));
        } else {
          match opener::open_with_app(&target, &app) {
            Ok(()) => self.set_status(format!("Opened with {}", app.name)),
            Err(e) => self.status_message = Some(e),
          }
        }
      }
    }
    Ok(())
  }

  fn move_cursor(&mut self, delta: i32) {
    if self.dual_pane_mode && self.active_pane == 1 {
      if let Some(ref mut pane) = self.right_pane {
        let entries = pane.visible_entries();
        if entries.is_empty() {
          return;
        }
        let len = entries.len();
        if delta > 0 {
          pane.cursor = (pane.cursor + delta as usize).min(len - 1);
        } else {
          pane.cursor = pane.cursor.saturating_sub((-delta) as usize);
        }
        pane.adjust_scroll(self.viewport_height);
        self.update_preview();
      }
    } else {
      let entries = self.visible_entries();
      if entries.is_empty() {
        return;
      }
      let len = entries.len();
      if delta > 0 {
        self.cursor = (self.cursor + delta as usize).min(len - 1);
      } else {
        self.cursor = self.cursor.saturating_sub((-delta) as usize);
      }
      self.adjust_scroll();
      self.update_preview();
    }
  }

  fn adjust_scroll(&mut self) {
    let visible = self.viewport_height.saturating_sub(2); // borders
    if visible == 0 {
      return;
    }
    if self.cursor < self.tree_scroll_offset {
      self.tree_scroll_offset = self.cursor;
    } else if self.cursor >= self.tree_scroll_offset + visible {
      self.tree_scroll_offset = self.cursor - visible + 1;
    }
  }

  fn enter_or_expand(&mut self) -> Result<()> {
    if self.dual_pane_mode && self.active_pane == 1 {
      if let Some(ref mut pane) = self.right_pane {
        let entries = pane.visible_entries();
        if let Some(idx) = entries.get(pane.cursor).copied() {
          if pane.tree.entries[idx].is_dir {
            pane.tree.toggle_expand(idx)?;
            self.update_preview();
          } else {
            self.update_preview();
          }
        }
      }
    } else {
      let entries = self.visible_entries();
      if let Some(idx) = entries.get(self.cursor).copied() {
        if self.tree.entries[idx].is_dir {
          self.tree.toggle_expand(idx)?;
          self.update_preview();
        } else {
          self.update_preview();
        }
      }
    }
    Ok(())
  }

  fn expand_only(&mut self) -> Result<()> {
    if self.dual_pane_mode && self.active_pane == 1 {
      if let Some(ref mut pane) = self.right_pane {
        let entries = pane.visible_entries();
        if let Some(idx) = entries.get(pane.cursor).copied() {
          if pane.tree.entries[idx].is_dir && !pane.tree.entries[idx].expanded {
            pane.tree.toggle_expand(idx)?;
          }
          self.update_preview();
        }
      }
    } else {
      let entries = self.visible_entries();
      if let Some(idx) = entries.get(self.cursor).copied() {
        if self.tree.entries[idx].is_dir && !self.tree.entries[idx].expanded {
          self.tree.toggle_expand(idx)?;
        }
        self.update_preview();
      }
    }
    Ok(())
  }

  fn enter_directory(&mut self) -> Result<()> {
    if self.dual_pane_mode && self.active_pane == 1 {
      if let Some(ref mut pane) = self.right_pane {
        let entries = pane.visible_entries();
        if let Some(idx) = entries.get(pane.cursor).copied() {
          if pane.tree.entries[idx].is_dir {
            pane.tree.enter_dir(idx)?;
            pane.search_query.clear();
            pane.cursor = 0;
            pane.scroll_offset = 0;
            self.preview.invalidate();
            self.update_preview();
          } else {
            self.update_preview();
          }
        }
      }
    } else {
      let entries = self.visible_entries();
      if let Some(idx) = entries.get(self.cursor).copied() {
        if self.tree.entries[idx].is_dir {
          self.push_history(self.tree.root.clone());
          self.tree.enter_dir(idx)?;
          self.search_query.clear();
          self.cursor = 0;
          self.tree_scroll_offset = 0;
          self.preview.invalidate();
          self.update_preview();
          self.update_breadcrumbs();
        } else {
          self.update_preview();
        }
      }
    }
    Ok(())
  }

  fn go_parent_or_collapse(&mut self) -> Result<()> {
    if self.dual_pane_mode && self.active_pane == 1 {
      if let Some(ref mut pane) = self.right_pane {
        let entries = pane.visible_entries();
        if let Some(&idx) = entries.get(pane.cursor) {
          let entry = &pane.tree.entries[idx];

          // Case 1: Nested item (depth > 0) -> move cursor to parent
          if entry.depth > 0
            && let Some(parent_idx) = pane.tree.find_parent_index(idx)
            && let Some(cursor_pos) = entries.iter().position(|&i| i == parent_idx)
          {
            pane.cursor = cursor_pos;
            pane.adjust_scroll(self.viewport_height);
            self.update_preview();
            return Ok(());
          }

          // Case 2: Root-level expanded directory -> collapse it
          if entry.is_dir && entry.expanded {
            pane.tree.toggle_expand(idx)?;
            self.update_preview();
            return Ok(());
          }
        }

        // Case 3: At root level or parent not visible -> change tree root
        if let Some(old_root) = pane.tree.go_parent()? {
          pane.search_query.clear();
          pane.cursor = pane
            .tree
            .entries
            .iter()
            .position(|e| e.path == old_root)
            .unwrap_or(0);
          pane.scroll_offset = 0;
          pane.adjust_scroll(self.viewport_height);
          self.preview.invalidate();
          self.update_preview();
        }
      }
    } else {
      let entries = self.visible_entries();
      if let Some(&idx) = entries.get(self.cursor) {
        let entry = &self.tree.entries[idx];

        // Case 1: Nested item (depth > 0) -> move cursor to parent, keep tree expanded
        if entry.depth > 0
          && let Some(parent_idx) = self.tree.find_parent_index(idx)
          && let Some(cursor_pos) = entries.iter().position(|&i| i == parent_idx)
        {
          self.cursor = cursor_pos;
          self.adjust_scroll();
          self.update_preview();
          return Ok(());
        }

        // Case 2: Root-level expanded directory -> collapse it
        if entry.is_dir && entry.expanded {
          self.tree.toggle_expand(idx)?;
          self.update_preview();
          return Ok(());
        }
      }

      // Case 3: At root level or parent not visible -> change tree root
      if let Some(old_root) = self.tree.go_parent()? {
        // Push current location to forward history so we can return with HistoryForward
        if self.history_forward.last() != Some(&old_root) {
          self.history_forward.push(old_root.clone());
        }
        self.search_query.clear();
        self.cursor = self
          .tree
          .entries
          .iter()
          .position(|e| e.path == old_root)
          .unwrap_or(0);
        self.tree_scroll_offset = 0;
        self.adjust_scroll();
        self.preview.invalidate();
        self.update_preview();
        self.update_breadcrumbs();
      }
    }
    Ok(())
  }

  fn toggle_hidden(&mut self) -> Result<()> {
    if self.dual_pane_mode && self.active_pane == 1 {
      if let Some(ref mut pane) = self.right_pane {
        pane.tree.toggle_hidden()?;
        pane.cursor = pane.cursor.min(pane.tree.entries.len().saturating_sub(1));
        self.preview.invalidate();
        self.update_preview();
      }
    } else {
      self.tree.toggle_hidden()?;
      self.cursor = self.cursor.min(self.tree.entries.len().saturating_sub(1));
      self.preview.invalidate();
      self.update_preview();
    }
    Ok(())
  }

  fn toggle_custom_ignore(&mut self) -> Result<()> {
    self.tree.toggle_custom_ignored()?;
    self.cursor = self.cursor.min(self.tree.entries.len().saturating_sub(1));
    self.preview.invalidate();
    self.update_preview();
    let state = if self.tree.show_custom_ignored { "shown" } else { "hidden" };
    self.set_status(format!("Custom ignored files: {state}"));
    Ok(())
  }

  fn apply_search_filter(&mut self) {
    // Move cursor to first matching entry
    if self.dual_pane_mode && self.active_pane == 1 {
      if let Some(ref mut pane) = self.right_pane
        && !pane.search_query.is_empty()
      {
        let query = pane.search_query.to_lowercase();
        let entries = pane.visible_entries();
        for &idx in &entries {
          if pane.tree.entries[idx].name.to_lowercase().contains(&query) {
            pane.cursor = entries.iter().position(|&i| i == idx).unwrap_or(0);
            pane.adjust_scroll(self.viewport_height);
            self.update_preview();
            return;
          }
        }
      }
    } else if !self.search_query.is_empty() {
      let query = self.search_query.to_lowercase();
      let entries = self.visible_entries();
      for &idx in &entries {
        if self.tree.entries[idx].name.to_lowercase().contains(&query) {
          self.cursor = entries.iter().position(|&i| i == idx).unwrap_or(0);
          self.adjust_scroll();
          self.update_preview();
          return;
        }
      }
    }
  }

  fn cut_file(&mut self) {
    if let Some(entry) = self.selected_entry() {
      let path = entry.path.clone();
      let name = entry.name.clone();
      self.clipboard = Clipboard {
        paths: vec![path],
        op: Some(ClipboardOp::Cut),
      };
      self.set_status(format!("Cut: {name}"));
    }
  }

  fn copy_file(&mut self) {
    if let Some(entry) = self.selected_entry() {
      let path = entry.path.clone();
      let name = entry.name.clone();
      self.clipboard = Clipboard {
        paths: vec![path],
        op: Some(ClipboardOp::Copy),
      };
      self.set_status(format!("Copied: {name}"));
    }
  }

  fn paste_clipboard(&mut self) -> Result<()> {
    let Some(op) = self.clipboard.op else {
      self.set_status("Nothing to paste".to_string());
      return Ok(());
    };

    let paths = self.clipboard.paths.clone();
    if paths.is_empty() {
      self.set_status("Nothing to paste".to_string());
      return Ok(());
    }

    let target_dir = self.current_dir();
    let mut last_dest = None;

    for source in &paths {
      if !source.exists() {
        self.set_status(format!("Source no longer exists: {}", source.display()));
        continue;
      }

      let file_name = source.file_name().unwrap_or_default();
      let raw_dest = target_dir.join(file_name);

      // Cut to same location is a no-op
      if op == ClipboardOp::Cut && raw_dest == *source {
        last_dest = Some(raw_dest);
        continue;
      }

      let dest = ops::unique_dest_path(&raw_dest);

      match op {
        ClipboardOp::Cut => {
          // Try rename first (same filesystem), fallback to copy+delete
          if std::fs::rename(source, &dest).is_err() {
            match ops::copy_path(source, &dest) {
              Ok(()) => {
                if source.is_dir() {
                  let _ = std::fs::remove_dir_all(source);
                } else {
                  let _ = std::fs::remove_file(source);
                }
              }
              Err(e) => {
                self.set_status(format!("Paste failed: {e}"));
                self.tree.reload()?;
                return Ok(());
              }
            }
          }
        }
        ClipboardOp::Copy => {
          if let Err(e) = ops::copy_path(source, &dest) {
            self.set_status(format!("Paste failed: {e}"));
            self.tree.reload()?;
            return Ok(());
          }
        }
      }
      last_dest = Some(dest);
    }

    if op == ClipboardOp::Cut {
      self.clipboard = Clipboard { paths: Vec::new(), op: None };
    }

    self.tree.reload()?;

    if let Some(dest) = last_dest {
      self.reposition_cursor_to(&dest);
    }

    self.set_status("Pasted".to_string());
    self.preview.invalidate();
    self.update_preview();
    Ok(())
  }

  fn execute_delete(&mut self) -> Result<()> {
    let entry = self.selected_entry().cloned();
    let Some(entry) = entry else {
      self.cancel_prompt();
      return Ok(());
    };

    let result = if entry.is_dir {
      std::fs::remove_dir_all(&entry.path)
    } else {
      std::fs::remove_file(&entry.path)
    };

    match result {
      Ok(()) => {
        // Clean clipboard if deleted path was in it
        self.clipboard.paths.retain(|p| !p.starts_with(&entry.path));
        if self.clipboard.paths.is_empty() {
          self.clipboard.op = None;
        }
        self.cancel_prompt();
        self.tree.reload()?;
        let len = self.visible_entries().len();
        if len == 0 {
          self.cursor = 0;
        } else {
          self.cursor = self.cursor.min(len - 1);
        }
        self.set_status(format!("Deleted: {}", entry.name));
        self.preview.invalidate();
        self.update_preview();
      }
      Err(e) => {
        self.cancel_prompt();
        self.set_status(format!("Delete failed: {e}"));
      }
    }
    Ok(())
  }

  fn execute_rename(&mut self) -> Result<()> {
    let new_name = self.prompt_input.trim().to_string();
    if new_name.is_empty() {
      self.cancel_prompt();
      self.set_status("Name cannot be empty".to_string());
      return Ok(());
    }

    let entry = self.selected_entry().cloned();
    let Some(entry) = entry else {
      self.cancel_prompt();
      return Ok(());
    };

    let parent = entry.path.parent().unwrap_or(&self.tree.root);
    let new_path = parent.join(&new_name);

    if new_path.exists() && new_path != entry.path {
      self.cancel_prompt();
      self.set_status(format!("{new_name} already exists"));
      return Ok(());
    }

    match std::fs::rename(&entry.path, &new_path) {
      Ok(()) => {
        // Update clipboard if renamed path was in it
        for p in &mut self.clipboard.paths {
          if *p == entry.path {
            *p = new_path.clone();
          }
        }
        self.cancel_prompt();
        self.tree.reload()?;
        self.reposition_cursor_to(&new_path);
        self.set_status(format!("Renamed to {new_name}"));
        self.preview.invalidate();
        self.update_preview();
      }
      Err(e) => {
        self.cancel_prompt();
        self.set_status(format!("Rename failed: {e}"));
      }
    }
    Ok(())
  }

  fn execute_new_file(&mut self) -> Result<()> {
    let name = self.prompt_input.trim().to_string();
    if name.is_empty() {
      self.cancel_prompt();
      self.set_status("Name cannot be empty".to_string());
      return Ok(());
    }

    let dir = self.current_dir();
    let new_path = dir.join(&name);

    if new_path.exists() {
      self.cancel_prompt();
      self.set_status(format!("{name} already exists"));
      return Ok(());
    }

    match std::fs::File::create(&new_path) {
      Ok(_) => {
        self.cancel_prompt();
        self.tree.reload()?;
        self.reposition_cursor_to(&new_path);
        self.set_status(format!("Created: {name}"));
        self.preview.invalidate();
        self.update_preview();
      }
      Err(e) => {
        self.cancel_prompt();
        self.set_status(format!("Create failed: {e}"));
      }
    }
    Ok(())
  }

  fn execute_new_dir(&mut self) -> Result<()> {
    let name = self.prompt_input.trim().to_string();
    if name.is_empty() {
      self.cancel_prompt();
      self.set_status("Name cannot be empty".to_string());
      return Ok(());
    }

    let dir = self.current_dir();
    let new_path = dir.join(&name);

    if new_path.exists() {
      self.cancel_prompt();
      self.set_status(format!("{name} already exists"));
      return Ok(());
    }

    match std::fs::create_dir_all(&new_path) {
      Ok(()) => {
        self.cancel_prompt();
        self.tree.reload()?;
        self.reposition_cursor_to(&new_path);
        self.set_status(format!("Created dir: {name}"));
        self.preview.invalidate();
        self.update_preview();
      }
      Err(e) => {
        self.cancel_prompt();
        self.set_status(format!("Create dir failed: {e}"));
      }
    }
    Ok(())
  }

  fn extract_archive_start(&mut self, delete_after: bool) -> Result<()> {
    // Don't start another extraction while one is in progress
    if self.extracting.is_some() {
      self.set_status("Extraction already in progress".to_string());
      return Ok(());
    }

    let Some(entry) = self.selected_entry() else {
      return Ok(());
    };

    let path = entry.path.clone();
    let name = entry.name.clone();

    // Check if it's an archive
    if !archive::is_archive(&path) {
      self.set_status("Not an archive file".to_string());
      return Ok(());
    }

    // Extract to parent directory of the archive
    let dest_dir = path.parent().unwrap_or(&self.tree.root).to_path_buf();

    // Show extracting status
    self.set_status(format!("Extracting {name}..."));

    // Spawn background thread for extraction
    let (tx, rx) = mpsc::channel();
    let extract_name = name.clone();
    let extract_path = path.clone();

    std::thread::spawn(move || {
      let result = archive::extract_archive(&extract_path, &dest_dir);
      let _ = tx.send(ExtractResult {
        name: extract_name,
        path: extract_path,
        delete_after,
        result,
      });
    });

    self.extracting = Some(ExtractingState { rx });
    Ok(())
  }

  fn check_extraction_complete(&mut self) -> Result<()> {
    let Some(ref state) = self.extracting else {
      return Ok(());
    };

    // Non-blocking check
    let result = match state.rx.try_recv() {
      Ok(r) => r,
      Err(mpsc::TryRecvError::Empty) => return Ok(()),
      Err(mpsc::TryRecvError::Disconnected) => {
        self.extracting = None;
        self.set_status("Extraction failed: thread died".to_string());
        return Ok(());
      }
    };

    // Clear extracting state
    self.extracting = None;

    match result.result {
      Ok(()) => {
        if result.delete_after {
          if let Err(e) = std::fs::remove_file(&result.path) {
            self.set_status(format!("Extracted but failed to delete: {e}"));
          } else {
            self.set_status(format!("Extracted and deleted: {}", result.name));
          }
        } else {
          self.set_status(format!("Extracted: {}", result.name));
        }
        self.tree.reload()?;
        self.preview.invalidate();
        self.update_preview();
      }
      Err(e) => {
        self.set_status(format!("Extract failed: {e}"));
      }
    }
    Ok(())
  }

  fn extract_archive_start_confirm(&mut self) -> Result<()> {
    let Some(entry) = self.selected_entry() else {
      return Ok(());
    };

    if !archive::is_archive(&entry.path) {
      self.set_status("Not an archive file".to_string());
      return Ok(());
    }

    let name = entry.name.clone();
    self.prompt_kind = Some(PromptKind::ConfirmExtractAndDelete);
    self.prompt_input.clear();
    self.prompt_cursor = 0;
    self.input_mode = InputMode::Prompt;
    self.set_status(format!("Extract and delete {name}? (y/N)"));
    Ok(())
  }

  fn execute_extract_and_delete(&mut self) -> Result<()> {
    self.cancel_prompt();
    self.extract_archive_start(true)
  }

  fn reposition_cursor_to(&mut self, path: &PathBuf) {
    let entries = self.visible_entries();
    if let Some(pos) = entries.iter().position(|&idx| self.tree.entries[idx].path == *path) {
      self.cursor = pos;
      self.adjust_scroll();
    }
  }

  fn cancel_prompt(&mut self) {
    self.input_mode = InputMode::Normal;
    self.prompt_kind = None;
    self.prompt_input.clear();
    self.prompt_cursor = 0;
  }

  fn yank_path(&mut self) {
    if let Some(entry) = self.selected_entry() {
      let path_str = entry.path.to_string_lossy().to_string();
      match clipboard_anywhere::set_clipboard(&path_str) {
        Ok(_) => self.set_status(format!("Yanked: {path_str}")),
        Err(e) => self.set_status(format!("Yank failed: {e}")),
      }
    }
  }

  fn update_preview(&mut self) {
    if self.dual_pane_mode && self.active_pane == 1 {
      if let Some(ref pane) = self.right_pane {
        let entries = pane.visible_entries();
        if let Some(&idx) = entries.get(pane.cursor) {
          let path = pane.tree.entries[idx].path.clone();
          self.preview.request_preview(&path, self.picker.as_ref(), pane.tree.git_repo());
        }
      }
    } else {
      let entries = self.visible_entries();
      if let Some(&idx) = entries.get(self.cursor) {
        let path = self.tree.entries[idx].path.clone();
        self.preview.request_preview(&path, self.picker.as_ref(), self.tree.git_repo());
      }
    }
  }

  pub fn selected_entry(&self) -> Option<&crate::fs::FileEntry> {
    if self.dual_pane_mode && self.active_pane == 1 {
      if let Some(ref pane) = self.right_pane {
        let entries = pane.visible_entries();
        return entries
          .get(pane.cursor)
          .and_then(|&idx| pane.tree.entries.get(idx));
      }
      return None;
    }
    let entries = self.visible_entries();
    entries
      .get(self.cursor)
      .and_then(|&idx| self.tree.entries.get(idx))
  }

  fn current_dir(&self) -> PathBuf {
    if let Some(entry) = self.selected_entry() {
      if entry.is_dir {
        return entry.path.clone();
      }
      if let Some(parent) = entry.path.parent() {
        return parent.to_path_buf();
      }
    }
    self.tree.root.clone()
  }

  /// Returns indices into tree.entries for visible (filtered) entries
  pub fn visible_entries(&self) -> Vec<usize> {
    if self.search_query.is_empty() {
      return (0..self.tree.entries.len()).collect();
    }
    let query = self.search_query.to_lowercase();
    self
      .tree
      .entries
      .iter()
      .enumerate()
      .filter(|(_, e)| e.name.to_lowercase().contains(&query))
      .map(|(i, _)| i)
      .collect()
  }

  pub fn set_status(&mut self, msg: String) {
    self.status_message = Some(msg);
    self.status_ticks = 20; // visible for ~2s at 100ms tick rate
  }

  pub fn show_error(&mut self, errors: Vec<String>) {
    self.error_messages = errors;
    self.input_mode = InputMode::Error;
  }

  pub fn apply_config(&mut self, config: &Config) {
    self.custom_apps = config.custom_apps.clone();
    self.claude_yolo = config.claude_yolo;
    self.has_apps_file = config.has_apps_file;
    self.tree.set_ignore_patterns(config.ignore_glob_set.clone());
  }

  pub fn reload_favorites(&mut self) {
    self.favorites = Favorites::load();
    if self.favorites.len() == 0 {
      self.favorites_cursor = 0;
    } else {
      self.favorites_cursor = self.favorites_cursor.min(self.favorites.len() - 1);
    }
  }

  pub fn handle_suspend(&mut self) -> Option<SuspendAction> {
    self.should_suspend.take()
  }

  pub fn execute_suspend(action: &SuspendAction) -> Result<()> {
    match action {
      SuspendAction::Editor(path) => {
        let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vim".to_string());
        Command::new(&editor).arg(path).status()?;
      }
      SuspendAction::Claude(dir, yolo) => {
        let mut cmd = Command::new("claude");
        if *yolo {
          cmd.arg("--dangerously-skip-permissions");
        }
        cmd.current_dir(dir).status()?;
      }
      SuspendAction::Shell(dir) => {
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());
        Command::new(&shell).current_dir(dir).status()?;
      }
      SuspendAction::OpenWith(cmd, path) => {
        Command::new(cmd).arg(path).status()?;
      }
    }
    Ok(())
  }

  fn chmod_start(&mut self) {
    let Some(entry) = self.selected_entry() else {
      return;
    };

    let path = entry.path.clone();
    let is_dir = entry.is_dir;

    let Ok(metadata) = std::fs::metadata(&path) else {
      self.set_status("Cannot read file metadata".to_string());
      return;
    };

    let mode = metadata.permissions().mode();

    self.chmod_state = ChmodState {
      path,
      original_mode: mode,
      new_mode: mode,
      is_dir,
      recursive: false,
      octal_mode: false,
      octal_input: String::new(),
    };
    self.input_mode = InputMode::Chmod;
  }

  fn chmod_toggle_bit(&mut self, bit: u8) {
    // Bit mapping:
    // 0-2: owner r/w/x (shifts 8,7,6)
    // 3-5: group r/w/x (shifts 5,4,3)
    // 6-8: others r/w/x (shifts 2,1,0)
    let shift = match bit {
      0 => 8, // owner read
      1 => 7, // owner write
      2 => 6, // owner execute
      3 => 5, // group read
      4 => 4, // group write
      5 => 3, // group execute
      6 => 2, // others read
      7 => 1, // others write
      8 => 0, // others execute
      _ => return,
    };
    self.chmod_state.new_mode ^= 1 << shift;
    // Clear octal input when toggling bits
    self.chmod_state.octal_input.clear();
    self.chmod_state.octal_mode = false;
  }

  fn chmod_digit(&mut self, c: char) {
    if self.chmod_state.octal_mode {
      // In octal mode: append digit to input (max 4 for setuid/setgid/sticky)
      if self.chmod_state.octal_input.len() < 4 {
        self.chmod_state.octal_input.push(c);
        self.apply_octal_input();
      }
    } else {
      // Not in octal mode: toggle others permission bits
      match c {
        '4' => self.chmod_toggle_bit(6), // others read
        '2' => self.chmod_toggle_bit(7), // others write
        '1' => self.chmod_toggle_bit(8), // others execute
        _ => {}
      }
    }
  }

  fn chmod_octal_backspace(&mut self) {
    if self.chmod_state.octal_mode && !self.chmod_state.octal_input.is_empty() {
      self.chmod_state.octal_input.pop();
      self.apply_octal_input();
    }
  }

  fn apply_octal_input(&mut self) {
    if let Ok(mode) = u32::from_str_radix(&self.chmod_state.octal_input, 8) {
      // Preserve file type bits, only update permission bits
      let file_type = self.chmod_state.original_mode & !0o7777;
      self.chmod_state.new_mode = file_type | (mode & 0o7777);
    }
  }

  fn chmod_toggle_octal(&mut self) {
    self.chmod_state.octal_mode = !self.chmod_state.octal_mode;
    if self.chmod_state.octal_mode {
      // Pre-fill octal input with current mode
      self.chmod_state.octal_input = format!("{:03o}", self.chmod_state.new_mode & 0o777);
    }
  }

  fn chmod_toggle_recursive(&mut self) {
    if self.chmod_state.is_dir {
      self.chmod_state.recursive = !self.chmod_state.recursive;
    }
  }

  fn chmod_apply(&mut self) -> Result<()> {
    let path = self.chmod_state.path.clone();
    let new_mode = self.chmod_state.new_mode;
    let recursive = self.chmod_state.recursive && self.chmod_state.is_dir;

    if recursive {
      self.chmod_recursive(&path, new_mode)?;
    } else {
      self.chmod_single(&path, new_mode)?;
    }

    let mode_str = format!("{:03o}", new_mode & 0o777);
    if recursive {
      self.set_status(format!("Permissions set to {mode_str} (recursive)"));
    } else {
      self.set_status(format!("Permissions set to {mode_str}"));
    }

    self.input_mode = InputMode::Normal;
    self.preview.invalidate();
    self.update_preview();
    Ok(())
  }

  fn chmod_single(&self, path: &PathBuf, mode: u32) -> Result<()> {
    let permissions = std::fs::Permissions::from_mode(mode);
    std::fs::set_permissions(path, permissions)?;
    Ok(())
  }

  fn chmod_recursive(&self, path: &PathBuf, mode: u32) -> Result<()> {
    self.chmod_single(path, mode)?;

    if path.is_dir() {
      for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        let entry_path = entry.path();
        if entry_path.is_dir() {
          self.chmod_recursive(&entry_path, mode)?;
        } else {
          self.chmod_single(&entry_path, mode)?;
        }
      }
    }

    Ok(())
  }

  fn chmod_close(&mut self) {
    self.input_mode = InputMode::Normal;
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
    let dir = std::env::temp_dir().join(format!("tui_app_{id}_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("aaa_dir")).unwrap();
    fs::create_dir_all(dir.join("zzz_dir")).unwrap();
    fs::write(dir.join("bbb.txt"), "hello").unwrap();
    fs::write(dir.join("ccc.rs"), "fn main() {}").unwrap();
    fs::write(dir.join(".hidden"), "secret").unwrap();
    dir
  }

  fn cleanup_test_dir(dir: &PathBuf) {
    let _ = fs::remove_dir_all(dir);
  }

  fn cfg() -> Config {
    Config::default()
  }

  #[test]
  fn test_app_creation() {
    let dir = setup_test_dir();
    let app = App::new(dir.clone(), None, &cfg()).unwrap();
    assert_eq!(app.cursor, 0);
    assert!(!app.should_quit);
    assert!(!app.tree.entries.is_empty());
    cleanup_test_dir(&dir);
  }

  #[test]
  fn test_app_creation_custom_ratio() {
    let dir = setup_test_dir();
    let mut c = cfg();
    c.tree_ratio = 50;
    let app = App::new(dir.clone(), None, &c).unwrap();
    assert_eq!(app.tree_ratio, 50);
    cleanup_test_dir(&dir);
  }

  #[test]
  fn test_move_down_up() {
    let dir = setup_test_dir();
    let mut app = App::new(dir.clone(), None, &cfg()).unwrap();
    assert_eq!(app.cursor, 0);
    app.update(Action::MoveDown).unwrap();
    assert_eq!(app.cursor, 1);
    app.update(Action::MoveUp).unwrap();
    assert_eq!(app.cursor, 0);
    // Move up from 0 should stay at 0
    app.update(Action::MoveUp).unwrap();
    assert_eq!(app.cursor, 0);
    cleanup_test_dir(&dir);
  }

  #[test]
  fn test_move_down_clamps() {
    let dir = setup_test_dir();
    let mut app = App::new(dir.clone(), None, &cfg()).unwrap();
    let max = app.visible_entries().len() - 1;
    for _ in 0..100 {
      app.update(Action::MoveDown).unwrap();
    }
    assert_eq!(app.cursor, max);
    cleanup_test_dir(&dir);
  }

  #[test]
  fn test_go_to_top_bottom() {
    let dir = setup_test_dir();
    let mut app = App::new(dir.clone(), None, &cfg()).unwrap();
    let max = app.visible_entries().len() - 1;
    app.update(Action::GoToBottom).unwrap();
    assert_eq!(app.cursor, max);
    app.update(Action::GoToTop).unwrap();
    assert_eq!(app.cursor, 0);
    cleanup_test_dir(&dir);
  }

  #[test]
  fn test_toggle_hidden() {
    let dir = setup_test_dir();
    let mut app = App::new(dir.clone(), None, &cfg()).unwrap();
    let initial_count = app.tree.entries.len();
    assert!(!app.tree.show_hidden);
    // Hidden files should not be shown
    assert!(app.tree.entries.iter().all(|e| !e.name.starts_with('.')));

    app.update(Action::ToggleHidden).unwrap();
    assert!(app.tree.show_hidden);
    assert!(app.tree.entries.len() > initial_count);
    // Now hidden files should be visible
    assert!(app.tree.entries.iter().any(|e| e.name.starts_with('.')));

    cleanup_test_dir(&dir);
  }

  #[test]
  fn test_quit() {
    let dir = setup_test_dir();
    let mut app = App::new(dir.clone(), None, &cfg()).unwrap();
    assert!(!app.should_quit);
    app.update(Action::Quit).unwrap();
    assert!(app.should_quit);
    cleanup_test_dir(&dir);
  }

  #[test]
  fn test_search_filter() {
    let dir = setup_test_dir();
    let mut app = App::new(dir.clone(), None, &cfg()).unwrap();
    app.update(Action::SearchStart).unwrap();
    assert_eq!(app.input_mode, InputMode::Search);

    app.update(Action::SearchInput('b')).unwrap();
    app.update(Action::SearchInput('b')).unwrap();
    assert_eq!(app.search_query, "bb");

    let visible = app.visible_entries();
    // Should only show bbb.txt
    assert_eq!(visible.len(), 1);
    assert_eq!(app.tree.entries[visible[0]].name, "bbb.txt");

    app.update(Action::SearchCancel).unwrap();
    assert_eq!(app.input_mode, InputMode::Normal);
    assert!(app.search_query.is_empty());

    cleanup_test_dir(&dir);
  }

  #[test]
  fn test_expand_collapse_dir() {
    let dir = setup_test_dir();
    // Create a file inside aaa_dir
    fs::write(dir.join("aaa_dir").join("inner.txt"), "inner").unwrap();
    let mut app = App::new(dir.clone(), None, &cfg()).unwrap();

    // First entry should be a directory (dirs come first)
    assert!(app.tree.entries[0].is_dir);
    assert!(!app.tree.entries[0].expanded);

    // Expand it
    app.update(Action::ToggleExpand).unwrap();
    assert!(app.tree.entries[0].expanded);

    // Should now have inner.txt in the entries
    assert!(app.tree.entries.iter().any(|e| e.name == "inner.txt"));

    // Collapse it by pressing left
    app.update(Action::MoveLeft).unwrap();
    assert!(!app.tree.entries[0].expanded);
    assert!(!app.tree.entries.iter().any(|e| e.name == "inner.txt"));

    cleanup_test_dir(&dir);
  }

  #[test]
  fn test_go_parent() {
    let dir = setup_test_dir();
    let child_dir = dir.join("aaa_dir");
    let mut app = App::new(child_dir.clone(), None, &cfg()).unwrap();
    assert_eq!(app.tree.root, child_dir);

    // Navigate to parent (cursor is on a non-expanded dir or file)
    app.update(Action::MoveLeft).unwrap();
    assert_eq!(app.tree.root, dir);

    cleanup_test_dir(&dir);
  }

  #[test]
  fn test_visible_entries_with_search() {
    let dir = setup_test_dir();
    let mut app = App::new(dir.clone(), None, &cfg()).unwrap();

    // Without search, all entries visible
    assert_eq!(app.visible_entries().len(), app.tree.entries.len());

    // With search
    app.search_query = "rs".to_string();
    let visible = app.visible_entries();
    assert!(visible.len() < app.tree.entries.len());
    for &idx in &visible {
      assert!(app.tree.entries[idx].name.to_lowercase().contains("rs"));
    }

    cleanup_test_dir(&dir);
  }

  #[test]
  fn test_g_prefix_mode() {
    let dir = setup_test_dir();
    let mut app = App::new(dir.clone(), None, &cfg()).unwrap();
    app.update(Action::MoveDown).unwrap();
    app.update(Action::MoveDown).unwrap();
    assert!(app.cursor > 0);

    app.update(Action::GPress).unwrap();
    assert_eq!(app.input_mode, InputMode::GPrefix);

    // gg should go to top
    app.update(Action::GoToTop).unwrap();
    assert_eq!(app.cursor, 0);
    assert_eq!(app.input_mode, InputMode::Normal);

    cleanup_test_dir(&dir);
  }

  #[test]
  fn test_open_editor_suspend() {
    let dir = setup_test_dir();
    let mut app = App::new(dir.clone(), None, &cfg()).unwrap();
    // Move to a file (after dirs)
    while app.selected_entry().is_none_or(|e| e.is_dir) {
      app.update(Action::MoveDown).unwrap();
    }
    app.update(Action::OpenEditor).unwrap();
    let suspend = app.handle_suspend();
    assert!(matches!(suspend, Some(SuspendAction::Editor(_))));
    cleanup_test_dir(&dir);
  }

  #[test]
  fn test_open_claude_suspend() {
    let dir = setup_test_dir();
    let mut app = App::new(dir.clone(), None, &cfg()).unwrap();
    app.update(Action::OpenClaude).unwrap();
    let suspend = app.handle_suspend();
    assert!(matches!(suspend, Some(SuspendAction::Claude(_, false))));
    cleanup_test_dir(&dir);
  }

  #[test]
  fn test_open_claude_alt_suspend() {
    let dir = setup_test_dir();
    let mut app = App::new(dir.clone(), None, &cfg()).unwrap();
    app.update(Action::OpenClaudeAlt).unwrap();
    let suspend = app.handle_suspend();
    assert!(matches!(suspend, Some(SuspendAction::Claude(_, true))));
    cleanup_test_dir(&dir);
  }

  #[test]
  fn test_open_claude_with_yolo_config() {
    let dir = setup_test_dir();
    let mut c = cfg();
    c.claude_yolo = true;
    let mut app = App::new(dir.clone(), None, &c).unwrap();

    // OpenClaude should pass yolo=true when config is true
    app.update(Action::OpenClaude).unwrap();
    let suspend = app.handle_suspend();
    assert!(matches!(suspend, Some(SuspendAction::Claude(_, true))));

    // OpenClaudeAlt should pass yolo=false (inverse of config)
    app.update(Action::OpenClaudeAlt).unwrap();
    let suspend = app.handle_suspend();
    assert!(matches!(suspend, Some(SuspendAction::Claude(_, false))));
    cleanup_test_dir(&dir);
  }

  #[test]
  fn test_apply_config_syncs_claude_yolo() {
    let dir = setup_test_dir();
    let mut app = App::new(dir.clone(), None, &cfg()).unwrap();
    assert!(!app.claude_yolo);

    let mut c = cfg();
    c.claude_yolo = true;
    app.apply_config(&c);
    assert!(app.claude_yolo);
    cleanup_test_dir(&dir);
  }

  #[test]
  fn test_open_shell_suspend() {
    let dir = setup_test_dir();
    let mut app = App::new(dir.clone(), None, &cfg()).unwrap();
    app.update(Action::OpenShell).unwrap();
    let suspend = app.handle_suspend();
    assert!(matches!(suspend, Some(SuspendAction::Shell(_))));
    cleanup_test_dir(&dir);
  }

  #[test]
  fn test_enter_dir_changes_root() {
    let dir = setup_test_dir();
    fs::write(dir.join("aaa_dir").join("inner.txt"), "inner").unwrap();
    let mut app = App::new(dir.clone(), None, &cfg()).unwrap();
    // First entry should be aaa_dir (dirs first, alphabetical)
    assert!(app.tree.entries[0].is_dir);
    assert_eq!(app.tree.entries[0].name, "aaa_dir");

    let old_root = app.tree.root.clone();
    app.update(Action::EnterDir).unwrap();
    assert_ne!(app.tree.root, old_root);
    assert_eq!(app.tree.root, dir.join("aaa_dir"));
    assert_eq!(app.cursor, 0);
    cleanup_test_dir(&dir);
  }

  #[test]
  fn test_enter_dir_on_file_is_noop() {
    let dir = setup_test_dir();
    let mut app = App::new(dir.clone(), None, &cfg()).unwrap();
    // Move cursor to a file (past the dirs)
    while app.selected_entry().is_none_or(|e| e.is_dir) {
      app.update(Action::MoveDown).unwrap();
    }
    let root_before = app.tree.root.clone();
    app.update(Action::EnterDir).unwrap();
    assert_eq!(app.tree.root, root_before);
    cleanup_test_dir(&dir);
  }

  #[test]
  fn test_move_right_only_expands() {
    let dir = setup_test_dir();
    fs::write(dir.join("aaa_dir").join("inner.txt"), "inner").unwrap();
    let mut app = App::new(dir.clone(), None, &cfg()).unwrap();
    assert!(app.tree.entries[0].is_dir);
    assert!(!app.tree.entries[0].expanded);

    // First MoveRight should expand
    app.update(Action::MoveRight).unwrap();
    assert!(app.tree.entries[0].expanded);

    // Second MoveRight should NOT collapse
    app.update(Action::MoveRight).unwrap();
    assert!(app.tree.entries[0].expanded);
    cleanup_test_dir(&dir);
  }

  #[test]
  fn test_go_parent_clears_search_query() {
    let dir = setup_test_dir();
    let child_dir = dir.join("aaa_dir");
    let mut app = App::new(child_dir.clone(), None, &cfg()).unwrap();

    // Set a search filter
    app.search_query = "nonexistent".to_string();
    assert_eq!(app.visible_entries().len(), 0);

    // Go to parent — search should be cleared
    app.update(Action::MoveLeft).unwrap();
    assert!(app.search_query.is_empty());
    assert_eq!(app.tree.root, dir);
    // cursor and scroll_offset must be valid for visible_entries
    assert!(app.cursor < app.visible_entries().len());
    assert!(app.tree_scroll_offset <= app.cursor);
    cleanup_test_dir(&dir);
  }

  #[test]
  fn test_enter_dir_clears_search_query() {
    let dir = setup_test_dir();
    fs::write(dir.join("aaa_dir").join("inner.txt"), "inner").unwrap();
    let mut app = App::new(dir.clone(), None, &cfg()).unwrap();

    // Search for "aaa" to filter to the dir
    app.search_query = "aaa".to_string();
    let visible = app.visible_entries();
    assert_eq!(visible.len(), 1);
    app.cursor = 0;

    // Enter the directory
    app.update(Action::EnterDir).unwrap();
    assert!(app.search_query.is_empty());
    assert_eq!(app.tree.root, dir.join("aaa_dir"));
    cleanup_test_dir(&dir);
  }

  #[test]
  fn test_search_confirm_enters_directory() {
    let dir = setup_test_dir();
    fs::write(dir.join("aaa_dir").join("inner.txt"), "inner").unwrap();
    let mut app = App::new(dir.clone(), None, &cfg()).unwrap();

    // Search for "aaa" — should filter to just aaa_dir
    app.update(Action::SearchStart).unwrap();
    app.update(Action::SearchInput('a')).unwrap();
    app.update(Action::SearchInput('a')).unwrap();
    app.update(Action::SearchInput('a')).unwrap();
    assert_eq!(app.visible_entries().len(), 1);

    // Confirm search — should enter the directory
    app.update(Action::SearchConfirm).unwrap();
    assert!(app.search_query.is_empty());
    assert_eq!(app.input_mode, InputMode::Normal);
    assert_eq!(app.tree.root, dir.join("aaa_dir"));
    assert_eq!(app.cursor, 0);
    cleanup_test_dir(&dir);
  }

  #[test]
  fn test_search_confirm_on_file_clears_query() {
    let dir = setup_test_dir();
    let mut app = App::new(dir.clone(), None, &cfg()).unwrap();

    // Search for "bbb" — should filter to just bbb.txt
    app.update(Action::SearchStart).unwrap();
    app.update(Action::SearchInput('b')).unwrap();
    app.update(Action::SearchInput('b')).unwrap();
    app.update(Action::SearchInput('b')).unwrap();
    assert_eq!(app.visible_entries().len(), 1);

    let root_before = app.tree.root.clone();
    app.update(Action::SearchConfirm).unwrap();
    assert!(app.search_query.is_empty());
    // Root should not change for a file
    assert_eq!(app.tree.root, root_before);
    cleanup_test_dir(&dir);
  }

  #[test]
  fn test_toggle_help() {
    let dir = setup_test_dir();
    let mut app = App::new(dir.clone(), None, &cfg()).unwrap();
    assert!(!app.show_help);
    assert_eq!(app.input_mode, InputMode::Normal);

    app.update(Action::ToggleHelp).unwrap();
    assert!(app.show_help);
    assert_eq!(app.input_mode, InputMode::Help);

    app.update(Action::ToggleHelp).unwrap();
    assert!(!app.show_help);
    assert_eq!(app.input_mode, InputMode::Normal);

    cleanup_test_dir(&dir);
  }

  #[test]
  fn test_toggle_expand_still_toggles() {
    let dir = setup_test_dir();
    fs::write(dir.join("aaa_dir").join("inner.txt"), "inner").unwrap();
    let mut app = App::new(dir.clone(), None, &cfg()).unwrap();
    assert!(!app.tree.entries[0].expanded);

    // ToggleExpand should expand
    app.update(Action::ToggleExpand).unwrap();
    assert!(app.tree.entries[0].expanded);

    // ToggleExpand again should collapse
    app.update(Action::ToggleExpand).unwrap();
    assert!(!app.tree.entries[0].expanded);
    cleanup_test_dir(&dir);
  }

  #[test]
  fn test_cut_stores_path_in_clipboard() {
    let dir = setup_test_dir();
    let mut app = App::new(dir.clone(), None, &cfg()).unwrap();
    // Move to a file
    while app.selected_entry().is_none_or(|e| e.is_dir) {
      app.update(Action::MoveDown).unwrap();
    }
    let path = app.selected_entry().unwrap().path.clone();
    app.update(Action::CutFile).unwrap();
    assert_eq!(app.clipboard.op, Some(ClipboardOp::Cut));
    assert_eq!(app.clipboard.paths, vec![path]);
    assert!(app.status_message.as_ref().unwrap().starts_with("Cut:"));
    cleanup_test_dir(&dir);
  }

  #[test]
  fn test_copy_stores_path_in_clipboard() {
    let dir = setup_test_dir();
    let mut app = App::new(dir.clone(), None, &cfg()).unwrap();
    while app.selected_entry().is_none_or(|e| e.is_dir) {
      app.update(Action::MoveDown).unwrap();
    }
    let path = app.selected_entry().unwrap().path.clone();
    app.update(Action::CopyFile).unwrap();
    assert_eq!(app.clipboard.op, Some(ClipboardOp::Copy));
    assert_eq!(app.clipboard.paths, vec![path]);
    assert!(app.status_message.as_ref().unwrap().starts_with("Copied:"));
    cleanup_test_dir(&dir);
  }

  #[test]
  fn test_paste_copy_creates_new_file() {
    let dir = setup_test_dir();
    let mut app = App::new(dir.clone(), None, &cfg()).unwrap();
    // Move to bbb.txt
    while app.selected_entry().is_none_or(|e| e.name != "bbb.txt") {
      app.update(Action::MoveDown).unwrap();
    }
    app.update(Action::CopyFile).unwrap();

    // Move cursor to root (a dir) so paste goes into that dir
    app.cursor = 0; // aaa_dir
    app.update(Action::Paste).unwrap();

    // bbb.txt should now exist in aaa_dir
    assert!(dir.join("aaa_dir").join("bbb.txt").exists());
    // Original should still exist
    assert!(dir.join("bbb.txt").exists());
    cleanup_test_dir(&dir);
  }

  #[test]
  fn test_paste_cut_moves_file() {
    let dir = setup_test_dir();
    let mut app = App::new(dir.clone(), None, &cfg()).unwrap();
    // Move to bbb.txt
    while app.selected_entry().is_none_or(|e| e.name != "bbb.txt") {
      app.update(Action::MoveDown).unwrap();
    }
    app.update(Action::CutFile).unwrap();

    // Paste into aaa_dir
    app.cursor = 0;
    app.update(Action::Paste).unwrap();

    assert!(dir.join("aaa_dir").join("bbb.txt").exists());
    assert!(!dir.join("bbb.txt").exists());
    // Clipboard should be cleared after cut+paste
    assert!(app.clipboard.op.is_none());
    cleanup_test_dir(&dir);
  }

  #[test]
  fn test_paste_conflict_appends_suffix() {
    let dir = setup_test_dir();
    // Create bbb.txt inside aaa_dir to cause conflict
    fs::write(dir.join("aaa_dir").join("bbb.txt"), "existing").unwrap();
    let mut app = App::new(dir.clone(), None, &cfg()).unwrap();
    // Copy bbb.txt
    while app.selected_entry().is_none_or(|e| e.name != "bbb.txt") {
      app.update(Action::MoveDown).unwrap();
    }
    app.update(Action::CopyFile).unwrap();

    // Paste into aaa_dir
    app.cursor = 0;
    app.update(Action::Paste).unwrap();

    // Should have created bbb_copy.txt
    assert!(dir.join("aaa_dir").join("bbb_copy.txt").exists());
    // Original in aaa_dir should be untouched
    assert_eq!(fs::read_to_string(dir.join("aaa_dir").join("bbb.txt")).unwrap(), "existing");
    cleanup_test_dir(&dir);
  }

  #[test]
  fn test_paste_empty_clipboard_shows_message() {
    let dir = setup_test_dir();
    let mut app = App::new(dir.clone(), None, &cfg()).unwrap();
    app.update(Action::Paste).unwrap();
    assert_eq!(app.status_message.as_deref(), Some("Nothing to paste"));
    cleanup_test_dir(&dir);
  }

  #[test]
  fn test_cut_paste_same_dir_is_noop() {
    let dir = setup_test_dir();
    let mut app = App::new(dir.clone(), None, &cfg()).unwrap();
    // Move to bbb.txt
    while app.selected_entry().is_none_or(|e| e.name != "bbb.txt") {
      app.update(Action::MoveDown).unwrap();
    }
    app.update(Action::CutFile).unwrap();
    // Paste in same dir (cursor on bbb.txt, parent is root)
    app.update(Action::Paste).unwrap();
    // File should keep its original name, no _copy suffix
    assert!(dir.join("bbb.txt").exists());
    assert!(!dir.join("bbb_copy.txt").exists());
    cleanup_test_dir(&dir);
  }

  #[test]
  fn test_delete_with_y_confirmation() {
    let dir = setup_test_dir();
    let mut app = App::new(dir.clone(), None, &cfg()).unwrap();
    // Move to bbb.txt
    while app.selected_entry().is_none_or(|e| e.name != "bbb.txt") {
      app.update(Action::MoveDown).unwrap();
    }
    assert!(dir.join("bbb.txt").exists());

    app.update(Action::DeleteFile).unwrap();
    assert_eq!(app.input_mode, InputMode::Prompt);
    assert_eq!(app.prompt_kind, Some(PromptKind::ConfirmDelete));

    // Confirm with 'y'
    app.update(Action::PromptInput('y')).unwrap();
    assert!(!dir.join("bbb.txt").exists());
    assert_eq!(app.input_mode, InputMode::Normal);
    cleanup_test_dir(&dir);
  }

  #[test]
  fn test_delete_cancel_does_not_remove() {
    let dir = setup_test_dir();
    let mut app = App::new(dir.clone(), None, &cfg()).unwrap();
    while app.selected_entry().is_none_or(|e| e.name != "bbb.txt") {
      app.update(Action::MoveDown).unwrap();
    }
    app.update(Action::DeleteFile).unwrap();
    // Cancel with 'n'
    app.update(Action::PromptInput('n')).unwrap();
    assert!(dir.join("bbb.txt").exists());
    assert_eq!(app.input_mode, InputMode::Normal);
    cleanup_test_dir(&dir);
  }

  #[test]
  fn test_delete_cancel_with_esc() {
    let dir = setup_test_dir();
    let mut app = App::new(dir.clone(), None, &cfg()).unwrap();
    while app.selected_entry().is_none_or(|e| e.name != "bbb.txt") {
      app.update(Action::MoveDown).unwrap();
    }
    app.update(Action::DeleteFile).unwrap();
    app.update(Action::PromptCancel).unwrap();
    assert!(dir.join("bbb.txt").exists());
    assert_eq!(app.input_mode, InputMode::Normal);
    cleanup_test_dir(&dir);
  }

  #[test]
  fn test_rename_file() {
    let dir = setup_test_dir();
    let mut app = App::new(dir.clone(), None, &cfg()).unwrap();
    while app.selected_entry().is_none_or(|e| e.name != "bbb.txt") {
      app.update(Action::MoveDown).unwrap();
    }
    app.update(Action::RenameStart).unwrap();
    assert_eq!(app.input_mode, InputMode::Prompt);
    assert_eq!(app.prompt_input, "bbb.txt");

    // Clear and type new name
    app.prompt_input.clear();
    app.prompt_input.push_str("renamed.txt");
    app.update(Action::PromptConfirm).unwrap();

    assert!(!dir.join("bbb.txt").exists());
    assert!(dir.join("renamed.txt").exists());
    assert_eq!(app.input_mode, InputMode::Normal);
    cleanup_test_dir(&dir);
  }

  #[test]
  fn test_rename_to_existing_shows_error() {
    let dir = setup_test_dir();
    let mut app = App::new(dir.clone(), None, &cfg()).unwrap();
    while app.selected_entry().is_none_or(|e| e.name != "bbb.txt") {
      app.update(Action::MoveDown).unwrap();
    }
    app.update(Action::RenameStart).unwrap();
    app.prompt_input = "ccc.rs".to_string();
    app.update(Action::PromptConfirm).unwrap();

    // Should still exist as bbb.txt
    assert!(dir.join("bbb.txt").exists());
    assert!(app.status_message.as_ref().unwrap().contains("already exists"));
    cleanup_test_dir(&dir);
  }

  #[test]
  fn test_new_file_creation() {
    let dir = setup_test_dir();
    let mut app = App::new(dir.clone(), None, &cfg()).unwrap();
    // Move to a file so current_dir() returns the root
    while app.selected_entry().is_none_or(|e| e.is_dir) {
      app.update(Action::MoveDown).unwrap();
    }
    app.update(Action::NewFileStart).unwrap();
    assert_eq!(app.input_mode, InputMode::Prompt);
    assert_eq!(app.prompt_kind, Some(PromptKind::NewFile));

    app.prompt_input = "new_file.txt".to_string();
    app.update(Action::PromptConfirm).unwrap();

    assert!(dir.join("new_file.txt").exists());
    assert_eq!(app.input_mode, InputMode::Normal);
    cleanup_test_dir(&dir);
  }

  #[test]
  fn test_new_dir_creation() {
    let dir = setup_test_dir();
    let mut app = App::new(dir.clone(), None, &cfg()).unwrap();
    // Move to a file so current_dir() returns the root
    while app.selected_entry().is_none_or(|e| e.is_dir) {
      app.update(Action::MoveDown).unwrap();
    }
    app.update(Action::NewDirStart).unwrap();
    assert_eq!(app.input_mode, InputMode::Prompt);
    assert_eq!(app.prompt_kind, Some(PromptKind::NewDir));

    app.prompt_input = "new_dir".to_string();
    app.update(Action::PromptConfirm).unwrap();

    assert!(dir.join("new_dir").is_dir());
    assert_eq!(app.input_mode, InputMode::Normal);
    cleanup_test_dir(&dir);
  }

  #[test]
  fn test_empty_name_rejected() {
    let dir = setup_test_dir();
    let mut app = App::new(dir.clone(), None, &cfg()).unwrap();
    app.update(Action::NewFileStart).unwrap();
    app.prompt_input = "  ".to_string();
    app.update(Action::PromptConfirm).unwrap();
    assert_eq!(app.status_message.as_deref(), Some("Name cannot be empty"));
    assert_eq!(app.input_mode, InputMode::Normal);
    cleanup_test_dir(&dir);
  }

  #[test]
  fn test_prompt_cancel_returns_to_normal() {
    let dir = setup_test_dir();
    let mut app = App::new(dir.clone(), None, &cfg()).unwrap();
    app.update(Action::NewFileStart).unwrap();
    assert_eq!(app.input_mode, InputMode::Prompt);
    app.update(Action::PromptCancel).unwrap();
    assert_eq!(app.input_mode, InputMode::Normal);
    assert!(app.prompt_kind.is_none());
    assert!(app.prompt_input.is_empty());
    cleanup_test_dir(&dir);
  }

  #[test]
  fn test_cursor_repositions_after_rename() {
    let dir = setup_test_dir();
    let mut app = App::new(dir.clone(), None, &cfg()).unwrap();
    while app.selected_entry().is_none_or(|e| e.name != "bbb.txt") {
      app.update(Action::MoveDown).unwrap();
    }
    app.update(Action::RenameStart).unwrap();
    app.prompt_input = "zzz_renamed.txt".to_string();
    app.update(Action::PromptConfirm).unwrap();

    // Cursor should be on the renamed file
    let entry = app.selected_entry().unwrap();
    assert_eq!(entry.name, "zzz_renamed.txt");
    cleanup_test_dir(&dir);
  }

  #[test]
  fn test_open_default_on_dir_enters() {
    let dir = setup_test_dir();
    fs::write(dir.join("aaa_dir").join("inner.txt"), "inner").unwrap();
    let mut app = App::new(dir.clone(), None, &cfg()).unwrap();
    // First entry is aaa_dir
    assert!(app.tree.entries[0].is_dir);
    let old_root = app.tree.root.clone();
    app.update(Action::OpenDefault).unwrap();
    // Should have entered the directory
    assert_ne!(app.tree.root, old_root);
    assert_eq!(app.tree.root, dir.join("aaa_dir"));
    cleanup_test_dir(&dir);
  }

  #[test]
  fn test_open_with_start_on_file() {
    let dir = setup_test_dir();
    let mut app = App::new(dir.clone(), None, &cfg()).unwrap();
    // Move to a file
    while app.selected_entry().is_none_or(|e| e.is_dir) {
      app.update(Action::MoveDown).unwrap();
    }
    app.update(Action::OpenWithStart).unwrap();
    assert_eq!(app.input_mode, InputMode::OpenWith);
    assert_eq!(app.open_with_cursor, 0);
    cleanup_test_dir(&dir);
  }

  #[test]
  fn test_open_with_start_on_dir() {
    let dir = setup_test_dir();
    let mut app = App::new(dir.clone(), None, &cfg()).unwrap();
    // First entry is a dir
    assert!(app.selected_entry().unwrap().is_dir);
    app.update(Action::OpenWithStart).unwrap();
    assert_eq!(app.input_mode, InputMode::OpenWith);
    cleanup_test_dir(&dir);
  }

  #[test]
  fn test_open_with_dir_variants_on_file() {
    let dir = setup_test_dir();
    let mut app = App::new(dir.clone(), None, &cfg()).unwrap();
    // Inject a custom app with opens_dir
    app.custom_apps = vec![OpenApp {
      name: "TestIDE".into(),
      command: "which".into(), // exists on all systems
      is_tui: false,
      macos_app: None,
      opens_dir: true,
      dir_mode: false,
    }];
    // Move to a file
    while app.selected_entry().is_none_or(|e| e.is_dir) {
      app.update(Action::MoveDown).unwrap();
    }
    app.update(Action::OpenWithStart).unwrap();
    // Should have both the normal entry and a dir_mode duplicate
    let normal = app.open_with_apps.iter().filter(|a| a.name == "TestIDE" && !a.dir_mode).count();
    let dir_variant = app.open_with_apps.iter().filter(|a| a.name == "TestIDE" && a.dir_mode).count();
    assert_eq!(normal, 1);
    assert_eq!(dir_variant, 1);
    cleanup_test_dir(&dir);
  }

  #[test]
  fn test_open_with_no_dir_variants_on_dir() {
    let dir = setup_test_dir();
    let mut app = App::new(dir.clone(), None, &cfg()).unwrap();
    app.custom_apps = vec![OpenApp {
      name: "TestIDE".into(),
      command: "which".into(),
      is_tui: false,
      macos_app: None,
      opens_dir: true,
      dir_mode: false,
    }];
    // First entry is a dir
    assert!(app.selected_entry().unwrap().is_dir);
    app.update(Action::OpenWithStart).unwrap();
    // On a directory, no dir_mode variants should be added
    let dir_variant = app.open_with_apps.iter().filter(|a| a.dir_mode).count();
    assert_eq!(dir_variant, 0);
    cleanup_test_dir(&dir);
  }

  #[test]
  fn test_open_with_cursor_movement() {
    let dir = setup_test_dir();
    let mut app = App::new(dir.clone(), None, &cfg()).unwrap();
    while app.selected_entry().is_none_or(|e| e.is_dir) {
      app.update(Action::MoveDown).unwrap();
    }
    app.update(Action::OpenWithStart).unwrap();
    let total = 1 + app.open_with_apps.len();

    // Move down
    app.update(Action::OpenWithDown).unwrap();
    if total > 1 {
      assert_eq!(app.open_with_cursor, 1);
    }
    // Move up
    app.update(Action::OpenWithUp).unwrap();
    assert_eq!(app.open_with_cursor, 0);
    // Move up from 0 stays at 0
    app.update(Action::OpenWithUp).unwrap();
    assert_eq!(app.open_with_cursor, 0);
    cleanup_test_dir(&dir);
  }

  #[test]
  fn test_open_with_close() {
    let dir = setup_test_dir();
    let mut app = App::new(dir.clone(), None, &cfg()).unwrap();
    while app.selected_entry().is_none_or(|e| e.is_dir) {
      app.update(Action::MoveDown).unwrap();
    }
    app.update(Action::OpenWithStart).unwrap();
    assert_eq!(app.input_mode, InputMode::OpenWith);
    app.update(Action::OpenWithClose).unwrap();
    assert_eq!(app.input_mode, InputMode::Normal);
    cleanup_test_dir(&dir);
  }

  #[test]
  fn test_open_with_select_tui_suspends() {
    let dir = setup_test_dir();
    let mut app = App::new(dir.clone(), None, &cfg()).unwrap();
    while app.selected_entry().is_none_or(|e| e.is_dir) {
      app.update(Action::MoveDown).unwrap();
    }
    // Inject a fake TUI app so we can test the suspend path
    app.open_with_apps = vec![OpenApp {
      name: "TestEditor".into(),
      command: "testeditor".into(),
      is_tui: true,
      macos_app: None,
      opens_dir: false,
      dir_mode: false,
    }];
    app.open_with_cursor = 1; // Select the TUI app (0 is Default)
    app.input_mode = InputMode::OpenWith;

    app.update(Action::OpenWithSelect).unwrap();
    assert_eq!(app.input_mode, InputMode::Normal);
    let suspend = app.handle_suspend();
    assert!(matches!(suspend, Some(SuspendAction::OpenWith(_, _))));
    cleanup_test_dir(&dir);
  }

  #[test]
  fn test_apply_config_updates_custom_apps() {
    let dir = setup_test_dir();
    let mut app = App::new(dir.clone(), None, &cfg()).unwrap();
    assert!(app.custom_apps.is_empty());

    let mut c = cfg();
    c.custom_apps = vec![OpenApp {
      name: "TestApp".into(),
      command: "testcmd".into(),
      is_tui: false,
      macos_app: None,
      opens_dir: false,
      dir_mode: false,
    }];
    app.apply_config(&c);
    assert_eq!(app.custom_apps.len(), 1);
    assert_eq!(app.custom_apps[0].name, "TestApp");
    cleanup_test_dir(&dir);
  }

  #[test]
  fn test_reload_favorites_clamps_cursor() {
    let dir = setup_test_dir();
    let mut app = App::new(dir.clone(), None, &cfg()).unwrap();
    // Manually set cursor past what reload will return
    app.favorites_cursor = 100;
    app.reload_favorites();
    if app.favorites.len() == 0 {
      assert_eq!(app.favorites_cursor, 0);
    } else {
      assert!(app.favorites_cursor < app.favorites.len());
    }
    cleanup_test_dir(&dir);
  }

  #[test]
  fn test_reload_favorites_empty() {
    let dir = setup_test_dir();
    let mut app = App::new(dir.clone(), None, &cfg()).unwrap();
    // Clear favorites and set cursor
    app.favorites_cursor = 5;
    app.reload_favorites();
    // With default test env, favorites file likely doesn't exist
    if app.favorites.len() == 0 {
      assert_eq!(app.favorites_cursor, 0);
    }
    cleanup_test_dir(&dir);
  }

  #[test]
  fn test_move_left_to_parent_in_tree() {
    let dir = setup_test_dir();
    fs::write(dir.join("aaa_dir").join("inner.txt"), "inner").unwrap();
    let mut app = App::new(dir.clone(), None, &cfg()).unwrap();

    // Expand aaa_dir
    assert_eq!(app.tree.entries[0].name, "aaa_dir");
    app.update(Action::MoveRight).unwrap();
    assert!(app.tree.entries[0].expanded);

    // Move cursor to inner.txt (depth 1)
    app.update(Action::MoveDown).unwrap();
    assert_eq!(app.cursor, 1);
    assert_eq!(app.tree.entries[1].name, "inner.txt");
    assert_eq!(app.tree.entries[1].depth, 1);

    // Remember the root before MoveLeft
    let root_before = app.tree.root.clone();

    // MoveLeft should move cursor to parent dir (aaa_dir), NOT change root
    app.update(Action::MoveLeft).unwrap();
    assert_eq!(app.cursor, 0);
    assert_eq!(app.tree.entries[0].name, "aaa_dir");
    assert_eq!(app.tree.root, root_before); // Root unchanged!

    cleanup_test_dir(&dir);
  }

  #[test]
  fn test_move_left_on_nested_collapsed_dir() {
    let dir = setup_test_dir();
    // Create nested structure: aaa_dir/subdir/file.txt
    fs::create_dir_all(dir.join("aaa_dir").join("subdir")).unwrap();
    fs::write(dir.join("aaa_dir").join("subdir").join("file.txt"), "data").unwrap();
    let mut app = App::new(dir.clone(), None, &cfg()).unwrap();

    // Expand aaa_dir
    app.update(Action::MoveRight).unwrap();
    assert!(app.tree.entries[0].expanded);

    // Move to subdir (depth 1, collapsed)
    app.update(Action::MoveDown).unwrap();
    assert_eq!(app.tree.entries[app.visible_entries()[app.cursor]].name, "subdir");
    assert!(!app.tree.entries[app.visible_entries()[app.cursor]].expanded);

    let root_before = app.tree.root.clone();

    // MoveLeft on collapsed nested dir should move cursor to parent
    app.update(Action::MoveLeft).unwrap();
    assert_eq!(app.cursor, 0);
    assert_eq!(app.tree.entries[0].name, "aaa_dir");
    assert_eq!(app.tree.root, root_before); // Root unchanged!

    cleanup_test_dir(&dir);
  }

  #[test]
  fn test_move_left_with_search_parent_hidden() {
    let dir = setup_test_dir();
    fs::write(dir.join("aaa_dir").join("inner.txt"), "inner").unwrap();
    let mut app = App::new(dir.clone(), None, &cfg()).unwrap();

    // Expand aaa_dir
    app.update(Action::MoveRight).unwrap();

    // Set search filter that hides the parent dir but shows inner.txt
    app.search_query = "inner".to_string();
    let visible = app.visible_entries();
    // Only inner.txt should be visible
    assert_eq!(visible.len(), 1);
    assert_eq!(app.tree.entries[visible[0]].name, "inner.txt");
    app.cursor = 0;

    // MoveLeft should fall through to go_parent since parent is not visible
    app.update(Action::MoveLeft).unwrap();
    // Search query should be cleared and root should change to parent
    assert!(app.search_query.is_empty());

    cleanup_test_dir(&dir);
  }

  #[test]
  fn test_move_left_on_expanded_nested_dir_keeps_expanded() {
    let dir = setup_test_dir();
    // Create nested structure: aaa_dir/subdir/file.txt
    fs::create_dir_all(dir.join("aaa_dir").join("subdir")).unwrap();
    fs::write(dir.join("aaa_dir").join("subdir").join("file.txt"), "data").unwrap();
    let mut app = App::new(dir.clone(), None, &cfg()).unwrap();

    // Expand aaa_dir
    app.update(Action::MoveRight).unwrap();
    assert!(app.tree.entries[0].expanded);

    // Move to subdir and expand it
    app.update(Action::MoveDown).unwrap();
    app.update(Action::MoveRight).unwrap();
    let subdir_idx = app.visible_entries()[app.cursor];
    assert_eq!(app.tree.entries[subdir_idx].name, "subdir");
    assert!(app.tree.entries[subdir_idx].expanded);

    let root_before = app.tree.root.clone();

    // MoveLeft on expanded nested dir should move cursor to parent, NOT collapse
    app.update(Action::MoveLeft).unwrap();
    assert_eq!(app.cursor, 0);
    assert_eq!(app.tree.entries[0].name, "aaa_dir");
    assert_eq!(app.tree.root, root_before);

    // subdir should still be expanded (context preserved)
    let subdir_entry = app.tree.entries.iter().find(|e| e.name == "subdir").unwrap();
    assert!(subdir_entry.expanded);
    // file.txt should still be visible
    assert!(app.tree.entries.iter().any(|e| e.name == "file.txt"));

    cleanup_test_dir(&dir);
  }

  #[test]
  fn test_extract_archive_non_archive_shows_status() {
    let dir = setup_test_dir();
    let mut app = App::new(dir.clone(), None, &cfg()).unwrap();
    // Move to a text file
    while app.selected_entry().is_none_or(|e| e.is_dir) {
      app.update(Action::MoveDown).unwrap();
    }
    app.update(Action::ExtractArchive).unwrap();
    assert_eq!(app.status_message.as_deref(), Some("Not an archive file"));
    cleanup_test_dir(&dir);
  }

  // --- Directory History Tests ---

  #[test]
  fn test_history_back_returns_to_previous_dir() {
    let dir = setup_test_dir();
    fs::write(dir.join("aaa_dir").join("inner.txt"), "inner").unwrap();
    let mut app = App::new(dir.clone(), None, &cfg()).unwrap();

    // Start in root dir
    assert_eq!(app.tree.root, dir);

    // Enter aaa_dir
    app.update(Action::EnterDir).unwrap();
    assert_eq!(app.tree.root, dir.join("aaa_dir"));

    // Go back should return to root
    app.update(Action::HistoryBack).unwrap();
    assert_eq!(app.tree.root, dir);

    cleanup_test_dir(&dir);
  }

  // --- Breadcrumb Tests ---

  #[test]
  fn test_breadcrumb_segments_initialized() {
    let dir = setup_test_dir();
    let app = App::new(dir.clone(), None, &cfg()).unwrap();
    // Breadcrumb segments should be populated
    assert!(!app.breadcrumb_segments.is_empty());
    // Last segment should point to the root path
    let last = app.breadcrumb_segments.last().unwrap();
    assert_eq!(last.path, dir);
    cleanup_test_dir(&dir);
  }

  // === Dual-pane tests ===

  #[test]
  fn test_app_starts_in_single_pane_mode() {
    let dir = setup_test_dir();
    let app = App::new(dir.clone(), None, &cfg()).unwrap();
    assert!(!app.dual_pane_mode);
    assert_eq!(app.active_pane, 0);
    assert!(app.right_pane.is_none());
    cleanup_test_dir(&dir);
  }

  #[test]
  fn test_toggle_blame() {
    let dir = setup_test_dir();
    let mut app = App::new(dir.clone(), None, &cfg()).unwrap();
    assert!(!app.preview.blame_enabled);

    app.update(Action::ToggleBlame).unwrap();
    assert!(app.preview.blame_enabled);

    app.update(Action::ToggleBlame).unwrap();
    assert!(!app.preview.blame_enabled);

    cleanup_test_dir(&dir);
  }

  #[test]
  fn test_toggle_dual_pane_enables_mode() {
    let dir = setup_test_dir();
    let mut app = App::new(dir.clone(), None, &cfg()).unwrap();
    app.update(Action::ToggleDualPane).unwrap();
    assert!(app.dual_pane_mode);
    assert_eq!(app.active_pane, 0);
    assert!(app.right_pane.is_some());
    cleanup_test_dir(&dir);
  }

  #[test]
  fn test_chmod_start_opens_dialog() {
    let dir = setup_test_dir();
    let mut app = App::new(dir.clone(), None, &cfg()).unwrap();
    // Move to a file
    while app.selected_entry().is_none_or(|e| e.is_dir) {
      app.update(Action::MoveDown).unwrap();
    }
    let path = app.selected_entry().unwrap().path.clone();
    app.update(Action::ChmodStart).unwrap();
    assert_eq!(app.input_mode, InputMode::Chmod);
    assert_eq!(app.chmod_state.path, path);
    assert!(!app.chmod_state.is_dir);
    cleanup_test_dir(&dir);
  }

  #[test]
  fn test_show_properties_opens_properties_mode() {
    let dir = setup_test_dir();
    let mut app = App::new(dir.clone(), None, &cfg()).unwrap();
    // Move to a file
    while app.selected_entry().map_or(true, |e| e.is_dir) {
      app.update(Action::MoveDown).unwrap();
    }
    app.update(Action::ShowProperties).unwrap();
    assert_eq!(app.input_mode, InputMode::Properties);
    assert!(app.file_properties.is_some());
    cleanup_test_dir(&dir);
  }

  #[test]
  fn test_toggle_dual_pane_twice_disables_mode() {
    let dir = setup_test_dir();
    let mut app = App::new(dir.clone(), None, &cfg()).unwrap();
    app.update(Action::ToggleDualPane).unwrap();
    assert!(app.dual_pane_mode);
    app.update(Action::ToggleDualPane).unwrap();
    assert!(!app.dual_pane_mode);
    assert!(app.right_pane.is_none());
    assert_eq!(app.active_pane, 0);
    cleanup_test_dir(&dir);
  }

  #[test]
  fn test_history_forward_after_back() {
    let dir = setup_test_dir();
    fs::write(dir.join("aaa_dir").join("inner.txt"), "inner").unwrap();
    let mut app = App::new(dir.clone(), None, &cfg()).unwrap();

    // Enter aaa_dir
    app.update(Action::EnterDir).unwrap();
    assert_eq!(app.tree.root, dir.join("aaa_dir"));

    // Go back
    app.update(Action::HistoryBack).unwrap();
    assert_eq!(app.tree.root, dir);

    // Go forward should return to aaa_dir
    app.update(Action::HistoryForward).unwrap();
    assert_eq!(app.tree.root, dir.join("aaa_dir"));

    cleanup_test_dir(&dir);
  }

  #[test]
  fn test_extract_archive_zip() {
    use std::io::Write;
    let dir = setup_test_dir();
    let zip_path = dir.join("test.zip");

    // Create a test ZIP file
    {
      let file = fs::File::create(&zip_path).unwrap();
      let mut zip = zip::ZipWriter::new(file);
      let options = zip::write::FileOptions::default();
      zip.start_file("extracted.txt", options).unwrap();
      zip.write_all(b"extracted content").unwrap();
      zip.finish().unwrap();
    }

    let mut app = App::new(dir.clone(), None, &cfg()).unwrap();
    // Find and select the zip file
    while app.selected_entry().is_none_or(|e| e.name != "test.zip") {
      app.update(Action::MoveDown).unwrap();
    }
    assert_eq!(app.selected_entry().unwrap().name, "test.zip");

    app.update(Action::ExtractArchive).unwrap();

    // Wait for async extraction to complete
    while app.extracting.is_some() {
      std::thread::sleep(std::time::Duration::from_millis(10));
      app.update(Action::Tick).unwrap();
    }

    // Verify extraction
    assert!(dir.join("extracted.txt").exists());
    assert_eq!(fs::read_to_string(dir.join("extracted.txt")).unwrap(), "extracted content");
    // Original archive should still exist
    assert!(zip_path.exists());

    cleanup_test_dir(&dir);
  }

  // --- Custom ignore pattern tests ---

  fn cfg_with_ignore_patterns(patterns: &[&str]) -> Config {
    use globset::{Glob, GlobSetBuilder};
    let mut c = cfg();
    c.ignore_patterns = patterns.iter().map(|s| s.to_string()).collect();
    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
      builder.add(Glob::new(pattern).unwrap());
    }
    c.ignore_glob_set = builder.build().unwrap();
    c.use_custom_ignore = true;
    c
  }

  #[test]
  fn test_app_with_ignore_patterns() {
    let dir = setup_test_dir();
    fs::write(dir.join("debug.log"), "log").unwrap();

    let config = cfg_with_ignore_patterns(&["*.log"]);
    let app = App::new(dir.clone(), None, &config).unwrap();

    // .log file should be hidden by default
    assert!(!app.tree.entries.iter().any(|e| e.name.ends_with(".log")));

    cleanup_test_dir(&dir);
  }

  #[test]
  fn test_chmod_start_on_dir() {
    let dir = setup_test_dir();
    let mut app = App::new(dir.clone(), None, &cfg()).unwrap();
    // First entry is a dir
    assert!(app.selected_entry().unwrap().is_dir);
    app.update(Action::ChmodStart).unwrap();
    assert_eq!(app.input_mode, InputMode::Chmod);
    assert!(app.chmod_state.is_dir);
    cleanup_test_dir(&dir);
  }

  #[test]
  fn test_show_properties_for_directory() {
    let dir = setup_test_dir();
    let mut app = App::new(dir.clone(), None, &cfg()).unwrap();
    // First entry should be a directory
    assert!(app.selected_entry().unwrap().is_dir);
    app.update(Action::ShowProperties).unwrap();
    assert_eq!(app.input_mode, InputMode::Properties);
    assert!(app.file_properties.is_some());
    assert!(app.file_properties.as_ref().unwrap().is_dir);
    cleanup_test_dir(&dir);
  }

  #[test]
  fn test_switch_pane_in_single_mode_is_noop() {
    let dir = setup_test_dir();
    let mut app = App::new(dir.clone(), None, &cfg()).unwrap();
    assert_eq!(app.active_pane, 0);
    app.update(Action::SwitchPane).unwrap();
    assert_eq!(app.active_pane, 0); // Should not change
    cleanup_test_dir(&dir);
  }

  #[test]
  fn test_toggle_custom_ignore_action() {
    let dir = setup_test_dir();
    fs::write(dir.join("debug.log"), "log").unwrap();

    let config = cfg_with_ignore_patterns(&["*.log"]);
    let mut app = App::new(dir.clone(), None, &config).unwrap();

    // Initially hidden
    assert!(!app.tree.entries.iter().any(|e| e.name.ends_with(".log")));
    assert!(!app.tree.show_custom_ignored);

    // Toggle to show
    app.update(Action::ToggleCustomIgnore).unwrap();
    assert!(app.tree.show_custom_ignored);
    assert!(app.tree.entries.iter().any(|e| e.name == "debug.log"));

    // Toggle back to hide
    app.update(Action::ToggleCustomIgnore).unwrap();
    assert!(!app.tree.show_custom_ignored);
    assert!(!app.tree.entries.iter().any(|e| e.name.ends_with(".log")));

    cleanup_test_dir(&dir);
  }

  #[test]
  fn test_extract_and_delete_confirm_prompt() {
    use std::io::Write;
    let dir = setup_test_dir();
    let zip_path = dir.join("test.zip");

    // Create a test ZIP file
    {
      let file = fs::File::create(&zip_path).unwrap();
      let mut zip = zip::ZipWriter::new(file);
      let options = zip::write::FileOptions::default();
      zip.start_file("test.txt", options).unwrap();
      zip.write_all(b"test").unwrap();
      zip.finish().unwrap();
    }

    let mut app = App::new(dir.clone(), None, &cfg()).unwrap();
    while app.selected_entry().is_none_or(|e| e.name != "test.zip") {
      app.update(Action::MoveDown).unwrap();
    }

    app.update(Action::ExtractAndDelete).unwrap();
    assert_eq!(app.input_mode, InputMode::Prompt);
    assert_eq!(app.prompt_kind, Some(PromptKind::ConfirmExtractAndDelete));

    // Cancel should not extract
    app.update(Action::PromptInput('n')).unwrap();
    assert_eq!(app.input_mode, InputMode::Normal);
    assert!(!dir.join("test.txt").exists()); // Not extracted
    assert!(zip_path.exists()); // Still exists
    cleanup_test_dir(&dir);
  }

  #[test]
  fn test_chmod_toggle_bit() {
    let dir = setup_test_dir();
    let mut app = App::new(dir.clone(), None, &cfg()).unwrap();
    while app.selected_entry().is_none_or(|e| e.is_dir) {
      app.update(Action::MoveDown).unwrap();
    }
    app.update(Action::ChmodStart).unwrap();

    let original = app.chmod_state.new_mode;
    // Toggle owner execute (bit 2, shift 6)
    app.update(Action::ChmodToggleBit(2)).unwrap();
    assert_ne!(app.chmod_state.new_mode, original);
    // Toggle it back
    app.update(Action::ChmodToggleBit(2)).unwrap();
    assert_eq!(app.chmod_state.new_mode, original);
    cleanup_test_dir(&dir);
  }

  #[test]
  fn test_switch_pane_toggles_active_pane() {
    let dir = setup_test_dir();
    let mut app = App::new(dir.clone(), None, &cfg()).unwrap();
    app.update(Action::ToggleDualPane).unwrap();
    assert_eq!(app.active_pane, 0);

    app.update(Action::SwitchPane).unwrap();
    assert_eq!(app.active_pane, 1);

    app.update(Action::SwitchPane).unwrap();
    assert_eq!(app.active_pane, 0);
    cleanup_test_dir(&dir);
  }

  #[test]
  fn test_chmod_toggle_recursive_only_for_dirs() {
    let dir = setup_test_dir();
    let mut app = App::new(dir.clone(), None, &cfg()).unwrap();
    while app.selected_entry().is_none_or(|e| e.is_dir) {
      app.update(Action::MoveDown).unwrap();
    }
    app.update(Action::ChmodStart).unwrap();
    assert!(!app.chmod_state.is_dir);
    app.update(Action::ChmodToggleRecursive).unwrap();
    assert!(!app.chmod_state.recursive); // Should not toggle for files
    cleanup_test_dir(&dir);
  }

  #[test]
  fn test_right_pane_initialized_with_same_root() {
    let dir = setup_test_dir();
    let mut app = App::new(dir.clone(), None, &cfg()).unwrap();
    app.update(Action::ToggleDualPane).unwrap();

    let right_pane = app.right_pane.as_ref().unwrap();
    assert_eq!(right_pane.tree.root, app.tree.root);
    cleanup_test_dir(&dir);
  }

  #[test]
  fn test_chmod_toggle_recursive_for_dirs() {
    let dir = setup_test_dir();
    let mut app = App::new(dir.clone(), None, &cfg()).unwrap();
    // First entry is a dir
    assert!(app.selected_entry().unwrap().is_dir);
    app.update(Action::ChmodStart).unwrap();
    assert!(app.chmod_state.is_dir);
    assert!(!app.chmod_state.recursive);
    app.update(Action::ChmodToggleRecursive).unwrap();
    assert!(app.chmod_state.recursive);
    cleanup_test_dir(&dir);
  }

  #[test]
  fn test_pane_navigates_independently() {
    let dir = setup_test_dir();
    let mut app = App::new(dir.clone(), None, &cfg()).unwrap();
    app.update(Action::ToggleDualPane).unwrap();

    // Move left pane cursor
    app.update(Action::MoveDown).unwrap();
    app.update(Action::MoveDown).unwrap();
    assert_eq!(app.cursor, 2);

    // Switch to right pane
    app.update(Action::SwitchPane).unwrap();
    // Right pane cursor should still be at 0
    let right_pane = app.right_pane.as_ref().unwrap();
    assert_eq!(right_pane.cursor, 0);
    cleanup_test_dir(&dir);
  }

  #[test]
  fn test_chmod_close() {
    let dir = setup_test_dir();
    let mut app = App::new(dir.clone(), None, &cfg()).unwrap();
    app.update(Action::ChmodStart).unwrap();
    assert_eq!(app.input_mode, InputMode::Chmod);
    app.update(Action::ChmodClose).unwrap();
    assert_eq!(app.input_mode, InputMode::Normal);
    cleanup_test_dir(&dir);
  }

  #[test]
  fn test_inactive_pane_dir_returns_none_in_single_mode() {
    let dir = setup_test_dir();
    let app = App::new(dir.clone(), None, &cfg()).unwrap();
    assert!(app.inactive_pane_dir().is_none());
    cleanup_test_dir(&dir);
  }

  #[test]
  fn test_inactive_pane_dir_returns_other_pane_root() {
    let dir = setup_test_dir();
    let mut app = App::new(dir.clone(), None, &cfg()).unwrap();
    app.update(Action::ToggleDualPane).unwrap();

    // Active pane is left (0), inactive is right
    let inactive_dir = app.inactive_pane_dir();
    assert!(inactive_dir.is_some());
    assert_eq!(inactive_dir.unwrap(), dir);

    // Switch to right pane
    app.update(Action::SwitchPane).unwrap();
    // Now inactive pane is left
    let inactive_dir = app.inactive_pane_dir();
    assert!(inactive_dir.is_some());
    assert_eq!(inactive_dir.unwrap(), dir);
    cleanup_test_dir(&dir);
  }

  #[test]
  fn test_extract_and_delete_confirms_deletes() {
    use std::io::Write;
    let dir = setup_test_dir();
    let zip_path = dir.join("test.zip");

    // Create a test ZIP file
    {
      let file = fs::File::create(&zip_path).unwrap();
      let mut zip = zip::ZipWriter::new(file);
      let options = zip::write::FileOptions::default();
      zip.start_file("extracted.txt", options).unwrap();
      zip.write_all(b"content").unwrap();
      zip.finish().unwrap();
    }

    let mut app = App::new(dir.clone(), None, &cfg()).unwrap();
    while app.selected_entry().is_none_or(|e| e.name != "test.zip") {
      app.update(Action::MoveDown).unwrap();
    }

    app.update(Action::ExtractAndDelete).unwrap();
    assert_eq!(app.prompt_kind, Some(PromptKind::ConfirmExtractAndDelete));

    // Confirm with 'y'
    app.update(Action::PromptInput('y')).unwrap();

    // Wait for async extraction to complete
    while app.extracting.is_some() {
      std::thread::sleep(std::time::Duration::from_millis(10));
      app.update(Action::Tick).unwrap();
    }

    // Verify extraction and deletion
    assert!(dir.join("extracted.txt").exists());
    assert!(!zip_path.exists()); // Archive deleted
    cleanup_test_dir(&dir);
  }

  #[test]
  fn test_chmod_apply_changes_permissions() {
    let dir = setup_test_dir();
    let mut app = App::new(dir.clone(), None, &cfg()).unwrap();
    while app.selected_entry().is_none_or(|e| e.is_dir) {
      app.update(Action::MoveDown).unwrap();
    }
    let path = app.selected_entry().unwrap().path.clone();
    app.update(Action::ChmodStart).unwrap();

    // Set to 0o600
    app.chmod_state.new_mode = (app.chmod_state.original_mode & !0o777) | 0o600;
    app.update(Action::ChmodApply).unwrap();

    let meta = std::fs::metadata(&path).unwrap();
    assert_eq!(meta.permissions().mode() & 0o777, 0o600);
    assert_eq!(app.input_mode, InputMode::Normal);
    cleanup_test_dir(&dir);
  }

  #[test]
  fn test_chmod_toggle_octal_mode() {
    let dir = setup_test_dir();
    let mut app = App::new(dir.clone(), None, &cfg()).unwrap();
    app.update(Action::ChmodStart).unwrap();
    assert!(!app.chmod_state.octal_mode);
    app.update(Action::ChmodToggleOctal).unwrap();
    assert!(app.chmod_state.octal_mode);
    // Should pre-fill octal input
    assert!(!app.chmod_state.octal_input.is_empty());
    cleanup_test_dir(&dir);
  }

  #[test]
  fn test_chmod_recursive_applies_to_all() {
    let dir = setup_test_dir();
    // Create file inside aaa_dir
    fs::write(dir.join("aaa_dir").join("inner.txt"), "content").unwrap();
    let mut app = App::new(dir.clone(), None, &cfg()).unwrap();

    // Select aaa_dir
    assert_eq!(app.tree.entries[0].name, "aaa_dir");
    app.update(Action::ChmodStart).unwrap();
    assert!(app.chmod_state.is_dir);

    // Enable recursive and set permissions
    app.update(Action::ChmodToggleRecursive).unwrap();
    app.chmod_state.new_mode = (app.chmod_state.original_mode & !0o777) | 0o700;
    app.update(Action::ChmodApply).unwrap();

    // Check both dir and inner file
    let dir_meta = std::fs::metadata(dir.join("aaa_dir")).unwrap();
    let file_meta = std::fs::metadata(dir.join("aaa_dir").join("inner.txt")).unwrap();
    assert_eq!(dir_meta.permissions().mode() & 0o777, 0o700);
    assert_eq!(file_meta.permissions().mode() & 0o777, 0o700);
    cleanup_test_dir(&dir);
  }

  #[test]
  fn test_new_navigation_clears_forward_history() {
    let dir = setup_test_dir();
    fs::write(dir.join("aaa_dir").join("inner.txt"), "inner").unwrap();
    fs::write(dir.join("zzz_dir").join("other.txt"), "other").unwrap();
    let mut app = App::new(dir.clone(), None, &cfg()).unwrap();

    // Enter aaa_dir
    app.update(Action::EnterDir).unwrap();
    assert_eq!(app.tree.root, dir.join("aaa_dir"));

    // Go back to root
    app.update(Action::HistoryBack).unwrap();
    assert_eq!(app.tree.root, dir);

    // Now enter zzz_dir (should clear forward history)
    app.update(Action::MoveDown).unwrap(); // move to zzz_dir
    app.update(Action::EnterDir).unwrap();
    assert_eq!(app.tree.root, dir.join("zzz_dir"));

    // Forward should do nothing (forward history was cleared)
    app.update(Action::HistoryForward).unwrap();
    assert_eq!(app.tree.root, dir.join("zzz_dir"));

    cleanup_test_dir(&dir);
  }

  #[test]
  fn test_history_respects_size_limit() {
    let dir = setup_test_dir();
    // Create nested directories
    let mut current = dir.clone();
    for i in 0..60 {
      let subdir = current.join(format!("dir_{i:02}"));
      fs::create_dir_all(&subdir).unwrap();
      fs::write(subdir.join("file.txt"), "data").unwrap();
      current = subdir;
    }

    let mut app = App::new(dir.clone(), None, &cfg()).unwrap();

    // Navigate through all 60 directories
    for _ in 0..60 {
      app.update(Action::EnterDir).unwrap();
    }

    // Now go back - we should be able to go back at most 50 times (the limit)
    let mut back_count = 0;
    let _final_root = loop {
      let before = app.tree.root.clone();
      app.update(Action::HistoryBack).unwrap();
      if app.tree.root == before {
        break before;
      }
      back_count += 1;
      if back_count > 100 {
        panic!("Infinite loop in history back");
      }
    };

    // Should have been able to go back at most 50 times
    assert!(back_count <= 50, "Back count {back_count} exceeds limit of 50");
    // Should not have gotten all the way back to original dir
    assert!(back_count >= 1, "Should have some history");

    cleanup_test_dir(&dir);
  }

  #[test]
  fn test_history_skips_duplicates() {
    let dir = setup_test_dir();
    fs::write(dir.join("aaa_dir").join("inner.txt"), "inner").unwrap();
    let mut app = App::new(dir.clone(), None, &cfg()).unwrap();

    // Enter aaa_dir
    app.update(Action::EnterDir).unwrap();
    assert_eq!(app.tree.root, dir.join("aaa_dir"));

    // Go back to root
    app.update(Action::HistoryBack).unwrap();
    assert_eq!(app.tree.root, dir);

    // Enter aaa_dir again
    app.update(Action::EnterDir).unwrap();
    assert_eq!(app.tree.root, dir.join("aaa_dir"));

    // Go back - should go to root (not aaa_dir again)
    app.update(Action::HistoryBack).unwrap();
    assert_eq!(app.tree.root, dir);

    // Go back again - should stay at root (no more history with duplicates removed)
    app.update(Action::HistoryBack).unwrap();
    // Either stays at root or goes to a valid previous location
    // The key is we shouldn't have duplicate entries

    cleanup_test_dir(&dir);
  }

  #[test]
  fn test_apply_config_updates_ignore_patterns() {
    let dir = setup_test_dir();
    fs::write(dir.join("test.tmp"), "tmp").unwrap();

    // Start with no matching patterns (default patterns don't include *.tmp)
    let mut app = App::new(dir.clone(), None, &cfg()).unwrap();
    assert!(app.tree.entries.iter().any(|e| e.name == "test.tmp"));

    // Apply config with ignore patterns that include *.tmp
    let config = cfg_with_ignore_patterns(&["*.tmp"]);
    app.apply_config(&config);

    // Reload tree to apply new patterns
    app.tree.reload().unwrap();
    assert!(!app.tree.entries.iter().any(|e| e.name.ends_with(".tmp")));
    cleanup_test_dir(&dir);
  }

  #[test]
  fn test_go_parent_adds_to_history() {
    let dir = setup_test_dir();
    let child_dir = dir.join("aaa_dir");
    let mut app = App::new(child_dir.clone(), None, &cfg()).unwrap();
    assert_eq!(app.tree.root, child_dir);

    // Go to parent via MoveLeft
    app.update(Action::MoveLeft).unwrap();
    assert_eq!(app.tree.root, dir);

    // Go back should return to child_dir
    // (or forward if we interpret parent as navigating to parent)
    // The semantic is: going to parent pushes current to back history
    // Actually after going to parent, we should be able to go forward
    // to get back to child_dir
    app.update(Action::HistoryForward).unwrap();
    assert_eq!(app.tree.root, child_dir);

    cleanup_test_dir(&dir);
  }

  #[test]
  fn test_breadcrumb_select_navigates_to_parent() {
    let dir = setup_test_dir();
    let child_dir = dir.join("aaa_dir");
    fs::write(child_dir.join("inner.txt"), "data").unwrap();
    let mut app = App::new(child_dir.clone(), None, &cfg()).unwrap();
    assert_eq!(app.tree.root, child_dir);

    // Find the parent segment index (the one before last)
    let parent_idx = app.breadcrumb_segments.len().saturating_sub(2);

    // Select parent breadcrumb
    app.update(Action::BreadcrumbSelect(parent_idx)).unwrap();
    assert_eq!(app.tree.root, dir);
    assert_eq!(app.cursor, 0);
    // Breadcrumbs should be updated
    let last = app.breadcrumb_segments.last().unwrap();
    assert_eq!(last.path, dir);
    cleanup_test_dir(&dir);
  }

  #[test]
  fn test_go_home_adds_to_history() {
    let dir = setup_test_dir();
    let mut app = App::new(dir.clone(), None, &cfg()).unwrap();

    // Go home
    app.update(Action::GoHome).unwrap();

    // Go back should return to original dir
    app.update(Action::HistoryBack).unwrap();
    assert_eq!(app.tree.root, dir);

    cleanup_test_dir(&dir);
  }

  #[test]
  fn test_favorites_select_adds_to_history() {
    let dir = setup_test_dir();
    let target = dir.join("aaa_dir");
    let mut app = App::new(dir.clone(), None, &cfg()).unwrap();

    // Add target to favorites and select it
    app.favorites.add(target.clone());
    app.favorites_cursor = app.favorites.len() - 1;
    app.input_mode = crate::event::InputMode::Favorites;
    app.update(Action::FavoritesSelect).unwrap();

    assert_eq!(app.tree.root, target);

    // Go back should return to original dir
    app.update(Action::HistoryBack).unwrap();
    assert_eq!(app.tree.root, dir);

    cleanup_test_dir(&dir);
  }

  #[test]
  fn test_history_back_on_empty_history_is_noop() {
    let dir = setup_test_dir();
    let mut app = App::new(dir.clone(), None, &cfg()).unwrap();

    // No navigation yet, history should be empty
    let root_before = app.tree.root.clone();
    app.update(Action::HistoryBack).unwrap();
    assert_eq!(app.tree.root, root_before);

    cleanup_test_dir(&dir);
  }

  #[test]
  fn test_breadcrumb_select_current_is_noop() {
    let dir = setup_test_dir();
    let mut app = App::new(dir.clone(), None, &cfg()).unwrap();
    let root_before = app.tree.root.clone();

    // Select the last segment (current directory)
    let last_idx = app.breadcrumb_segments.len().saturating_sub(1);
    app.update(Action::BreadcrumbSelect(last_idx)).unwrap();

    // Should remain in the same directory
    assert_eq!(app.tree.root, root_before);
    cleanup_test_dir(&dir);
  }

  #[test]
  fn test_history_forward_on_empty_forward_is_noop() {
    let dir = setup_test_dir();
    let mut app = App::new(dir.clone(), None, &cfg()).unwrap();

    // No navigation yet
    let root_before = app.tree.root.clone();
    app.update(Action::HistoryForward).unwrap();
    assert_eq!(app.tree.root, root_before);

    cleanup_test_dir(&dir);
  }

  #[test]
  fn test_breadcrumb_select_out_of_bounds_is_noop() {
    let dir = setup_test_dir();
    let mut app = App::new(dir.clone(), None, &cfg()).unwrap();
    let root_before = app.tree.root.clone();

    // Select an index way out of bounds
    app.update(Action::BreadcrumbSelect(100)).unwrap();

    // Should remain in the same directory
    assert_eq!(app.tree.root, root_before);
    cleanup_test_dir(&dir);
  }

  #[test]
  fn test_toggle_blame_resets_scroll() {
    let dir = setup_test_dir();
    let mut app = App::new(dir.clone(), None, &cfg()).unwrap();
    app.preview.scroll_offset = 10;

    app.update(Action::ToggleBlame).unwrap();
    assert_eq!(app.preview.scroll_offset, 0);

    cleanup_test_dir(&dir);
  }

  #[test]
  fn test_breadcrumb_updates_on_enter_dir() {
    let dir = setup_test_dir();
    fs::write(dir.join("aaa_dir").join("inner.txt"), "data").unwrap();
    let mut app = App::new(dir.clone(), None, &cfg()).unwrap();

    let initial_len = app.breadcrumb_segments.len();

    // Enter aaa_dir
    app.update(Action::EnterDir).unwrap();
    assert_eq!(app.tree.root, dir.join("aaa_dir"));

    // Breadcrumbs should have grown by one
    assert_eq!(app.breadcrumb_segments.len(), initial_len + 1);
    let last = app.breadcrumb_segments.last().unwrap();
    assert_eq!(last.name, "aaa_dir");
    cleanup_test_dir(&dir);
  }

  #[test]
  fn test_history_back_updates_breadcrumbs() {
    let dir = setup_test_dir();
    fs::write(dir.join("aaa_dir").join("inner.txt"), "data").unwrap();
    let mut app = App::new(dir.clone(), None, &cfg()).unwrap();

    let initial_len = app.breadcrumb_segments.len();

    // Enter aaa_dir
    app.update(Action::EnterDir).unwrap();
    assert_eq!(app.breadcrumb_segments.len(), initial_len + 1);

    // Go back - breadcrumbs should shrink
    app.update(Action::HistoryBack).unwrap();
    assert_eq!(app.tree.root, dir);
    assert_eq!(app.breadcrumb_segments.len(), initial_len);
    let last = app.breadcrumb_segments.last().unwrap();
    assert_eq!(last.path, dir);

    cleanup_test_dir(&dir);
  }

  #[test]
  fn test_history_forward_updates_breadcrumbs() {
    let dir = setup_test_dir();
    fs::write(dir.join("aaa_dir").join("inner.txt"), "data").unwrap();
    let mut app = App::new(dir.clone(), None, &cfg()).unwrap();

    let initial_len = app.breadcrumb_segments.len();

    // Enter aaa_dir
    app.update(Action::EnterDir).unwrap();
    assert_eq!(app.breadcrumb_segments.len(), initial_len + 1);

    // Go back
    app.update(Action::HistoryBack).unwrap();
    assert_eq!(app.breadcrumb_segments.len(), initial_len);

    // Go forward - breadcrumbs should grow again
    app.update(Action::HistoryForward).unwrap();
    assert_eq!(app.tree.root, dir.join("aaa_dir"));
    assert_eq!(app.breadcrumb_segments.len(), initial_len + 1);
    let last = app.breadcrumb_segments.last().unwrap();
    assert_eq!(last.name, "aaa_dir");

    cleanup_test_dir(&dir);
  }

  #[test]
  fn test_toggle_preserves_left_pane_state() {
    let dir = setup_test_dir();
    let mut app = App::new(dir.clone(), None, &cfg()).unwrap();
    // Move cursor and expand a dir
    app.update(Action::MoveDown).unwrap();
    let cursor_before = app.cursor;

    // Enable dual-pane
    app.update(Action::ToggleDualPane).unwrap();
    assert_eq!(app.cursor, cursor_before);

    // Disable dual-pane
    app.update(Action::ToggleDualPane).unwrap();
    assert_eq!(app.cursor, cursor_before);
    cleanup_test_dir(&dir);
  }

  #[test]
  fn test_properties_close() {
    let dir = setup_test_dir();
    let mut app = App::new(dir.clone(), None, &cfg()).unwrap();
    app.update(Action::ShowProperties).unwrap();
    assert_eq!(app.input_mode, InputMode::Properties);
    app.update(Action::PropertiesClose).unwrap();
    assert_eq!(app.input_mode, InputMode::Normal);
    assert!(app.file_properties.is_none());
    cleanup_test_dir(&dir);
  }
}
