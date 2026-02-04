use std::path::{Path, PathBuf};

/// A segment of the breadcrumb path
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BreadcrumbSegment {
  /// Display name for this segment
  pub name: String,
  /// Full path up to and including this segment
  pub path: PathBuf,
  /// Start position (column) in the rendered breadcrumb
  pub start_col: u16,
  /// Width of this segment in characters
  pub width: u16,
}

/// Parses a path into breadcrumb segments
/// Home directory is shown as `~`
/// Root (`/`) is shown as first segment
pub fn parse_breadcrumb_segments(path: &Path) -> Vec<BreadcrumbSegment> {
  let home = dirs::home_dir();
  let mut segments = Vec::new();

  // Get all components of the path
  let components: Vec<_> = path.components().collect();
  if components.is_empty() {
    return segments;
  }

  let mut accumulated = PathBuf::new();
  let mut col_offset: u16 = 0;

  for (i, component) in components.iter().enumerate() {
    accumulated.push(component);

    let name = if i == 0 {
      // First component is typically "/" on Unix
      match component {
        std::path::Component::RootDir => "/".to_string(),
        std::path::Component::Prefix(p) => p.as_os_str().to_string_lossy().to_string(),
        _ => component.as_os_str().to_string_lossy().to_string(),
      }
    } else {
      component.as_os_str().to_string_lossy().to_string()
    };

    // Check if this accumulated path equals home dir, replace with ~
    let (display_name, display_path) = if let Some(ref home_path) = home {
      if accumulated == *home_path {
        // Replace everything up to and including home with ~
        ("~".to_string(), accumulated.clone())
      } else if accumulated.starts_with(home_path) && i > 0 {
        // We're inside home, continue normally
        (name, accumulated.clone())
      } else if home_path.starts_with(&accumulated) && i < components.len() - 1 {
        // We're a parent of home, skip this segment
        continue;
      } else {
        (name, accumulated.clone())
      }
    } else {
      (name, accumulated.clone())
    };

    let width = display_name.chars().count() as u16;

    segments.push(BreadcrumbSegment {
      name: display_name,
      path: display_path,
      start_col: col_offset,
      width,
    });

    // Account for separator " > " (3 chars)
    col_offset += width + 3;
  }

  segments
}

/// Truncates breadcrumb segments to fit within the given width
/// Returns the segments to display and whether truncation occurred
pub fn truncate_breadcrumbs(
  segments: &[BreadcrumbSegment],
  max_width: u16,
) -> (Vec<BreadcrumbSegment>, bool) {
  if segments.is_empty() {
    return (Vec::new(), false);
  }

  // Calculate total width needed
  let total_width: u16 = segments.iter().map(|s| s.width + 3).sum::<u16>().saturating_sub(3);

  if total_width <= max_width {
    return (segments.to_vec(), false);
  }

  // We need to truncate. Keep first and last segments, show ellipsis in middle
  // Format: "first > ... > last" or just "... > last" if even that's too long
  let ellipsis_width = 3u16; // "..."
  let separator_width = 3u16; // " > "

  // Minimum: "... > last" needs ellipsis + separator + last
  let last = segments.last().unwrap();
  let min_width = ellipsis_width + separator_width + last.width;

  if max_width < min_width {
    // Even the minimum doesn't fit, just show truncated last segment
    let mut truncated = last.clone();
    truncated.start_col = 0;
    return (vec![truncated], true);
  }

  // Try to fit "first > ... > last"
  let first = &segments[0];
  let ideal_width = first.width + separator_width + ellipsis_width + separator_width + last.width;

  if ideal_width <= max_width && segments.len() > 2 {
    // We can show "first > ... > last"
    let mut result = Vec::new();

    let mut col = 0u16;
    let mut first_seg = first.clone();
    first_seg.start_col = col;
    col += first.width + separator_width;
    result.push(first_seg);

    // The ellipsis is represented as a special segment with empty path
    col += ellipsis_width + separator_width;

    let mut last_seg = last.clone();
    last_seg.start_col = col;
    result.push(last_seg);

    return (result, true);
  }

  // Just show "... > last"
  let mut last_seg = last.clone();
  last_seg.start_col = ellipsis_width + separator_width;
  (vec![last_seg], true)
}

