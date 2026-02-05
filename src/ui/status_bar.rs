use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};

use crate::app::App;
use crate::date_filter::TimeType;
use crate::event::{InputMode, PromptKind};
use crate::fs::{GitFileStatus, GitStatus};
use crate::preview::directory::format_size;

fn git_status_label(status: &GitStatus) -> Option<&'static str> {
  if status.is_clean() {
    return None;
  }
  if status.staged == Some(GitFileStatus::Conflicted)
    || status.unstaged == Some(GitFileStatus::Conflicted)
  {
    return Some("conflicted");
  }
  if status.unstaged == Some(GitFileStatus::Untracked) {
    return Some("untracked");
  }
  if status.staged.is_some() && status.unstaged.is_some() {
    return Some("modified+staged");
  }
  if status.unstaged == Some(GitFileStatus::Modified) {
    return Some("modified");
  }
  if status.unstaged == Some(GitFileStatus::Deleted) {
    return Some("deleted");
  }
  match status.staged {
    Some(GitFileStatus::Added) => Some("staged"),
    Some(GitFileStatus::Modified) => Some("staged"),
    Some(GitFileStatus::Deleted) => Some("staged:deleted"),
    Some(GitFileStatus::Renamed) => Some("renamed"),
    _ => Some("changed"),
  }
}

fn prompt_input_spans(input: &str, cursor: usize, cursor_color: Color) -> Vec<Span<'static>> {
  let text_style = Style::default().fg(Color::Indexed(252));
  let cursor_style = Style::default().fg(Color::Indexed(234)).bg(cursor_color);

  let char_count = input.chars().count();
  let byte_at = |pos: usize| -> usize {
    input.char_indices().nth(pos).map(|(i, _)| i).unwrap_or(input.len())
  };

  let before = &input[..byte_at(cursor)];
  if cursor < char_count {
    let cur_start = byte_at(cursor);
    let cur_end = byte_at(cursor + 1);
    let cur_ch = &input[cur_start..cur_end];
    let after = &input[cur_end..];
    vec![
      Span::styled(before.to_string(), text_style),
      Span::styled(cur_ch.to_string(), cursor_style),
      Span::styled(after.to_string(), text_style),
    ]
  } else {
    vec![
      Span::styled(before.to_string(), text_style),
      Span::styled(" ".to_string(), cursor_style),
    ]
  }
}

fn time_type_label(tt: TimeType) -> &'static str {
  match tt {
    TimeType::Modified => "mod",
    TimeType::Created => "cre",
    TimeType::Accessed => "acc",
  }
}

