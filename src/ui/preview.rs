use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, StatefulWidget, Widget};
use ratatui_image::StatefulImage;
use ratatui_image::protocol::StatefulProtocol;

use crate::app::App;
use crate::preview::metadata::{format_permissions, format_size, format_time};
use crate::preview::{PreviewContent, PreviewType};

const METADATA_PANEL_HEIGHT: u16 = 7;

pub fn render_preview(app: &mut App, area: Rect, buf: &mut Buffer) {
  let blame_enabled = app.preview.blame_enabled;
  let title = if blame_enabled { " Blame " } else { " Preview " };

  let block = Block::default()
    .borders(Borders::ALL)
    .border_style(Style::default().fg(Color::Indexed(240)))
    .title(title)
    .title_style(Style::default().fg(if blame_enabled { Color::Indexed(214) } else { Color::Indexed(75) }));

  let inner = block.inner(area);
  block.render(area, buf);

  let content = app.preview.get_content();

  // In blame mode, don't show metadata panel
  let has_metadata = !blame_enabled && content.is_some_and(|c| c.metadata.is_some() || !c.git_commits.is_empty());

  // Split area: content at top, metadata panel at bottom
  let (content_area, metadata_area) = if has_metadata && inner.height > METADATA_PANEL_HEIGHT + 3 {
    let chunks = Layout::vertical([
      Constraint::Min(3),
      Constraint::Length(METADATA_PANEL_HEIGHT),
    ])
    .split(inner);
    (chunks[0], Some(chunks[1]))
  } else {
    (inner, None)
  };

  // Check if we have an image to render
  let is_image = app.preview.get_content().is_some_and(|c| c.preview_type == PreviewType::Image);
  if is_image && !blame_enabled {
    if let Some(ref mut protocol) = app.preview.image_protocol {
      let image: StatefulImage<StatefulProtocol> = StatefulImage::default();
      StatefulWidget::render(image, content_area, buf, protocol);
    }

    if let Some(meta_area) = metadata_area
      && let Some(content) = app.preview.get_content()
    {
      render_metadata_panel(content, meta_area, buf);
    }
    return;
  }

  // Check if blame view is enabled and we have blame data
  if blame_enabled
    && let Some(content) = app.preview.get_content()
  {
    if let Some(ref blame_data) = content.blame_data {
      let lines = blame_data.render(content_area.height as usize, app.preview.scroll_offset);
      let paragraph = Paragraph::new(lines);
      paragraph.render(content_area, buf);
      return;
    }
    // No blame data available - show message
    let msg = if content.preview_type == PreviewType::Text {
      " File not tracked by git"
    } else {
      " Blame not available for this file type"
    };
    let lines = vec![Line::from(msg)];
    let paragraph = Paragraph::new(lines);
    paragraph.render(content_area, buf);
    return;
  }

  // Text-based preview - use get_display_lines() for formatted/raw toggle
  let lines: Vec<Line> = if let Some(display_lines) = app.preview.get_display_lines() {
    let scroll = app.preview.scroll_offset;
    display_lines
      .iter()
      .skip(scroll)
      .take(content_area.height as usize)
      .cloned()
      .collect()
  } else {
    vec![Line::from("  No file selected")]
  };

  let paragraph = Paragraph::new(lines);
  paragraph.render(content_area, buf);

  // Render metadata panel
  if let Some(meta_area) = metadata_area
    && let Some(content) = app.preview.get_content()
  {
    render_metadata_panel(content, meta_area, buf);
  }
}

