pub mod file_tree;
pub mod preview;
pub mod status_bar;

use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};
use ratatui::Frame;

use crate::app::App;

pub fn draw(frame: &mut Frame, app: &mut App) {
  let area = frame.area();

  // Vertical layout: header, main, status bar
  let chunks = Layout::default()
    .direction(Direction::Vertical)
    .constraints([
      Constraint::Length(1), // header
      Constraint::Min(3),   // main area
      Constraint::Length(1), // status bar
    ])
    .split(area);

  // Header
  render_header(app, chunks[0], frame.buffer_mut());

  // Main area: horizontal split
  let main_chunks = Layout::default()
    .direction(Direction::Horizontal)
    .constraints([
      Constraint::Percentage(app.tree_ratio),
      Constraint::Percentage(100 - app.tree_ratio),
    ])
    .split(chunks[1]);

  // Update viewport height
  app.viewport_height = main_chunks[0].height.saturating_sub(2) as usize;

  // File tree (left pane)
  file_tree::render_file_tree(app, main_chunks[0], frame.buffer_mut());

  // Preview (right pane)
  preview::render_preview(app, main_chunks[1], frame.buffer_mut());

  // Status bar
  status_bar::render_status_bar(app, chunks[2], frame.buffer_mut());
}

fn render_header(app: &App, area: Rect, buf: &mut Buffer) {
  let path_str = app.tree.root.to_string_lossy();
  let line = Line::from(vec![
    Span::styled(" ", Style::default().fg(Color::Indexed(75))),
    Span::styled(
      path_str.to_string(),
      Style::default()
        .fg(Color::Indexed(252))
        .add_modifier(Modifier::BOLD),
    ),
  ]);
  let paragraph = Paragraph::new(line).style(Style::default().bg(Color::Indexed(236)));
  paragraph.render(area, buf);
}
