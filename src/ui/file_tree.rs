use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Widget};

use crate::app::{App, ClipboardOp};
use crate::icons::{file_icon, file_name_color};

/// Splits a name into spans with highlighted matches
fn highlight_name<'a>(name: &'a str, ranges: &[(usize, usize)], base_style: Style, highlight_style: Style) -> Vec<Span<'a>> {
  if ranges.is_empty() {
    return vec![Span::styled(name.to_string(), base_style)];
  }

  let mut spans = Vec::new();
  let mut last_end = 0;

  for &(start, end) in ranges {
    // Clamp to valid byte boundaries
    let start = start.min(name.len());
    let end = end.min(name.len());
    if start >= end || start < last_end {
      continue;
    }

    // Text before the match
    if last_end < start {
      spans.push(Span::styled(name[last_end..start].to_string(), base_style));
    }

    // The matched text
    spans.push(Span::styled(name[start..end].to_string(), highlight_style));
    last_end = end;
  }

  // Text after the last match
  if last_end < name.len() {
    spans.push(Span::styled(name[last_end..].to_string(), base_style));
  }

  spans
}

pub fn render_file_tree(app: &App, area: Rect, buf: &mut Buffer) {
  render_file_tree_with_active(app, area, buf, true, false);
}

pub fn render_file_tree_with_active(app: &App, area: Rect, buf: &mut Buffer, is_active: bool, is_right_pane: bool) {
  let (entries, cursor, scroll_offset, tree, search_query) = if is_right_pane {
    if let Some(ref pane) = app.right_pane {
      (pane.visible_entries(), pane.cursor, pane.scroll_offset, &pane.tree, &pane.search_query)
    } else {
      return;
    }
  } else {
    (app.visible_entries(), app.cursor, app.tree_scroll_offset, &app.tree, &app.search_query)
  };

  let inner_height = area.height.saturating_sub(2) as usize; // borders

  let mut lines: Vec<Line> = Vec::with_capacity(inner_height);

  let start = scroll_offset.min(entries.len());
  let end = (start + inner_height).min(entries.len());

  for (view_idx, &entry_idx) in entries[start..end].iter().enumerate() {
    let entry = &tree.entries[entry_idx];
    let is_selected = start + view_idx == cursor;

    let indent = "  ".repeat(entry.depth);
    let icon = file_icon(&entry.name, entry.is_dir, entry.expanded, entry.is_symlink);
    let name_color = file_name_color(&entry.name, entry.is_dir, entry.is_symlink);
    let symlink_indicator = if let Some(ref target) = entry.symlink_target {
      format!(" -> {target}")
    } else {
      String::new()
    };

    let is_cut = app.clipboard.op == Some(ClipboardOp::Cut)
      && app.clipboard.paths.contains(&entry.path);

    let (icon_style, name_style) = if is_selected && is_active {
      // Active pane with selected item
      let sel = Style::default()
        .fg(Color::Indexed(234))
        .bg(Color::Indexed(75))
        .add_modifier(Modifier::BOLD);
      (sel, sel)
    } else if is_selected && !is_active {
      // Inactive pane with selected item - dimmer highlight
      let sel = Style::default()
        .fg(Color::Indexed(234))
        .bg(Color::Indexed(240));
      (sel, sel)
    } else if is_cut {
      (
        Style::default().fg(icon.color).add_modifier(Modifier::DIM | Modifier::CROSSED_OUT),
        Style::default().fg(name_color).add_modifier(Modifier::DIM | Modifier::CROSSED_OUT),
      )
    } else if entry.is_git_ignored {
      (
        Style::default().fg(icon.color).add_modifier(Modifier::DIM),
        Style::default().fg(name_color).add_modifier(Modifier::DIM),
      )
    } else if let Some(status_color) = entry.git_status.display_color() {
      (
        Style::default().fg(status_color),
        Style::default().fg(status_color),
      )
    } else {
      (
        Style::default().fg(icon.color),
        Style::default().fg(name_color),
      )
    };

    // Get match ranges for highlighting (only if not selected, as selection has its own highlight)
    let match_ranges = if is_selected {
      Vec::new()
    } else {
      app.search_match_ranges(&entry.name)
    };

    // Highlight style for search matches (yellow/gold color, underlined)
    let highlight_style = name_style
      .fg(Color::Indexed(220))
      .add_modifier(Modifier::BOLD | Modifier::UNDERLINED);

    let mut spans = vec![
      Span::styled(indent, name_style),
      Span::styled(icon.glyph, icon_style),
    ];
    spans.extend(highlight_name(&entry.name, &match_ranges, name_style, highlight_style));
    spans.push(Span::styled(symlink_indicator, Style::default().fg(Color::DarkGray)));

    let line = Line::from(spans);

    lines.push(line);
  }

  // Get directory name from tree root
  let dir_name = tree.root
    .file_name()
    .and_then(|n| n.to_str())
    .unwrap_or("~");

  let title = if tree.show_hidden {
    format!(" {} [hidden: on] ", dir_name)
  } else {
    format!(" {} ", dir_name)
  };

  let border_color = if is_active {
    Color::Indexed(75) // Blue for active pane
  } else {
    Color::Indexed(240) // Gray for inactive pane
  };

  let title_color = if is_active {
    Color::Indexed(75)
  } else {
    Color::Indexed(245)
  };

  let block = Block::default()
    .borders(Borders::ALL)
    .border_style(Style::default().fg(border_color))
    .title(title)
    .title_style(Style::default().fg(title_color));

  let paragraph = Paragraph::new(lines).block(block);
  paragraph.render(area, buf);

  // Suppress unused variable warning
  let _ = search_query;
}

