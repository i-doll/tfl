use std::collections::HashMap;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Widget};

use crate::action::Action;
use crate::app::App;
use crate::config::Config;
use crate::theme::Theme;

/// A group of help lines (header + entries) that stays together in a column.
struct Section {
  lines: Vec<Line<'static>>,
}

impl Section {
  fn height(&self) -> usize {
    self.lines.len()
  }
}

const COL_WIDTH: u16 = 40;
const COL_GAP: u16 = 2;
const FOOTER_LINES: u16 = 2; // blank + "Press q…"

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

fn build_sections(lookup: &HashMap<Action, Vec<String>>, key_style: Style, desc_style: Style, section_style: Style) -> Vec<Section> {
  let s = |title| section_line(title, section_style);
  let e = |action, desc| entry_line(lookup, action, desc, key_style, desc_style);

  vec![
    Section {
      lines: vec![
        s("Navigation"),
        e(Action::MoveDown, "Move down"),
        e(Action::MoveUp, "Move up"),
        e(Action::MoveLeft, "Collapse / parent"),
        e(Action::MoveRight, "Expand / select"),
        e(Action::ToggleExpand, "Toggle expand"),
        e(Action::GoToTop, "Go to top"),
        e(Action::GoToBottom, "Go to bottom"),
        e(Action::GoHome, "Go to home"),
        e(Action::FavoritesOpen, "Open favorites"),
        e(Action::FavoriteAdd, "Add to favorites"),
      ],
    },
    Section {
      lines: vec![
        s("Search"),
        e(Action::SearchStart, "Start search"),
        e(Action::SearchConfirm, "Confirm"),
        e(Action::SearchCancel, "Cancel"),
      ],
    },
    Section {
      lines: vec![
        s("Preview"),
        e(Action::ScrollPreviewDown, "Scroll down"),
        e(Action::ScrollPreviewUp, "Scroll up"),
        e(Action::ShrinkTree, "Shrink tree pane"),
        e(Action::GrowTree, "Grow tree pane"),
      ],
    },
    Section {
      lines: vec![
        s("Actions"),
        e(Action::OpenDefault, "Open file / enter dir"),
        e(Action::OpenWithStart, "Open with..."),
        e(Action::ShowProperties, "Show properties"),
        e(Action::OpenEditor, "Open in $EDITOR"),
        e(Action::OpenClaude, "Open Claude Code"),
        e(Action::OpenShell, "Open $SHELL"),
        e(Action::YankPath, "Yank path"),
        e(Action::ToggleHidden, "Toggle hidden files"),
      ],
    },
    Section {
      lines: vec![
        s("Marking"),
        e(Action::ToggleMark, "Toggle mark"),
        e(Action::MarkAll, "Mark all"),
        e(Action::ClearMarks, "Clear marks"),
      ],
    },
    Section {
      lines: vec![
        s("File Operations"),
        e(Action::RenameStart, "Rename"),
        e(Action::DeleteFile, "Delete"),
        e(Action::CopyFile, "Copy"),
        e(Action::CutFile, "Cut"),
        e(Action::Paste, "Paste"),
        e(Action::NewFileStart, "New file"),
        e(Action::NewDirStart, "New directory"),
        e(Action::CompressStart, "Compress to archive"),
      ],
    },
    Section {
      lines: vec![
        s("Quit"),
        e(Action::Quit, "Quit"),
      ],
    },
  ]
}

/// Determine column count from available inner width.
fn column_count(inner_width: u16) -> u16 {
  if inner_width >= 3 * COL_WIDTH + 2 * COL_GAP {
    3
  } else if inner_width >= 2 * COL_WIDTH + COL_GAP {
    2
  } else {
    1
  }
}

/// Distribute sections across `n` columns using greedy shortest-column.
/// Returns a Vec of columns, each column being a Vec of lines.
fn distribute_sections(sections: Vec<Section>, n: usize) -> Vec<Vec<Line<'static>>> {
  let mut col_heights = vec![0usize; n];
  let mut columns: Vec<Vec<Line<'static>>> = vec![Vec::new(); n];

  for section in sections {
    // Find the shortest column
    let target = col_heights
      .iter()
      .enumerate()
      .min_by_key(|(_, h)| **h)
      .map(|(i, _)| i)
      .unwrap_or(0);

    let h = section.height();
    col_heights[target] += h;
    columns[target].extend(section.lines);
  }

  columns
}

