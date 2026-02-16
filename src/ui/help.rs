use std::collections::HashMap;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Widget};

use crate::action::Action;
use crate::config::Config;
use crate::theme::Theme;

fn section_line(title: &str, section_style: Style) -> Line<'static> {
  Line::from(Span::styled(
    format!(" {title}"),
    section_style.add_modifier(Modifier::DIM),
  ))
}

fn entry_line(lookup: &HashMap<Action, Vec<String>>, action: Action, desc: &str, key_style: Style, desc_style: Style) -> Line<'static> {
  let keys = lookup
    .get(&action)
    .map(|v| v.join(" / "))
    .unwrap_or_else(|| "—".to_string());
  Line::from(vec![
    Span::styled(
      format!("  {keys:<16}"),
      key_style.add_modifier(Modifier::BOLD),
    ),
    Span::styled(desc.to_string(), desc_style),
  ])
}

pub fn render_help(config: &Config, area: Rect, buf: &mut Buffer, theme: &Theme) {
  let width = 44.min(area.width.saturating_sub(4));
  let height = 48.min(area.height.saturating_sub(2));

  if width < 10 || height < 5 {
    return;
  }

  let x = area.x + (area.width.saturating_sub(width)) / 2;
  let y = area.y + (area.height.saturating_sub(height)) / 2;
  let popup = Rect::new(x, y, width, height);

  Clear.render(popup, buf);

  let lookup = config.reverse_lookup();

  let key_style = Style::default().fg(theme.accent);
  let desc_style = Style::default().fg(theme.text);
  let section_style = Style::default().fg(theme.title_inactive);

  let lines: Vec<Line> = vec![
    section_line("Navigation", section_style),
    entry_line(&lookup, Action::MoveDown, "Move down", key_style, desc_style),
    entry_line(&lookup, Action::MoveUp, "Move up", key_style, desc_style),
    entry_line(&lookup, Action::MoveLeft, "Collapse / parent", key_style, desc_style),
    entry_line(&lookup, Action::MoveRight, "Expand / select", key_style, desc_style),
    entry_line(&lookup, Action::ToggleExpand, "Toggle expand", key_style, desc_style),
    entry_line(&lookup, Action::GoToTop, "Go to top", key_style, desc_style),
    entry_line(&lookup, Action::GoToBottom, "Go to bottom", key_style, desc_style),
    entry_line(&lookup, Action::GoHome, "Go to home", key_style, desc_style),
    entry_line(&lookup, Action::FavoritesOpen, "Open favorites", key_style, desc_style),
    entry_line(&lookup, Action::FavoriteAdd, "Add to favorites", key_style, desc_style),
    section_line("Search", section_style),
    entry_line(&lookup, Action::SearchStart, "Start search", key_style, desc_style),
    entry_line(&lookup, Action::SearchConfirm, "Confirm", key_style, desc_style),
    entry_line(&lookup, Action::SearchCancel, "Cancel", key_style, desc_style),
    section_line("Preview", section_style),
    entry_line(&lookup, Action::ScrollPreviewDown, "Scroll down", key_style, desc_style),
    entry_line(&lookup, Action::ScrollPreviewUp, "Scroll up", key_style, desc_style),
    entry_line(&lookup, Action::ShrinkTree, "Shrink tree pane", key_style, desc_style),
    entry_line(&lookup, Action::GrowTree, "Grow tree pane", key_style, desc_style),
    section_line("Actions", section_style),
    entry_line(&lookup, Action::OpenDefault, "Open file / enter dir", key_style, desc_style),
    entry_line(&lookup, Action::OpenWithStart, "Open with...", key_style, desc_style),
    entry_line(&lookup, Action::ShowProperties, "Show properties", key_style, desc_style),
    entry_line(&lookup, Action::OpenEditor, "Open in $EDITOR", key_style, desc_style),
    entry_line(&lookup, Action::OpenClaude, "Open Claude Code", key_style, desc_style),
    entry_line(&lookup, Action::OpenShell, "Open $SHELL", key_style, desc_style),
    entry_line(&lookup, Action::YankPath, "Yank path", key_style, desc_style),
    entry_line(&lookup, Action::ToggleHidden, "Toggle hidden files", key_style, desc_style),
    section_line("Marking", section_style),
    entry_line(&lookup, Action::ToggleMark, "Toggle mark", key_style, desc_style),
    entry_line(&lookup, Action::MarkAll, "Mark all", key_style, desc_style),
    entry_line(&lookup, Action::ClearMarks, "Clear marks", key_style, desc_style),
    section_line("File Operations", section_style),
    entry_line(&lookup, Action::RenameStart, "Rename", key_style, desc_style),
    entry_line(&lookup, Action::DeleteFile, "Delete", key_style, desc_style),
    entry_line(&lookup, Action::CopyFile, "Copy", key_style, desc_style),
    entry_line(&lookup, Action::CutFile, "Cut", key_style, desc_style),
    entry_line(&lookup, Action::Paste, "Paste", key_style, desc_style),
    entry_line(&lookup, Action::NewFileStart, "New file", key_style, desc_style),
    entry_line(&lookup, Action::NewDirStart, "New directory", key_style, desc_style),
    entry_line(&lookup, Action::CompressStart, "Compress to archive", key_style, desc_style),
    section_line("Quit", section_style),
    entry_line(&lookup, Action::Quit, "Quit", key_style, desc_style),
    Line::from(""),
    Line::from(Span::styled(
      " Press q, ? or Esc to close".to_string(),
      Style::default().fg(theme.text_muted),
    )),
  ];

  let block = Block::default()
    .borders(Borders::ALL)
    .title(" Help ")
    .border_style(Style::default().fg(theme.title_inactive))
    .style(Style::default().bg(theme.bg_overlay));

  let paragraph = Paragraph::new(lines).block(block);
  paragraph.render(popup, buf);
}
