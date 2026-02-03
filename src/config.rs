use std::collections::HashMap;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde::Deserialize;

use crate::action::Action;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyBinding {
  pub code: KeyCode,
  pub modifiers: KeyModifiers,
}

pub struct Config {
  pub tree_ratio: u16,
  pub min_tree_ratio: u16,
  pub max_tree_ratio: u16,
  pub ratio_step: u16,
  pub tick_rate_ms: u64,
  pub normal_keys: HashMap<KeyBinding, Action>,
  pub g_prefix_keys: HashMap<KeyBinding, Action>,
}

#[derive(Deserialize, Default)]
struct TomlConfig {
  general: Option<GeneralConfig>,
  keys: Option<KeysConfig>,
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
    "tab" => Some(KeyCode::Tab),
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
    let mut normal_keys = HashMap::new();
    let mut g_prefix_keys = HashMap::new();

    let bind = |map: &mut HashMap<KeyBinding, Action>, code: KeyCode, mods: KeyModifiers, action: Action| {
      map.insert(KeyBinding { code, modifiers: mods }, action);
    };

    let n = KeyModifiers::NONE;

    bind(&mut normal_keys, KeyCode::Char('q'), n, Action::Quit);
    bind(&mut normal_keys, KeyCode::Esc, n, Action::Quit);
    bind(&mut normal_keys, KeyCode::Char('j'), n, Action::MoveDown);
    bind(&mut normal_keys, KeyCode::Down, n, Action::MoveDown);
    bind(&mut normal_keys, KeyCode::Char('k'), n, Action::MoveUp);
    bind(&mut normal_keys, KeyCode::Up, n, Action::MoveUp);
    bind(&mut normal_keys, KeyCode::Char('h'), n, Action::MoveLeft);
    bind(&mut normal_keys, KeyCode::Left, n, Action::MoveLeft);
    bind(&mut normal_keys, KeyCode::Char('l'), n, Action::MoveRight);
    bind(&mut normal_keys, KeyCode::Right, n, Action::MoveRight);
    bind(&mut normal_keys, KeyCode::Char(' '), n, Action::ToggleExpand);
    bind(&mut normal_keys, KeyCode::Enter, n, Action::ToggleExpand);
    bind(&mut normal_keys, KeyCode::Char('J'), n, Action::ScrollPreviewDown);
    bind(&mut normal_keys, KeyCode::Char('K'), n, Action::ScrollPreviewUp);
    bind(&mut normal_keys, KeyCode::Char('.'), n, Action::ToggleHidden);
    bind(&mut normal_keys, KeyCode::Char('g'), n, Action::GPress);
    bind(&mut normal_keys, KeyCode::Char('G'), n, Action::GoToBottom);
    bind(&mut normal_keys, KeyCode::Char('/'), n, Action::SearchStart);
    bind(&mut normal_keys, KeyCode::Char('y'), n, Action::YankPath);
    bind(&mut normal_keys, KeyCode::Char('e'), n, Action::OpenEditor);
    bind(&mut normal_keys, KeyCode::Char('c'), n, Action::OpenClaude);
    bind(&mut normal_keys, KeyCode::Char('c'), KeyModifiers::CONTROL, Action::Quit);
    bind(&mut normal_keys, KeyCode::Char('s'), n, Action::OpenShell);
    bind(&mut normal_keys, KeyCode::Char('ø'), n, Action::ShrinkTree);
    bind(&mut normal_keys, KeyCode::Char('æ'), n, Action::GrowTree);

    bind(&mut g_prefix_keys, KeyCode::Char('g'), n, Action::GoToTop);

    Config {
      tree_ratio: 30,
      min_tree_ratio: 15,
      max_tree_ratio: 60,
      ratio_step: 5,
      tick_rate_ms: 100,
      normal_keys,
      g_prefix_keys,
    }
  }
}

impl Config {
  pub fn load() -> Config {
    let config_dir = dirs::config_dir().map(|d| d.join("tfl"));
    let config_path = config_dir.map(|d| d.join("config.toml"));

    let content = config_path.and_then(|p| std::fs::read_to_string(p).ok());

    match content {
      Some(s) => Self::load_from_str(&s),
      None => Config::default(),
    }
  }

