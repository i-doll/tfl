use std::path::PathBuf;
use std::process::Command;

use anyhow::Result;
use ratatui_image::picker::Picker;

use crate::action::Action;
use crate::config::Config;
use crate::event::{InputMode, PromptKind};
use crate::favorites::Favorites;
use crate::fs::FileTree;
use crate::fs::ops;
use crate::opener::{self, OpenApp};
use crate::preview::PreviewState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClipboardOp {
  Cut,
  Copy,
}

#[derive(Debug, Clone)]
pub struct Clipboard {
  pub paths: Vec<PathBuf>,
  pub op: Option<ClipboardOp>,
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
}

#[derive(Debug, Clone)]
pub enum SuspendAction {
  Editor(PathBuf),
  Claude(PathBuf),
  Shell(PathBuf),
  OpenWith(String, PathBuf),
}

impl App {
  pub fn new(root: PathBuf, picker: Option<Picker>, config: &Config) -> Result<Self> {
    let tree = FileTree::new(root)?;
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
    })
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
      Action::GoToTop => {
        self.cursor = 0;
        self.tree_scroll_offset = 0;
        self.input_mode = InputMode::Normal;
        self.update_preview();
      }
      Action::GoToBottom => {
        if !self.tree.entries.is_empty() {
          self.cursor = self.visible_entries().len().saturating_sub(1);
          self.adjust_scroll();
          self.update_preview();
        }
      }
      Action::GPress => {
        self.input_mode = InputMode::GPrefix;
      }
      Action::SearchStart => {
        self.input_mode = InputMode::Search;
        self.search_query.clear();
      }
      Action::SearchInput(c) => {
        self.search_query.push(c);
        self.apply_search_filter();
      }
      Action::SearchBackspace => {
        self.search_query.pop();
        self.apply_search_filter();
      }
      Action::SearchConfirm => {
        self.input_mode = InputMode::Normal;
        // Enter directory while filter is still active so cursor resolves correctly
        self.enter_directory()?;
        // Clear query for non-dir entries (enter_directory already clears for dirs)
        self.search_query.clear();
      }
      Action::SearchCancel => {
        self.input_mode = InputMode::Normal;
        self.search_query.clear();
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
        self.should_suspend = Some(SuspendAction::Claude(dir));
      }
      Action::OpenShell => {
        let dir = self.current_dir();
        self.should_suspend = Some(SuspendAction::Shell(dir));
      }
      Action::ShrinkTree => {
        self.tree_ratio = self.tree_ratio.saturating_sub(self.ratio_step).max(self.min_tree_ratio);
      }
      Action::GrowTree => {
        self.tree_ratio = (self.tree_ratio + self.ratio_step).min(self.max_tree_ratio);
      }
      Action::ToggleHelp => {
        self.show_help = !self.show_help;
        self.input_mode = if self.show_help { InputMode::Help } else { InputMode::Normal };
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
        if self.prompt_kind != Some(PromptKind::ConfirmDelete) && self.prompt_cursor > 0 {
          let byte_pos = self.prompt_input.char_indices()
            .nth(self.prompt_cursor - 1)
            .map(|(i, _)| i)
            .unwrap_or(0);
          self.prompt_input.remove(byte_pos);
          self.prompt_cursor -= 1;
        }
      }
      Action::PromptDelete => {
        if self.prompt_kind != Some(PromptKind::ConfirmDelete)
          && self.prompt_cursor < self.prompt_input.chars().count()
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
      Action::Tick => {
        self.preview.check_image_loaded();
      }
      Action::None => {}
    }
    Ok(())
  }

  fn go_home(&mut self) -> Result<()> {
    if let Some(home) = dirs::home_dir() {
      self.tree.navigate_to(&home)?;
      self.search_query.clear();
      self.cursor = 0;
      self.tree_scroll_offset = 0;
      self.input_mode = InputMode::Normal;
      self.preview.invalidate();
      self.update_preview();
    }
    Ok(())
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
        self.tree.navigate_to(&path)?;
        self.search_query.clear();
        self.cursor = 0;
        self.tree_scroll_offset = 0;
        self.preview.invalidate();
        self.update_preview();
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
      if entry.is_dir {
        return;
      }
      self.open_with_apps = opener::detect_apps(&self.custom_apps);
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
        self.input_mode = InputMode::Normal;
        if app.is_tui {
          self.should_suspend = Some(SuspendAction::OpenWith(app.command.clone(), path));
        } else {
          match opener::open_with_app(&path, &app) {
            Ok(()) => self.set_status(format!("Opened with {}", app.name)),
            Err(e) => self.status_message = Some(e),
          }
        }
      }
    }
    Ok(())
  }

  fn move_cursor(&mut self, delta: i32) {
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
    let entries = self.visible_entries();
    if let Some(idx) = entries.get(self.cursor).copied() {
      if self.tree.entries[idx].is_dir {
        self.tree.toggle_expand(idx)?;
        self.update_preview();
      } else {
        // File selected - just update preview
        self.update_preview();
      }
    }
    Ok(())
  }

  fn expand_only(&mut self) -> Result<()> {
    let entries = self.visible_entries();
    if let Some(idx) = entries.get(self.cursor).copied() {
      if self.tree.entries[idx].is_dir && !self.tree.entries[idx].expanded {
        self.tree.toggle_expand(idx)?;
      }
      self.update_preview();
    }
    Ok(())
  }

  fn enter_directory(&mut self) -> Result<()> {
    let entries = self.visible_entries();
    if let Some(idx) = entries.get(self.cursor).copied() {
      if self.tree.entries[idx].is_dir {
        self.tree.enter_dir(idx)?;
        self.search_query.clear();
        self.cursor = 0;
        self.tree_scroll_offset = 0;
        self.preview.invalidate();
        self.update_preview();
      } else {
        self.update_preview();
      }
    }
    Ok(())
  }

  fn go_parent_or_collapse(&mut self) -> Result<()> {
    let entries = self.visible_entries();
    if let Some(&idx) = entries.get(self.cursor)
      && self.tree.entries[idx].is_dir && self.tree.entries[idx].expanded {
        self.tree.toggle_expand(idx)?;
        self.update_preview();
        return Ok(());
      }

    // Go to parent directory
    if let Some(old_root) = self.tree.go_parent()? {
      self.search_query.clear();
      // Try to position cursor on the old root dir
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
    }
    Ok(())
  }

  fn toggle_hidden(&mut self) -> Result<()> {
    self.tree.toggle_hidden()?;
    self.cursor = self.cursor.min(self.tree.entries.len().saturating_sub(1));
    self.preview.invalidate();
    self.update_preview();
    Ok(())
  }

  fn apply_search_filter(&mut self) {
    // Move cursor to first matching entry
    if !self.search_query.is_empty() {
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
    let entries = self.visible_entries();
    if let Some(&idx) = entries.get(self.cursor) {
      let path = self.tree.entries[idx].path.clone();
      self.preview.request_preview(&path, self.picker.as_ref());
    }
  }

  pub fn selected_entry(&self) -> Option<&crate::fs::FileEntry> {
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
      SuspendAction::Claude(dir) => {
        Command::new("claude").current_dir(dir).status()?;
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
    while app.selected_entry().map_or(true, |e| e.is_dir) {
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
    assert!(matches!(suspend, Some(SuspendAction::Claude(_))));
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
    while app.selected_entry().map_or(true, |e| e.is_dir) {
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
    while app.selected_entry().map_or(true, |e| e.is_dir) {
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
    while app.selected_entry().map_or(true, |e| e.is_dir) {
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
    while app.selected_entry().map_or(true, |e| e.name != "bbb.txt") {
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
    while app.selected_entry().map_or(true, |e| e.name != "bbb.txt") {
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
    while app.selected_entry().map_or(true, |e| e.name != "bbb.txt") {
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
    while app.selected_entry().map_or(true, |e| e.name != "bbb.txt") {
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
    while app.selected_entry().map_or(true, |e| e.name != "bbb.txt") {
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
    while app.selected_entry().map_or(true, |e| e.name != "bbb.txt") {
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
    while app.selected_entry().map_or(true, |e| e.name != "bbb.txt") {
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
    while app.selected_entry().map_or(true, |e| e.name != "bbb.txt") {
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
    while app.selected_entry().map_or(true, |e| e.name != "bbb.txt") {
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
    while app.selected_entry().map_or(true, |e| e.is_dir) {
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
    while app.selected_entry().map_or(true, |e| e.is_dir) {
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
    while app.selected_entry().map_or(true, |e| e.name != "bbb.txt") {
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
    while app.selected_entry().map_or(true, |e| e.is_dir) {
      app.update(Action::MoveDown).unwrap();
    }
    app.update(Action::OpenWithStart).unwrap();
    assert_eq!(app.input_mode, InputMode::OpenWith);
    assert_eq!(app.open_with_cursor, 0);
    cleanup_test_dir(&dir);
  }

  #[test]
  fn test_open_with_start_on_dir_noop() {
    let dir = setup_test_dir();
    let mut app = App::new(dir.clone(), None, &cfg()).unwrap();
    // First entry is a dir
    assert!(app.selected_entry().unwrap().is_dir);
    app.update(Action::OpenWithStart).unwrap();
    assert_eq!(app.input_mode, InputMode::Normal);
    cleanup_test_dir(&dir);
  }

  #[test]
  fn test_open_with_cursor_movement() {
    let dir = setup_test_dir();
    let mut app = App::new(dir.clone(), None, &cfg()).unwrap();
    while app.selected_entry().map_or(true, |e| e.is_dir) {
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
    while app.selected_entry().map_or(true, |e| e.is_dir) {
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
    while app.selected_entry().map_or(true, |e| e.is_dir) {
      app.update(Action::MoveDown).unwrap();
    }
    // Inject a fake TUI app so we can test the suspend path
    app.open_with_apps = vec![OpenApp {
      name: "TestEditor".into(),
      command: "testeditor".into(),
      is_tui: true,
      macos_app: None,
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
}
