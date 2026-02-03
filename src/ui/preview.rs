use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph, StatefulWidget, Widget};
use ratatui_image::StatefulImage;
use ratatui_image::protocol::StatefulProtocol;

use crate::app::App;
use crate::preview::PreviewType;

pub fn render_preview(app: &mut App, area: Rect, buf: &mut Buffer) {
  let block = Block::default()
    .borders(Borders::ALL)
    .border_style(Style::default().fg(Color::Indexed(240)))
    .title(" Preview ")
    .title_style(Style::default().fg(Color::Indexed(75)));

  let inner = block.inner(area);
  block.render(area, buf);

  // Check if we have an image to render
  if let Some(content) = app.preview.get_content()
    && content.preview_type == PreviewType::Image
      && let Some(ref mut protocol) = app.preview.image_protocol {
        let image: StatefulImage<StatefulProtocol> = StatefulImage::default();
        StatefulWidget::render(image, inner, buf, protocol);
        return;
      }

  // Text-based preview
  let lines: Vec<Line> = if let Some(content) = app.preview.get_content() {
    let scroll = app.preview.scroll_offset;
    content
      .lines
      .iter()
      .skip(scroll)
      .take(inner.height as usize)
      .cloned()
      .collect()
  } else {
    vec![Line::from("  No file selected")]
  };

  let paragraph = Paragraph::new(lines);
  paragraph.render(inner, buf);
}
