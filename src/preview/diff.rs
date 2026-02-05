//! Git diff preview module
//!
//! Provides diff generation for modified files and rendering with color highlighting.

use std::path::Path;

use git2::{DiffOptions, Repository};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

/// Represents a line in a diff hunk
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiffLineKind {
  Context,
  Added,
  Removed,
  Header,
  HunkHeader,
}

/// A single line in the diff output
#[derive(Debug, Clone)]
pub struct DiffLine {
  pub kind: DiffLineKind,
  pub content: String,
  pub old_line_no: Option<u32>,
  pub new_line_no: Option<u32>,
}

/// Contains the parsed diff for a file
#[derive(Debug, Clone)]
pub struct FileDiff {
  pub lines: Vec<DiffLine>,
  pub hunks: Vec<usize>, // Indices of hunk headers in lines
}

impl FileDiff {
  pub fn new() -> Self {
    Self {
      lines: Vec::new(),
      hunks: Vec::new(),
    }
  }

  /// Returns true if the diff is empty (no changes)
  pub fn is_empty(&self) -> bool {
    self.lines.is_empty()
  }
}

impl Default for FileDiff {
  fn default() -> Self {
    Self::new()
  }
}

/// Generate a diff for the given file path against the git index (HEAD)
pub fn generate_diff(repo_root: &Path, file_path: &Path) -> Option<FileDiff> {
  let repo = Repository::open(repo_root).ok()?;

  // Make path relative to repo root
  let rel_path = file_path.strip_prefix(repo_root).ok()?;
  let rel_path_str = rel_path.to_string_lossy();

  let mut diff_opts = DiffOptions::new();
  diff_opts.pathspec(&*rel_path_str);

  // Get diff between HEAD and working directory
  let diff = repo
    .diff_index_to_workdir(None, Some(&mut diff_opts))
    .ok()?;

  let mut file_diff = FileDiff::new();

  diff
    .print(git2::DiffFormat::Patch, |_delta, _hunk, line| {
      let kind = match line.origin() {
        '+' => DiffLineKind::Added,
        '-' => DiffLineKind::Removed,
        ' ' => DiffLineKind::Context,
        'H' => DiffLineKind::HunkHeader,
        'F' => DiffLineKind::Header,
        _ => DiffLineKind::Header,
      };

      // Track hunk positions
      if kind == DiffLineKind::HunkHeader {
        file_diff.hunks.push(file_diff.lines.len());
      }

      let content = String::from_utf8_lossy(line.content()).to_string();

      file_diff.lines.push(DiffLine {
        kind,
        content,
        old_line_no: line.old_lineno(),
        new_line_no: line.new_lineno(),
      });

      true
    })
    .ok()?;

  if file_diff.is_empty() {
    None
  } else {
    Some(file_diff)
  }
}

/// Render the diff as styled ratatui Lines
pub fn render_diff(diff: &FileDiff) -> Vec<Line<'static>> {
  diff
    .lines
    .iter()
    .map(|line| {
      let (prefix, style) = match line.kind {
        DiffLineKind::Added => (
          "+",
          Style::default().fg(Color::Indexed(114)), // Green
        ),
        DiffLineKind::Removed => (
          "-",
          Style::default().fg(Color::Indexed(167)), // Red
        ),
        DiffLineKind::Context => (" ", Style::default().fg(Color::Indexed(252))),
        DiffLineKind::HunkHeader => (
          "",
          Style::default().fg(Color::Indexed(75)), // Blue
        ),
        DiffLineKind::Header => ("", Style::default().fg(Color::Indexed(246))),
      };

      // Build line number gutter
      let gutter = match line.kind {
        DiffLineKind::Added => {
          format!("{:>4}  ", line.new_line_no.map(|n| n.to_string()).unwrap_or_default())
        }
        DiffLineKind::Removed => {
          format!("  {:>4} ", line.old_line_no.map(|n| n.to_string()).unwrap_or_default())
        }
        DiffLineKind::Context => {
          let old = line.old_line_no.map(|n| n.to_string()).unwrap_or_default();
          let new = line.new_line_no.map(|n| n.to_string()).unwrap_or_default();
          format!("{:>4} {:>4} ", old, new)
        }
        DiffLineKind::HunkHeader | DiffLineKind::Header => String::new(),
      };

      let content = line.content.trim_end_matches('\n');

      Line::from(vec![
        Span::styled(gutter, Style::default().fg(Color::DarkGray)),
        Span::styled(format!("{prefix}{content}"), style),
      ])
    })
    .collect()
}

