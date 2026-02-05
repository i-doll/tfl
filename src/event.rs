use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use anyhow::Result;
use crossterm::event::{self, Event as CrosstermEvent, KeyCode, KeyEvent};
use notify::{RecommendedWatcher, Watcher};

use crate::action::Action;
use crate::config::{Config, normalize_key_event};

const WATCHED_FILES: &[&str] = &["config.toml", "apps.toml", "favorites"];

pub enum Event {
  Key(KeyEvent),
  Resize(u16, u16),
  Tick,
  ConfigChanged,
}

pub struct EventLoop {
  rx: mpsc::Receiver<Event>,
  paused: Arc<AtomicBool>,
  _watcher: Option<RecommendedWatcher>,
}

impl EventLoop {
  pub fn new(tick_rate: Duration, config_dir: Option<&Path>) -> Self {
    let (tx, rx) = mpsc::channel();
    let paused = Arc::new(AtomicBool::new(false));
    let thread_paused = paused.clone();

    let watcher = config_dir.and_then(|dir| {
      if !dir.is_dir() {
        eprintln!("tfl: config dir does not exist: {}", dir.display());
        return None;
      }
      let watch_tx = tx.clone();
      let mut watcher = match notify::recommended_watcher(move |res: std::result::Result<notify::Event, notify::Error>| {
        if let Ok(ev) = res {
          let dominated = ev.paths.iter().any(|p| {
            p.file_name()
              .and_then(|f| f.to_str())
              .is_some_and(|name| WATCHED_FILES.contains(&name))
          });
          if dominated {
            let _ = watch_tx.send(Event::ConfigChanged);
          }
        }
      }) {
        Ok(w) => w,
        Err(e) => {
          eprintln!("tfl: failed to create file watcher: {e}");
          return None;
        }
      };
      if let Err(e) = watcher.watch(dir, notify::RecursiveMode::NonRecursive) {
        eprintln!("tfl: failed to watch {}: {e}", dir.display());
        return None;
      }
      Some(watcher)
    });

    thread::spawn(move || loop {
      if thread_paused.load(Ordering::Relaxed) {
        thread::sleep(tick_rate);
        continue;
      }
      if event::poll(tick_rate).unwrap_or(false) {
        match event::read() {
          Ok(CrosstermEvent::Key(key)) => {
            if tx.send(Event::Key(key)).is_err() {
              break;
            }
          }
          Ok(CrosstermEvent::Resize(w, h)) => {
            if tx.send(Event::Resize(w, h)).is_err() {
              break;
            }
          }
          _ => {}
        }
      } else if tx.send(Event::Tick).is_err() {
        break;
      }
    });

    Self { rx, paused, _watcher: watcher }
  }

  pub fn pause(&self) {
    self.paused.store(true, Ordering::Relaxed);
  }

  pub fn resume(&self) -> bool {
    self.paused.store(false, Ordering::Relaxed);
    let mut config_changed = false;
    while let Ok(ev) = self.rx.try_recv() {
      if matches!(ev, Event::ConfigChanged) {
        config_changed = true;
      }
    }
    config_changed
  }

  pub fn next(&self) -> Result<Event> {
    Ok(self.rx.recv()?)
  }
}

/// Whether the app is in search/filter mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
  Normal,
  Search,
  GPrefix,
  Help,
  Prompt,
  Favorites,
  OpenWith,
  Error,
  Visual,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptKind {
  Rename,
  NewFile,
  NewDir,
  ConfirmDelete,
}

