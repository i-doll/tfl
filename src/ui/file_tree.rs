use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Widget};

use crate::app::App;
use crate::icons::{file_icon, file_name_color};

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
    let icon = file_icon(&entry.name, entry.is_dir, entry.expanded, entry.is_symlink);
    let name_color = file_name_color(&entry.name, entry.is_dir, entry.is_symlink);
    let symlink_indicator = if let Some(ref target) = entry.symlink_target {
      format!(" → {target}")
    } else {
      String::new()
    };

    let (icon_style, name_style) = if is_selected {
      let sel = Style::default()
        .fg(Color::Indexed(234))
        .bg(Color::Indexed(75))
        .add_modifier(Modifier::BOLD);
      (sel, sel)
    } else if entry.is_git_ignored {
      (
        Style::default().fg(icon.color).add_modifier(Modifier::DIM),
        Style::default().fg(name_color).add_modifier(Modifier::DIM),
      )
    } else {
      (
        Style::default().fg(icon.color),
        Style::default().fg(name_color),
      )
    };

    let line = Line::from(vec![
      Span::styled(indent, name_style),
      Span::styled(icon.glyph, icon_style),
      Span::styled(entry.name.clone(), name_style),
      Span::styled(symlink_indicator, Style::default().fg(Color::DarkGray)),
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

