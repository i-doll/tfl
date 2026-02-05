use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Widget};

use crate::app::App;

pub fn render_chmod(app: &App, area: Rect, buf: &mut Buffer) {
  let width = 50.min(area.width.saturating_sub(4));
  let height = 14.min(area.height.saturating_sub(2));

  if width < 20 || height < 8 {
    return;
  }

  let x = area.x + (area.width.saturating_sub(width)) / 2;
  let y = area.y + (area.height.saturating_sub(height)) / 2;
  let popup = Rect::new(x, y, width, height);

  Clear.render(popup, buf);

  let chmod_state = &app.chmod_state;
  let mode = chmod_state.new_mode;
  let original_mode = chmod_state.original_mode;

  let dim = Style::default().fg(Color::Indexed(241));
  let changed = Style::default().fg(Color::Indexed(114));
  let highlight = Style::default().fg(Color::Indexed(75)).add_modifier(Modifier::BOLD);

  let mut lines: Vec<Line> = Vec::new();

  // File name
  let name = chmod_state.path.file_name()
    .map(|n| n.to_string_lossy().to_string())
    .unwrap_or_else(|| chmod_state.path.to_string_lossy().to_string());
  lines.push(Line::from(vec![
    Span::styled(" File: ", dim),
    Span::styled(name, Style::default().fg(Color::Indexed(252))),
  ]));

  lines.push(Line::from(""));

  // Permissions grid header
  lines.push(Line::from(vec![
    Span::styled("         ", dim),
    Span::styled("  r   w   x", dim),
  ]));

  // Owner row
  let owner_spans = render_permission_row(
    PermRowConfig { label: "Owner", r_key: 'r', w_key: 'w', x_key: 'x', shift: 6 },
    mode, original_mode, changed,
  );
  lines.push(Line::from(owner_spans));

  // Group row
  let group_spans = render_permission_row(
    PermRowConfig { label: "Group", r_key: 'R', w_key: 'W', x_key: 'X', shift: 3 },
    mode, original_mode, changed,
  );
  lines.push(Line::from(group_spans));

  // Others row
  let others_spans = render_permission_row(
    PermRowConfig { label: "Others", r_key: '4', w_key: '2', x_key: '1', shift: 0 },
    mode, original_mode, changed,
  );
  lines.push(Line::from(others_spans));

  lines.push(Line::from(""));

  // Octal display
  let octal_str = format!("{:03o}", mode & 0o777);
  let original_octal = format!("{:03o}", original_mode & 0o777);
  let octal_style = if octal_str != original_octal { changed } else { Style::default().fg(Color::Indexed(252)) };

  if chmod_state.octal_mode {
    lines.push(Line::from(vec![
      Span::styled(" Octal: ", dim),
      Span::styled(&chmod_state.octal_input, highlight),
      Span::styled("_", Style::default().fg(Color::Indexed(245))),
      Span::styled(format!(" (was {})", original_octal), dim),
    ]));
  } else {
    lines.push(Line::from(vec![
      Span::styled(" Octal: ", dim),
      Span::styled(octal_str, octal_style),
      Span::styled(format!(" (was {})", original_octal), dim),
    ]));
  }

  // Recursive option (only for directories)
  if chmod_state.is_dir {
    let recursive_style = if chmod_state.recursive { changed } else { dim };
    let recursive_text = if chmod_state.recursive { "[x]" } else { "[ ]" };
    lines.push(Line::from(vec![
      Span::styled(" Recursive: ", dim),
      Span::styled(recursive_text, recursive_style),
      Span::styled(" (d)", dim),
    ]));
  }

  lines.push(Line::from(""));

  // Hints
  lines.push(Line::from(vec![
    Span::styled(" [Enter] Apply  [Esc] Cancel  [Tab] Octal mode", dim),
  ]));

  let title = if chmod_state.is_dir { " Permissions (directory) " } else { " Permissions " };
  let block = Block::default()
    .borders(Borders::ALL)
    .title(title)
    .border_style(Style::default().fg(Color::Indexed(245)))
    .style(Style::default().bg(Color::Indexed(235)));

  let paragraph = Paragraph::new(lines).block(block);
  paragraph.render(popup, buf);
}

