mod action;
mod app;
mod config;
mod event;
mod fs;
mod icons;
mod preview;
mod ui;

use std::io;
use std::panic;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::Result;
use crossterm::execute;
use crossterm::terminal::{
  EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui_image::picker::Picker;

use crate::app::{App, SuspendAction};
use crate::event::{Event, EventLoop, map_key};

fn main() -> Result<()> {
  // Detect Kitty protocol support BEFORE entering alternate screen
  let picker = Picker::from_query_stdio().ok();

  // Install panic hook that restores terminal
  let original_hook = panic::take_hook();
  panic::set_hook(Box::new(move |info| {
    let _ = restore_terminal();
    original_hook(info);
  }));

  let root = std::env::args()
    .nth(1)
    .map(PathBuf::from)
    .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

  let root = std::fs::canonicalize(root)?;

  setup_terminal()?;
  let backend = CrosstermBackend::new(io::stdout());
  let mut terminal = Terminal::new(backend)?;

  let mut app = App::new(root, picker)?;
  // Trigger initial preview
  if !app.tree.entries.is_empty() {
    let path = app.tree.entries[0].path.clone();
    app.preview.request_preview(&path, app.picker.as_ref());
  }

  let events = EventLoop::new(Duration::from_millis(config::TICK_RATE_MS));

  loop {
    terminal.draw(|frame| ui::draw(frame, &mut app))?;

    match events.next()? {
      Event::Key(key) => {
        let action = map_key(key, app.input_mode);
        app.update(action)?;
      }
      Event::Resize(w, h) => {
        app.update(crate::action::Action::Resize(w, h))?;
      }
      Event::Tick => {
        app.update(crate::action::Action::Tick)?;
        // Clear status message after a tick
        app.status_message = None;
      }
    }

    // Handle suspend actions (editor, claude, shell)
    if let Some(suspend) = app.handle_suspend() {
      restore_terminal()?;
      terminal = suspend_and_resume(terminal, &suspend)?;
      // Reload tree after returning
      app.tree.reload()?;
      app.preview.invalidate();
    }

    if app.should_quit {
      break;
    }
  }

  restore_terminal()?;
  Ok(())
}

fn setup_terminal() -> Result<()> {
  enable_raw_mode()?;
  execute!(io::stdout(), EnterAlternateScreen)?;
  Ok(())
}

fn restore_terminal() -> Result<()> {
  disable_raw_mode()?;
  execute!(io::stdout(), LeaveAlternateScreen)?;
  Ok(())
}

fn suspend_and_resume(
  terminal: Terminal<CrosstermBackend<io::Stdout>>,
  action: &SuspendAction,
) -> Result<Terminal<CrosstermBackend<io::Stdout>>> {
  drop(terminal);
  App::execute_suspend(action)?;
  setup_terminal()?;
  let backend = CrosstermBackend::new(io::stdout());
  let terminal = Terminal::new(backend)?;
  Ok(terminal)
}
