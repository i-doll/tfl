use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Widget};

use std::path::Path;

use crate::app::{App, ClipboardOp};
use crate::date_filter::{DateFilter, TimeType};
use crate::icons::{file_icon, file_name_color};

fn get_time_indicator(path: &Path, time_type: TimeType) -> String {
  let metadata = match path.metadata() {
    Ok(m) => m,
    Err(_) => return String::new(),
  };

  let file_time = match time_type {
    TimeType::Modified => metadata.modified(),
    TimeType::Created => metadata.created(),
    TimeType::Accessed => metadata.accessed(),
  };

  match file_time {
    Ok(t) => format!(" {}", DateFilter::format_time(t)),
    Err(_) => String::new(),
  }
}

pub fn render_file_tree(app: &App, area: Rect, buf: &mut Buffer) {
  let entries = app.visible_entries();
  let inner_height = area.height.saturating_sub(2) as usize; // borders

  let mut lines: Vec<Line> = Vec::with_capacity(inner_height);

  let start = app.tree_scroll_offset.min(entries.len());
  let end = (start + inner_height).min(entries.len());

  for (view_idx, &entry_idx) in entries[start..end].iter().enumerate() {
    let entry = &app.tree.entries[entry_idx];
    let is_selected = start + view_idx == app.cursor;

    let indent = "  ".repeat(entry.depth);
    let icon = file_icon(&entry.name, entry.is_dir, entry.expanded, entry.is_symlink);
    let name_color = file_name_color(&entry.name, entry.is_dir, entry.is_symlink);
    let symlink_indicator = if let Some(ref target) = entry.symlink_target {
      format!(" → {target}")
    } else {
      String::new()
    };

    let is_cut = app.clipboard.op == Some(ClipboardOp::Cut)
      && app.clipboard.paths.contains(&entry.path);

    let (icon_style, name_style) = if is_selected {
      let sel = Style::default()
        .fg(Color::Indexed(234))
        .bg(Color::Indexed(75))
        .add_modifier(Modifier::BOLD);
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

    // Show matching time when date filter is active
    let time_indicator = if app.date_filter.is_some() {
      get_time_indicator(&entry.path, app.date_filter_time_type)
    } else {
      String::new()
    };

    let line = Line::from(vec![
      Span::styled(indent, name_style),
      Span::styled(icon.glyph, icon_style),
      Span::styled(entry.name.clone(), name_style),
      Span::styled(symlink_indicator, Style::default().fg(Color::DarkGray)),
      Span::styled(time_indicator, Style::default().fg(Color::Indexed(178))),
    ]);

    lines.push(line);
  }

  let title = if app.tree.show_hidden {
    " Files [hidden: on] "
  } else {
    " Files "
  };

  let block = Block::default()
    .borders(Borders::ALL)
    .border_style(Style::default().fg(Color::Indexed(240)))
    .title(title)
    .title_style(Style::default().fg(Color::Indexed(75)));

  let paragraph = Paragraph::new(lines).block(block);
  paragraph.render(area, buf);
}