/// Find which breadcrumb segment (if any) is at the given column position
pub fn segment_at_column(segments: &[BreadcrumbSegment], col: u16) -> Option<usize> {
  for (i, segment) in segments.iter().enumerate() {
    if col >= segment.start_col && col < segment.start_col + segment.width {
      return Some(i);
    }
  }
  None
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_parse_root_path() {
    let segments = parse_breadcrumb_segments(Path::new("/"));
    assert_eq!(segments.len(), 1);
    assert_eq!(segments[0].name, "/");
    assert_eq!(segments[0].path, PathBuf::from("/"));
  }

  #[test]
  fn test_parse_simple_path() {
    let segments = parse_breadcrumb_segments(Path::new("/usr/local/bin"));
    // We may get 4 segments: /, usr, local, bin OR fewer if home is a parent
    // For /usr/local/bin, home is typically /home/user, so we should get all 4
    assert!(segments.len() >= 1);
    let last = segments.last().unwrap();
    assert_eq!(last.name, "bin");
    assert_eq!(last.path, PathBuf::from("/usr/local/bin"));
  }

  #[test]
  fn test_parse_home_shows_tilde() {
    if let Some(home) = dirs::home_dir() {
      let segments = parse_breadcrumb_segments(&home);
      assert!(!segments.is_empty());
      let last = segments.last().unwrap();
      assert_eq!(last.name, "~");
      assert_eq!(last.path, home);
    }
  }

  #[test]
  fn test_parse_path_inside_home() {
    if let Some(home) = dirs::home_dir() {
      let test_path = home.join("Documents").join("test");
      let segments = parse_breadcrumb_segments(&test_path);
      // Should have: ~ > Documents > test
      assert!(segments.len() >= 3);
      // Find the home segment
      let home_seg = segments.iter().find(|s| s.name == "~");
      assert!(home_seg.is_some());
      let last = segments.last().unwrap();
      assert_eq!(last.name, "test");
    }
  }

  #[test]
  fn test_truncate_short_path_no_truncation() {
    let segments = vec![
      BreadcrumbSegment {
        name: "/".to_string(),
        path: PathBuf::from("/"),
        start_col: 0,
        width: 1,
      },
      BreadcrumbSegment {
        name: "usr".to_string(),
        path: PathBuf::from("/usr"),
        start_col: 4,
        width: 3,
      },
    ];
    let (result, truncated) = truncate_breadcrumbs(&segments, 50);
    assert!(!truncated);
    assert_eq!(result.len(), 2);
  }

  #[test]
  fn test_truncate_long_path() {
    let segments = vec![
      BreadcrumbSegment {
        name: "/".to_string(),
        path: PathBuf::from("/"),
        start_col: 0,
        width: 1,
      },
      BreadcrumbSegment {
        name: "very_long_directory_name".to_string(),
        path: PathBuf::from("/very_long_directory_name"),
        start_col: 4,
        width: 24,
      },
      BreadcrumbSegment {
        name: "another_long_name".to_string(),
        path: PathBuf::from("/very_long_directory_name/another_long_name"),
        start_col: 31,
        width: 17,
      },
      BreadcrumbSegment {
        name: "final".to_string(),
        path: PathBuf::from("/very_long_directory_name/another_long_name/final"),
        start_col: 51,
        width: 5,
      },
    ];
    let (result, truncated) = truncate_breadcrumbs(&segments, 20);
    assert!(truncated);
    // Should show "/ > ... > final" or similar truncated form
    assert!(result.len() <= 2);
  }

  #[test]
  fn test_segment_at_column_finds_correct_segment() {
    let segments = vec![
      BreadcrumbSegment {
        name: "/".to_string(),
        path: PathBuf::from("/"),
        start_col: 0,
        width: 1,
      },
      BreadcrumbSegment {
        name: "usr".to_string(),
        path: PathBuf::from("/usr"),
        start_col: 4,
        width: 3,
      },
      BreadcrumbSegment {
        name: "local".to_string(),
        path: PathBuf::from("/usr/local"),
        start_col: 10,
        width: 5,
      },
    ];

    assert_eq!(segment_at_column(&segments, 0), Some(0));
    assert_eq!(segment_at_column(&segments, 4), Some(1));
    assert_eq!(segment_at_column(&segments, 6), Some(1));
    assert_eq!(segment_at_column(&segments, 10), Some(2));
    assert_eq!(segment_at_column(&segments, 14), Some(2));
    assert_eq!(segment_at_column(&segments, 15), None); // Past end
    assert_eq!(segment_at_column(&segments, 2), None); // In separator
  }

  #[test]
  fn test_empty_segments() {
    let segments: Vec<BreadcrumbSegment> = Vec::new();
    assert_eq!(segment_at_column(&segments, 0), None);
    let (result, truncated) = truncate_breadcrumbs(&segments, 50);
    assert!(!truncated);
    assert!(result.is_empty());
  }
}
