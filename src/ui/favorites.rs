use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Widget};

use crate::app::App;
use crate::theme::Theme;

fn contract_home(path: &std::path::Path) -> String {
  if let Some(home) = dirs::home_dir()
    && let Ok(rest) = path.strip_prefix(&home)
  {
    return format!("~/{}", rest.display());
  }
  path.to_string_lossy().to_string()
}

pub fn render_favorites(app: &App, area: Rect, buf: &mut Buffer, theme: &Theme) {
  let width = 50.min(area.width.saturating_sub(4));
  let favs = app.favorites.list();
  let content_height = if favs.is_empty() { 3 } else { favs.len() as u16 + 2 };
  let height = content_height.min(area.height.saturating_sub(2));

  if width < 10 || height < 3 {
    return;
  }

  let x = area.x + (area.width.saturating_sub(width)) / 2;
  let y = area.y + (area.height.saturating_sub(height)) / 2;
  let popup = Rect::new(x, y, width, height);

  Clear.render(popup, buf);

  let lines: Vec<Line> = if favs.is_empty() {
    vec![
      Line::from(""),
      Line::from(Span::styled(
        " No favorites — press a to add current dir",
        Style::default().fg(theme.text_muted),
      )),
    ]
  } else {
    favs
      .iter()
      .enumerate()
      .map(|(i, path)| {
        let display = contract_home(path);
        if i == app.favorites_cursor {
          Line::from(Span::styled(
            format!(" > {display}"),
            Style::default()
              .fg(theme.accent)
              .add_modifier(Modifier::BOLD),
          ))
        } else {
          Line::from(Span::styled(
            format!("   {display}"),
            Style::default().fg(theme.text),
          ))
        }
      })
      .collect()
  };

  let block = Block::default()
    .borders(Borders::ALL)
    .title(" Favorites ")
    .border_style(Style::default().fg(theme.title_inactive))
    .style(Style::default().bg(theme.bg_overlay));

  let paragraph = Paragraph::new(lines).block(block);
  paragraph.render(popup, buf);
}
