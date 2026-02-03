use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use anyhow::Result;
use crossterm::event::{self, Event as CrosstermEvent, KeyCode, KeyEvent, KeyModifiers};

use crate::action::Action;

pub enum Event {
  Key(KeyEvent),
  Resize(u16, u16),
  Tick,
}

pub struct EventLoop {
  rx: mpsc::Receiver<Event>,
}

impl EventLoop {
  pub fn new(tick_rate: Duration) -> Self {
    let (tx, rx) = mpsc::channel();

    thread::spawn(move || loop {
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

    Self { rx }
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
}

pub fn map_key(key: KeyEvent, mode: InputMode) -> Action {
  match mode {
    InputMode::Search => match key.code {
      KeyCode::Esc => Action::SearchCancel,
      KeyCode::Enter => Action::SearchConfirm,
      KeyCode::Backspace => Action::SearchBackspace,
      KeyCode::Char(c) => Action::SearchInput(c),
      _ => Action::None,
    },
    InputMode::GPrefix => match key.code {
      KeyCode::Char('g') => Action::GoToTop,
      _ => Action::None,
    },
    InputMode::Normal => match key.code {
      KeyCode::Char('q') => Action::Quit,
      KeyCode::Esc => Action::Quit,
      KeyCode::Char('j') | KeyCode::Down => Action::MoveDown,
      KeyCode::Char('k') | KeyCode::Up => Action::MoveUp,
      KeyCode::Char('h') | KeyCode::Left => Action::MoveLeft,
      KeyCode::Char('l') | KeyCode::Right => Action::MoveRight,
      KeyCode::Char(' ') | KeyCode::Enter => Action::ToggleExpand,
      KeyCode::Char('J') => Action::ScrollPreviewDown,
      KeyCode::Char('K') => Action::ScrollPreviewUp,
      KeyCode::Char('.') => Action::ToggleHidden,
      KeyCode::Char('g') => Action::GPress,
      KeyCode::Char('G') => Action::GoToBottom,
      KeyCode::Char('/') => Action::SearchStart,
      KeyCode::Char('y') => Action::YankPath,
      KeyCode::Char('e') => Action::OpenEditor,
      KeyCode::Char('c') => {
        if key.modifiers.contains(KeyModifiers::CONTROL) {
          Action::Quit
        } else {
          Action::OpenClaude
        }
      }
      KeyCode::Char('s') => Action::OpenShell,
      KeyCode::Char('ø') => Action::ShrinkTree,
      KeyCode::Char('æ') => Action::GrowTree,
      _ => Action::None,
    },
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

  #[test]
  fn test_normal_mode_quit() {
    assert_eq!(map_key(key(KeyCode::Char('q')), InputMode::Normal), Action::Quit);
    assert_eq!(map_key(key(KeyCode::Esc), InputMode::Normal), Action::Quit);
  }

  #[test]
  fn test_normal_mode_navigation() {
    assert_eq!(map_key(key(KeyCode::Char('j')), InputMode::Normal), Action::MoveDown);
    assert_eq!(map_key(key(KeyCode::Char('k')), InputMode::Normal), Action::MoveUp);
    assert_eq!(map_key(key(KeyCode::Down), InputMode::Normal), Action::MoveDown);
    assert_eq!(map_key(key(KeyCode::Up), InputMode::Normal), Action::MoveUp);
    assert_eq!(map_key(key(KeyCode::Char('h')), InputMode::Normal), Action::MoveLeft);
    assert_eq!(map_key(key(KeyCode::Char('l')), InputMode::Normal), Action::MoveRight);
  }

  #[test]
  fn test_normal_mode_actions() {
    assert_eq!(map_key(key(KeyCode::Char(' ')), InputMode::Normal), Action::ToggleExpand);
    assert_eq!(map_key(key(KeyCode::Char('.')), InputMode::Normal), Action::ToggleHidden);
    assert_eq!(map_key(key(KeyCode::Char('G')), InputMode::Normal), Action::GoToBottom);
    assert_eq!(map_key(key(KeyCode::Char('g')), InputMode::Normal), Action::GPress);
    assert_eq!(map_key(key(KeyCode::Char('/')), InputMode::Normal), Action::SearchStart);
    assert_eq!(map_key(key(KeyCode::Char('J')), InputMode::Normal), Action::ScrollPreviewDown);
    assert_eq!(map_key(key(KeyCode::Char('K')), InputMode::Normal), Action::ScrollPreviewUp);
  }

  #[test]
  fn test_ctrl_c_quits() {
    assert_eq!(
      map_key(key_with_mod(KeyCode::Char('c'), KeyModifiers::CONTROL), InputMode::Normal),
      Action::Quit
    );
  }

  #[test]
  fn test_c_opens_claude() {
    assert_eq!(map_key(key(KeyCode::Char('c')), InputMode::Normal), Action::OpenClaude);
  }

  #[test]
  fn test_search_mode() {
    assert_eq!(map_key(key(KeyCode::Char('a')), InputMode::Search), Action::SearchInput('a'));
    assert_eq!(map_key(key(KeyCode::Enter), InputMode::Search), Action::SearchConfirm);
    assert_eq!(map_key(key(KeyCode::Esc), InputMode::Search), Action::SearchCancel);
    assert_eq!(map_key(key(KeyCode::Backspace), InputMode::Search), Action::SearchBackspace);
  }

  #[test]
  fn test_g_prefix_mode() {
    assert_eq!(map_key(key(KeyCode::Char('g')), InputMode::GPrefix), Action::GoToTop);
    assert_eq!(map_key(key(KeyCode::Char('x')), InputMode::GPrefix), Action::None);
  }
}