  pub fn load_from_str(s: &str) -> Config {
    let toml_config: TomlConfig = match toml::from_str(s) {
      Ok(c) => c,
      Err(e) => {
        eprintln!("tfl: failed to parse config.toml: {e}");
        return Config::default();
      }
    };

    let mut config = Config::default();

    if let Some(general) = toml_config.general {
      if let Some(ratio) = general.tree_ratio {
        config.tree_ratio = ratio;
      }
      if let Some(tick) = general.tick_rate_ms {
        config.tick_rate_ms = tick;
      }
    }

    if let Some(keys) = toml_config.keys {
      if let Some(normal) = keys.normal {
        for (key_str, action_str) in &normal {
          let Some(kb) = parse_key_binding(key_str) else {
            eprintln!("tfl: invalid key binding: {key_str:?}");
            continue;
          };
          let Some(action) = Action::from_name(action_str) else {
            eprintln!("tfl: invalid action: {action_str:?}");
            continue;
          };
          config.normal_keys.insert(kb, action);
        }
      }
      if let Some(g_prefix) = keys.g_prefix {
        for (key_str, action_str) in &g_prefix {
          let Some(kb) = parse_key_binding(key_str) else {
            eprintln!("tfl: invalid key binding: {key_str:?}");
            continue;
          };
          let Some(action) = Action::from_name(action_str) else {
            eprintln!("tfl: invalid action: {action_str:?}");
            continue;
          };
          config.g_prefix_keys.insert(kb, action);
        }
      }
    }

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
      (KeyCode::Enter, n, Action::ToggleExpand),
      (KeyCode::Char('J'), n, Action::ScrollPreviewDown),
      (KeyCode::Char('K'), n, Action::ScrollPreviewUp),
      (KeyCode::Char('.'), n, Action::ToggleHidden),
      (KeyCode::Char('g'), n, Action::GPress),
      (KeyCode::Char('G'), n, Action::GoToBottom),
      (KeyCode::Char('/'), n, Action::SearchStart),
      (KeyCode::Char('y'), n, Action::YankPath),
      (KeyCode::Char('e'), n, Action::OpenEditor),
      (KeyCode::Char('c'), n, Action::OpenClaude),
      (KeyCode::Char('c'), KeyModifiers::CONTROL, Action::Quit),
      (KeyCode::Char('s'), n, Action::OpenShell),
      (KeyCode::Char('ø'), n, Action::ShrinkTree),
      (KeyCode::Char('æ'), n, Action::GrowTree),
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
  }

  // --- Config::load_from_str tests ---

  #[test]
  fn test_load_empty_string() {
    let config = Config::load_from_str("");
    assert_eq!(config.tree_ratio, 30);
    assert_eq!(config.tick_rate_ms, 100);
  }

  #[test]
  fn test_load_partial_key_override() {
    let toml = r#"
[keys.normal]
j = "move_up"
"#;
    let config = Config::load_from_str(toml);
    let kb = KeyBinding { code: KeyCode::Char('j'), modifiers: KeyModifiers::NONE };
    assert_eq!(config.normal_keys.get(&kb), Some(&Action::MoveUp));
    // Other keys should still be at defaults
    let kb_k = KeyBinding { code: KeyCode::Char('k'), modifiers: KeyModifiers::NONE };
    assert_eq!(config.normal_keys.get(&kb_k), Some(&Action::MoveUp));
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
    // j should remain at default (MoveDown) because invalid_action was skipped
    let kb_j = KeyBinding { code: KeyCode::Char('j'), modifiers: KeyModifiers::NONE };
    assert_eq!(config.normal_keys.get(&kb_j), Some(&Action::MoveDown));
    // k should be overridden to Quit
    let kb_k = KeyBinding { code: KeyCode::Char('k'), modifiers: KeyModifiers::NONE };
    assert_eq!(config.normal_keys.get(&kb_k), Some(&Action::Quit));
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
  fn test_load_malformed_toml_returns_default() {
    let config = Config::load_from_str("this is not [valid toml");
    assert_eq!(config.tree_ratio, 30);
    assert_eq!(config.tick_rate_ms, 100);
  }
}