/// Render a message when file has no diff (unmodified)
pub fn render_no_diff_message() -> Vec<Line<'static>> {
  vec![Line::from(Span::styled(
    " File has no uncommitted changes",
    Style::default().fg(Color::Indexed(246)),
  ))]
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::fs;
  use std::sync::atomic::{AtomicU32, Ordering};

  static COUNTER: AtomicU32 = AtomicU32::new(0);

  fn make_test_dir() -> std::path::PathBuf {
    let id = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!("tfl_diff_test_{id}_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
  }

  fn init_git_repo(dir: &Path) -> Repository {
    let repo = Repository::init(dir).unwrap();
    let mut config = repo.config().unwrap();
    config.set_str("user.email", "test@test.com").unwrap();
    config.set_str("user.name", "Test").unwrap();
    repo
  }

  fn create_initial_commit(repo: &Repository, dir: &Path) {
    let file = dir.join("test.txt");
    fs::write(&file, "line 1\nline 2\nline 3\n").unwrap();

    let mut index = repo.index().unwrap();
    index.add_path(Path::new("test.txt")).unwrap();
    index.write().unwrap();

    let tree_id = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    let sig = repo.signature().unwrap();

    repo
      .commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
      .unwrap();
  }

  #[test]
  fn test_file_diff_new_is_empty() {
    let diff = FileDiff::new();
    assert!(diff.is_empty());
    assert!(diff.hunks.is_empty());
  }

  #[test]
  fn test_file_diff_default_is_empty() {
    let diff = FileDiff::default();
    assert!(diff.is_empty());
  }

  #[test]
  fn test_diff_line_kind_equality() {
    assert_eq!(DiffLineKind::Added, DiffLineKind::Added);
    assert_ne!(DiffLineKind::Added, DiffLineKind::Removed);
  }

  #[test]
  fn test_generate_diff_modified_file() {
    let dir = make_test_dir();
    let repo = init_git_repo(&dir);
    create_initial_commit(&repo, &dir);

    // Modify the file
    let file = dir.join("test.txt");
    fs::write(&file, "line 1\nmodified line 2\nline 3\nnew line 4\n").unwrap();

    let diff = generate_diff(&dir, &file);
    assert!(diff.is_some());

    let diff = diff.unwrap();
    assert!(!diff.is_empty());

    // Should have some added and removed lines
    let has_added = diff.lines.iter().any(|l| l.kind == DiffLineKind::Added);
    let has_removed = diff.lines.iter().any(|l| l.kind == DiffLineKind::Removed);
    assert!(has_added, "diff should have added lines");
    assert!(has_removed, "diff should have removed lines");

    let _ = fs::remove_dir_all(&dir);
  }

  #[test]
  fn test_generate_diff_unmodified_file() {
    let dir = make_test_dir();
    let repo = init_git_repo(&dir);
    create_initial_commit(&repo, &dir);

    // File unchanged
    let file = dir.join("test.txt");
    let diff = generate_diff(&dir, &file);
    assert!(diff.is_none(), "unmodified file should have no diff");

    let _ = fs::remove_dir_all(&dir);
  }

  #[test]
  fn test_generate_diff_non_git_repo() {
    let dir = make_test_dir();
    let file = dir.join("test.txt");
    fs::write(&file, "content").unwrap();

    let diff = generate_diff(&dir, &file);
    assert!(diff.is_none());

    let _ = fs::remove_dir_all(&dir);
  }

  #[test]
  fn test_render_diff_colors() {
    let mut diff = FileDiff::new();
    diff.lines.push(DiffLine {
      kind: DiffLineKind::Added,
      content: "added line".to_string(),
      old_line_no: None,
      new_line_no: Some(1),
    });
    diff.lines.push(DiffLine {
      kind: DiffLineKind::Removed,
      content: "removed line".to_string(),
      old_line_no: Some(1),
      new_line_no: None,
    });
    diff.lines.push(DiffLine {
      kind: DiffLineKind::Context,
      content: "context line".to_string(),
      old_line_no: Some(2),
      new_line_no: Some(2),
    });

    let lines = render_diff(&diff);
    assert_eq!(lines.len(), 3);

    // Verify spans exist (gutter + content)
    assert!(lines[0].spans.len() >= 2);
    assert!(lines[1].spans.len() >= 2);
    assert!(lines[2].spans.len() >= 2);
  }

  #[test]
  fn test_render_no_diff_message() {
    let lines = render_no_diff_message();
    assert_eq!(lines.len(), 1);
    let content: String = lines[0].spans.iter().map(|s| s.content.to_string()).collect();
    assert!(content.contains("no uncommitted changes"));
  }

  #[test]
  fn test_hunk_tracking() {
    let mut diff = FileDiff::new();
    // Header
    diff.lines.push(DiffLine {
      kind: DiffLineKind::Header,
      content: "diff --git".to_string(),
      old_line_no: None,
      new_line_no: None,
    });
    // First hunk at index 1
    diff.hunks.push(1);
    diff.lines.push(DiffLine {
      kind: DiffLineKind::HunkHeader,
      content: "@@ -1,3 +1,4 @@".to_string(),
      old_line_no: None,
      new_line_no: None,
    });
    diff.lines.push(DiffLine {
      kind: DiffLineKind::Context,
      content: "context".to_string(),
      old_line_no: Some(1),
      new_line_no: Some(1),
    });
    // Second hunk at index 3
    diff.hunks.push(3);
    diff.lines.push(DiffLine {
      kind: DiffLineKind::HunkHeader,
      content: "@@ -10,3 +11,4 @@".to_string(),
      old_line_no: None,
      new_line_no: None,
    });

    // Test hunk positions are tracked
    assert_eq!(diff.hunks.len(), 2);
    assert_eq!(diff.hunks[0], 1);
    assert_eq!(diff.hunks[1], 3);
  }

  #[test]
  fn test_generate_diff_tracks_hunks() {
    let dir = make_test_dir();
    let repo = init_git_repo(&dir);
    create_initial_commit(&repo, &dir);

    // Modify file
    let file = dir.join("test.txt");
    fs::write(&file, "modified\nline 2\nline 3\n").unwrap();

    let diff = generate_diff(&dir, &file);
    assert!(diff.is_some());

    let diff = diff.unwrap();
    // Should have at least one hunk
    assert!(!diff.hunks.is_empty(), "diff should track hunk positions");

    let _ = fs::remove_dir_all(&dir);
  }

  #[test]
  fn test_diff_line_numbers_in_gutter() {
    let mut diff = FileDiff::new();
    diff.lines.push(DiffLine {
      kind: DiffLineKind::Added,
      content: "new".to_string(),
      old_line_no: None,
      new_line_no: Some(42),
    });

    let lines = render_diff(&diff);
    let gutter = &lines[0].spans[0].content;
    assert!(gutter.contains("42"), "gutter should show line number 42");
  }
}