fn render_metadata_panel(content: &PreviewContent, area: Rect, buf: &mut Buffer) {
  // Draw separator line
  let sep_style = Style::default().fg(Color::Indexed(240));
  for x in area.x..area.x + area.width {
    buf[(x, area.y)].set_symbol("─").set_style(sep_style);
  }

  let inner = Rect {
    x: area.x,
    y: area.y + 1,
    width: area.width,
    height: area.height.saturating_sub(1),
  };

  let mut top_lines: Vec<Line> = Vec::new();
  let mut git_lines: Vec<Line> = Vec::new();

  // Prepare all column values first, then calculate widths for alignment
  let size_str = content.metadata.as_ref().map(|m| format_size(m.size));
  let owner_group = content.metadata.as_ref().and_then(|m| {
    m.owner.as_ref().map(|owner| {
      if let Some(ref group) = m.group {
        format!("{owner}:{group}")
      } else {
        owner.clone()
      }
    })
  });
  let mod_str = content
    .metadata
    .as_ref()
    .and_then(|m| m.modified.map(|t| format!("mod {}", format_time(t))));
  let created_str = content
    .metadata
    .as_ref()
    .and_then(|m| m.created.map(|t| format!("created {}", format_time(t))));
  let perms_str = content
    .metadata
    .as_ref()
    .and_then(|m| m.permissions.map(format_permissions));
  let lines_str = content
    .metadata
    .as_ref()
    .and_then(|m| m.line_count.map(|n| format!("{n} lines")));

  // Image metadata strings
  let img_dims_str = content
    .image_metadata
    .as_ref()
    .map(|m| format!("{}×{} {}", m.width, m.height, m.aspect_ratio));
  let img_camera_str = content
    .image_metadata
    .as_ref()
    .and_then(|m| m.exif.as_ref().and_then(|e| e.camera.clone()));
  let img_iso_exposure_str = content.image_metadata.as_ref().and_then(|m| {
    m.exif.as_ref().map(|e| {
      let mut s = String::new();
      if let Some(ref iso) = e.iso {
        s.push_str(iso);
      }
      if let Some(ref exp) = e.exposure {
        if !s.is_empty() {
          s.push(' ');
        }
        s.push_str(exp);
      }
      s
    })
  });

  // Calculate column widths including all rows
  let col1_width = [
    size_str.as_ref().map(|s| s.len()),
    owner_group.as_ref().map(|s| s.len()),
    img_dims_str.as_ref().map(|s| s.len()),
  ]
  .into_iter()
  .flatten()
  .max()
  .unwrap_or(0);

  let col2_width = [
    mod_str.as_ref().map(|s| s.len()),
    created_str.as_ref().map(|s| s.len()),
    img_camera_str.as_ref().map(|s| s.len()),
  ]
  .into_iter()
  .flatten()
  .max()
  .unwrap_or(0);

  let col3_width = [
    perms_str.as_ref().map(|s| s.len()),
    lines_str.as_ref().map(|s| s.len()),
    img_iso_exposure_str.as_ref().map(|s| s.len()),
  ]
  .into_iter()
  .flatten()
  .max()
  .unwrap_or(0);

  // Line 1: size │ mod time │ permissions
  if content.metadata.is_some() {
    let mut spans1: Vec<Span> = Vec::new();
    if let Some(ref size) = size_str {
      spans1.push(Span::styled(
        format!("{:width$}", size, width = col1_width),
        Style::default().fg(Color::Indexed(75)),
      ));
    }
    if let Some(ref mod_s) = mod_str {
      spans1.push(Span::raw(" │ "));
      spans1.push(Span::styled(
        format!("{:width$}", mod_s, width = col2_width),
        Style::default().fg(Color::Indexed(252)),
      ));
    }
    if let Some(ref perms) = perms_str {
      spans1.push(Span::raw(" │ "));
      spans1.push(Span::styled(
        format!("{:width$}", perms, width = col3_width),
        Style::default().fg(Color::Indexed(246)),
      ));
    }
    if !spans1.is_empty() {
      top_lines.push(Line::from(spans1));
    }

    // Line 2: owner:group │ created time │ line count
    if owner_group.is_some() || created_str.is_some() || lines_str.is_some() {
      let mut spans2: Vec<Span> = Vec::new();
      if let Some(ref og) = owner_group {
        spans2.push(Span::styled(
          format!("{:width$}", og, width = col1_width),
          Style::default().fg(Color::Indexed(246)),
        ));
      } else {
        spans2.push(Span::raw(format!("{:width$}", "", width = col1_width)));
      }
      if created_str.is_some() || lines_str.is_some() {
        spans2.push(Span::raw(" │ "));
        if let Some(ref created) = created_str {
          spans2.push(Span::styled(
            format!("{:width$}", created, width = col2_width),
            Style::default().fg(Color::Indexed(246)),
          ));
        } else {
          spans2.push(Span::raw(format!("{:width$}", "", width = col2_width)));
        }
      }
      if let Some(ref lines) = lines_str {
        spans2.push(Span::raw(" │ "));
        spans2.push(Span::styled(
          format!("{:width$}", lines, width = col3_width),
          Style::default().fg(Color::Indexed(114)),
        ));
      }
      top_lines.push(Line::from(spans2));
    }
  }

  // Line 3 (images): dimensions aspect │ camera │ ISO exposure
  if content.image_metadata.is_some() {
    let mut spans3: Vec<Span> = Vec::new();
    if let Some(ref dims) = img_dims_str {
      spans3.push(Span::styled(
        format!("{:width$}", dims, width = col1_width),
        Style::default().fg(Color::Indexed(75)),
      ));
    }
    if let Some(ref camera) = img_camera_str {
      spans3.push(Span::raw(" │ "));
      spans3.push(Span::styled(
        format!("{:width$}", camera, width = col2_width),
        Style::default().fg(Color::Indexed(252)),
      ));
    } else if img_iso_exposure_str.is_some() {
      spans3.push(Span::raw(" │ "));
      spans3.push(Span::raw(format!("{:width$}", "", width = col2_width)));
    }
    if let Some(ref iso_exp) = img_iso_exposure_str
      && !iso_exp.is_empty()
    {
      spans3.push(Span::raw(" │ "));
      spans3.push(Span::styled(
        format!("{:width$}", iso_exp, width = col3_width),
        Style::default().fg(Color::Indexed(214)),
      ));
    }
    if !spans3.is_empty() {
      top_lines.push(Line::from(spans3));
    }
  }

  // Git commits
  for commit in content.git_commits.iter().take(3) {
    let mut spans: Vec<Span> = Vec::new();

    // Hash (short)
    spans.push(Span::styled(
      &commit.hash,
      Style::default()
        .fg(Color::Indexed(214))
        .add_modifier(Modifier::BOLD),
    ));

    // Date
    spans.push(Span::raw(" "));
    spans.push(Span::styled(
      &commit.date,
      Style::default().fg(Color::Indexed(246)),
    ));

    // Message (truncated to fit)
    let used_width = commit.hash.len() + 1 + commit.date.len() + 2;
    let max_msg_width = area.width as usize - used_width.min(area.width as usize);
    let msg = if commit.message.len() > max_msg_width {
      format!("{}…", &commit.message[..max_msg_width.saturating_sub(1)])
    } else {
      commit.message.clone()
    };

    spans.push(Span::raw(" "));
    spans.push(Span::styled(msg, Style::default().fg(Color::Indexed(252))));

    git_lines.push(Line::from(spans));
  }

  // Render top metadata at top
  let top_paragraph = Paragraph::new(top_lines);
  top_paragraph.render(inner, buf);

  // Render git commits at bottom (always bottom-aligned)
  if !git_lines.is_empty() {
    let git_height = git_lines.len() as u16;
    let git_y = inner.y + inner.height.saturating_sub(git_height);
    let git_area = Rect {
      x: inner.x,
      y: git_y,
      width: inner.width,
      height: git_height,
    };
    let git_paragraph = Paragraph::new(git_lines);
    git_paragraph.render(git_area, buf);
  }
}
