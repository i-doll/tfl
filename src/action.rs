#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
  Quit,
  MoveUp,
  MoveDown,
  MoveLeft,
  MoveRight,
  ToggleExpand,
  ScrollPreviewUp,
  ScrollPreviewDown,
  ToggleHidden,
  GoToTop,
  GoToBottom,
  SearchStart,
  SearchInput(char),
  SearchBackspace,
  SearchConfirm,
  SearchCancel,
  YankPath,
  OpenEditor,
  OpenClaude,
  OpenShell,
  ShrinkTree,
  GrowTree,
  GPress,
  Resize(u16, u16),
  Tick,
  None,
}

impl Action {
  pub fn from_name(name: &str) -> Option<Action> {
    match name {
      "quit" => Some(Action::Quit),
      "move_up" => Some(Action::MoveUp),
      "move_down" => Some(Action::MoveDown),
      "move_left" => Some(Action::MoveLeft),
      "move_right" => Some(Action::MoveRight),
      "toggle_expand" => Some(Action::ToggleExpand),
      "scroll_preview_up" => Some(Action::ScrollPreviewUp),
      "scroll_preview_down" => Some(Action::ScrollPreviewDown),
      "toggle_hidden" => Some(Action::ToggleHidden),
      "go_to_top" => Some(Action::GoToTop),
      "go_to_bottom" => Some(Action::GoToBottom),
      "search_start" => Some(Action::SearchStart),
      "yank_path" => Some(Action::YankPath),
      "open_editor" => Some(Action::OpenEditor),
      "open_claude" => Some(Action::OpenClaude),
      "open_shell" => Some(Action::OpenShell),
      "shrink_tree" => Some(Action::ShrinkTree),
      "grow_tree" => Some(Action::GrowTree),
      "g_press" => Some(Action::GPress),
      "none" => Some(Action::None),
      _ => None,
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_from_name_bindable_actions() {
    assert_eq!(Action::from_name("quit"), Some(Action::Quit));
    assert_eq!(Action::from_name("move_up"), Some(Action::MoveUp));
    assert_eq!(Action::from_name("move_down"), Some(Action::MoveDown));
    assert_eq!(Action::from_name("move_left"), Some(Action::MoveLeft));
    assert_eq!(Action::from_name("move_right"), Some(Action::MoveRight));
    assert_eq!(Action::from_name("toggle_expand"), Some(Action::ToggleExpand));
    assert_eq!(Action::from_name("scroll_preview_up"), Some(Action::ScrollPreviewUp));
    assert_eq!(Action::from_name("scroll_preview_down"), Some(Action::ScrollPreviewDown));
    assert_eq!(Action::from_name("toggle_hidden"), Some(Action::ToggleHidden));
    assert_eq!(Action::from_name("go_to_top"), Some(Action::GoToTop));
    assert_eq!(Action::from_name("go_to_bottom"), Some(Action::GoToBottom));
    assert_eq!(Action::from_name("search_start"), Some(Action::SearchStart));
    assert_eq!(Action::from_name("yank_path"), Some(Action::YankPath));
    assert_eq!(Action::from_name("open_editor"), Some(Action::OpenEditor));
    assert_eq!(Action::from_name("open_claude"), Some(Action::OpenClaude));
    assert_eq!(Action::from_name("open_shell"), Some(Action::OpenShell));
    assert_eq!(Action::from_name("shrink_tree"), Some(Action::ShrinkTree));
    assert_eq!(Action::from_name("grow_tree"), Some(Action::GrowTree));
    assert_eq!(Action::from_name("g_press"), Some(Action::GPress));
  }

  #[test]
  fn test_from_name_none() {
    assert_eq!(Action::from_name("none"), Some(Action::None));
  }

  #[test]
  fn test_from_name_invalid() {
    assert_eq!(Action::from_name("garbage"), None);
    assert_eq!(Action::from_name(""), None);
  }

  #[test]
  fn test_from_name_unbindable() {
    assert_eq!(Action::from_name("search_input"), None);
    assert_eq!(Action::from_name("resize"), None);
    assert_eq!(Action::from_name("tick"), None);
  }
}
