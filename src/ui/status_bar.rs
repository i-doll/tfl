use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};

use crate::app::App;
use crate::event::{InputMode, PromptKind};
use crate::fs::{GitFileStatus, GitStatus};
use crate::preview::directory::format_size;
use crate::theme::Theme;

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

fn prompt_input_spans(input: &str, cursor: usize, cursor_color: Color, theme: &Theme) -> Vec<Span<'static>> {
  let text_style = Style::default().fg(theme.text);
  let cursor_style = Style::default().fg(theme.bg_selected).bg(cursor_color);

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

pub fn render_status_bar(app: &App, area: Rect, buf: &mut Buffer, theme: &Theme) {
  let line = match app.input_mode {
    InputMode::Search => {
      Line::from(vec![
        Span::styled(" /", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
        Span::styled(&app.search_query, Style::default().fg(theme.text)),
        Span::styled("▌", Style::default().fg(theme.accent)),
      ])
    }
    InputMode::GPrefix => {
      Line::from(vec![
        Span::styled(" g", Style::default().fg(theme.marked).add_modifier(Modifier::BOLD)),
        Span::styled(" (press g for top)", Style::default().fg(theme.text_dim)),
      ])
    }
    InputMode::Help => {
      Line::from(vec![
        Span::styled(" ? ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
        Span::styled("Help — press q, ? or Esc to close", Style::default().fg(theme.text_dim)),
      ])
    }
    InputMode::Prompt => {
      match app.prompt_kind {
        Some(PromptKind::Rename) => {
          let mut spans = vec![
            Span::styled(" Rename: ", Style::default().fg(theme.marked).add_modifier(Modifier::BOLD)),
          ];
          spans.extend(prompt_input_spans(&app.prompt_input, app.prompt_cursor, theme.marked, theme));
          Line::from(spans)
        }
        Some(PromptKind::NewFile) => {
          let mut spans = vec![
            Span::styled(" New file: ", Style::default().fg(theme.success).add_modifier(Modifier::BOLD)),
          ];
          spans.extend(prompt_input_spans(&app.prompt_input, app.prompt_cursor, theme.success, theme));
          Line::from(spans)
        }
        Some(PromptKind::NewDir) => {
          let mut spans = vec![
            Span::styled(" New dir: ", Style::default().fg(theme.success).add_modifier(Modifier::BOLD)),
          ];
          spans.extend(prompt_input_spans(&app.prompt_input, app.prompt_cursor, theme.success, theme));
          Line::from(spans)
        }
        Some(PromptKind::ConfirmDelete) => {
          let name = app.selected_entry().map(|e| e.name.as_str()).unwrap_or("?");
          Line::from(vec![
            Span::styled(
              format!(" Delete {name}? (y/N)"),
              Style::default().fg(theme.error).add_modifier(Modifier::BOLD),
            ),
          ])
        }
        Some(PromptKind::ConfirmDeleteMulti(count)) => {
          Line::from(vec![
            Span::styled(
              format!(" Delete {count} items? (y/N)"),
              Style::default().fg(theme.error).add_modifier(Modifier::BOLD),
            ),
          ])
        }
        Some(PromptKind::ConfirmExtractAndDelete) => {
          let name = app.selected_entry().map(|e| e.name.as_str()).unwrap_or("?");
          Line::from(vec![
            Span::styled(
              format!(" Extract and delete {name}? (y/N)"),
              Style::default().fg(theme.marked).add_modifier(Modifier::BOLD),
            ),
          ])
        }
        None => {
          Line::from(vec![
            Span::styled(" ...", Style::default().fg(theme.text_dim)),
          ])
        }
      }
    }
    InputMode::Favorites => {
      Line::from(vec![
        Span::styled(" Favorites ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
        Span::styled("a:add  d:remove  Enter:go  Esc:close", Style::default().fg(theme.text_dim)),
      ])
    }
    InputMode::OpenWith => {
      Line::from(vec![
        Span::styled(" Open with ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
        Span::styled("Enter:open  Esc:close", Style::default().fg(theme.text_dim)),
      ])
    }
    InputMode::Chmod => {
      Line::from(vec![
        Span::styled(" Chmod ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
        Span::styled("r/w/x:toggle  Tab:octal  Enter:apply  Esc:cancel", Style::default().fg(theme.text_dim)),
      ])
    }
    InputMode::Properties => {
      Line::from(vec![
        Span::styled(" Properties ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
        Span::styled("i/q/Esc:close", Style::default().fg(theme.text_dim)),
      ])
    }
    InputMode::Compress => {
      Line::from(vec![
        Span::styled(" Compress ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
        Span::styled("1-4:format  Esc:cancel", Style::default().fg(theme.text_dim)),
      ])
    }
    InputMode::Error => {
      Line::from(vec![
        Span::styled(" Error ", Style::default().fg(theme.error).add_modifier(Modifier::BOLD)),
        Span::styled("Esc:dismiss", Style::default().fg(theme.text_dim)),
      ])
    }
    InputMode::Normal => {
      let mut badges: Vec<Span<'static>> = Vec::new();

      if app.picker_mode.is_some() {
        badges.push(Span::styled(
          " PICK ",
          Style::default()
            .fg(theme.bg_selected)
            .bg(theme.marked)
            .add_modifier(Modifier::BOLD),
        ));
      }

      let mark_count = app.active_marks().len();
      if mark_count > 0 {
        badges.push(Span::styled(
          format!(" {mark_count} marked "),
          Style::default()
            .fg(theme.bg_selected)
            .bg(theme.marked)
            .add_modifier(Modifier::BOLD),
        ));
      }

      if let Some(ref msg) = app.status_message {
        let mut spans = badges;
        spans.push(Span::styled(format!(" {msg}"), Style::default().fg(theme.info)));
        Line::from(spans)
      } else if let Some(entry) = app.selected_entry() {
        let mut spans = badges;
        spans.push(Span::styled(
            format!(" {}", entry.name),
            Style::default().fg(theme.text).add_modifier(Modifier::BOLD),
          ));
        spans.push(Span::styled(
            format!(" | {}", format_size(entry.size)),
            Style::default().fg(theme.text_dim),
          ));

        // Per-file git status label
        if let Some(label) = git_status_label(&entry.git_status) {
          let color = entry.git_status.display_color(theme).unwrap_or(theme.text_dim);
          spans.push(Span::styled(
            format!(" [{label}]"),
            Style::default().fg(color),
          ));
        }

        if let Some(content) = app.preview.get_content() {
          if content.file_size > 0 && content.file_size != entry.size {
            spans.push(Span::styled(
              format!(" ({})", format_size(content.file_size)),
              Style::default().fg(theme.text_dim),
            ));
          }
          if !content.extension.is_empty() {
            spans.push(Span::styled(
              format!(" | {}", content.extension),
              Style::default().fg(theme.text_dim),
            ));
          }
          if content.line_count > 0 {
            spans.push(Span::styled(
              format!(" | {} lines", content.line_count),
              Style::default().fg(theme.text_dim),
            ));
          }
        }

        // Git summary stats
        let info = &app.tree.git_info;
        let has_git_stats = info.staged_count > 0
          || info.modified_count > 0
          || info.untracked_count > 0;
        if has_git_stats {
          spans.push(Span::styled("  ", Style::default().fg(theme.text_dim)));
          if info.staged_count > 0 {
            spans.push(Span::styled(
              format!("+{}", info.staged_count),
              Style::default().fg(theme.success),
            ));
            spans.push(Span::styled(" ", Style::default()));
          }
          if info.modified_count > 0 {
            spans.push(Span::styled(
              format!("~{}", info.modified_count),
              Style::default().fg(theme.warning),
            ));
            spans.push(Span::styled(" ", Style::default()));
          }
          if info.untracked_count > 0 {
            spans.push(Span::styled(
              format!("?{}", info.untracked_count),
              Style::default().fg(theme.error),
            ));
          }
        }

        let has_upstream = info.branch.is_some() && (info.ahead > 0 || info.behind > 0);
        if has_upstream {
          spans.push(Span::styled(
            format!(" \u{2191}{}\u{2193}{}", info.ahead, info.behind),
            Style::default().fg(theme.accent),
          ));
        }

        // Position info on the right
        let pos_info = format!(
          " {}/{} ",
          app.cursor + 1,
          app.visible_entries().len()
        );
        spans.push(Span::styled(pos_info, Style::default().fg(theme.text_dim)));

        Line::from(spans)
      } else {
        let mut spans = badges;
        spans.push(Span::styled(" No selection", Style::default().fg(theme.text_dim)));
        Line::from(spans)
      }
    }
  };

  let paragraph = Paragraph::new(line)
    .style(Style::default().bg(theme.bg_bar));
  paragraph.render(area, buf);
}
