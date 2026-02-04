use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenApp {
  pub name: String,
  pub command: String,
  pub is_tui: bool,
  pub macos_app: Option<String>,
}

pub fn known_apps() -> Vec<OpenApp> {
  vec![
    OpenApp {
      name: "VS Code".into(),
      command: "code".into(),
      is_tui: false,
      macos_app: Some("Visual Studio Code".into()),
    },
    OpenApp {
      name: "Cursor".into(),
      command: "cursor".into(),
      is_tui: false,
      macos_app: Some("Cursor".into()),
    },
    OpenApp {
      name: "Zed".into(),
      command: "zed".into(),
      is_tui: false,
      macos_app: Some("Zed".into()),
    },
    OpenApp {
      name: "Sublime Text".into(),
      command: "subl".into(),
      is_tui: false,
      macos_app: Some("Sublime Text".into()),
    },
    OpenApp {
      name: "IntelliJ IDEA".into(),
      command: "idea".into(),
      is_tui: false,
      macos_app: Some("IntelliJ IDEA".into()),
    },
    OpenApp {
      name: "Neovim".into(),
      command: "nvim".into(),
      is_tui: true,
      macos_app: None,
    },
    OpenApp {
      name: "Vim".into(),
      command: "vim".into(),
      is_tui: true,
      macos_app: None,
    },
    OpenApp {
      name: "Helix".into(),
      command: "hx".into(),
      is_tui: true,
      macos_app: None,
    },
    OpenApp {
      name: "Emacs".into(),
      command: "emacs".into(),
      is_tui: true,
      macos_app: None,
    },
    OpenApp {
      name: "Nano".into(),
      command: "nano".into(),
      is_tui: true,
      macos_app: None,
    },
    OpenApp {
      name: "Micro".into(),
      command: "micro".into(),
      is_tui: true,
      macos_app: None,
    },
  ]
}

pub fn command_exists(cmd: &str) -> bool {
  Command::new("which")
    .arg(cmd)
    .stdout(std::process::Stdio::null())
    .stderr(std::process::Stdio::null())
    .status()
    .map(|s| s.success())
    .unwrap_or(false)
}

#[cfg(target_os = "macos")]
pub fn macos_app_exists(app_name: &str) -> bool {
  let mut dirs = vec![
    "/Applications".into(),
    "/System/Applications".into(),
    "/System/Applications/Utilities".into(),
  ];
  if let Some(home) = dirs::home_dir() {
    dirs.push(home.join("Applications").to_string_lossy().into_owned());
  }
  dirs.iter()
    .any(|dir| Path::new(&format!("{dir}/{app_name}.app")).exists())
}

#[cfg(not(target_os = "macos"))]
pub fn macos_app_exists(_app_name: &str) -> bool {
  false
}

pub fn app_available(app: &OpenApp) -> bool {
  if cfg!(target_os = "macos")
    && let Some(ref mac_app) = app.macos_app
    && macos_app_exists(mac_app)
  {
    return true;
  }
  command_exists(&app.command)
}

fn dedup_key(app: &OpenApp) -> String {
  if app.command.is_empty() {
    app.macos_app.clone().unwrap_or_default()
  } else {
    app.command.clone()
  }
}

pub fn detect_apps(custom: &[OpenApp]) -> Vec<OpenApp> {
  let mut apps = Vec::new();
  let mut seen = std::collections::HashSet::new();

  // Custom apps first
  for app in custom {
    if app_available(app) && seen.insert(dedup_key(app)) {
      apps.push(app.clone());
    }
  }

  // Built-in apps
  for app in known_apps() {
    if app_available(&app) && seen.insert(dedup_key(&app)) {
      apps.push(app);
    }
  }

  apps
}

pub fn open_default(path: &Path) -> Result<(), String> {
  open::that_detached(path).map_err(|e| format!("Failed to open: {e}"))
}

pub fn open_with_app(path: &Path, app: &OpenApp) -> Result<(), String> {
  #[cfg(target_os = "macos")]
  {
    if let Some(ref mac_app) = app.macos_app {
      return Command::new("open")
        .arg("-a")
        .arg(mac_app)
        .arg(path)
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("Failed to open with {}: {e}", app.name));
    }
  }

  Command::new(&app.command)
    .arg(path)
    .spawn()
    .map(|_| ())
    .map_err(|e| format!("Failed to open with {}: {e}", app.name))
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_known_apps_not_empty() {
    assert!(!known_apps().is_empty());
  }

  #[test]
  fn test_known_apps_have_names_and_commands() {
    for app in known_apps() {
      assert!(!app.name.is_empty());
      assert!(!app.command.is_empty());
    }
  }

  #[test]
  fn test_detect_apps_dedup_custom() {
    // If custom app has same command as built-in, only custom appears
    let custom = vec![OpenApp {
      name: "My Vim".into(),
      command: "vim".into(),
      is_tui: true,
      macos_app: None,
    }];
    let apps = detect_apps(&custom);
    let vim_count = apps.iter().filter(|a| a.command == "vim").count();
    assert!(vim_count <= 1, "vim should appear at most once, found {vim_count}");
    // If vim is available, the custom one should be first
    if let Some(vim) = apps.iter().find(|a| a.command == "vim") {
      assert_eq!(vim.name, "My Vim");
    }
  }

  #[test]
  fn test_detect_apps_custom_first() {
    let custom = vec![OpenApp {
      name: "Custom App".into(),
      command: "custom_nonexistent_binary_12345".into(),
      is_tui: false,
      macos_app: None,
    }];
    let apps = detect_apps(&custom);
    // Custom nonexistent app should not appear
    assert!(!apps.iter().any(|a| a.command == "custom_nonexistent_binary_12345"));
  }

  #[test]
  fn test_command_exists_which() {
    // `which` itself should always exist
    assert!(command_exists("which"));
  }

  #[test]
  fn test_command_not_exists() {
    assert!(!command_exists("nonexistent_binary_xyz_99999"));
  }
}
