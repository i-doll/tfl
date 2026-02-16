use std::io::Cursor;

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use syntect::easy::HighlightLines;
use syntect::highlighting::{FontStyle, ThemeSet};
use syntect::parsing::{SyntaxDefinition, SyntaxSet};
use syntect::util::LinesWithEndings;

const CATPPUCCIN_MOCHA_THEME: &[u8] = include_bytes!("themes/catppuccin-mocha.tmTheme");

pub struct SyntaxHighlighter {
  syntax_set: SyntaxSet,
  theme_set: ThemeSet,
  theme_name: String,
}

impl SyntaxHighlighter {
  pub fn new(syntax_theme: &str) -> Self {
    let mut builder = SyntaxSet::load_defaults_newlines().into_builder();
    let toml_syntax = SyntaxDefinition::load_from_str(
      include_str!("syntaxes/TOML.sublime-syntax"),
      true,
      Some("TOML"),
    )
    .expect("valid TOML syntax definition");
    builder.add(toml_syntax);

    let mut theme_set = ThemeSet::load_defaults();

    // Load Catppuccin Mocha theme
    if let Ok(theme) = ThemeSet::load_from_reader(&mut Cursor::new(CATPPUCCIN_MOCHA_THEME)) {
      theme_set.themes.insert("Catppuccin Mocha".to_string(), theme);
    }

    Self {
      syntax_set: builder.build(),
      theme_set,
      theme_name: syntax_theme.to_string(),
    }
  }

  pub fn set_theme_name(&mut self, name: &str) {
    self.theme_name = name.to_string();
  }

  pub fn highlight<'a>(&self, content: &str, extension: &str) -> Vec<Line<'a>> {
    let syntax = parse_vim_modeline(content)
      .and_then(|ft| self.syntax_set.find_syntax_by_token(&ft))
      .or_else(|| self.syntax_set.find_syntax_by_extension(extension))
      .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text());

    let theme = self.theme_set.themes.get(&self.theme_name)
      .or_else(|| self.theme_set.themes.get("base16-ocean.dark"))
      .expect("fallback theme must exist");
    let mut highlighter = HighlightLines::new(syntax, theme);
    let mut lines = Vec::new();

    for (line_num, line) in LinesWithEndings::from(content).enumerate() {
      let line_number = format!("{:>4} ", line_num + 1);
      let mut spans = vec![Span::styled(
        line_number,
        Style::default().fg(Color::DarkGray),
      )];

      match highlighter.highlight_line(line, &self.syntax_set) {
        Ok(ranges) => {
          for (style, text) in ranges {
            let fg = Color::Rgb(style.foreground.r, style.foreground.g, style.foreground.b);
            let mut ratatui_style = Style::default().fg(fg);
            if style.font_style.contains(FontStyle::BOLD) {
              ratatui_style = ratatui_style.add_modifier(Modifier::BOLD);
            }
            if style.font_style.contains(FontStyle::ITALIC) {
              ratatui_style = ratatui_style.add_modifier(Modifier::ITALIC);
            }
            spans.push(Span::styled(text.trim_end_matches('\n').to_string(), ratatui_style));
          }
        }
        Err(_) => {
          spans.push(Span::raw(line.trim_end_matches('\n').to_string()));
        }
      }

      lines.push(Line::from(spans));
    }

    lines
  }
}

fn extract_ft(line: &str) -> Option<String> {
  let marker_pos = line.find("vim:").map(|p| (p, 4))
    .or_else(|| line.find("vi:").map(|p| (p, 3)))?;
  let after_marker = &line[marker_pos.0 + marker_pos.1..];
  let trimmed = after_marker.trim_start();
  let body = if trimmed.starts_with("set ") || trimmed.starts_with("set\t") {
    trimmed[4..].trim_start()
  } else {
    trimmed
  };
  for token in body.split(|c: char| c.is_whitespace() || c == ':') {
    if let Some(val) = token.strip_prefix("ft=").or_else(|| token.strip_prefix("filetype="))
      && !val.is_empty()
    {
      return Some(val.to_string());
    }
  }
  None
}

