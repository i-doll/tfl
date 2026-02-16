use ratatui::style::Color;

#[derive(Debug, Clone)]
pub struct Theme {
  pub accent: Color,
  pub text: Color,
  pub text_dim: Color,
  pub text_muted: Color,
  pub border: Color,
  pub title_inactive: Color,
  pub bg_selected: Color,
  pub bg_overlay: Color,
  pub bg_bar: Color,
  pub bg_inline_code: Color,
  pub success: Color,
  pub warning: Color,
  pub error: Color,
  pub marked: Color,
  pub info: Color,
  pub git_staged: Color,
  pub git_modified: Color,
  pub git_untracked: Color,
  pub git_conflicted: Color,
  pub meta_secondary: Color,
}

impl Theme {
  pub fn dark() -> Self {
    Self {
      accent: Color::Indexed(75),
      text: Color::Indexed(252),
      text_dim: Color::DarkGray,
      text_muted: Color::Indexed(241),
      border: Color::Indexed(240),
      title_inactive: Color::Indexed(245),
      bg_selected: Color::Indexed(234),
      bg_overlay: Color::Indexed(235),
      bg_bar: Color::Indexed(236),
      bg_inline_code: Color::Indexed(236),
      success: Color::Indexed(114),
      warning: Color::Indexed(214),
      error: Color::Indexed(167),
      marked: Color::Indexed(208),
      info: Color::Indexed(150),
      git_staged: Color::Indexed(114),
      git_modified: Color::Indexed(214),
      git_untracked: Color::Indexed(167),
      git_conflicted: Color::Indexed(196),
      meta_secondary: Color::Indexed(246),
    }
  }

  pub fn light() -> Self {
    Self {
      accent: Color::Indexed(27),
      text: Color::Indexed(235),
      text_dim: Color::Indexed(243),
      text_muted: Color::Indexed(245),
      border: Color::Indexed(250),
      title_inactive: Color::Indexed(243),
      bg_selected: Color::Indexed(254),
      bg_overlay: Color::Indexed(255),
      bg_bar: Color::Indexed(253),
      bg_inline_code: Color::Indexed(254),
      success: Color::Indexed(28),
      warning: Color::Indexed(172),
      error: Color::Indexed(124),
      marked: Color::Indexed(166),
      info: Color::Indexed(30),
      git_staged: Color::Indexed(28),
      git_modified: Color::Indexed(172),
      git_untracked: Color::Indexed(124),
      git_conflicted: Color::Indexed(160),
      meta_secondary: Color::Indexed(241),
    }
  }

  pub fn catppuccin_mocha() -> Self {
    Self {
      accent: Color::Rgb(137, 180, 250),       // Blue
      text: Color::Rgb(205, 214, 244),          // Text
      text_dim: Color::Rgb(127, 132, 156),      // Overlay1
      text_muted: Color::Rgb(108, 112, 134),    // Overlay0
      border: Color::Rgb(69, 71, 90),           // Surface1
      title_inactive: Color::Rgb(166, 173, 200),// Subtext0
      bg_selected: Color::Rgb(49, 50, 68),      // Surface0
      bg_overlay: Color::Rgb(24, 24, 37),       // Mantle
      bg_bar: Color::Rgb(24, 24, 37),           // Mantle
      bg_inline_code: Color::Rgb(49, 50, 68),   // Surface0
      success: Color::Rgb(166, 227, 161),        // Green
      warning: Color::Rgb(249, 226, 175),        // Yellow
      error: Color::Rgb(243, 139, 168),          // Red
      marked: Color::Rgb(250, 179, 135),         // Peach
      info: Color::Rgb(148, 226, 213),           // Teal
      git_staged: Color::Rgb(166, 227, 161),     // Green
      git_modified: Color::Rgb(249, 226, 175),   // Yellow
      git_untracked: Color::Rgb(243, 139, 168),  // Red
      git_conflicted: Color::Rgb(235, 160, 172), // Maroon
      meta_secondary: Color::Rgb(186, 194, 222), // Subtext1
    }
  }

  pub fn from_name(name: &str) -> Option<Self> {
    match name {
      "dark" => Some(Self::dark()),
      "light" => Some(Self::light()),
      "catppuccin-mocha" => Some(Self::catppuccin_mocha()),
      _ => None,
    }
  }

  pub fn available_themes() -> &'static [&'static str] {
    &["dark", "light", "catppuccin-mocha"]
  }
}

impl Default for Theme {
  fn default() -> Self {
    Self::dark()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_dark_theme() {
    let theme = Theme::dark();
    assert_eq!(theme.accent, Color::Indexed(75));
    assert_eq!(theme.text, Color::Indexed(252));
  }

  #[test]
  fn test_light_theme() {
    let theme = Theme::light();
    assert_eq!(theme.accent, Color::Indexed(27));
  }

  #[test]
  fn test_catppuccin_mocha() {
    let theme = Theme::catppuccin_mocha();
    assert_eq!(theme.accent, Color::Rgb(137, 180, 250));
  }

  #[test]
  fn test_from_name() {
    assert!(Theme::from_name("dark").is_some());
    assert!(Theme::from_name("light").is_some());
    assert!(Theme::from_name("catppuccin-mocha").is_some());
    assert!(Theme::from_name("nonexistent").is_none());
  }

  #[test]
  fn test_available_themes() {
    let themes = Theme::available_themes();
    assert_eq!(themes.len(), 3);
    assert!(themes.contains(&"dark"));
    assert!(themes.contains(&"light"));
    assert!(themes.contains(&"catppuccin-mocha"));
  }

  #[test]
  fn test_default_is_dark() {
    let default = Theme::default();
    let dark = Theme::dark();
    assert_eq!(default.accent, dark.accent);
    assert_eq!(default.text, dark.text);
  }
}