pub fn render_help(app: &mut App, config: &Config, area: Rect, buf: &mut Buffer, theme: &Theme) {
  if area.width < 14 || area.height < 7 {
    return;
  }

  let lookup = config.reverse_lookup();
  let key_style = Style::default().fg(theme.accent);
  let desc_style = Style::default().fg(theme.text);
  let section_style = Style::default().fg(theme.title_inactive);

  let sections = build_sections(&lookup, key_style, desc_style, section_style);

  // Determine column layout based on terminal width
  // Inner width = area width - 4 (margin) - 2 (borders)
  let max_inner = area.width.saturating_sub(6);
  let cols = column_count(max_inner);

  let popup_inner_w = if cols == 1 {
    COL_WIDTH
  } else {
    cols * COL_WIDTH + (cols - 1) * COL_GAP
  };
  let popup_w = (popup_inner_w + 2).min(area.width.saturating_sub(2)); // +2 for borders

  // Distribute sections into columns
  let columns = distribute_sections(sections, cols as usize);

  // Calculate tallest column height
  let tallest: u16 = columns.iter().map(|c| c.len() as u16).max().unwrap_or(0);

  // Total content height = tallest column + footer
  let content_h = tallest + FOOTER_LINES;
  // Available inner height (borders top/bottom = 2)
  let max_inner_h = area.height.saturating_sub(4); // 2 border + 2 margin
  let inner_h = content_h.min(max_inner_h);
  let popup_h = inner_h + 2; // +2 for borders

  let x = area.x + (area.width.saturating_sub(popup_w)) / 2;
  let y = area.y + (area.height.saturating_sub(popup_h)) / 2;
  let popup = Rect::new(x, y, popup_w, popup_h);

  Clear.render(popup, buf);

  // Render outer block
  let block = Block::default()
    .borders(Borders::ALL)
    .title(" Help ")
    .border_style(Style::default().fg(theme.title_inactive))
    .style(Style::default().bg(theme.bg_overlay));
  block.render(popup, buf);

  // Inner area (inside borders)
  let inner = Rect::new(popup.x + 1, popup.y + 1, popup_w.saturating_sub(2), popup_h.saturating_sub(2));

  // Space for columns (above footer)
  let col_area_h = inner.height.saturating_sub(FOOTER_LINES);
  let needs_scroll = tallest > col_area_h;

  // Clamp scroll offset
  let max_scroll = if needs_scroll { tallest.saturating_sub(col_area_h) as usize } else { 0 };
  app.help_scroll = app.help_scroll.min(max_scroll);
  let scroll_offset = app.help_scroll as u16;

  // Render each column
  for (i, col_lines) in columns.into_iter().enumerate() {
    let col_x = inner.x + (i as u16) * (COL_WIDTH + COL_GAP);
    let col_w = COL_WIDTH.min(inner.x + inner.width - col_x);
    let col_rect = Rect::new(col_x, inner.y, col_w, col_area_h);

    let paragraph = Paragraph::new(col_lines).scroll((scroll_offset, 0));
    paragraph.render(col_rect, buf);
  }

  // Render scroll indicators
  if needs_scroll {
    let indicator_x = inner.x + inner.width.saturating_sub(1);
    if scroll_offset > 0 {
      // Up indicator
      buf[(indicator_x, inner.y)]
        .set_symbol("▲")
        .set_style(Style::default().fg(theme.text_muted));
    }
    if (scroll_offset as usize) < max_scroll {
      // Down indicator
      let indicator_y = inner.y + col_area_h.saturating_sub(1);
      buf[(indicator_x, indicator_y)]
        .set_symbol("▼")
        .set_style(Style::default().fg(theme.text_muted));
    }
  }

  // Render footer at bottom of inner area
  let footer_y = inner.y + inner.height.saturating_sub(FOOTER_LINES);
  let footer_rect = Rect::new(inner.x, footer_y, inner.width, FOOTER_LINES);
  let footer = Paragraph::new(vec![
    Line::from(""),
    Line::from(Span::styled(
      " Press q, ? or Esc to close".to_string(),
      Style::default().fg(theme.text_muted),
    )),
  ]);
  footer.render(footer_rect, buf);
}
