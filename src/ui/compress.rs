use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Widget};

use crate::app::App;

pub fn render_compress(app: &App, area: Rect, buf: &mut Buffer) {
  let count = app.operation_targets().len();
  let title = if count == 1 {
    " Compress (1 file) ".to_string()
  } else {
    format!(" Compress ({count} files) ")
  };

  let width = 28.min(area.width.saturating_sub(4));
  let height = 9.min(area.height.saturating_sub(2));

  if width < 10 || height < 5 {
    return;
  }

  let x = area.x + (area.width.saturating_sub(width)) / 2;
  let y = area.y + (area.height.saturating_sub(height)) / 2;
  let popup = Rect::new(x, y, width, height);

  Clear.render(popup, buf);

  let item_style = Style::default().fg(Color::Indexed(252));
  let key_style = Style::default().fg(Color::Indexed(75)).add_modifier(Modifier::BOLD);
  let dim_style = Style::default().fg(Color::Indexed(241));

  let lines = vec![
    Line::from(vec![
      Span::styled("  ", item_style),
      Span::styled("1", key_style),
      Span::styled(") .zip", item_style),
    ]),
    Line::from(vec![
      Span::styled("  ", item_style),
      Span::styled("2", key_style),
      Span::styled(") .tar.gz", item_style),
    ]),
    Line::from(vec![
      Span::styled("  ", item_style),
      Span::styled("3", key_style),
      Span::styled(") .tar.bz2", item_style),
    ]),
    Line::from(vec![
      Span::styled("  ", item_style),
      Span::styled("4", key_style),
      Span::styled(") .tar.xz", item_style),
    ]),
    Line::from(""),
    Line::from(Span::styled("  Esc to cancel", dim_style)),
  ];

  let block = Block::default()
    .borders(Borders::ALL)
    .title(title)
    .border_style(Style::default().fg(Color::Indexed(245)))
    .style(Style::default().bg(Color::Indexed(235)));

  let paragraph = Paragraph::new(lines).block(block);
  paragraph.render(popup, buf);
}
