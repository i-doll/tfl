mod action;
mod app;
mod config;
mod event;
mod favorites;
mod fs;
mod git;
mod icons;
mod opener;
mod preview;
mod ui;

use std::io;
use std::panic;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use anyhow::Result;
use crossterm::event::EnableMouseCapture;
use crossterm::event::DisableMouseCapture;
use crossterm::execute;
use crossterm::terminal::{
  EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui_image::picker::Picker;

use crate::app::{App, SuspendAction};
#[cfg(target_os = "linux")]
use crate::app::PickerOutput;
use crate::event::{Event, EventLoop, map_breadcrumb_click, map_key};

fn main() -> Result<()> {
  let args: Vec<String> = std::env::args().skip(1).collect();

  // Parse flags and positional path in a single pass
  let mut show_help = false;
  let mut show_version = false;
  let mut show_init = false;
  let mut show_hidden = false;
  #[cfg(target_os = "linux")]
  let mut pick_stdout = false;
  #[cfg(target_os = "linux")]
  let mut chooser_file: Option<String> = None;
  #[cfg(target_os = "linux")]
  let mut install_handler = false;
  #[cfg(target_os = "linux")]
  let mut uninstall_handler = false;
  #[cfg(target_os = "linux")]
  let mut install_portal = false;
  #[cfg(target_os = "linux")]
  let mut uninstall_portal = false;
  let mut path_arg: Option<String> = None;

  for arg in &args {
    match arg.as_str() {
      "--help" | "-h" => show_help = true,
      "--version" | "-V" => show_version = true,
      "--init" => show_init = true,
      "--all" | "-a" => show_hidden = true,
      #[cfg(target_os = "linux")]
      "--pick" => pick_stdout = true,
      #[cfg(target_os = "linux")]
      a if a.starts_with("--chooser-file=") => {
        chooser_file = Some(a.strip_prefix("--chooser-file=").unwrap().to_string());
      }
      #[cfg(target_os = "linux")]
      "--install-handler" => install_handler = true,
      #[cfg(target_os = "linux")]
      "--uninstall-handler" => uninstall_handler = true,
      #[cfg(target_os = "linux")]
      "--install-portal" => install_portal = true,
      #[cfg(target_os = "linux")]
      "--uninstall-portal" => uninstall_portal = true,
      #[cfg(not(target_os = "linux"))]
      "--pick" | "--install-handler" | "--uninstall-handler" | "--install-portal" | "--uninstall-portal" => {
        eprintln!("tfl: {arg} is only supported on Linux");
        std::process::exit(1);
      }
      #[cfg(not(target_os = "linux"))]
      a if a.starts_with("--chooser-file=") => {
        eprintln!("tfl: --chooser-file is only supported on Linux");
        std::process::exit(1);
      }
      a if !a.starts_with('-') => path_arg = Some(a.to_string()),
      _ => {
        eprintln!("tfl: unknown option '{arg}'");
        std::process::exit(1);
      }
    }
  }

  if show_help {
    println!("\
tfl - terminal file explorer

Usage: tfl [options] [path]

Options:
  -a, --all                Show hidden files");
    #[cfg(target_os = "linux")]
    println!(concat!(
      "  --pick                   File picker mode: print selected path to stdout\n",
      "  --chooser-file=PATH      File picker mode: write selected path to PATH\n",
      "  --install-handler        Set tfl as default file manager\n",
      "  --uninstall-handler      Restore previous default file manager\n",
      "  --install-portal         Set up file dialog integration\n",
      "  --uninstall-portal       Restore previous file dialog config",
    ));
    println!(concat!(
      "  --init                   Write default config files to ~/.config/tfl/\n",
      "  -h, --help               Print this help message\n",
      "  -V, --version            Print version\n",
      "\n",
      "If no path is given, opens the current directory.",
    ));
    return Ok(());
  }

  if show_version {
    println!("tfl {}", env!("CARGO_PKG_VERSION"));
    return Ok(());
  }

  #[cfg(target_os = "linux")]
  {
    if install_handler {
      return handler::install();
    }
    if uninstall_handler {
      return handler::uninstall();
    }
    if install_portal {
      return portal::install();
    }
    if uninstall_portal {
      return portal::uninstall();
    }
  }

  if show_init {
    let config_path = match config::Config::config_path() {
      Ok(p) => p,
      Err(e) => {
        eprintln!("tfl: {e}");
        std::process::exit(1);
      }
    };

    let apps_path = match config::Config::apps_path() {
      Ok(p) => p,
      Err(e) => {
        eprintln!("tfl: {e}");
        std::process::exit(1);
      }
    };

    let write_config = if config_path.exists() {
      eprint!("{} already exists. Overwrite? [y/N] ", config_path.display());
      let mut answer = String::new();
      io::stdin().read_line(&mut answer).unwrap_or(0);
      answer.trim().eq_ignore_ascii_case("y")
    } else {
      true
    };

    let write_apps = if apps_path.exists() {
      eprint!("{} already exists. Overwrite? [y/N] ", apps_path.display());
      let mut answer = String::new();
      io::stdin().read_line(&mut answer).unwrap_or(0);
      answer.trim().eq_ignore_ascii_case("y")
    } else {
      true
    };

    if write_config {
      match config::Config::dump_default_config(&config_path) {
        Ok(()) => println!("{}", config_path.display()),
        Err(e) => {
          eprintln!("tfl: {e}");
          std::process::exit(1);
        }
      }
    }

    if write_apps {
      match config::Config::dump_default_apps(&apps_path) {
        Ok(()) => println!("{}", apps_path.display()),
        Err(e) => {
          eprintln!("tfl: {e}");
          std::process::exit(1);
        }
      }
    }

    return Ok(());
  }

  // Determine picker mode
  #[cfg(target_os = "linux")]
  let picker_mode = if pick_stdout {
    Some(PickerOutput::Stdout)
  } else {
    chooser_file.map(|path| PickerOutput::ChooserFile(PathBuf::from(path)))
  };
  #[cfg(not(target_os = "linux"))]
  let picker_mode = None;

  let (mut config, config_errors) = config::Config::load();
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

  let mut app = App::new(root, picker, &config, picker_mode)?;

  if show_hidden {
    app.tree.show_hidden = true;
    app.tree.reload()?;
  }

  // Trigger initial preview
  if !app.tree.entries.is_empty() {
    let path = app.tree.entries[0].path.clone();
    app.preview.request_preview(&path, app.picker.as_ref(), app.tree.git_repo());
  }

  if !config_errors.is_empty() {
    app.show_error(config_errors);
  }

  let events = EventLoop::new(Duration::from_millis(config.tick_rate_ms), config_dir.as_deref());
  let mut last_reload = Instant::now() - Duration::from_secs(1);

  loop {
    terminal.draw(|frame| ui::draw(frame, &mut app, &config))?;

    match events.next()? {
      Event::Key(key) => {
        let action = map_key(key, app.input_mode, &config);
        app.update(action)?;
      }
      Event::Mouse(mouse) => {
        // Handle mouse clicks in the header row (row 0) for breadcrumb navigation
        if mouse.row == 0
          && let Some(action) = map_breadcrumb_click(mouse.column, &app.breadcrumb_segments)
        {
          app.update(action)?;
        }
      }
      Event::Resize(w, h) => {
        app.update(crate::action::Action::Resize(w, h))?;
      }
      Event::ConfigChanged => {
        if app.wrote_config {
          app.wrote_config = false;
          last_reload = Instant::now();
        } else if last_reload.elapsed() > Duration::from_millis(500) {
          reload_config(&mut config, &mut app);
          last_reload = Instant::now();
        }
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
        last_reload = Instant::now();
      }
      app.tree.reload()?;
      app.preview.invalidate();
      // Re-request preview for currently selected file
      if let Some(entry) = app.selected_entry() {
        let path = entry.path.clone();
        app.preview.request_preview(&path, app.picker.as_ref(), app.tree.git_repo());
      }
    }

    if app.should_quit {
      break;
    }
  }

  restore_terminal()?;

  // Handle picker output
  let is_picker = app.picker_mode.is_some();
  if let Err(e) = app.write_picked_paths() {
    eprintln!("tfl: {e}");
    std::process::exit(1);
  }
  if is_picker && app.picked_paths.is_empty() {
    std::process::exit(1);
  }

  Ok(())
}

fn setup_terminal() -> Result<()> {
  enable_raw_mode()?;
  execute!(io::stdout(), EnterAlternateScreen, EnableMouseCapture)?;
  Ok(())
}

fn restore_terminal() -> Result<()> {
  disable_raw_mode()?;
  execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture)?;
  Ok(())
}

fn reload_config(config: &mut config::Config, app: &mut App) {
  let (new, errors) = config::Config::load();
  config.normal_keys = new.normal_keys;
  config.g_prefix_keys = new.g_prefix_keys;
  config.search_keys = new.search_keys;
  config.custom_apps = new.custom_apps;
  config.claude_yolo = new.claude_yolo;
  config.has_apps_file = new.has_apps_file;
  config.ignore_patterns = new.ignore_patterns;
  config.use_gitignore = new.use_gitignore;
  config.use_custom_ignore = new.use_custom_ignore;
  config.ignore_glob_set = new.ignore_glob_set;
  app.apply_config(config);
  app.reload_favorites();
  if errors.is_empty() {
    app.set_status("Config reloaded".to_string());
  } else {
    app.show_error(errors);
  }
}

fn suspend_and_resume(
  terminal: Terminal<CrosstermBackend<io::Stdout>>,
  action: &SuspendAction,
) -> Result<Terminal<CrosstermBackend<io::Stdout>>> {
  drop(terminal);
  App::execute_suspend(action)?;
  setup_terminal()?;
  // Drain stale keystrokes buffered in the TTY while the subprocess ran
  while crossterm::event::poll(std::time::Duration::ZERO).unwrap_or(false) {
    let _ = crossterm::event::read();
  }
  let backend = CrosstermBackend::new(io::stdout());
  let terminal = Terminal::new(backend)?;
  Ok(terminal)
}

/// Returns the backup directory for handler/portal operations.
#[cfg(target_os = "linux")]
fn backup_dir() -> Result<PathBuf> {
  let dir = dirs::config_dir()
    .ok_or_else(|| anyhow::anyhow!("Could not determine config directory"))?
    .join("tfl")
    .join("handler-backup");
  std::fs::create_dir_all(&dir)?;
  Ok(dir)
}

#[cfg(target_os = "linux")]
mod handler {
  use std::path::PathBuf;
  use std::process::Command;

  use anyhow::Result;

  const DESKTOP_ENTRY: &str = "\
[Desktop Entry]
Type=Application
Name=tfl
GenericName=File Manager
Comment=Terminal file explorer with vim-style navigation
Exec=tfl %f
Icon=system-file-manager
Terminal=true
Categories=System;FileManager;ConsoleOnly;
MimeType=inode/directory;
";

  pub fn install() -> Result<()> {
    let backup = super::backup_dir()?;

    // Query current default handler
    let output = Command::new("xdg-mime")
      .args(["query", "default", "inode/directory"])
      .output()?;
    let current = String::from_utf8_lossy(&output.stdout).trim().to_string();

    if !current.is_empty() && current != "tfl.desktop" {
      let backup_file = backup.join("mime-handler");
      std::fs::write(&backup_file, &current)?;
      println!("Backed up current handler: {current}");
    }

    // Write desktop file
    let apps_dir = dirs::data_dir()
      .ok_or_else(|| anyhow::anyhow!("Could not determine data directory"))?
      .join("applications");
    std::fs::create_dir_all(&apps_dir)?;
    let desktop_path = apps_dir.join("tfl.desktop");
    std::fs::write(&desktop_path, DESKTOP_ENTRY)?;
    println!("Installed: {}", desktop_path.display());

    // Set as default
    let status = Command::new("xdg-mime")
      .args(["default", "tfl.desktop", "inode/directory"])
      .status()?;
    if !status.success() {
      anyhow::bail!("xdg-mime default failed");
    }
    println!("Set tfl as default file manager for inode/directory");

    Ok(())
  }

  pub fn uninstall() -> Result<()> {
    let backup = super::backup_dir()?;
    let backup_file = backup.join("mime-handler");

    // Restore previous handler
    if backup_file.exists() {
      let old_handler = std::fs::read_to_string(&backup_file)?.trim().to_string();
      if !old_handler.is_empty() {
        let status = Command::new("xdg-mime")
          .args(["default", &old_handler, "inode/directory"])
          .status()?;
        if !status.success() {
          anyhow::bail!("xdg-mime default failed");
        }
        println!("Restored default handler: {old_handler}");
      }
      std::fs::remove_file(&backup_file)?;
    } else {
      println!("No backup found — nothing to restore");
    }

    // Remove desktop file
    let desktop_path: PathBuf = dirs::data_dir()
      .ok_or_else(|| anyhow::anyhow!("Could not determine data directory"))?
      .join("applications")
      .join("tfl.desktop");
    if desktop_path.exists() {
      std::fs::remove_file(&desktop_path)?;
      println!("Removed: {}", desktop_path.display());
    }

    Ok(())
  }
}

#[cfg(target_os = "linux")]
mod portal {
  use std::path::PathBuf;

  use anyhow::Result;

  const WRAPPER_SCRIPT: &str = r#"#!/bin/bash
# tfl wrapper for xdg-desktop-portal-termfilechooser
# Args: $1=multiple $2=directory $3=save $4=path $5=out_file $6=debug
set -euo pipefail

multiple="$1"
directory="$2"
save="$3"
path="$4"
out="$5"

if [ "$save" = "1" ]; then
  dir="$(dirname "$path")"
  tfl --chooser-file="$out" "$dir"
elif [ "$directory" = "1" ]; then
  tfl --chooser-file="$out" "${path:-.}"
else
  tfl --chooser-file="$out" "${path:-.}"
fi
"#;

  fn find_portal_file() -> bool {
    let search_dirs = [
      "/usr/share/xdg-desktop-portal/portals",
      "/usr/lib/xdg-desktop-portal/portals",
    ];
    for dir in &search_dirs {
      let path = PathBuf::from(dir).join("termfilechooser.portal");
      if path.exists() {
        return true;
      }
    }
    false
  }

  pub fn install() -> Result<()> {
    if !find_portal_file() {
      anyhow::bail!(
        "xdg-desktop-portal-termfilechooser not found.\n\
         Install it first: https://github.com/GermainZ/xdg-desktop-portal-termfilechooser"
      );
    }

    let backup = super::backup_dir()?;
    let config_home = dirs::config_dir()
      .ok_or_else(|| anyhow::anyhow!("Could not determine config directory"))?;

    // termfilechooser config dir
    let tfc_dir = config_home.join("xdg-desktop-portal-termfilechooser");
    std::fs::create_dir_all(&tfc_dir)?;

    // Backup existing termfilechooser config
    let tfc_config = tfc_dir.join("config");
    if tfc_config.exists() {
      let backup_file = backup.join("termfilechooser-config");
      std::fs::copy(&tfc_config, &backup_file)?;
      println!("Backed up: {}", tfc_config.display());
    }

    // Backup existing portals.conf
    let portal_dir = config_home.join("xdg-desktop-portal");
    std::fs::create_dir_all(&portal_dir)?;
    let portals_conf = portal_dir.join("portals.conf");
    if portals_conf.exists() {
      let backup_file = backup.join("portals.conf");
      std::fs::copy(&portals_conf, &backup_file)?;
      println!("Backed up: {}", portals_conf.display());
    }

    // Write wrapper script
    let wrapper_path = tfc_dir.join("tfl-wrapper.sh");
    std::fs::write(&wrapper_path, WRAPPER_SCRIPT)?;
    #[cfg(unix)]
    {
      use std::os::unix::fs::PermissionsExt;
      std::fs::set_permissions(&wrapper_path, std::fs::Permissions::from_mode(0o755))?;
    }
    println!("Installed: {}", wrapper_path.display());

    // Write termfilechooser config
    std::fs::write(&tfc_config, "[filechooser]\ncmd=tfl-wrapper.sh\n")?;
    println!("Wrote: {}", tfc_config.display());

    // Update portals.conf
    let portal_key = "org.freedesktop.impl.portal.FileChooser";
    let portal_value = "termfilechooser";
    let new_line = format!("{portal_key}={portal_value}");

    if portals_conf.exists() {
      let content = std::fs::read_to_string(&portals_conf)?;
      if content.contains(portal_key) {
        // Replace existing line
        let updated: Vec<String> = content
          .lines()
          .map(|line| {
            if line.trim_start().starts_with(portal_key) {
              new_line.clone()
            } else {
              line.to_string()
            }
          })
          .collect();
        std::fs::write(&portals_conf, updated.join("\n") + "\n")?;
      } else {
        // Append to file
        use std::io::Write;
        let mut f = std::fs::OpenOptions::new().append(true).open(&portals_conf)?;
        writeln!(f, "{new_line}")?;
      }
    } else {
      std::fs::write(&portals_conf, format!("[preferred]\n{new_line}\n"))?;
    }
    println!("Updated: {}", portals_conf.display());
    println!("\nRestart the portal to apply: systemctl --user restart xdg-desktop-portal");

    Ok(())
  }

  pub fn uninstall() -> Result<()> {
    let backup = super::backup_dir()?;
    let config_home = dirs::config_dir()
      .ok_or_else(|| anyhow::anyhow!("Could not determine config directory"))?;

    let tfc_dir = config_home.join("xdg-desktop-portal-termfilechooser");

    // Restore termfilechooser config
    let backup_tfc = backup.join("termfilechooser-config");
    let tfc_config = tfc_dir.join("config");
    if backup_tfc.exists() {
      std::fs::copy(&backup_tfc, &tfc_config)?;
      std::fs::remove_file(&backup_tfc)?;
      println!("Restored: {}", tfc_config.display());
    } else if tfc_config.exists() {
      std::fs::remove_file(&tfc_config)?;
      println!("Removed: {}", tfc_config.display());
    }

    // Restore portals.conf
    let portal_dir = config_home.join("xdg-desktop-portal");
    let portals_conf = portal_dir.join("portals.conf");
    let backup_portals = backup.join("portals.conf");
    if backup_portals.exists() {
      std::fs::copy(&backup_portals, &portals_conf)?;
      std::fs::remove_file(&backup_portals)?;
      println!("Restored: {}", portals_conf.display());
    }

    // Remove wrapper script
    let wrapper_path = tfc_dir.join("tfl-wrapper.sh");
    if wrapper_path.exists() {
      std::fs::remove_file(&wrapper_path)?;
      println!("Removed: {}", wrapper_path.display());
    }

    println!("\nRestart the portal to apply: systemctl --user restart xdg-desktop-portal");

    Ok(())
  }
}
