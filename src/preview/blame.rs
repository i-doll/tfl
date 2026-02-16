use std::collections::HashMap;
use std::path::Path;

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use crate::git::GitRepo;
use crate::theme::Theme;

/// Information about a single line's blame
#[derive(Debug, Clone)]
pub struct BlameLine {
  pub line_num: usize,
  pub commit_hash: String,
  pub author: String,
  pub date: String,
  pub content: String,
}

/// Complete blame data for a file
#[derive(Debug, Clone)]
pub struct BlameData {
  pub lines: Vec<BlameLine>,
  pub commit_colors: HashMap<String, Color>,
}

impl BlameData {
  pub fn new(lines: Vec<BlameLine>) -> Self {
    let mut commit_colors = HashMap::new();
    let colors = [
      Color::Indexed(174), // pink
      Color::Indexed(108), // green
      Color::Indexed(110), // blue
      Color::Indexed(180), // yellow
      Color::Indexed(139), // purple
      Color::Indexed(73),  // cyan
      Color::Indexed(216), // orange
      Color::Indexed(151), // teal
    ];

    let mut color_idx = 0;
    for line in &lines {
      if !commit_colors.contains_key(&line.commit_hash) {
        commit_colors.insert(line.commit_hash.clone(), colors[color_idx % colors.len()]);
        color_idx += 1;
      }
    }

    Self { lines, commit_colors }
  }

  pub fn render(&self, max_lines: usize, scroll_offset: usize, theme: &Theme) -> Vec<Line<'static>> {
    self.lines
      .iter()
      .skip(scroll_offset)
      .take(max_lines)
      .map(|line| self.render_line(line, theme))
      .collect()
  }

  fn render_line(&self, line: &BlameLine, theme: &Theme) -> Line<'static> {
    let color = self.commit_colors.get(&line.commit_hash).copied().unwrap_or(Color::Gray);

    let mut spans = Vec::new();

    // Commit hash (7 chars)
    spans.push(Span::styled(
      format!("{} ", line.commit_hash),
      Style::default().fg(color).add_modifier(Modifier::BOLD),
    ));

    // Author (truncated to 12 chars)
    let author = if line.author.len() > 12 {
      format!("{}.. ", &line.author[..10])
    } else {
      format!("{:12} ", line.author)
    };
    spans.push(Span::styled(author, Style::default().fg(theme.meta_secondary)));

    // Date (8 chars)
    spans.push(Span::styled(
      format!("{:8} ", line.date),
      Style::default().fg(theme.border),
    ));

    // Line number
    spans.push(Span::styled(
      format!("{:>4} ", line.line_num),
      Style::default().fg(theme.text_dim),
    ));

    // Content
    spans.push(Span::styled(
      line.content.clone(),
      Style::default().fg(theme.text),
    ));

    Line::from(spans)
  }
}

/// Get blame data for a file
pub fn get_blame(git_repo: &GitRepo, path: &Path) -> Option<BlameData> {
  git_repo.get_file_blame(path)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_blame_data_new_assigns_colors() {
    let lines = vec![
      BlameLine {
        line_num: 1,
        commit_hash: "abc1234".to_string(),
        author: "Alice".to_string(),
        date: "2d ago".to_string(),
        content: "line 1".to_string(),
      },
      BlameLine {
        line_num: 2,
        commit_hash: "def5678".to_string(),
        author: "Bob".to_string(),
        date: "1w ago".to_string(),
        content: "line 2".to_string(),
      },
      BlameLine {
        line_num: 3,
        commit_hash: "abc1234".to_string(),
        author: "Alice".to_string(),
        date: "2d ago".to_string(),
        content: "line 3".to_string(),
      },
    ];

    let blame = BlameData::new(lines);

    // Same commit should have same color
    assert_eq!(
      blame.commit_colors.get("abc1234"),
      blame.commit_colors.get("abc1234")
    );

    // Different commits should have different colors
    assert_ne!(
      blame.commit_colors.get("abc1234"),
      blame.commit_colors.get("def5678")
    );
  }

  #[test]
  fn test_blame_data_render_respects_scroll() {
    let lines = vec![
      BlameLine {
        line_num: 1,
        commit_hash: "abc1234".to_string(),
        author: "Alice".to_string(),
        date: "2d ago".to_string(),
        content: "line 1".to_string(),
      },
      BlameLine {
        line_num: 2,
        commit_hash: "def5678".to_string(),
        author: "Bob".to_string(),
        date: "1w ago".to_string(),
        content: "line 2".to_string(),
      },
      BlameLine {
        line_num: 3,
        commit_hash: "ghi9012".to_string(),
        author: "Charlie".to_string(),
        date: "1mo ago".to_string(),
        content: "line 3".to_string(),
      },
    ];

    let blame = BlameData::new(lines);

    // Render with scroll offset 1
    let rendered = blame.render(10, 1, &Theme::dark());
    assert_eq!(rendered.len(), 2);
  }

  #[test]
  fn test_blame_data_render_respects_max_lines() {
    let lines: Vec<BlameLine> = (1..=10)
      .map(|i| BlameLine {
        line_num: i,
        commit_hash: format!("commit{i}"),
        author: "Author".to_string(),
        date: "1d ago".to_string(),
        content: format!("line {i}"),
      })
      .collect();

    let blame = BlameData::new(lines);

    let rendered = blame.render(5, 0, &Theme::dark());
    assert_eq!(rendered.len(), 5);
  }

  #[test]
  fn test_blame_line_render_truncates_author() {
    let lines = vec![BlameLine {
      line_num: 1,
      commit_hash: "abc1234".to_string(),
      author: "VeryLongAuthorName".to_string(),
      date: "2d ago".to_string(),
      content: "content".to_string(),
    }];

    let blame = BlameData::new(lines);
    let rendered = blame.render(1, 0, &Theme::dark());
    assert_eq!(rendered.len(), 1);

    // Check that the author was truncated (contains "..")
    let line_str: String = rendered[0].spans.iter().map(|s| s.content.as_ref()).collect();
    assert!(line_str.contains(".."));
  }
}
