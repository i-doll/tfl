use std::collections::HashMap;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde::Deserialize;

use crate::action::Action;
use crate::opener::OpenApp;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyBinding {
  pub code: KeyCode,
  pub modifiers: KeyModifiers,
}

impl KeyBinding {
  pub fn display_key(&self) -> String {
    let key_name = match self.code {
      KeyCode::Char(' ') => "Space".to_string(),
      KeyCode::Char(c) => c.to_string(),
      KeyCode::Enter => "Enter".to_string(),
      KeyCode::Esc => "Esc".to_string(),
      KeyCode::Backspace => "Backspace".to_string(),
      KeyCode::Delete => "Delete".to_string(),
      KeyCode::Tab => "Tab".to_string(),
      KeyCode::PageUp => "PageUp".to_string(),
      KeyCode::PageDown => "PageDown".to_string(),
      KeyCode::Up => "Up".to_string(),
      KeyCode::Down => "Down".to_string(),
      KeyCode::Left => "Left".to_string(),
      KeyCode::Right => "Right".to_string(),
      KeyCode::F(n) => format!("F{n}"),
      _ => format!("{:?}", self.code),
    };

    if self.modifiers.contains(KeyModifiers::CONTROL) {
      format!("Ctrl+{key_name}")
    } else if self.modifiers.contains(KeyModifiers::ALT) {
      format!("Alt+{key_name}")
    } else {
      key_name
    }
  }
}

pub struct Config {
  pub tree_ratio: u16,
  pub min_tree_ratio: u16,
  pub max_tree_ratio: u16,
  pub ratio_step: u16,
  pub tick_rate_ms: u64,
  pub normal_keys: HashMap<KeyBinding, Action>,
  pub g_prefix_keys: HashMap<KeyBinding, Action>,
  pub custom_apps: Vec<OpenApp>,
}

#[derive(Deserialize, Default)]
struct TomlConfig {
  general: Option<GeneralConfig>,
  keys: Option<KeysConfig>,
}

#[derive(Deserialize)]
struct AppEntry {
  name: String,
  command: Option<String>,
  macos_app: Option<String>,
  tui: Option<bool>,
}

#[derive(Deserialize, Default)]
struct AppsFile {
  #[serde(default)]
  apps: Vec<AppEntry>,
}

#[derive(Deserialize, Default)]
struct GeneralConfig {
  tree_ratio: Option<u16>,
  tick_rate_ms: Option<u64>,
}

#[derive(Deserialize, Default)]
struct KeysConfig {
  normal: Option<HashMap<String, String>>,
  g_prefix: Option<HashMap<String, String>>,
}

pub fn parse_key_binding(s: &str) -> Option<KeyBinding> {
  if s.is_empty() {
    return None;
  }

  let parts: Vec<&str> = s.split('+').collect();

  if parts.len() == 1 {
    let key = parts[0];
    if let Some(code) = named_key(key) {
      return Some(KeyBinding { code, modifiers: KeyModifiers::NONE });
    }
    let chars: Vec<char> = key.chars().collect();
    if chars.len() == 1 {
      let c = chars[0];
      if c.is_uppercase() {
        return Some(KeyBinding { code: KeyCode::Char(c), modifiers: KeyModifiers::NONE });
      }
      return Some(KeyBinding { code: KeyCode::Char(c), modifiers: KeyModifiers::NONE });
    }
    return None;
  }

  if parts.len() == 2 {
    let modifier_str = parts[0].to_lowercase();
    let key_str = parts[1];

    let modifiers = match modifier_str.as_str() {
      "ctrl" => KeyModifiers::CONTROL,
      "shift" => {
        let chars: Vec<char> = key_str.chars().collect();
        if chars.len() == 1 {
          let c = chars[0].to_uppercase().next().unwrap_or(chars[0]);
          return Some(KeyBinding { code: KeyCode::Char(c), modifiers: KeyModifiers::NONE });
        }
        if let Some(code) = named_key(key_str) {
          return Some(KeyBinding { code, modifiers: KeyModifiers::SHIFT });
        }
        return None;
      }
      "alt" => KeyModifiers::ALT,
      _ => return None,
    };

    if let Some(code) = named_key(key_str) {
      return Some(KeyBinding { code, modifiers });
    }
    let chars: Vec<char> = key_str.chars().collect();
    if chars.len() == 1 {
      return Some(KeyBinding { code: KeyCode::Char(chars[0]), modifiers });
    }
    return None;
  }

  None
}

