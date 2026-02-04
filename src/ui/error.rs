use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Widget, Wrap};

pub fn render_error(messages: &[String], area: Rect, buf: &mut Buffer) {
  let max_width = (area.width * 95 / 100).max(20);
  let hint_len = " [Esc] dismiss".len() as u16;

  // +3 = 1 leading space in content + 2 border columns
  let content_width = messages
    .iter()
    .map(|m| m.len() as u16 + 1)
    .max()
    .unwrap_or(0)
    .max(hint_len)
    + 2;

  let width = content_width.min(max_width);
  let inner_width = width.saturating_sub(2) as usize;

  // Estimate line count with word-wrapping
  let mut line_count: u16 = 0;
  for msg in messages {
    // +1 for the leading space we prepend
    line_count += (((msg.len() + 1) / inner_width.max(1)) as u16) + 1;
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

  let mut lines: Vec<Line> = messages
    .iter()
    .map(|msg| {
      Line::from(Span::styled(
        format!(" {msg}"),
        Style::default().fg(Color::Indexed(252)),
      ))
    })
    .collect();

  lines.push(Line::from(""));
  lines.push(Line::from(Span::styled(
    " [Esc] dismiss",
    Style::default().fg(Color::Indexed(241)),
  )));

  let block = Block::default()
    .borders(Borders::ALL)
    .title(" Error ")
    .border_style(Style::default().fg(Color::Indexed(167)))
    .style(Style::default().bg(Color::Indexed(235)));

  let paragraph = Paragraph::new(lines).block(block).wrap(Wrap { trim: false });
  paragraph.render(popup, buf);
}
