use std::path::Path;

use ratatui::style::Style;
use ratatui::text::{Line, Span};

use crate::icons::{file_icon, file_name_color};
use crate::theme::Theme;

pub struct DirSummary {
  pub file_count: usize,
  pub dir_count: usize,
  pub total_size: u64,
  pub entries: Vec<DirEntry>,
}

pub struct DirEntry {
  pub name: String,
  pub is_dir: bool,
  pub size: u64,
}

pub fn summarize_dir(path: &Path) -> DirSummary {
  let mut summary = DirSummary {
    file_count: 0,
    dir_count: 0,
    total_size: 0,
    entries: Vec::new(),
  };

  let read_dir = match std::fs::read_dir(path) {
    Ok(rd) => rd,
    Err(_) => return summary,
  };

  for entry in read_dir.flatten() {
    let meta = entry.metadata();
    let is_dir = meta.as_ref().is_ok_and(|m| m.is_dir());
    let size = meta.as_ref().map_or(0, |m| m.len());
    let name = entry.file_name().to_string_lossy().to_string();

    if is_dir {
      summary.dir_count += 1;
    } else {
      summary.file_count += 1;
      summary.total_size += size;
    }

    summary.entries.push(DirEntry { name, is_dir, size });
  }

  // Sort: dirs first, then alphabetical
  summary.entries.sort_by(|a, b| {
    b.is_dir
      .cmp(&a.is_dir)
      .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
  });

  summary
}

pub fn render_dir_summary<'a>(summary: &DirSummary, theme: &Theme) -> Vec<Line<'a>> {
  let mut lines = Vec::new();

  lines.push(Line::from(vec![
    Span::styled(
      format!(
        " {} files, {} directories, {}",
        summary.file_count,
        summary.dir_count,
        format_size(summary.total_size)
      ),
      Style::default().fg(theme.text),
    ),
  ]));
  lines.push(Line::from(""));

  for entry in &summary.entries {
    let icon = file_icon(&entry.name, entry.is_dir, false, false);
    let color = file_name_color(&entry.name, entry.is_dir, false);
    let size_str = if entry.is_dir {
      String::new()
    } else {
      format!("  {}", format_size(entry.size))
    };

    lines.push(Line::from(vec![
      Span::styled(" ", Style::default()),
      Span::styled(icon.glyph, Style::default().fg(icon.color)),
      Span::styled(entry.name.clone(), Style::default().fg(color)),
      Span::styled(size_str, Style::default().fg(theme.text_dim)),
    ]));
  }

  lines
}

pub fn format_size(bytes: u64) -> String {
  const KB: u64 = 1024;
  const MB: u64 = 1024 * KB;
  const GB: u64 = 1024 * MB;

  if bytes >= GB {
    format!("{:.1} GB", bytes as f64 / GB as f64)
  } else if bytes >= MB {
    format!("{:.1} MB", bytes as f64 / MB as f64)
  } else if bytes >= KB {
    format!("{:.1} KB", bytes as f64 / KB as f64)
  } else {
    format!("{bytes} B")
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_format_size_bytes() {
    assert_eq!(format_size(0), "0 B");
    assert_eq!(format_size(512), "512 B");
    assert_eq!(format_size(1023), "1023 B");
  }

  #[test]
  fn test_format_size_kb() {
    assert_eq!(format_size(1024), "1.0 KB");
    assert_eq!(format_size(2560), "2.5 KB");
  }

  #[test]
  fn test_format_size_mb() {
    assert_eq!(format_size(1024 * 1024), "1.0 MB");
  }

  #[test]
  fn test_format_size_gb() {
    assert_eq!(format_size(1024 * 1024 * 1024), "1.0 GB");
  }

  #[test]
  fn test_summarize_tempdir() {
    let dir = std::env::temp_dir().join("tui_explorer_test_dir_summary");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("file1.txt"), "hello").unwrap();
    std::fs::write(dir.join("file2.txt"), "world!!").unwrap();
    std::fs::create_dir(dir.join("subdir")).unwrap();

    let summary = summarize_dir(&dir);
    assert_eq!(summary.file_count, 2);
    assert_eq!(summary.dir_count, 1);
    assert_eq!(summary.total_size, 12); // 5 + 7

    let _ = std::fs::remove_dir_all(&dir);
  }

  #[test]
  fn test_render_dir_summary() {
    let summary = DirSummary {
      file_count: 3,
      dir_count: 1,
      total_size: 1024,
      entries: vec![
        DirEntry { name: "src".to_string(), is_dir: true, size: 0 },
        DirEntry { name: "main.rs".to_string(), is_dir: false, size: 512 },
      ],
    };
    let lines = render_dir_summary(&summary, &Theme::dark());
    assert!(!lines.is_empty());
    // First line should mention counts
    let first_line_text: String = lines[0].spans.iter().map(|s| s.content.to_string()).collect();
    assert!(first_line_text.contains("3 files"));
  }
}