fn named_key(s: &str) -> Option<KeyCode> {
  match s.to_lowercase().as_str() {
    "enter" => Some(KeyCode::Enter),
    "space" => Some(KeyCode::Char(' ')),
    "esc" => Some(KeyCode::Esc),
    "up" => Some(KeyCode::Up),
    "down" => Some(KeyCode::Down),
    "left" => Some(KeyCode::Left),
    "right" => Some(KeyCode::Right),
    "backspace" => Some(KeyCode::Backspace),
    "delete" => Some(KeyCode::Delete),
    "tab" => Some(KeyCode::Tab),
    "pageup" => Some(KeyCode::PageUp),
    "pagedown" => Some(KeyCode::PageDown),
    s if s.starts_with('f') && s.len() > 1 => {
      s[1..].parse::<u8>().ok().filter(|&n| (1..=24).contains(&n)).map(KeyCode::F)
    }
    _ => None,
  }
}

pub fn normalize_key_event(key: KeyEvent) -> KeyBinding {
  let mut modifiers = key.modifiers;
  if let KeyCode::Char(c) = key.code
    && c.is_uppercase()
  {
    modifiers -= KeyModifiers::SHIFT;
  }
  KeyBinding { code: key.code, modifiers }
}

impl Default for Config {
  fn default() -> Self {
    let mut config = Config::empty();
    let mut errors = Vec::new();
    config.apply_toml_str(Config::default_toml(), &mut errors);
    config
  }
}

impl Config {
  fn empty() -> Self {
    Config {
      tree_ratio: 30,
      min_tree_ratio: 15,
      max_tree_ratio: 60,
      ratio_step: 5,
      tick_rate_ms: 100,
      normal_keys: HashMap::new(),
      g_prefix_keys: HashMap::new(),
      custom_apps: Vec::new(),
    }
  }

  fn apply_toml_str(&mut self, s: &str, errors: &mut Vec<String>) {
    let toml_config: TomlConfig = match toml::from_str(s) {
      Ok(c) => c,
      Err(e) => {
        errors.push(format!("failed to parse config.toml: {e}"));
        return;
      }
    };

    if let Some(general) = toml_config.general {
      if let Some(ratio) = general.tree_ratio {
        self.tree_ratio = ratio;
      }
      if let Some(tick) = general.tick_rate_ms {
        self.tick_rate_ms = tick;
      }
    }

    if let Some(keys) = toml_config.keys {
      if let Some(normal) = keys.normal {
        self.normal_keys.clear();
        for (key_str, action_str) in &normal {
          let Some(kb) = parse_key_binding(key_str) else {
            errors.push(format!("invalid key binding: {key_str:?}"));
            continue;
          };
          let Some(action) = Action::from_name(action_str) else {
            errors.push(format!("invalid action: {action_str:?}"));
            continue;
          };
          self.normal_keys.insert(kb, action);
        }
      }
      if let Some(g_prefix) = keys.g_prefix {
        self.g_prefix_keys.clear();
        for (key_str, action_str) in &g_prefix {
          let Some(kb) = parse_key_binding(key_str) else {
            errors.push(format!("invalid key binding: {key_str:?}"));
            continue;
          };
          let Some(action) = Action::from_name(action_str) else {
            errors.push(format!("invalid action: {action_str:?}"));
            continue;
          };
          self.g_prefix_keys.insert(kb, action);
        }
      }
    }

  }

