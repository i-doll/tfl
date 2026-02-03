use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};

use crate::app::App;
use crate::event::InputMode;
use crate::preview::directory::format_size;

pub fn render_status_bar(app: &App, area: Rect, buf: &mut Buffer) {
  let line = match app.input_mode {
    InputMode::Search => {
      Line::from(vec![
        Span::styled(" /", Style::default().fg(Color::Indexed(75)).add_modifier(Modifier::BOLD)),
        Span::styled(&app.search_query, Style::default().fg(Color::Indexed(252))),
        Span::styled("▌", Style::default().fg(Color::Indexed(75))),
      ])
    }
    InputMode::GPrefix => {
      Line::from(vec![
        Span::styled(" g", Style::default().fg(Color::Indexed(208)).add_modifier(Modifier::BOLD)),
        Span::styled(" (press g for top)", Style::default().fg(Color::DarkGray)),
      ])
    }
    InputMode::Normal => {
      if let Some(ref msg) = app.status_message {
        Line::from(vec![
          Span::styled(format!(" {msg}"), Style::default().fg(Color::Indexed(150))),
        ])
      } else if let Some(entry) = app.selected_entry() {
        let mut spans = vec![
          Span::styled(
            format!(" {}", entry.name),
            Style::default().fg(Color::Indexed(252)).add_modifier(Modifier::BOLD),
          ),
          Span::styled(
            format!(" | {}", format_size(entry.size)),
            Style::default().fg(Color::DarkGray),
          ),
        ];

        if let Some(content) = app.preview.get_content() {
          if content.file_size > 0 && content.file_size != entry.size {
            spans.push(Span::styled(
              format!(" ({})", format_size(content.file_size)),
              Style::default().fg(Color::DarkGray),
            ));
          }
          if !content.extension.is_empty() {
            spans.push(Span::styled(
              format!(" | {}", content.extension),
              Style::default().fg(Color::DarkGray),
            ));
          }
          if content.line_count > 0 {
            spans.push(Span::styled(
              format!(" | {} lines", content.line_count),
              Style::default().fg(Color::DarkGray),
            ));
          }
        }

        // Position info on the right
        let pos_info = format!(
          " {}/{} ",
          app.cursor + 1,
          app.visible_entries().len()
        );
        spans.push(Span::styled(pos_info, Style::default().fg(Color::DarkGray)));

        Line::from(spans)
      } else {
        Line::from(vec![
          Span::styled(" No selection", Style::default().fg(Color::DarkGray)),
        ])
      }
    }
  };

  let paragraph = Paragraph::new(line)
    .style(Style::default().bg(Color::Indexed(236)));
  paragraph.render(area, buf);
}
