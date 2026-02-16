use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Widget};

use crate::app::App;
use crate::theme::Theme;

pub fn render_open_with(app: &App, area: Rect, buf: &mut Buffer, theme: &Theme) {
  let apps = &app.open_with_apps;
  let item_count = 1 + apps.len(); // "Default Application" + detected apps
  let width = 40.min(area.width.saturating_sub(4));
  let content_height = (item_count as u16 + 2).min(area.height.saturating_sub(2));

  if width < 10 || content_height < 3 {
    return;
  }

  let x = area.x + (area.width.saturating_sub(width)) / 2;
  let y = area.y + (area.height.saturating_sub(content_height)) / 2;
  let popup = Rect::new(x, y, width, content_height);

  Clear.render(popup, buf);

  let mut lines: Vec<Line> = Vec::with_capacity(item_count);

  // "Default Application" entry at index 0
  let selected = app.open_with_cursor == 0;
  lines.push(app_line("Default Application", None, selected, theme));

  // Detected apps
  for (i, app_entry) in apps.iter().enumerate() {
    let selected = app.open_with_cursor == i + 1;
    let suffix = if app_entry.is_tui {
      Some("tui")
    } else if app_entry.dir_mode {
      Some("dir")
    } else {
      None
    };
    lines.push(app_line(&app_entry.name, suffix, selected, theme));
  }

  let block = Block::default()
    .borders(Borders::ALL)
    .title(" Open with ")
    .border_style(Style::default().fg(theme.title_inactive))
    .style(Style::default().bg(theme.bg_overlay));

  let paragraph = Paragraph::new(lines).block(block);
  paragraph.render(popup, buf);
}

fn app_line(name: &str, suffix: Option<&str>, selected: bool, theme: &Theme) -> Line<'static> {
  let prefix = if selected { " > " } else { "   " };
  let style = if selected {
    Style::default()
      .fg(theme.accent)
      .add_modifier(Modifier::BOLD)
  } else {
    Style::default().fg(theme.text)
  };

  let mut spans = vec![Span::styled(format!("{prefix}{name}"), style)];

  if let Some(tag) = suffix {
    spans.push(Span::styled(
      format!(" ({tag})"),
      Style::default().fg(theme.text_muted),
    ));
  }

  Line::from(spans)
}
