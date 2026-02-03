use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use syntect::easy::HighlightLines;
use syntect::highlighting::{FontStyle, ThemeSet};
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;

pub struct SyntaxHighlighter {
  syntax_set: SyntaxSet,
  theme_set: ThemeSet,
}

impl SyntaxHighlighter {
  pub fn new() -> Self {
    Self {
      syntax_set: SyntaxSet::load_defaults_newlines(),
      theme_set: ThemeSet::load_defaults(),
    }
  }

  pub fn highlight<'a>(&self, content: &str, extension: &str) -> Vec<Line<'a>> {
    let syntax = self
      .syntax_set
      .find_syntax_by_extension(extension)
      .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text());

    let theme = &self.theme_set.themes["base16-ocean.dark"];
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

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_highlighter_creates() {
    let h = SyntaxHighlighter::new();
    assert!(!h.syntax_set.syntaxes().is_empty());
  }

  #[test]
  fn test_highlight_plain_text() {
    let h = SyntaxHighlighter::new();
    let lines = h.highlight("hello\nworld\n", "txt");
    assert_eq!(lines.len(), 2);
  }

  #[test]
  fn test_highlight_rust() {
    let h = SyntaxHighlighter::new();
    let lines = h.highlight("fn main() {}\n", "rs");
    assert_eq!(lines.len(), 1);
    // First span is line number
    assert!(lines[0].spans[0].content.contains('1'));
  }

  #[test]
  fn test_highlight_line_numbers() {
    let h = SyntaxHighlighter::new();
    let content = (1..=15).map(|i| format!("line {i}")).collect::<Vec<_>>().join("\n");
    let lines = h.highlight(&content, "txt");
    assert_eq!(lines.len(), 15);
    // Line 1 should have "   1 " prefix
    assert!(lines[0].spans[0].content.trim().starts_with('1'));
    // Line 15 should have "  15 " prefix
    assert!(lines[14].spans[0].content.trim().starts_with("15"));
  }
}
