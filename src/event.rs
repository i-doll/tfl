use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use anyhow::Result;
use crossterm::event::{self, Event as CrosstermEvent, KeyCode, KeyEvent};

use crate::action::Action;
use crate::config::{Config, normalize_key_event};

pub enum Event {
  Key(KeyEvent),
  Resize(u16, u16),
  Tick,
}

pub struct EventLoop {
  rx: mpsc::Receiver<Event>,
  paused: Arc<AtomicBool>,
}

impl EventLoop {
  pub fn new(tick_rate: Duration) -> Self {
    let (tx, rx) = mpsc::channel();
    let paused = Arc::new(AtomicBool::new(false));
    let thread_paused = paused.clone();

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

    Self { rx, paused }
  }

  pub fn pause(&self) {
    self.paused.store(true, Ordering::Relaxed);
  }

  pub fn resume(&self) {
    self.paused.store(false, Ordering::Relaxed);
    while self.rx.try_recv().is_ok() {}
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
      KeyCode::Char(c) => Action::PromptInput(c),
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
    assert_eq!(map_key(key(KeyCode::Char(' ')), InputMode::Normal, &c), Action::ToggleExpand);
    assert_eq!(map_key(key(KeyCode::Char('.')), InputMode::Normal, &c), Action::ToggleHidden);
    assert_eq!(map_key(key(KeyCode::Char('G')), InputMode::Normal, &c), Action::GoToBottom);
    assert_eq!(map_key(key(KeyCode::Char('g')), InputMode::Normal, &c), Action::GPress);
    assert_eq!(map_key(key(KeyCode::Char('/')), InputMode::Normal, &c), Action::SearchStart);
    assert_eq!(map_key(key(KeyCode::Char('J')), InputMode::Normal, &c), Action::ScrollPreviewDown);
    assert_eq!(map_key(key(KeyCode::Char('K')), InputMode::Normal, &c), Action::ScrollPreviewUp);
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
  fn test_custom_config_remaps_key() {
    let mut c = cfg();
    let kb = crate::config::KeyBinding {
      code: KeyCode::Char('j'),
      modifiers: KeyModifiers::NONE,
    };
    c.normal_keys.insert(kb, Action::Quit);
    assert_eq!(map_key(key(KeyCode::Char('j')), InputMode::Normal, &c), Action::Quit);
  }
}
