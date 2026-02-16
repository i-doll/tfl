use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Widget, Wrap};

use crate::theme::Theme;

pub fn render_error(messages: &[String], area: Rect, buf: &mut Buffer, theme: &Theme) {
  let max_width = (area.width * 95 / 100).max(20);
  let hint_len = " [Esc] dismiss".len() as u16;

  // Split messages on newlines; each sub-line gets a leading space
  let msg_style = Style::default().fg(theme.text);
  let mut lines: Vec<Line> = Vec::new();
  for msg in messages {
    for sub in msg.split('\n') {
      lines.push(Line::from(Span::styled(format!(" {sub}"), msg_style)));
    }
  }

  // +2 for border columns
  let content_width = lines
    .iter()
    .map(|l| l.width() as u16)
    .max()
    .unwrap_or(0)
    .max(hint_len)
    + 2;

  let width = content_width.min(max_width);
  let inner_width = width.saturating_sub(2) as usize;

  // Estimate displayed line count accounting for word-wrapping
  let mut line_count: u16 = 0;
  for line in &lines {
    let w = line.width();
    line_count += ((w / inner_width.max(1)) as u16) + 1;
  }
  // +2 for borders, +1 for hint line, +1 for blank line before hint
  let height = (line_count + 4).min(area.height.saturating_sub(2));

  if width < 10 || height < 3 {
    return;
  }

  let x = area.x + (area.width.saturating_sub(width)) / 2;
  let y = area.y + (area.height.saturating_sub(height)) / 2;
  let popup = Rect::new(x, y, width, height);

  Clear.render(popup, buf);

  lines.push(Line::from(""));
  lines.push(Line::from(Span::styled(
    " [Esc] dismiss",
    Style::default().fg(theme.text_muted),
  )));

  let block = Block::default()
    .borders(Borders::ALL)
    .title(" Error ")
    .border_style(Style::default().fg(theme.error))
    .style(Style::default().bg(theme.bg_overlay));

  let paragraph = Paragraph::new(lines).block(block).wrap(Wrap { trim: false });
  paragraph.render(popup, buf);
}