pub fn map_key(key: KeyEvent, mode: InputMode, config: &Config) -> Action {
  match mode {
    InputMode::Search => match key.code {
      KeyCode::Esc => Action::SearchCancel,
      KeyCode::Enter => Action::SearchConfirm,
      KeyCode::Backspace => Action::SearchBackspace,
      KeyCode::Char(c) => Action::SearchInput(c),
      _ => Action::None,
    },
    InputMode::GPrefix => {
      let kb = normalize_key_event(key);
      config.g_prefix_keys.get(&kb).cloned().unwrap_or(Action::None)
    }
    InputMode::Help => match key.code {
      KeyCode::Esc => Action::ToggleHelp,
      KeyCode::Char('?') => Action::ToggleHelp,
      _ => Action::None,
    },
    InputMode::Prompt => match key.code {
      KeyCode::Esc => Action::PromptCancel,
      KeyCode::Enter => Action::PromptConfirm,
      KeyCode::Backspace => Action::PromptBackspace,
      KeyCode::Delete => Action::PromptDelete,
      KeyCode::Left => Action::PromptLeft,
      KeyCode::Right => Action::PromptRight,
      KeyCode::Home => Action::PromptHome,
      KeyCode::End => Action::PromptEnd,
      KeyCode::Char(c) => Action::PromptInput(c),
      _ => Action::None,
    },
    InputMode::Favorites => match key.code {
      KeyCode::Char('j') | KeyCode::Down => Action::FavoritesDown,
      KeyCode::Char('k') | KeyCode::Up => Action::FavoritesUp,
      KeyCode::Enter => Action::FavoritesSelect,
      KeyCode::Esc | KeyCode::Char('q') => Action::FavoritesClose,
      KeyCode::Char('d') | KeyCode::Delete => Action::FavoritesRemove,
      KeyCode::Char('a') => Action::FavoritesAddCurrent,
      _ => Action::None,
    },
    InputMode::OpenWith => match key.code {
      KeyCode::Char('j') | KeyCode::Down => Action::OpenWithDown,
      KeyCode::Char('k') | KeyCode::Up => Action::OpenWithUp,
      KeyCode::Enter => Action::OpenWithSelect,
      KeyCode::Esc | KeyCode::Char('q') => Action::OpenWithClose,
      _ => Action::None,
    },
    InputMode::Error => match key.code {
      KeyCode::Esc | KeyCode::Enter | KeyCode::Char('q') => Action::ErrorClose,
      _ => Action::None,
    },
    InputMode::Visual => match key.code {
      KeyCode::Char('j') | KeyCode::Down => Action::VisualModeDown,
      KeyCode::Char('k') | KeyCode::Up => Action::VisualModeUp,
      KeyCode::Esc => Action::ClearSelection,
      KeyCode::Char(' ') => Action::ToggleSelection,
      KeyCode::Char('d') | KeyCode::Delete => Action::DeleteFile,
      KeyCode::Char('y') => Action::YankPath,
      _ => Action::None,
    },
    InputMode::Normal => {
      let kb = normalize_key_event(key);
      config.normal_keys.get(&kb).cloned().unwrap_or(Action::None)
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};

  fn key(code: KeyCode) -> KeyEvent {
    KeyEvent {
      code,
      modifiers: KeyModifiers::NONE,
      kind: KeyEventKind::Press,
      state: KeyEventState::NONE,
    }
  }

  fn key_with_mod(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
    KeyEvent {
      code,
      modifiers,
      kind: KeyEventKind::Press,
      state: KeyEventState::NONE,
    }
  }

  fn cfg() -> Config {
    Config::default()
  }

  #[test]
  fn test_normal_mode_quit() {
    let c = cfg();
    assert_eq!(map_key(key(KeyCode::Char('q')), InputMode::Normal, &c), Action::Quit);
    assert_eq!(map_key(key(KeyCode::Esc), InputMode::Normal, &c), Action::Quit);
  }

  #[test]
  fn test_normal_mode_navigation() {
    let c = cfg();
    assert_eq!(map_key(key(KeyCode::Char('j')), InputMode::Normal, &c), Action::MoveDown);
    assert_eq!(map_key(key(KeyCode::Char('k')), InputMode::Normal, &c), Action::MoveUp);
    assert_eq!(map_key(key(KeyCode::Down), InputMode::Normal, &c), Action::MoveDown);
    assert_eq!(map_key(key(KeyCode::Up), InputMode::Normal, &c), Action::MoveUp);
    assert_eq!(map_key(key(KeyCode::Char('h')), InputMode::Normal, &c), Action::MoveLeft);
    assert_eq!(map_key(key(KeyCode::Char('l')), InputMode::Normal, &c), Action::MoveRight);
  }

  #[test]
  fn test_normal_mode_actions() {
    let c = cfg();
    assert_eq!(map_key(key(KeyCode::Char(' ')), InputMode::Normal, &c), Action::ToggleSelection);
    assert_eq!(map_key(key(KeyCode::Tab), InputMode::Normal, &c), Action::ToggleExpand);
    assert_eq!(map_key(key(KeyCode::Char('.')), InputMode::Normal, &c), Action::ToggleHidden);
    assert_eq!(map_key(key(KeyCode::Char('G')), InputMode::Normal, &c), Action::GoToBottom);
    assert_eq!(map_key(key(KeyCode::Char('g')), InputMode::Normal, &c), Action::GPress);
    assert_eq!(map_key(key(KeyCode::Char('/')), InputMode::Normal, &c), Action::SearchStart);
    assert_eq!(map_key(key(KeyCode::Char('J')), InputMode::Normal, &c), Action::ScrollPreviewDown);
    assert_eq!(map_key(key(KeyCode::Char('K')), InputMode::Normal, &c), Action::ScrollPreviewUp);
    assert_eq!(map_key(key(KeyCode::Char('V')), InputMode::Normal, &c), Action::VisualModeStart);
  }

  #[test]
  fn test_ctrl_c_copies_file() {
    let c = cfg();
    assert_eq!(
      map_key(key_with_mod(KeyCode::Char('c'), KeyModifiers::CONTROL), InputMode::Normal, &c),
      Action::CopyFile
    );
  }

  #[test]
  fn test_c_opens_claude() {
    let c = cfg();
    assert_eq!(map_key(key(KeyCode::Char('c')), InputMode::Normal, &c), Action::OpenClaude);
  }

  #[test]
  fn test_search_mode() {
    let c = cfg();
    assert_eq!(map_key(key(KeyCode::Char('a')), InputMode::Search, &c), Action::SearchInput('a'));
    assert_eq!(map_key(key(KeyCode::Enter), InputMode::Search, &c), Action::SearchConfirm);
    assert_eq!(map_key(key(KeyCode::Esc), InputMode::Search, &c), Action::SearchCancel);
    assert_eq!(map_key(key(KeyCode::Backspace), InputMode::Search, &c), Action::SearchBackspace);
  }

  #[test]
  fn test_g_prefix_mode() {
    let c = cfg();
    assert_eq!(map_key(key(KeyCode::Char('g')), InputMode::GPrefix, &c), Action::GoToTop);
    assert_eq!(map_key(key(KeyCode::Char('x')), InputMode::GPrefix, &c), Action::None);
  }

  #[test]
  fn test_help_mode_question_mark_toggles() {
    let c = cfg();
    assert_eq!(map_key(key(KeyCode::Char('?')), InputMode::Help, &c), Action::ToggleHelp);
  }

  #[test]
  fn test_help_mode_esc_toggles() {
    let c = cfg();
    assert_eq!(map_key(key(KeyCode::Esc), InputMode::Help, &c), Action::ToggleHelp);
  }

  #[test]
  fn test_help_mode_other_keys_ignored() {
    let c = cfg();
    assert_eq!(map_key(key(KeyCode::Char('j')), InputMode::Help, &c), Action::None);
    assert_eq!(map_key(key(KeyCode::Char('q')), InputMode::Help, &c), Action::None);
    assert_eq!(map_key(key(KeyCode::Enter), InputMode::Help, &c), Action::None);
  }

  #[test]
  fn test_prompt_mode_char() {
    let c = cfg();
    assert_eq!(map_key(key(KeyCode::Char('a')), InputMode::Prompt, &c), Action::PromptInput('a'));
    assert_eq!(map_key(key(KeyCode::Char('y')), InputMode::Prompt, &c), Action::PromptInput('y'));
  }

  #[test]
  fn test_prompt_mode_enter() {
    let c = cfg();
    assert_eq!(map_key(key(KeyCode::Enter), InputMode::Prompt, &c), Action::PromptConfirm);
  }

  #[test]
  fn test_prompt_mode_esc() {
    let c = cfg();
    assert_eq!(map_key(key(KeyCode::Esc), InputMode::Prompt, &c), Action::PromptCancel);
  }

  #[test]
  fn test_prompt_mode_backspace() {
    let c = cfg();
    assert_eq!(map_key(key(KeyCode::Backspace), InputMode::Prompt, &c), Action::PromptBackspace);
  }

  #[test]
  fn test_favorites_mode_navigation() {
    let c = cfg();
    assert_eq!(map_key(key(KeyCode::Char('j')), InputMode::Favorites, &c), Action::FavoritesDown);
    assert_eq!(map_key(key(KeyCode::Down), InputMode::Favorites, &c), Action::FavoritesDown);
    assert_eq!(map_key(key(KeyCode::Char('k')), InputMode::Favorites, &c), Action::FavoritesUp);
    assert_eq!(map_key(key(KeyCode::Up), InputMode::Favorites, &c), Action::FavoritesUp);
    assert_eq!(map_key(key(KeyCode::Enter), InputMode::Favorites, &c), Action::FavoritesSelect);
    assert_eq!(map_key(key(KeyCode::Esc), InputMode::Favorites, &c), Action::FavoritesClose);
    assert_eq!(map_key(key(KeyCode::Char('q')), InputMode::Favorites, &c), Action::FavoritesClose);
    assert_eq!(map_key(key(KeyCode::Char('d')), InputMode::Favorites, &c), Action::FavoritesRemove);
    assert_eq!(map_key(key(KeyCode::Delete), InputMode::Favorites, &c), Action::FavoritesRemove);
    assert_eq!(map_key(key(KeyCode::Char('a')), InputMode::Favorites, &c), Action::FavoritesAddCurrent);
  }

  #[test]
  fn test_favorites_mode_other_keys_ignored() {
    let c = cfg();
    assert_eq!(map_key(key(KeyCode::Char('x')), InputMode::Favorites, &c), Action::None);
    assert_eq!(map_key(key(KeyCode::Char('z')), InputMode::Favorites, &c), Action::None);
  }

  #[test]
  fn test_open_with_mode_navigation() {
    let c = cfg();
    assert_eq!(map_key(key(KeyCode::Char('j')), InputMode::OpenWith, &c), Action::OpenWithDown);
    assert_eq!(map_key(key(KeyCode::Down), InputMode::OpenWith, &c), Action::OpenWithDown);
    assert_eq!(map_key(key(KeyCode::Char('k')), InputMode::OpenWith, &c), Action::OpenWithUp);
    assert_eq!(map_key(key(KeyCode::Up), InputMode::OpenWith, &c), Action::OpenWithUp);
    assert_eq!(map_key(key(KeyCode::Enter), InputMode::OpenWith, &c), Action::OpenWithSelect);
    assert_eq!(map_key(key(KeyCode::Esc), InputMode::OpenWith, &c), Action::OpenWithClose);
    assert_eq!(map_key(key(KeyCode::Char('q')), InputMode::OpenWith, &c), Action::OpenWithClose);
  }

  #[test]
  fn test_open_with_mode_other_keys_ignored() {
    let c = cfg();
    assert_eq!(map_key(key(KeyCode::Char('x')), InputMode::OpenWith, &c), Action::None);
    assert_eq!(map_key(key(KeyCode::Char('a')), InputMode::OpenWith, &c), Action::None);
    assert_eq!(map_key(key(KeyCode::Char(' ')), InputMode::OpenWith, &c), Action::None);
  }

  #[test]
  fn test_custom_config_remaps_key() {
    let mut c = cfg();
    let kb = crate::config::KeyBinding {
      code: KeyCode::Char('j'),
      modifiers: KeyModifiers::NONE,
    };
    c.normal_keys.insert(kb, Action::Quit);
    assert_eq!(map_key(key(KeyCode::Char('j')), InputMode::Normal, &c), Action::Quit);
  }

  #[test]
  fn test_visual_mode_navigation() {
    let c = cfg();
    assert_eq!(map_key(key(KeyCode::Char('j')), InputMode::Visual, &c), Action::VisualModeDown);
    assert_eq!(map_key(key(KeyCode::Down), InputMode::Visual, &c), Action::VisualModeDown);
    assert_eq!(map_key(key(KeyCode::Char('k')), InputMode::Visual, &c), Action::VisualModeUp);
    assert_eq!(map_key(key(KeyCode::Up), InputMode::Visual, &c), Action::VisualModeUp);
  }

  #[test]
  fn test_visual_mode_actions() {
    let c = cfg();
    assert_eq!(map_key(key(KeyCode::Esc), InputMode::Visual, &c), Action::ClearSelection);
    assert_eq!(map_key(key(KeyCode::Char(' ')), InputMode::Visual, &c), Action::ToggleSelection);
    assert_eq!(map_key(key(KeyCode::Char('d')), InputMode::Visual, &c), Action::DeleteFile);
    assert_eq!(map_key(key(KeyCode::Delete), InputMode::Visual, &c), Action::DeleteFile);
    assert_eq!(map_key(key(KeyCode::Char('y')), InputMode::Visual, &c), Action::YankPath);
  }

  #[test]
  fn test_visual_mode_other_keys_ignored() {
    let c = cfg();
    assert_eq!(map_key(key(KeyCode::Char('x')), InputMode::Visual, &c), Action::None);
    assert_eq!(map_key(key(KeyCode::Char('q')), InputMode::Visual, &c), Action::None);
  }
}