fn parse_vim_modeline(content: &str) -> Option<String> {
  let lines: Vec<&str> = content.lines().collect();
  let len = lines.len();
  let first = lines.iter().take(5);
  let last = if len > 5 {
    lines[len.saturating_sub(5)..].iter()
  } else {
    [].iter()
  };
  for line in first.chain(last) {
    if let Some(ft) = extract_ft(line) {
      return Some(ft);
    }
  }
  None
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_highlighter_creates() {
    let h = SyntaxHighlighter::new("base16-ocean.dark");
    assert!(!h.syntax_set.syntaxes().is_empty());
  }

  #[test]
  fn test_highlight_plain_text() {
    let h = SyntaxHighlighter::new("base16-ocean.dark");
    let lines = h.highlight("hello\nworld\n", "txt");
    assert_eq!(lines.len(), 2);
  }

  #[test]
  fn test_highlight_rust() {
    let h = SyntaxHighlighter::new("base16-ocean.dark");
    let lines = h.highlight("fn main() {}\n", "rs");
    assert_eq!(lines.len(), 1);
    // First span is line number
    assert!(lines[0].spans[0].content.contains('1'));
  }

  #[test]
  fn test_highlight_toml() {
    let h = SyntaxHighlighter::new("base16-ocean.dark");
    let toml_content = "[package]\nname = \"my-app\"\nversion = \"0.1.0\"\n";
    let lines = h.highlight(toml_content, "toml");
    assert_eq!(lines.len(), 3);
    // Each line should have more than 2 spans (line number + multiple styled spans)
    // Plain text fallback would produce exactly 2 spans per line (line number + raw text)
    for line in &lines {
      assert!(line.spans.len() > 2, "TOML should produce multiple styled spans, got: {:?}", line.spans);
    }
  }

  #[test]
  fn test_highlight_line_numbers() {
    let h = SyntaxHighlighter::new("base16-ocean.dark");
    let content = (1..=15).map(|i| format!("line {i}")).collect::<Vec<_>>().join("\n");
    let lines = h.highlight(&content, "txt");
    assert_eq!(lines.len(), 15);
    // Line 1 should have "   1 " prefix
    assert!(lines[0].spans[0].content.trim().starts_with('1'));
    // Line 15 should have "  15 " prefix
    assert!(lines[14].spans[0].content.trim().starts_with("15"));
  }

  #[test]
  fn test_parse_modeline_ft() {
    let content = "#!/bin/bash\n# vim: ft=python\nprint('hello')\n";
    assert_eq!(parse_vim_modeline(content), Some("python".to_string()));
  }

  #[test]
  fn test_parse_modeline_set_ft() {
    let content = "// vim: set ft=javascript:\nvar x = 1;\n";
    assert_eq!(parse_vim_modeline(content), Some("javascript".to_string()));
  }

  #[test]
  fn test_parse_modeline_vi_filetype() {
    let content = "# vi: filetype=yaml\nkey: value\n";
    assert_eq!(parse_vim_modeline(content), Some("yaml".to_string()));
  }

  #[test]
  fn test_parse_modeline_no_spaces() {
    let content = "# vim:ft=sh\necho hello\n";
    assert_eq!(parse_vim_modeline(content), Some("sh".to_string()));
  }

  #[test]
  fn test_parse_modeline_last_lines() {
    let mut lines: Vec<String> = (1..=18).map(|i| format!("line {i}")).collect();
    lines.push("# vim: ft=python".to_string());
    lines.push("# end".to_string());
    let content = lines.join("\n");
    assert_eq!(parse_vim_modeline(&content), Some("python".to_string()));
  }

  #[test]
  fn test_parse_modeline_none() {
    let content = "just some text\nno modeline here\n";
    assert_eq!(parse_vim_modeline(content), None);
  }

  #[test]
  fn test_parse_modeline_middle_ignored() {
    let mut lines: Vec<String> = (1..=6).map(|i| format!("line {i}")).collect();
    lines.push("# vim: ft=python".to_string());
    lines.extend((8..=14).map(|i| format!("line {i}")));
    let content = lines.join("\n");
    assert_eq!(parse_vim_modeline(&content), None);
  }

  #[test]
  fn test_highlight_modeline_overrides_ext() {
    let h = SyntaxHighlighter::new("base16-ocean.dark");
    let content = "# vim: ft=python\ndef hello():\n  pass\n";
    let lines_py = h.highlight(content, "txt");
    let lines_txt = h.highlight("def hello():\n  pass\n", "txt");
    // Python-highlighted version should have more spans (keywords colored)
    let py_spans: usize = lines_py.iter().map(|l| l.spans.len()).sum();
    let txt_spans: usize = lines_txt.iter().map(|l| l.spans.len()).sum();
    assert!(py_spans > txt_spans, "modeline should trigger Python highlighting");
  }

  #[test]
  fn test_catppuccin_mocha_theme_loaded() {
    let h = SyntaxHighlighter::new("Catppuccin Mocha");
    assert!(h.theme_set.themes.contains_key("Catppuccin Mocha"));
    // Verify it can highlight with the theme
    let lines = h.highlight("fn main() {}\n", "rs");
    assert_eq!(lines.len(), 1);
  }

  #[test]
  fn test_set_theme_name() {
    let mut h = SyntaxHighlighter::new("base16-ocean.dark");
    assert_eq!(h.theme_name, "base16-ocean.dark");
    h.set_theme_name("Catppuccin Mocha");
    assert_eq!(h.theme_name, "Catppuccin Mocha");
  }

  #[test]
  fn test_fallback_theme_on_invalid_name() {
    let h = SyntaxHighlighter::new("nonexistent-theme");
    // Should still work with fallback
    let lines = h.highlight("hello\n", "txt");
    assert_eq!(lines.len(), 1);
  }
}
