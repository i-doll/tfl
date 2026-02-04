pub mod breadcrumb;
pub mod chmod;
pub mod error;
pub mod favorites;
pub mod file_tree;
pub mod help;
pub mod open_with;
pub mod preview;
pub mod properties;
pub mod status_bar;

use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};
use ratatui::Frame;

use crate::app::App;
use crate::config::Config;

pub fn draw(frame: &mut Frame, app: &mut App, config: &Config) {
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

  // Header with breadcrumb navigation
  render_header(app, chunks[0], frame.buffer_mut());

  if app.dual_pane_mode {
    // Dual-pane layout: left tree | right tree | preview
    let left_pct = app.dual_left_ratio;
    let right_pct = app.dual_right_ratio;
    let preview_pct = 100u16.saturating_sub(left_pct + right_pct).max(10);

    let main_chunks = Layout::default()
      .direction(Direction::Horizontal)
      .constraints([
        Constraint::Percentage(left_pct),    // left tree
        Constraint::Percentage(right_pct),   // right tree
        Constraint::Percentage(preview_pct), // preview
      ])
      .split(chunks[1]);

    // Update viewport height
    app.viewport_height = main_chunks[0].height.saturating_sub(2) as usize;

    // Left tree (active indicator based on active_pane)
    file_tree::render_file_tree_with_active(app, main_chunks[0], frame.buffer_mut(), app.active_pane == 0, false);

    // Right tree
    file_tree::render_file_tree_with_active(app, main_chunks[1], frame.buffer_mut(), app.active_pane == 1, true);

    // Preview (smaller)
    preview::render_preview(app, main_chunks[2], frame.buffer_mut());
  } else {
    // Single-pane layout: tree | preview
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
  }

  // Status bar
  status_bar::render_status_bar(app, chunks[2], frame.buffer_mut());

  // Overlays
  if app.show_help {
    help::render_help(config, area, frame.buffer_mut());
  }
  if app.input_mode == crate::event::InputMode::Favorites {
    favorites::render_favorites(app, area, frame.buffer_mut());
  }
  if app.input_mode == crate::event::InputMode::OpenWith {
    open_with::render_open_with(app, area, frame.buffer_mut());
  }
  if app.input_mode == crate::event::InputMode::Chmod {
    chmod::render_chmod(app, area, frame.buffer_mut());
  }
  if app.input_mode == crate::event::InputMode::Properties
    && let Some(ref props) = app.file_properties
  {
    properties::render_properties(props, area, frame.buffer_mut());
  }
  if !app.error_messages.is_empty() {
    error::render_error(&app.error_messages, area, frame.buffer_mut());
  }
}

fn render_header(app: &mut App, area: Rect, buf: &mut Buffer) {
  // Calculate available width for breadcrumbs (subtract git branch if present)
  let git_branch_width = app.tree.git_info.branch.as_ref()
    .map(|b| b.len() as u16 + 4) // "  " + branch
    .unwrap_or(0);
  let breadcrumb_width = area.width.saturating_sub(git_branch_width + 2); // +2 for padding

  // Truncate breadcrumbs if needed
  let (segments, truncated) = breadcrumb::truncate_breadcrumbs(
    &app.breadcrumb_segments,
    breadcrumb_width,
  );
  app.breadcrumb_truncated = truncated;

  // Current segment is the last one (the current directory)
  let current_segment = segments.len().saturating_sub(1);

  let mut spans = vec![Span::styled(" ", Style::default().fg(Color::Indexed(75)))];

  let separator = " > ";
  let separator_style = Style::default().fg(Color::Indexed(240));
  let ellipsis_style = Style::default().fg(Color::Indexed(240));

  // Add ellipsis prefix if truncated and first segment doesn't start at 0
  if truncated && segments.first().map(|s| s.start_col > 0).unwrap_or(false) {
    spans.push(Span::styled("...", ellipsis_style));
    spans.push(Span::styled(separator, separator_style));
  }

  for (i, segment) in segments.iter().enumerate() {
    if i > 0 {
      // Add ellipsis between first and last when truncated
      if truncated && i == segments.len() - 1 && segments.len() == 2 {
        spans.push(Span::styled(separator, separator_style));
        spans.push(Span::styled("...", ellipsis_style));
      }
      spans.push(Span::styled(separator, separator_style));
    }

    let style = if i == current_segment {
      Style::default()
        .fg(Color::Indexed(75))
        .add_modifier(Modifier::BOLD)
    } else {
      Style::default().fg(Color::Indexed(252))
    };

    spans.push(Span::styled(&segment.name, style));
  }

  if let Some(ref branch) = app.tree.git_info.branch {
    spans.push(Span::styled("  ", Style::default().fg(Color::Indexed(114))));
    spans.push(Span::styled(
      branch.clone(),
      Style::default().fg(Color::Indexed(114)),
    ));
  }

  let line = Line::from(spans);
  let paragraph = Paragraph::new(line).style(Style::default().bg(Color::Indexed(236)));
  paragraph.render(area, buf);
}
