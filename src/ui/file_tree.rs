use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Widget};

use crate::app::App;

pub fn render_file_tree(app: &App, area: Rect, buf: &mut Buffer) {
  let entries = app.visible_entries();
  let inner_height = area.height.saturating_sub(2) as usize; // borders

  let mut lines: Vec<Line> = Vec::with_capacity(inner_height);

  let start = app.tree_scroll_offset;
  let end = (start + inner_height).min(entries.len());

  for (view_idx, &entry_idx) in entries[start..end].iter().enumerate() {
    let entry = &app.tree.entries[entry_idx];
    let is_selected = start + view_idx == app.cursor;

    let indent = "  ".repeat(entry.depth);
    let icon = if entry.is_dir {
      if entry.expanded { "v " } else { "> " }
    } else {
      "  "
    };

    let symlink_indicator = if entry.is_symlink { " → " } else { "" };

    let name_color = if entry.is_dir {
      Color::Indexed(75) // blue
    } else if entry.name.ends_with(".rs") {
      Color::Indexed(208) // orange
    } else if entry.name.ends_with(".toml") || entry.name.ends_with(".json") || entry.name.ends_with(".yaml") || entry.name.ends_with(".yml") {
      Color::Indexed(150) // green-ish
    } else if entry.is_symlink {
      Color::Indexed(176) // purple
    } else {
      Color::Indexed(252) // light gray
    };

    let style = if is_selected {
      Style::default()
        .fg(Color::Indexed(234))
        .bg(Color::Indexed(75))
        .add_modifier(Modifier::BOLD)
    } else {
      Style::default().fg(name_color)
    };

    let line = Line::from(vec![
      Span::styled(format!("{indent}{icon}"), style),
      Span::styled(entry.name.clone(), style),
      Span::styled(symlink_indicator.to_string(), Style::default().fg(Color::DarkGray)),
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

