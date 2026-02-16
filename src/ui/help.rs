use std::collections::HashMap;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Widget};

use crate::action::Action;
use crate::config::Config;

const KEY_STYLE: Style = Style::new().fg(Color::Indexed(75));
const DESC_STYLE: Style = Style::new().fg(Color::Indexed(252));
const SECTION_STYLE: Style = Style::new().fg(Color::Indexed(245));

fn section_line(title: &str) -> Line<'static> {
  Line::from(Span::styled(
    format!(" {title}"),
    SECTION_STYLE.add_modifier(Modifier::DIM),
  ))
}

fn entry_line(lookup: &HashMap<Action, Vec<String>>, action: Action, desc: &str) -> Line<'static> {
  let keys = lookup
    .get(&action)
    .map(|v| v.join(" / "))
    .unwrap_or_else(|| "—".to_string());
  Line::from(vec![
    Span::styled(
      format!("  {keys:<16}"),
      KEY_STYLE.add_modifier(Modifier::BOLD),
    ),
    Span::styled(desc.to_string(), DESC_STYLE),
  ])
}

fn hardcoded_line(key: &str, desc: &str) -> Line<'static> {
  Line::from(vec![
    Span::styled(
      format!("  {key:<16}"),
      KEY_STYLE.add_modifier(Modifier::BOLD),
    ),
    Span::styled(desc.to_string(), DESC_STYLE),
  ])
}

pub fn render_help(config: &Config, area: Rect, buf: &mut Buffer) {
  let width = 44.min(area.width.saturating_sub(4));
  let height = 42.min(area.height.saturating_sub(2));

  if width < 10 || height < 5 {
    return;
  }

  let x = area.x + (area.width.saturating_sub(width)) / 2;
  let y = area.y + (area.height.saturating_sub(height)) / 2;
  let popup = Rect::new(x, y, width, height);

  Clear.render(popup, buf);

  let lookup = config.reverse_lookup();

  let lines: Vec<Line> = vec![
    section_line("Navigation"),
    entry_line(&lookup, Action::MoveDown, "Move down"),
    entry_line(&lookup, Action::MoveUp, "Move up"),
    entry_line(&lookup, Action::MoveLeft, "Collapse / parent"),
    entry_line(&lookup, Action::MoveRight, "Expand / select"),
    entry_line(&lookup, Action::ToggleExpand, "Toggle expand"),
    entry_line(&lookup, Action::EnterDir, "Enter directory"),
    entry_line(&lookup, Action::GoToTop, "Go to top"),
    entry_line(&lookup, Action::GoToBottom, "Go to bottom"),
    entry_line(&lookup, Action::GoHome, "Go to home"),
    entry_line(&lookup, Action::FavoritesOpen, "Open favorites"),
    entry_line(&lookup, Action::FavoriteAdd, "Add to favorites"),
    section_line("Search"),
    entry_line(&lookup, Action::SearchStart, "Start search"),
    hardcoded_line("Enter", "Confirm"),
    hardcoded_line("Esc", "Cancel"),
    section_line("Preview"),
    entry_line(&lookup, Action::ScrollPreviewDown, "Scroll down"),
    entry_line(&lookup, Action::ScrollPreviewUp, "Scroll up"),
    entry_line(&lookup, Action::ShrinkTree, "Shrink tree pane"),
    entry_line(&lookup, Action::GrowTree, "Grow tree pane"),
    section_line("Actions"),
    entry_line(&lookup, Action::OpenDefault, "Open file / enter dir"),
    entry_line(&lookup, Action::OpenWithStart, "Open with..."),
    entry_line(&lookup, Action::ShowProperties, "Show properties"),
    entry_line(&lookup, Action::OpenEditor, "Open in $EDITOR"),
    entry_line(&lookup, Action::OpenClaude, "Open Claude Code"),
    entry_line(&lookup, Action::OpenShell, "Open $SHELL"),
    entry_line(&lookup, Action::YankPath, "Yank path"),
    entry_line(&lookup, Action::ToggleHidden, "Toggle hidden files"),
    section_line("File Operations"),
    entry_line(&lookup, Action::RenameStart, "Rename"),
    entry_line(&lookup, Action::DeleteFile, "Delete"),
    entry_line(&lookup, Action::CopyFile, "Copy"),
    entry_line(&lookup, Action::CutFile, "Cut"),
    entry_line(&lookup, Action::Paste, "Paste"),
    entry_line(&lookup, Action::NewFileStart, "New file"),
    entry_line(&lookup, Action::NewDirStart, "New directory"),
    section_line("Quit"),
    entry_line(&lookup, Action::Quit, "Quit"),
    Line::from(""),
    Line::from(Span::styled(
      " Press q, ? or Esc to close".to_string(),
      Style::default().fg(Color::Indexed(241)),
    )),
  ];

  let block = Block::default()
    .borders(Borders::ALL)
    .title(" Help ")
    .border_style(Style::default().fg(Color::Indexed(245)))
    .style(Style::default().bg(Color::Indexed(235)));

  let paragraph = Paragraph::new(lines).block(block);
  paragraph.render(popup, buf);
}
