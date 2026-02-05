use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Widget};

use crate::app::App;

pub fn render_saved_searches(app: &App, area: Rect, buf: &mut Buffer) {
  let width = 60.min(area.width.saturating_sub(4));
  let searches = app.saved_searches.list();
  let content_height = if searches.is_empty() { 3 } else { searches.len() as u16 + 2 };
  let height = content_height.min(area.height.saturating_sub(2));

  if width < 10 || height < 3 {
    return;
  }

  let x = area.x + (area.width.saturating_sub(width)) / 2;
  let y = area.y + (area.height.saturating_sub(height)) / 2;
  let popup = Rect::new(x, y, width, height);

  Clear.render(popup, buf);

  let lines: Vec<Line> = if searches.is_empty() {
    vec![
      Line::from(""),
      Line::from(Span::styled(
        " No saved searches - use Ctrl+s to save current search",
        Style::default().fg(Color::Indexed(241)),
      )),
    ]
  } else {
    searches
      .iter()
      .enumerate()
      .map(|(i, search)| {
        let number = if i < 9 {
          format!("{} ", i + 1)
        } else {
          "  ".to_string()
        };

        let pattern = &search.pattern;
        let filters = build_filter_string(search);

        let display = if filters.is_empty() {
          format!("{}: {}", search.name, pattern)
        } else {
          format!("{}: {} ({})", search.name, pattern, filters)
        };

        if i == app.saved_searches_cursor {
          Line::from(vec![
            Span::styled(
              format!(" {number}> "),
              Style::default().fg(Color::Indexed(75)),
            ),
            Span::styled(
              display,
              Style::default()
                .fg(Color::Indexed(75))
                .add_modifier(Modifier::BOLD),
            ),
          ])
        } else {
          Line::from(vec![
            Span::styled(
              format!(" {number}  "),
              Style::default().fg(Color::Indexed(241)),
            ),
            Span::styled(
              display,
              Style::default().fg(Color::Indexed(252)),
            ),
          ])
        }
      })
      .collect()
  };

  let block = Block::default()
    .borders(Borders::ALL)
    .title(" Saved Searches (1-9 to quick select, d to delete) ")
    .border_style(Style::default().fg(Color::Indexed(245)))
    .style(Style::default().bg(Color::Indexed(235)));

  let paragraph = Paragraph::new(lines).block(block);
  paragraph.render(popup, buf);
}

fn build_filter_string(search: &crate::saved_searches::SavedSearch) -> String {
  let mut parts = Vec::new();
  if search.regex {
    parts.push("regex".to_string());
  }
  if let Some(ref size) = search.size {
    parts.push(format!("size:{}", size));
  }
  if let Some(ref date) = search.date {
    parts.push(format!("date:{}", date));
  }
  parts.join(", ")
}
