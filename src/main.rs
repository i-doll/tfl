mod action;
mod app;
mod config;
mod event;
mod favorites;
mod fs;
mod icons;
mod opener;
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
  let args: Vec<String> = std::env::args().skip(1).collect();

  // Parse flags and positional path in a single pass
  let mut show_help = false;
  let mut show_version = false;
  let mut show_init = false;
  let mut show_hidden = false;
  let mut path_arg: Option<String> = None;

  for arg in &args {
    match arg.as_str() {
      "--help" | "-h" => show_help = true,
      "--version" | "-V" => show_version = true,
      "--init" => show_init = true,
      "--all" | "-a" => show_hidden = true,
      a if !a.starts_with('-') => path_arg = Some(a.to_string()),
      _ => {
        eprintln!("tfl: unknown option '{arg}'");
        std::process::exit(1);
      }
    }
  }

  if show_help {
    println!(
      "\
tfl - terminal file explorer

Usage: tfl [options] [path]

Options:
  -a, --all      Show hidden files
  --init         Write default config to ~/.config/tfl/config.toml
  -h, --help     Print this help message
  -V, --version  Print version

If no path is given, opens the current directory."
    );
    return Ok(());
  }

  if show_version {
    println!("tfl {}", env!("CARGO_PKG_VERSION"));
    return Ok(());
  }

  if show_init {
    let path = match config::Config::config_path() {
      Ok(p) => p,
      Err(e) => {
        eprintln!("tfl: {e}");
        std::process::exit(1);
      }
    };

    if path.exists() {
      eprint!("{} already exists. Overwrite? [y/N] ", path.display());
      let mut answer = String::new();
      io::stdin().read_line(&mut answer).unwrap_or(0);
      if !answer.trim().eq_ignore_ascii_case("y") {
        return Ok(());
      }
    }

    match config::Config::dump_default_config(&path) {
      Ok(()) => {
        println!("{}", path.display());
        return Ok(());
      }
      Err(e) => {
        eprintln!("tfl: {e}");
        std::process::exit(1);
      }
    }
  }

  let mut config = config::Config::load();
  let config_dir = dirs::config_dir().map(|d| d.join("tfl"));

  // Detect Kitty protocol support BEFORE entering alternate screen
  let picker = Picker::from_query_stdio().ok();

  // Install panic hook that restores terminal
  let original_hook = panic::take_hook();
  panic::set_hook(Box::new(move |info| {
    let _ = restore_terminal();
    original_hook(info);
  }));

  let root = path_arg
    .map(PathBuf::from)
    .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

  let root = std::fs::canonicalize(root)?;

  setup_terminal()?;
  let backend = CrosstermBackend::new(io::stdout());
  let mut terminal = Terminal::new(backend)?;

  let mut app = App::new(root, picker, &config)?;

  if show_hidden {
    app.tree.show_hidden = true;
    app.tree.reload()?;
  }

  // Trigger initial preview
  if !app.tree.entries.is_empty() {
    let path = app.tree.entries[0].path.clone();
    app.preview.request_preview(&path, app.picker.as_ref());
  }

  let events = EventLoop::new(Duration::from_millis(config.tick_rate_ms), config_dir.as_deref());

  loop {
    terminal.draw(|frame| ui::draw(frame, &mut app, &config))?;

    match events.next()? {
      Event::Key(key) => {
        let action = map_key(key, app.input_mode, &config);
        app.update(action)?;
      }
      Event::Resize(w, h) => {
        app.update(crate::action::Action::Resize(w, h))?;
      }
      Event::ConfigChanged => {
        reload_config(&mut config, &mut app);
      }
      Event::Tick => {
        app.update(crate::action::Action::Tick)?;
        // Clear status message after it's been visible for a few ticks
        if app.input_mode == crate::event::InputMode::Normal {
          if app.status_ticks > 0 {
            app.status_ticks -= 1;
          } else {
            app.status_message = None;
          }
        }
      }
    }

    // Handle suspend actions (editor, claude, shell)
    if let Some(suspend) = app.handle_suspend() {
      events.pause();
      restore_terminal()?;
      terminal = suspend_and_resume(terminal, &suspend)?;
      let config_changed = events.resume();
      if config_changed {
        reload_config(&mut config, &mut app);
      }
      app.tree.reload()?;
      app.preview.invalidate();
      // Re-request preview for currently selected file
      if let Some(entry) = app.selected_entry() {
        let path = entry.path.clone();
        app.preview.request_preview(&path, app.picker.as_ref());
      }
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

fn reload_config(config: &mut config::Config, app: &mut App) {
  let new = config::Config::load();
  config.normal_keys = new.normal_keys;
  config.g_prefix_keys = new.g_prefix_keys;
  config.custom_apps = new.custom_apps;
  app.apply_config(config);
  app.reload_favorites();
  app.set_status("Config reloaded".to_string());
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