pub fn render_status_bar(app: &App, area: Rect, buf: &mut Buffer) {
  let line = match app.input_mode {
    InputMode::Search => {
      Line::from(vec![
        Span::styled(" /", Style::default().fg(Color::Indexed(75)).add_modifier(Modifier::BOLD)),
        Span::styled(&app.search_query, Style::default().fg(Color::Indexed(252))),
        Span::styled("▌", Style::default().fg(Color::Indexed(75))),
      ])
    }
    InputMode::DateFilter => {
      let tt_label = time_type_label(app.date_filter_time_type);
      let valid = app.date_filter.is_some() || app.date_filter_query.is_empty();
      let query_color = if valid { Color::Indexed(252) } else { Color::Indexed(167) };
      Line::from(vec![
        Span::styled(
          format!(" d[{tt_label}]:"),
          Style::default().fg(Color::Indexed(178)).add_modifier(Modifier::BOLD),
        ),
        Span::styled(&app.date_filter_query, Style::default().fg(query_color)),
        Span::styled("▌", Style::default().fg(Color::Indexed(178))),
        Span::styled(" (Tab:type)", Style::default().fg(Color::DarkGray)),
      ])
    }
    InputMode::GPrefix => {
      Line::from(vec![
        Span::styled(" g", Style::default().fg(Color::Indexed(208)).add_modifier(Modifier::BOLD)),
        Span::styled(" (press g for top)", Style::default().fg(Color::DarkGray)),
      ])
    }
    InputMode::Help => {
      Line::from(vec![
        Span::styled(" ? ", Style::default().fg(Color::Indexed(75)).add_modifier(Modifier::BOLD)),
        Span::styled("Help — press ? or Esc to close", Style::default().fg(Color::DarkGray)),
      ])
    }
    InputMode::Prompt => {
      match app.prompt_kind {
        Some(PromptKind::Rename) => {
          let mut spans = vec![
            Span::styled(" Rename: ", Style::default().fg(Color::Indexed(208)).add_modifier(Modifier::BOLD)),
          ];
          spans.extend(prompt_input_spans(&app.prompt_input, app.prompt_cursor, Color::Indexed(208)));
          Line::from(spans)
        }
        Some(PromptKind::NewFile) => {
          let mut spans = vec![
            Span::styled(" New file: ", Style::default().fg(Color::Indexed(114)).add_modifier(Modifier::BOLD)),
          ];
          spans.extend(prompt_input_spans(&app.prompt_input, app.prompt_cursor, Color::Indexed(114)));
          Line::from(spans)
        }
        Some(PromptKind::NewDir) => {
          let mut spans = vec![
            Span::styled(" New dir: ", Style::default().fg(Color::Indexed(114)).add_modifier(Modifier::BOLD)),
          ];
          spans.extend(prompt_input_spans(&app.prompt_input, app.prompt_cursor, Color::Indexed(114)));
          Line::from(spans)
        }
        Some(PromptKind::ConfirmDelete) => {
          let name = app.selected_entry().map(|e| e.name.as_str()).unwrap_or("?");
          Line::from(vec![
            Span::styled(
              format!(" Delete {name}? (y/N)"),
              Style::default().fg(Color::Indexed(167)).add_modifier(Modifier::BOLD),
            ),
          ])
        }
        None => {
          Line::from(vec![
            Span::styled(" ...", Style::default().fg(Color::DarkGray)),
          ])
        }
      }
    }
    InputMode::Favorites => {
      Line::from(vec![
        Span::styled(" Favorites ", Style::default().fg(Color::Indexed(75)).add_modifier(Modifier::BOLD)),
        Span::styled("a:add  d:remove  Enter:go  Esc:close", Style::default().fg(Color::DarkGray)),
      ])
    }
    InputMode::OpenWith => {
      Line::from(vec![
        Span::styled(" Open with ", Style::default().fg(Color::Indexed(75)).add_modifier(Modifier::BOLD)),
        Span::styled("Enter:open  Esc:close", Style::default().fg(Color::DarkGray)),
      ])
    }
    InputMode::Error => {
      Line::from(vec![
        Span::styled(" Error ", Style::default().fg(Color::Indexed(167)).add_modifier(Modifier::BOLD)),
        Span::styled("Esc:dismiss", Style::default().fg(Color::DarkGray)),
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

        // Per-file git status label
        if let Some(label) = git_status_label(&entry.git_status) {
          let color = entry.git_status.display_color().unwrap_or(Color::DarkGray);
          spans.push(Span::styled(
            format!(" [{label}]"),
            Style::default().fg(color),
          ));
        }

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

        // Git summary stats
        let info = &app.tree.git_info;
        let has_git_stats = info.staged_count > 0
          || info.modified_count > 0
          || info.untracked_count > 0;
        if has_git_stats {
          spans.push(Span::styled("  ", Style::default().fg(Color::DarkGray)));
          if info.staged_count > 0 {
            spans.push(Span::styled(
              format!("+{}", info.staged_count),
              Style::default().fg(Color::Indexed(114)),
            ));
            spans.push(Span::styled(" ", Style::default()));
          }
          if info.modified_count > 0 {
            spans.push(Span::styled(
              format!("~{}", info.modified_count),
              Style::default().fg(Color::Indexed(214)),
            ));
            spans.push(Span::styled(" ", Style::default()));
          }
          if info.untracked_count > 0 {
            spans.push(Span::styled(
              format!("?{}", info.untracked_count),
              Style::default().fg(Color::Indexed(167)),
            ));
          }
        }

        let has_upstream = info.branch.is_some() && (info.ahead > 0 || info.behind > 0);
        if has_upstream {
          spans.push(Span::styled(
            format!(" \u{2191}{}\u{2193}{}", info.ahead, info.behind),
            Style::default().fg(Color::Indexed(75)),
          ));
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