  pub fn default_toml() -> &'static str {
    r#"[general]
tree_ratio = 30       # initial tree pane width (percentage)
tick_rate_ms = 100    # event loop tick rate in ms

[keys.normal]
j = "move_down"
k = "move_up"
h = "move_left"
l = "move_right"
down = "move_down"
up = "move_up"
left = "move_left"
right = "move_right"
space = "toggle_expand"
enter = "open_default"
o = "open_with"
"shift+j" = "scroll_preview_down"
"shift+k" = "scroll_preview_up"
pagedown = "scroll_preview_down"
pageup = "scroll_preview_up"
"." = "toggle_hidden"
"shift+g" = "go_to_bottom"
g = "g_press"
"/" = "search_start"
y = "yank_path"
e = "open_editor"
c = "open_claude"
s = "open_shell"
q = "quit"
esc = "quit"
delete = "delete_file"
"ctrl+x" = "cut_file"
"ctrl+v" = "paste"
"ctrl+c" = "copy_file"
r = "rename_start"
f2 = "rename_start"
a = "new_file_start"
"shift+a" = "new_dir_start"
"ø" = "shrink_tree"
"æ" = "grow_tree"
"?" = "toggle_help"
"~" = "go_home"
f = "favorites_open"
"shift+f" = "favorite_add"

[keys.g_prefix]
g = "go_to_top"
h = "go_home"
"#
  }

  pub fn reverse_lookup(&self) -> HashMap<Action, Vec<String>> {
    let mut map: HashMap<Action, Vec<String>> = HashMap::new();
    for (kb, action) in &self.normal_keys {
      map.entry(action.clone()).or_default().push(kb.display_key());
    }
    for (kb, action) in &self.g_prefix_keys {
      let key_str = format!("g{}", kb.display_key());
      map.entry(action.clone()).or_default().push(key_str);
    }
    // Sort keys for deterministic display
    for keys in map.values_mut() {
      keys.sort();
    }
    map
  }

  pub fn config_path() -> Result<std::path::PathBuf, String> {
    dirs::config_dir()
      .map(|d| d.join("tfl").join("config.toml"))
      .ok_or_else(|| "could not determine config directory".to_string())
  }

  pub fn dump_default_config(path: &std::path::Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
      std::fs::create_dir_all(parent)
        .map_err(|e| format!("failed to create {}: {e}", parent.display()))?;
    }

    std::fs::write(path, Self::default_toml())
      .map_err(|e| format!("failed to write {}: {e}", path.display()))?;

    Ok(())
  }

  pub fn load() -> (Config, Vec<String>) {
    let config_dir = dirs::config_dir().map(|d| d.join("tfl"));
    let mut errors = Vec::new();

    let content = config_dir
      .as_ref()
      .map(|d| d.join("config.toml"))
      .and_then(|p| std::fs::read_to_string(p).ok());

    let mut config = match content {
      Some(s) => Self::load_from_str_with_errors(&s, &mut errors),
      None => Config::default(),
    };

    let apps_content = config_dir
      .map(|d| d.join("apps.toml"))
      .and_then(|p| std::fs::read_to_string(p).ok());

    if let Some(s) = apps_content {
      config.load_apps_str(&s, &mut errors);
    }

    (config, errors)
  }

  fn load_apps_str(&mut self, s: &str, errors: &mut Vec<String>) {
    let apps_file: AppsFile = match toml::from_str(s) {
      Ok(f) => f,
      Err(e) => {
        errors.push(format!("failed to parse apps.toml: {e}"));
        return;
      }
    };

    for entry in apps_file.apps {
      if entry.command.is_none() && entry.macos_app.is_none() {
        errors.push(format!("app {:?} needs command or macos_app", entry.name));
        continue;
      }
      self.custom_apps.push(OpenApp {
        name: entry.name,
        command: entry.command.unwrap_or_default(),
        is_tui: entry.tui.unwrap_or(false),
        macos_app: entry.macos_app,
      });
    }
  }

  pub fn load_from_str(s: &str) -> Config {
    let mut errors = Vec::new();
    Self::load_from_str_with_errors(s, &mut errors)
  }

  fn load_from_str_with_errors(s: &str, errors: &mut Vec<String>) -> Config {
    let mut config = Config::default();
    config.apply_toml_str(s, errors);
    config
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};

  // --- parse_key_binding tests ---

  #[test]
  fn test_parse_single_char() {
    let kb = parse_key_binding("j").unwrap();
    assert_eq!(kb.code, KeyCode::Char('j'));
    assert_eq!(kb.modifiers, KeyModifiers::NONE);
  }

  #[test]
  fn test_parse_uppercase_char() {
    let kb = parse_key_binding("J").unwrap();
    assert_eq!(kb.code, KeyCode::Char('J'));
    assert_eq!(kb.modifiers, KeyModifiers::NONE);
  }

  #[test]
  fn test_parse_shift_modifier() {
    let kb = parse_key_binding("shift+j").unwrap();
    assert_eq!(kb.code, KeyCode::Char('J'));
    assert_eq!(kb.modifiers, KeyModifiers::NONE);
    // shift+j and J produce the same KeyBinding
    assert_eq!(kb, parse_key_binding("J").unwrap());
  }

  #[test]
  fn test_parse_ctrl_modifier() {
    let kb = parse_key_binding("ctrl+c").unwrap();
    assert_eq!(kb.code, KeyCode::Char('c'));
    assert_eq!(kb.modifiers, KeyModifiers::CONTROL);
  }

  #[test]
  fn test_parse_named_keys() {
    assert_eq!(parse_key_binding("enter").unwrap().code, KeyCode::Enter);
    assert_eq!(parse_key_binding("space").unwrap().code, KeyCode::Char(' '));
    assert_eq!(parse_key_binding("esc").unwrap().code, KeyCode::Esc);
    assert_eq!(parse_key_binding("up").unwrap().code, KeyCode::Up);
    assert_eq!(parse_key_binding("down").unwrap().code, KeyCode::Down);
    assert_eq!(parse_key_binding("left").unwrap().code, KeyCode::Left);
    assert_eq!(parse_key_binding("right").unwrap().code, KeyCode::Right);
    assert_eq!(parse_key_binding("backspace").unwrap().code, KeyCode::Backspace);
    assert_eq!(parse_key_binding("tab").unwrap().code, KeyCode::Tab);
  }

  #[test]
  fn test_parse_multibyte_char() {
    let kb = parse_key_binding("ø").unwrap();
    assert_eq!(kb.code, KeyCode::Char('ø'));
    assert_eq!(kb.modifiers, KeyModifiers::NONE);
  }

  #[test]
  fn test_parse_empty_string() {
    assert!(parse_key_binding("").is_none());
  }

  #[test]
  fn test_parse_invalid_string() {
    assert!(parse_key_binding("foobar").is_none());
  }

  // --- normalize_key_event tests ---

  fn key_event(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
    KeyEvent {
      code,
      modifiers,
      kind: KeyEventKind::Press,
      state: KeyEventState::NONE,
    }
  }

  #[test]
  fn test_normalize_plain_key() {
    let kb = normalize_key_event(key_event(KeyCode::Char('j'), KeyModifiers::NONE));
    assert_eq!(kb.code, KeyCode::Char('j'));
    assert_eq!(kb.modifiers, KeyModifiers::NONE);
  }

  #[test]
  fn test_normalize_uppercase_strips_shift() {
    let kb = normalize_key_event(key_event(KeyCode::Char('J'), KeyModifiers::SHIFT));
    assert_eq!(kb.code, KeyCode::Char('J'));
    assert_eq!(kb.modifiers, KeyModifiers::NONE);
  }

  #[test]
  fn test_normalize_ctrl_preserves_modifier() {
    let kb = normalize_key_event(key_event(KeyCode::Char('c'), KeyModifiers::CONTROL));
    assert_eq!(kb.code, KeyCode::Char('c'));
    assert_eq!(kb.modifiers, KeyModifiers::CONTROL);
  }

  // --- Config::default tests ---

  #[test]
  fn test_default_general_values() {
    let config = Config::default();
    assert_eq!(config.tree_ratio, 30);
    assert_eq!(config.min_tree_ratio, 15);
    assert_eq!(config.max_tree_ratio, 60);
    assert_eq!(config.ratio_step, 5);
    assert_eq!(config.tick_rate_ms, 100);
  }

  #[test]
  fn test_default_has_all_normal_bindings() {
    let config = Config::default();
    let n = KeyModifiers::NONE;

    let expected = vec![
      (KeyCode::Char('q'), n, Action::Quit),
      (KeyCode::Esc, n, Action::Quit),
      (KeyCode::Char('j'), n, Action::MoveDown),
      (KeyCode::Down, n, Action::MoveDown),
      (KeyCode::Char('k'), n, Action::MoveUp),
      (KeyCode::Up, n, Action::MoveUp),
      (KeyCode::Char('h'), n, Action::MoveLeft),
      (KeyCode::Left, n, Action::MoveLeft),
      (KeyCode::Char('l'), n, Action::MoveRight),
      (KeyCode::Right, n, Action::MoveRight),
      (KeyCode::Char(' '), n, Action::ToggleExpand),
      (KeyCode::Enter, n, Action::OpenDefault),
      (KeyCode::Char('o'), n, Action::OpenWithStart),
      (KeyCode::Char('J'), n, Action::ScrollPreviewDown),
      (KeyCode::Char('K'), n, Action::ScrollPreviewUp),
      (KeyCode::PageDown, n, Action::ScrollPreviewDown),
      (KeyCode::PageUp, n, Action::ScrollPreviewUp),
      (KeyCode::Char('.'), n, Action::ToggleHidden),
      (KeyCode::Char('g'), n, Action::GPress),
      (KeyCode::Char('G'), n, Action::GoToBottom),
      (KeyCode::Char('/'), n, Action::SearchStart),
      (KeyCode::Char('y'), n, Action::YankPath),
      (KeyCode::Char('e'), n, Action::OpenEditor),
      (KeyCode::Char('c'), n, Action::OpenClaude),
      (KeyCode::Char('s'), n, Action::OpenShell),
      (KeyCode::Delete, n, Action::DeleteFile),
      (KeyCode::Char('x'), KeyModifiers::CONTROL, Action::CutFile),
      (KeyCode::Char('v'), KeyModifiers::CONTROL, Action::Paste),
      (KeyCode::Char('c'), KeyModifiers::CONTROL, Action::CopyFile),
      (KeyCode::Char('r'), n, Action::RenameStart),
      (KeyCode::F(2), n, Action::RenameStart),
      (KeyCode::Char('a'), n, Action::NewFileStart),
      (KeyCode::Char('A'), n, Action::NewDirStart),
      (KeyCode::Char('ø'), n, Action::ShrinkTree),
      (KeyCode::Char('æ'), n, Action::GrowTree),
      (KeyCode::Char('?'), n, Action::ToggleHelp),
      (KeyCode::Char('~'), n, Action::GoHome),
      (KeyCode::Char('f'), n, Action::FavoritesOpen),
      (KeyCode::Char('F'), n, Action::FavoriteAdd),
    ];

    for (code, mods, action) in expected {
      let kb = KeyBinding { code, modifiers: mods };
      assert_eq!(
        config.normal_keys.get(&kb),
        Some(&action),
        "missing binding for {code:?} with {mods:?}"
      );
    }
  }

  #[test]
  fn test_default_g_prefix_bindings() {
    let config = Config::default();
    let kb = KeyBinding { code: KeyCode::Char('g'), modifiers: KeyModifiers::NONE };
    assert_eq!(config.g_prefix_keys.get(&kb), Some(&Action::GoToTop));
    let kb_h = KeyBinding { code: KeyCode::Char('h'), modifiers: KeyModifiers::NONE };
    assert_eq!(config.g_prefix_keys.get(&kb_h), Some(&Action::GoHome));
  }

  // --- Config::load_from_str tests ---

  #[test]
  fn test_load_empty_string() {
    let config = Config::load_from_str("");
    assert_eq!(config.tree_ratio, 30);
    assert_eq!(config.tick_rate_ms, 100);
  }

  #[test]
  fn test_load_keys_section_replaces_all_defaults() {
    let toml = r#"
[keys.normal]
j = "move_up"
"#;
    let config = Config::load_from_str(toml);
    let kb = KeyBinding { code: KeyCode::Char('j'), modifiers: KeyModifiers::NONE };
    assert_eq!(config.normal_keys.get(&kb), Some(&Action::MoveUp));
    // Only the user-specified key should exist
    assert_eq!(config.normal_keys.len(), 1);
    // Default keys not in user config should be gone
    let kb_k = KeyBinding { code: KeyCode::Char('k'), modifiers: KeyModifiers::NONE };
    assert_eq!(config.normal_keys.get(&kb_k), None);
  }

  #[test]
  fn test_load_general_overrides() {
    let toml = r#"
[general]
tree_ratio = 50
tick_rate_ms = 200
"#;
    let config = Config::load_from_str(toml);
    assert_eq!(config.tree_ratio, 50);
    assert_eq!(config.tick_rate_ms, 200);
  }

  #[test]
  fn test_load_invalid_action_skipped() {
    let toml = r#"
[keys.normal]
j = "invalid_action"
k = "quit"
"#;
    let config = Config::load_from_str(toml);
    // j should be absent because invalid_action was skipped
    let kb_j = KeyBinding { code: KeyCode::Char('j'), modifiers: KeyModifiers::NONE };
    assert_eq!(config.normal_keys.get(&kb_j), None);
    // k should be bound to Quit
    let kb_k = KeyBinding { code: KeyCode::Char('k'), modifiers: KeyModifiers::NONE };
    assert_eq!(config.normal_keys.get(&kb_k), Some(&Action::Quit));
    assert_eq!(config.normal_keys.len(), 1);
  }

  #[test]
  fn test_load_invalid_key_skipped() {
    let toml = r#"
[keys.normal]
"" = "quit"
k = "quit"
"#;
    let config = Config::load_from_str(toml);
    let kb_k = KeyBinding { code: KeyCode::Char('k'), modifiers: KeyModifiers::NONE };
    assert_eq!(config.normal_keys.get(&kb_k), Some(&Action::Quit));
  }

  #[test]
  fn test_load_unbind_with_none() {
    let toml = r#"
[keys.normal]
q = "none"
"#;
    let config = Config::load_from_str(toml);
    let kb = KeyBinding { code: KeyCode::Char('q'), modifiers: KeyModifiers::NONE };
    assert_eq!(config.normal_keys.get(&kb), Some(&Action::None));
  }

  #[test]
  fn test_load_ctrl_binding() {
    let toml = r#"
[keys.normal]
"ctrl+c" = "open_claude"
"#;
    let config = Config::load_from_str(toml);
    let kb = KeyBinding { code: KeyCode::Char('c'), modifiers: KeyModifiers::CONTROL };
    assert_eq!(config.normal_keys.get(&kb), Some(&Action::OpenClaude));
  }

  #[test]
  fn test_load_g_prefix_override() {
    let toml = r#"
[keys.g_prefix]
g = "quit"
"#;
    let config = Config::load_from_str(toml);
    let kb = KeyBinding { code: KeyCode::Char('g'), modifiers: KeyModifiers::NONE };
    assert_eq!(config.g_prefix_keys.get(&kb), Some(&Action::Quit));
  }

  #[test]
  fn test_default_toml_is_valid_toml() {
    let result: Result<TomlConfig, _> = toml::from_str(Config::default_toml());
    assert!(result.is_ok(), "default_toml() is not valid TOML: {:?}", result.err());
  }

  #[test]
  fn test_default_derives_from_toml_not_hardcoded() {
    // Config::default() must derive bindings from default_toml().
    // An empty() config has no bindings; default() should have them.
    let empty = Config::empty();
    let default = Config::default();
    assert!(empty.normal_keys.is_empty());
    assert!(empty.g_prefix_keys.is_empty());
    assert!(!default.normal_keys.is_empty());
    assert!(!default.g_prefix_keys.is_empty());
  }

  #[test]
  fn test_default_and_load_from_default_toml_are_identical() {
    // Parsing default_toml() on top of defaults should be idempotent
    let default = Config::default();
    let reloaded = Config::load_from_str(Config::default_toml());
    assert_eq!(default.tree_ratio, reloaded.tree_ratio);
    assert_eq!(default.tick_rate_ms, reloaded.tick_rate_ms);
    assert_eq!(default.normal_keys.len(), reloaded.normal_keys.len());
    assert_eq!(default.g_prefix_keys.len(), reloaded.g_prefix_keys.len());
    for (kb, action) in &default.normal_keys {
      assert_eq!(reloaded.normal_keys.get(kb), Some(action), "mismatch for {kb:?}");
    }
    for (kb, action) in &default.g_prefix_keys {
      assert_eq!(reloaded.g_prefix_keys.get(kb), Some(action), "g_prefix mismatch for {kb:?}");
    }
  }

  #[test]
  fn test_user_override_does_not_keep_stale_defaults() {
    // If a user overrides j to a different action, the old action
    // should not also appear on j — only the override should apply.
    let toml = r#"
[keys.normal]
j = "quit"
"#;
    let config = Config::load_from_str(toml);
    let kb_j = KeyBinding { code: KeyCode::Char('j'), modifiers: KeyModifiers::NONE };
    assert_eq!(config.normal_keys.get(&kb_j), Some(&Action::Quit));
    // j must not also map to the old default (MoveDown)
    let j_actions: Vec<_> = config.normal_keys.iter()
      .filter(|(k, _)| k.code == KeyCode::Char('j') && k.modifiers == KeyModifiers::NONE)
      .collect();
    assert_eq!(j_actions.len(), 1);
    assert_eq!(j_actions[0].1, &Action::Quit);
  }

  #[test]
  fn test_user_keys_section_replaces_defaults() {
    // When the user provides [keys.normal], it replaces all default
    // normal bindings. Old defaults like ø/æ must not survive.
    let toml = r#"
[keys.normal]
o = "shrink_tree"
p = "grow_tree"
j = "move_down"
"#;
    let config = Config::load_from_str(toml);
    // User-specified keys exist
    let kb_o = KeyBinding { code: KeyCode::Char('o'), modifiers: KeyModifiers::NONE };
    let kb_p = KeyBinding { code: KeyCode::Char('p'), modifiers: KeyModifiers::NONE };
    let kb_j = KeyBinding { code: KeyCode::Char('j'), modifiers: KeyModifiers::NONE };
    assert_eq!(config.normal_keys.get(&kb_o), Some(&Action::ShrinkTree));
    assert_eq!(config.normal_keys.get(&kb_p), Some(&Action::GrowTree));
    assert_eq!(config.normal_keys.get(&kb_j), Some(&Action::MoveDown));
    // Old defaults that were NOT in the user config must be gone
    let kb_oe = KeyBinding { code: KeyCode::Char('ø'), modifiers: KeyModifiers::NONE };
    let kb_ae = KeyBinding { code: KeyCode::Char('æ'), modifiers: KeyModifiers::NONE };
    let kb_q = KeyBinding { code: KeyCode::Char('q'), modifiers: KeyModifiers::NONE };
    assert_eq!(config.normal_keys.get(&kb_oe), None, "ø should not survive when [keys.normal] is provided");
    assert_eq!(config.normal_keys.get(&kb_ae), None, "æ should not survive when [keys.normal] is provided");
    assert_eq!(config.normal_keys.get(&kb_q), None, "q should not survive when [keys.normal] is provided");
    // Only user-specified keys should be present
    assert_eq!(config.normal_keys.len(), 3);
  }

  #[test]
  fn test_user_g_prefix_section_replaces_defaults() {
    let toml = r#"
[keys.g_prefix]
t = "go_to_top"
"#;
    let config = Config::load_from_str(toml);
    let kb_t = KeyBinding { code: KeyCode::Char('t'), modifiers: KeyModifiers::NONE };
    let kb_g = KeyBinding { code: KeyCode::Char('g'), modifiers: KeyModifiers::NONE };
    assert_eq!(config.g_prefix_keys.get(&kb_t), Some(&Action::GoToTop));
    assert_eq!(config.g_prefix_keys.get(&kb_g), None, "default g should not survive when [keys.g_prefix] is provided");
    assert_eq!(config.g_prefix_keys.len(), 1);
  }

  #[test]
  fn test_no_keys_section_keeps_all_defaults() {
    // When no [keys] section is provided, all defaults remain
    let toml = r#"
[general]
tree_ratio = 40
"#;
    let config = Config::load_from_str(toml);
    assert_eq!(config.tree_ratio, 40);
    let default = Config::default();
    assert_eq!(config.normal_keys.len(), default.normal_keys.len());
    assert_eq!(config.g_prefix_keys.len(), default.g_prefix_keys.len());
  }

  #[test]
  fn test_load_malformed_toml_returns_default() {
    let config = Config::load_from_str("this is not [valid toml");
    assert_eq!(config.tree_ratio, 30);
    assert_eq!(config.tick_rate_ms, 100);
  }

  // --- display_key tests ---

  #[test]
  fn test_display_key_plain_char() {
    let kb = KeyBinding { code: KeyCode::Char('j'), modifiers: KeyModifiers::NONE };
    assert_eq!(kb.display_key(), "j");
  }

  #[test]
  fn test_display_key_space() {
    let kb = KeyBinding { code: KeyCode::Char(' '), modifiers: KeyModifiers::NONE };
    assert_eq!(kb.display_key(), "Space");
  }

  #[test]
  fn test_display_key_ctrl() {
    let kb = KeyBinding { code: KeyCode::Char('c'), modifiers: KeyModifiers::CONTROL };
    assert_eq!(kb.display_key(), "Ctrl+c");
  }

  #[test]
  fn test_display_key_named() {
    let kb = KeyBinding { code: KeyCode::Enter, modifiers: KeyModifiers::NONE };
    assert_eq!(kb.display_key(), "Enter");
    let kb = KeyBinding { code: KeyCode::Esc, modifiers: KeyModifiers::NONE };
    assert_eq!(kb.display_key(), "Esc");
  }

  #[test]
  fn test_display_key_multibyte() {
    let kb = KeyBinding { code: KeyCode::Char('ø'), modifiers: KeyModifiers::NONE };
    assert_eq!(kb.display_key(), "ø");
  }

  // --- reverse_lookup tests ---

  #[test]
  fn test_reverse_lookup_contains_quit() {
    let config = Config::default();
    let lookup = config.reverse_lookup();
    let quit_keys = lookup.get(&Action::Quit).expect("Quit should have keys");
    assert!(quit_keys.contains(&"q".to_string()));
    assert!(quit_keys.contains(&"Esc".to_string()));
  }

  #[test]
  fn test_reverse_lookup_contains_go_to_top_with_g_prefix() {
    let config = Config::default();
    let lookup = config.reverse_lookup();
    let top_keys = lookup.get(&Action::GoToTop).expect("GoToTop should have keys");
    assert!(top_keys.contains(&"gg".to_string()));
  }

  #[test]
  fn test_default_enter_binds_open_default() {
    let config = Config::default();
    let kb = KeyBinding { code: KeyCode::Enter, modifiers: KeyModifiers::NONE };
    assert_eq!(config.normal_keys.get(&kb), Some(&Action::OpenDefault));
  }

  #[test]
  fn test_default_o_binds_open_with() {
    let config = Config::default();
    let kb = KeyBinding { code: KeyCode::Char('o'), modifiers: KeyModifiers::NONE };
    assert_eq!(config.normal_keys.get(&kb), Some(&Action::OpenWithStart));
  }

  #[test]
  fn test_load_custom_apps() {
    let apps_toml = r#"
[[apps]]
name = "Kakoune"
command = "kak"
tui = true

[[apps]]
name = "Lite XL"
command = "lite-xl"
"#;
    let mut config = Config::default();
    config.load_apps_str(apps_toml, &mut Vec::new());
    assert_eq!(config.custom_apps.len(), 2);
    assert_eq!(config.custom_apps[0].name, "Kakoune");
    assert_eq!(config.custom_apps[0].command, "kak");
    assert!(config.custom_apps[0].is_tui);
    assert_eq!(config.custom_apps[1].name, "Lite XL");
    assert_eq!(config.custom_apps[1].command, "lite-xl");
    assert!(!config.custom_apps[1].is_tui);
  }

  #[test]
  fn test_custom_apps_default_tui_false() {
    let apps_toml = r#"
[[apps]]
name = "MyApp"
command = "myapp"
"#;
    let mut config = Config::default();
    config.load_apps_str(apps_toml, &mut Vec::new());
    assert_eq!(config.custom_apps.len(), 1);
    assert!(!config.custom_apps[0].is_tui);
  }

  #[test]
  fn test_custom_apps_macos_only() {
    let apps_toml = r#"
[[apps]]
name = "Pages"
macos_app = "Pages"
"#;
    let mut config = Config::default();
    config.load_apps_str(apps_toml, &mut Vec::new());
    assert_eq!(config.custom_apps.len(), 1);
    assert_eq!(config.custom_apps[0].name, "Pages");
    assert_eq!(config.custom_apps[0].macos_app, Some("Pages".into()));
    assert!(config.custom_apps[0].command.is_empty());
  }

  #[test]
  fn test_apps_in_config_toml_ignored() {
    // [[apps]] in config.toml should be silently ignored (unknown field)
    let toml = r#"
[[apps]]
name = "Kakoune"
command = "kak"
"#;
    let config = Config::load_from_str(toml);
    assert!(config.custom_apps.is_empty());
  }

  #[test]
  fn test_default_has_toggle_help_binding() {
    let config = Config::default();
    let kb = KeyBinding { code: KeyCode::Char('?'), modifiers: KeyModifiers::NONE };
    assert_eq!(config.normal_keys.get(&kb), Some(&Action::ToggleHelp));
  }
}
