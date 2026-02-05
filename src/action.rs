#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Action {
  Quit,
  MoveUp,
  MoveDown,
  MoveLeft,
  MoveRight,
  ToggleExpand,
  EnterDir,
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
  DateFilterStart,
  DateFilterInput(char),
  DateFilterBackspace,
  DateFilterConfirm,
  DateFilterCancel,
  DateFilterCycleTimeType,
  YankPath,
  OpenEditor,
  OpenClaude,
  OpenClaudeAlt,
  OpenShell,
  ShrinkTree,
  GrowTree,
  GPress,
  ToggleHelp,
  CutFile,
  CopyFile,
  Paste,
  DeleteFile,
  RenameStart,
  NewFileStart,
  NewDirStart,
  PromptInput(char),
  PromptBackspace,
  PromptDelete,
  PromptLeft,
  PromptRight,
  PromptHome,
  PromptEnd,
  PromptConfirm,
  PromptCancel,
  GoHome,
  FavoriteAdd,
  FavoritesOpen,
  FavoritesDown,
  FavoritesUp,
  FavoritesSelect,
  FavoritesClose,
  FavoritesRemove,
  FavoritesAddCurrent,
  OpenDefault,
  OpenWithStart,
  OpenWithDown,
  OpenWithUp,
  OpenWithSelect,
  OpenWithClose,
  ErrorClose,
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
      "enter_dir" => Some(Action::EnterDir),
      "scroll_preview_up" => Some(Action::ScrollPreviewUp),
      "scroll_preview_down" => Some(Action::ScrollPreviewDown),
      "toggle_hidden" => Some(Action::ToggleHidden),
      "go_to_top" => Some(Action::GoToTop),
      "go_to_bottom" => Some(Action::GoToBottom),
      "search_start" => Some(Action::SearchStart),
      "date_filter_start" => Some(Action::DateFilterStart),
      "yank_path" => Some(Action::YankPath),
      "open_editor" => Some(Action::OpenEditor),
      "open_claude" => Some(Action::OpenClaude),
      "open_claude_alt" => Some(Action::OpenClaudeAlt),
      "open_shell" => Some(Action::OpenShell),
      "shrink_tree" => Some(Action::ShrinkTree),
      "grow_tree" => Some(Action::GrowTree),
      "g_press" => Some(Action::GPress),
      "toggle_help" => Some(Action::ToggleHelp),
      "cut_file" => Some(Action::CutFile),
      "copy_file" => Some(Action::CopyFile),
      "paste" => Some(Action::Paste),
      "delete_file" => Some(Action::DeleteFile),
      "rename_start" => Some(Action::RenameStart),
      "new_file_start" => Some(Action::NewFileStart),
      "new_dir_start" => Some(Action::NewDirStart),
      "go_home" => Some(Action::GoHome),
      "favorite_add" => Some(Action::FavoriteAdd),
      "favorites_open" => Some(Action::FavoritesOpen),
      "open_default" => Some(Action::OpenDefault),
      "open_with" => Some(Action::OpenWithStart),
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
    assert_eq!(Action::from_name("enter_dir"), Some(Action::EnterDir));
    assert_eq!(Action::from_name("scroll_preview_up"), Some(Action::ScrollPreviewUp));
    assert_eq!(Action::from_name("scroll_preview_down"), Some(Action::ScrollPreviewDown));
    assert_eq!(Action::from_name("toggle_hidden"), Some(Action::ToggleHidden));
    assert_eq!(Action::from_name("go_to_top"), Some(Action::GoToTop));
    assert_eq!(Action::from_name("go_to_bottom"), Some(Action::GoToBottom));
    assert_eq!(Action::from_name("search_start"), Some(Action::SearchStart));
    assert_eq!(Action::from_name("date_filter_start"), Some(Action::DateFilterStart));
    assert_eq!(Action::from_name("yank_path"), Some(Action::YankPath));
    assert_eq!(Action::from_name("open_editor"), Some(Action::OpenEditor));
    assert_eq!(Action::from_name("open_claude"), Some(Action::OpenClaude));
    assert_eq!(Action::from_name("open_claude_alt"), Some(Action::OpenClaudeAlt));
    assert_eq!(Action::from_name("open_shell"), Some(Action::OpenShell));
    assert_eq!(Action::from_name("shrink_tree"), Some(Action::ShrinkTree));
    assert_eq!(Action::from_name("grow_tree"), Some(Action::GrowTree));
    assert_eq!(Action::from_name("g_press"), Some(Action::GPress));
    assert_eq!(Action::from_name("toggle_help"), Some(Action::ToggleHelp));
    assert_eq!(Action::from_name("cut_file"), Some(Action::CutFile));
    assert_eq!(Action::from_name("copy_file"), Some(Action::CopyFile));
    assert_eq!(Action::from_name("paste"), Some(Action::Paste));
    assert_eq!(Action::from_name("delete_file"), Some(Action::DeleteFile));
    assert_eq!(Action::from_name("rename_start"), Some(Action::RenameStart));
    assert_eq!(Action::from_name("new_file_start"), Some(Action::NewFileStart));
    assert_eq!(Action::from_name("new_dir_start"), Some(Action::NewDirStart));
    assert_eq!(Action::from_name("go_home"), Some(Action::GoHome));
    assert_eq!(Action::from_name("favorite_add"), Some(Action::FavoriteAdd));
    assert_eq!(Action::from_name("favorites_open"), Some(Action::FavoritesOpen));
    assert_eq!(Action::from_name("open_default"), Some(Action::OpenDefault));
    assert_eq!(Action::from_name("open_with"), Some(Action::OpenWithStart));
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
    assert_eq!(Action::from_name("prompt_input"), None);
    assert_eq!(Action::from_name("resize"), None);
    assert_eq!(Action::from_name("tick"), None);
  }
}