struct PermRowConfig<'a> {
  label: &'a str,
  r_key: char,
  w_key: char,
  x_key: char,
  shift: u8,
}

#[allow(clippy::too_many_arguments)]
fn render_permission_row<'a>(
  cfg: PermRowConfig<'a>,
  mode: u32,
  original_mode: u32,
  changed: Style,
) -> Vec<Span<'a>> {
  let dim = Style::default().fg(Color::Indexed(241));
  let on = Style::default().fg(Color::Indexed(114));
  let off = Style::default().fg(Color::Indexed(167));

  let r_bit = 0o4 << cfg.shift;
  let w_bit = 0o2 << cfg.shift;
  let x_bit = 0o1 << cfg.shift;

  let r_on = mode & r_bit != 0;
  let w_on = mode & w_bit != 0;
  let x_on = mode & x_bit != 0;

  let r_changed = (mode & r_bit) != (original_mode & r_bit);
  let w_changed = (mode & w_bit) != (original_mode & w_bit);
  let x_changed = (mode & x_bit) != (original_mode & x_bit);

  let r_style = if r_changed { changed } else if r_on { on } else { off };
  let w_style = if w_changed { changed } else if w_on { on } else { off };
  let x_style = if x_changed { changed } else if x_on { on } else { off };

  vec![
    Span::styled(format!(" {:6}", cfg.label), dim),
    Span::styled("  ", dim),
    Span::styled(if r_on { "r" } else { "-" }, r_style),
    Span::styled(format!("({})", cfg.r_key), dim),
    Span::styled(if w_on { "w" } else { "-" }, w_style),
    Span::styled(format!("({})", cfg.w_key), dim),
    Span::styled(if x_on { "x" } else { "-" }, x_style),
    Span::styled(format!("({})", cfg.x_key), dim),
  ]
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::app::ChmodState;
  use std::path::PathBuf;

  fn make_chmod_state(mode: u32, is_dir: bool) -> ChmodState {
    ChmodState {
      path: PathBuf::from("/test/file.txt"),
      original_mode: mode,
      new_mode: mode,
      is_dir,
      recursive: false,
      octal_mode: false,
      octal_input: String::new(),
    }
  }

  #[test]
  fn test_render_permission_row_all_on() {
    let changed = Style::default().fg(Color::Indexed(114));
    let cfg = PermRowConfig { label: "Owner", r_key: 'r', w_key: 'w', x_key: 'x', shift: 6 };
    let spans = render_permission_row(cfg, 0o700, 0o700, changed);
    // Check that rwx are rendered
    assert!(spans.iter().any(|s| s.content.contains("r")));
    assert!(spans.iter().any(|s| s.content.contains("w")));
    assert!(spans.iter().any(|s| s.content.contains("x")));
  }

  #[test]
  fn test_render_permission_row_all_off() {
    let changed = Style::default().fg(Color::Indexed(114));
    let cfg = PermRowConfig { label: "Owner", r_key: 'r', w_key: 'w', x_key: 'x', shift: 6 };
    let spans = render_permission_row(cfg, 0o000, 0o000, changed);
    // Check that --- are rendered (dashes instead of letters)
    let content: String = spans.iter().map(|s| s.content.to_string()).collect();
    assert!(content.contains("-"));
  }

  #[test]
  fn test_chmod_state_initial() {
    let state = make_chmod_state(0o644, false);
    assert_eq!(state.original_mode, 0o644);
    assert_eq!(state.new_mode, 0o644);
    assert!(!state.is_dir);
    assert!(!state.recursive);
  }
}
