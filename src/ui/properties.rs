use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Widget};

use crate::fs::FileProperties;

const LABEL_STYLE: Style = Style::new().fg(Color::Indexed(245));
const VALUE_STYLE: Style = Style::new().fg(Color::Indexed(252));
const PATH_STYLE: Style = Style::new().fg(Color::Indexed(75));

pub fn render_properties(props: &FileProperties, area: Rect, buf: &mut Buffer) {
  let width = 60.min(area.width.saturating_sub(4));
  let height = 20.min(area.height.saturating_sub(2));

  if width < 20 || height < 8 {
    return;
  }

  let x = area.x + (area.width.saturating_sub(width)) / 2;
  let y = area.y + (area.height.saturating_sub(height)) / 2;
  let popup = Rect::new(x, y, width, height);

  Clear.render(popup, buf);

  let mut lines: Vec<Line> = Vec::new();

  // Path (may need truncation)
  let max_path_len = (width as usize).saturating_sub(12);
  let path_display = if props.path.len() > max_path_len {
    format!("...{}", &props.path[props.path.len().saturating_sub(max_path_len - 3)..])
  } else {
    props.path.clone()
  };
  lines.push(property_line("Path", &path_display, PATH_STYLE));

  // Type
  lines.push(property_line("Type", &props.file_type, VALUE_STYLE));

  // MIME type (if available)
  if let Some(ref mime) = props.mime_type {
    lines.push(property_line("MIME", mime, VALUE_STYLE));
  }

  // Size
  if !props.is_dir {
    let size_str = format!("{} ({} bytes)", props.size_human, props.size);
    lines.push(property_line("Size", &size_str, VALUE_STYLE));
  }

  // Symlink target
  if let Some(ref target) = props.symlink_target {
    let max_target_len = (width as usize).saturating_sub(14);
    let target_display = if target.len() > max_target_len {
      format!("...{}", &target[target.len().saturating_sub(max_target_len - 3)..])
    } else {
      target.clone()
    };
    lines.push(property_line("Target", &target_display, PATH_STYLE));
  }

  // Permissions
  let perms_str = format!("{} ({})", props.permissions_rwx, props.permissions_octal);
  lines.push(property_line("Permissions", &perms_str, VALUE_STYLE));

  // Owner/Group
  let owner_str = format!("{} / {}", props.owner, props.group);
  lines.push(property_line("Owner/Group", &owner_str, VALUE_STYLE));

  // Timestamps
  if let Some(ref modified) = props.modified {
    lines.push(property_line("Modified", modified, VALUE_STYLE));
  }
  if let Some(ref accessed) = props.accessed {
    lines.push(property_line("Accessed", accessed, VALUE_STYLE));
  }
  if let Some(ref created) = props.created {
    lines.push(property_line("Created", created, VALUE_STYLE));
  }

  // Footer
  lines.push(Line::from(""));
  lines.push(Line::from(Span::styled(
    " Press i, q, or Esc to close".to_string(),
    Style::default().fg(Color::Indexed(241)),
  )));

  let block = Block::default()
    .borders(Borders::ALL)
    .title(" Properties ")
    .border_style(Style::default().fg(Color::Indexed(245)))
    .style(Style::default().bg(Color::Indexed(235)));

  let paragraph = Paragraph::new(lines).block(block);
  paragraph.render(popup, buf);
}

fn property_line(label: &str, value: &str, value_style: Style) -> Line<'static> {
  Line::from(vec![
    Span::styled(
      format!("  {label:<12}"),
      LABEL_STYLE.add_modifier(Modifier::BOLD),
    ),
    Span::styled(value.to_string(), value_style),
  ])
}

#[cfg(test)]
mod tests {
  use super::*;

  fn make_test_props() -> FileProperties {
    FileProperties {
      path: "/home/user/test.txt".to_string(),
      size: 1234,
      size_human: "1.21 KB".to_string(),
      permissions_octal: "0644".to_string(),
      permissions_rwx: "-rw-r--r--".to_string(),
      owner: "user".to_string(),
      group: "staff".to_string(),
      created: Some("2024-01-15 10:30:00".to_string()),
      modified: Some("2024-01-20 14:45:30".to_string()),
      accessed: Some("2024-01-21 09:00:00".to_string()),
      file_type: "Text file".to_string(),
      mime_type: Some("text/plain".to_string()),
      symlink_target: None,
      is_dir: false,
      is_symlink: false,
    }
  }

  #[test]
  fn test_property_line_formatting() {
    let line = property_line("Size", "1.21 KB", VALUE_STYLE);
    // Just verify it creates a line with 2 spans
    assert_eq!(line.spans.len(), 2);
  }

  #[test]
  fn test_render_properties_small_area_returns_early() {
    let props = make_test_props();
    let area = Rect::new(0, 0, 10, 5); // Too small
    let mut buf = Buffer::empty(area);
    render_properties(&props, area, &mut buf);
    // Should return early without crashing
  }

  #[test]
  fn test_render_properties_normal_area() {
    let props = make_test_props();
    let area = Rect::new(0, 0, 80, 30);
    let mut buf = Buffer::empty(area);
    render_properties(&props, area, &mut buf);
    // Should complete without crashing
  }

  #[test]
  fn test_render_properties_with_symlink() {
    let mut props = make_test_props();
    props.is_symlink = true;
    props.symlink_target = Some("/path/to/target".to_string());
    props.file_type = "Symbolic link".to_string();

    let area = Rect::new(0, 0, 80, 30);
    let mut buf = Buffer::empty(area);
    render_properties(&props, area, &mut buf);
    // Should complete without crashing and include target line
  }

  #[test]
  fn test_render_properties_directory() {
    let mut props = make_test_props();
    props.is_dir = true;
    props.file_type = "Directory".to_string();
    props.mime_type = None;

    let area = Rect::new(0, 0, 80, 30);
    let mut buf = Buffer::empty(area);
    render_properties(&props, area, &mut buf);
    // Should complete without crashing, size line should be skipped
  }
}
