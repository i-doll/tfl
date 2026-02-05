//! Size filter module for parsing and applying size-based file filters.
//!
//! Supported expressions:
//! - `>1M` - larger than 1 megabyte
//! - `<100K` - smaller than 100 kilobytes
//! - `1M-10M` - between 1 and 10 megabytes
//! - `=0` - exactly 0 bytes (empty files)
//! - `500` - exactly 500 bytes (no unit)

/// Represents a parsed size filter expression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SizeFilter {
  /// Greater than a size
  GreaterThan(u64),
  /// Less than a size
  LessThan(u64),
  /// Exactly equal to a size
  Equal(u64),
  /// Between two sizes (inclusive)
  Range(u64, u64),
}

impl SizeFilter {
  /// Parse a size filter expression.
  ///
  /// Supported formats:
  /// - `>1M` - greater than 1 megabyte
  /// - `<100K` - less than 100 kilobytes
  /// - `1M-10M` - range between 1MB and 10MB
  /// - `=0` - exactly 0 bytes
  /// - `500` - exactly 500 bytes
  pub fn parse(expr: &str) -> Option<SizeFilter> {
    let expr = expr.trim();
    if expr.is_empty() {
      return None;
    }

    // Check for range format: 1M-10M
    if let Some(idx) = expr.find('-') {
      // Make sure it's not just a negative number at the start
      if idx > 0 {
        let left = &expr[..idx];
        let right = &expr[idx + 1..];
        if !left.is_empty() && !right.is_empty() {
          let min = parse_size(left)?;
          let max = parse_size(right)?;
          return Some(SizeFilter::Range(min, max));
        }
      }
    }

    // Check for operator prefixes
    let first = expr.chars().next()?;
    match first {
      '>' => {
        let size = parse_size(&expr[1..])?;
        Some(SizeFilter::GreaterThan(size))
      }
      '<' => {
        let size = parse_size(&expr[1..])?;
        Some(SizeFilter::LessThan(size))
      }
      '=' => {
        let size = parse_size(&expr[1..])?;
        Some(SizeFilter::Equal(size))
      }
      _ => {
        // Treat as exact size
        let size = parse_size(expr)?;
        Some(SizeFilter::Equal(size))
      }
    }
  }

  /// Check if a file size matches this filter.
  pub fn matches(&self, size: u64) -> bool {
    match self {
      SizeFilter::GreaterThan(threshold) => size > *threshold,
      SizeFilter::LessThan(threshold) => size < *threshold,
      SizeFilter::Equal(value) => size == *value,
      SizeFilter::Range(min, max) => size >= *min && size <= *max,
    }
  }
}

/// Parse a size string with optional unit suffix.
///
/// Supported units:
/// - B (bytes, default)
/// - K (kilobytes, 1024 bytes)
/// - M (megabytes, 1024^2 bytes)
/// - G (gigabytes, 1024^3 bytes)
fn parse_size(s: &str) -> Option<u64> {
  let s = s.trim();
  if s.is_empty() {
    return None;
  }

  // Find where the numeric part ends
  let (num_str, unit) = if let Some(pos) = s.find(|c: char| c.is_alphabetic()) {
    (&s[..pos], &s[pos..])
  } else {
    (s, "")
  };

  let num: u64 = num_str.trim().parse().ok()?;

  let multiplier = match unit.to_uppercase().as_str() {
    "" | "B" => 1,
    "K" | "KB" => 1024,
    "M" | "MB" => 1024 * 1024,
    "G" | "GB" => 1024 * 1024 * 1024,
    _ => return None,
  };

  Some(num * multiplier)
}

/// Format a size in bytes to human-readable form.
#[allow(dead_code)]
pub fn format_size_short(bytes: u64) -> String {
  if bytes < 1024 {
    format!("{}B", bytes)
  } else if bytes < 1024 * 1024 {
    format!("{}K", bytes / 1024)
  } else if bytes < 1024 * 1024 * 1024 {
    format!("{}M", bytes / (1024 * 1024))
  } else {
    format!("{}G", bytes / (1024 * 1024 * 1024))
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  // --- parse_size tests ---

  #[test]
  fn test_parse_size_plain_number() {
    assert_eq!(parse_size("100"), Some(100));
    assert_eq!(parse_size("0"), Some(0));
    assert_eq!(parse_size("1000000"), Some(1_000_000));
  }

  #[test]
  fn test_parse_size_bytes() {
    assert_eq!(parse_size("100B"), Some(100));
    assert_eq!(parse_size("100b"), Some(100));
  }

  #[test]
  fn test_parse_size_kilobytes() {
    assert_eq!(parse_size("1K"), Some(1024));
    assert_eq!(parse_size("100K"), Some(100 * 1024));
    assert_eq!(parse_size("1KB"), Some(1024));
    assert_eq!(parse_size("1k"), Some(1024));
  }

  #[test]
  fn test_parse_size_megabytes() {
    assert_eq!(parse_size("1M"), Some(1024 * 1024));
    assert_eq!(parse_size("10M"), Some(10 * 1024 * 1024));
    assert_eq!(parse_size("1MB"), Some(1024 * 1024));
    assert_eq!(parse_size("1m"), Some(1024 * 1024));
  }

  #[test]
  fn test_parse_size_gigabytes() {
    assert_eq!(parse_size("1G"), Some(1024 * 1024 * 1024));
    assert_eq!(parse_size("1GB"), Some(1024 * 1024 * 1024));
    assert_eq!(parse_size("1g"), Some(1024 * 1024 * 1024));
  }

  #[test]
  fn test_parse_size_invalid() {
    assert_eq!(parse_size(""), None);
    assert_eq!(parse_size("abc"), None);
    assert_eq!(parse_size("1X"), None);
    assert_eq!(parse_size("-1"), None);
  }

  #[test]
  fn test_parse_size_with_whitespace() {
    assert_eq!(parse_size("  100  "), Some(100));
    assert_eq!(parse_size(" 1M "), Some(1024 * 1024));
  }

  // --- SizeFilter::parse tests ---

  #[test]
  fn test_parse_greater_than() {
    assert_eq!(SizeFilter::parse(">1M"), Some(SizeFilter::GreaterThan(1024 * 1024)));
    assert_eq!(SizeFilter::parse(">100K"), Some(SizeFilter::GreaterThan(100 * 1024)));
    assert_eq!(SizeFilter::parse(">0"), Some(SizeFilter::GreaterThan(0)));
  }

  #[test]
  fn test_parse_less_than() {
    assert_eq!(SizeFilter::parse("<1M"), Some(SizeFilter::LessThan(1024 * 1024)));
    assert_eq!(SizeFilter::parse("<100K"), Some(SizeFilter::LessThan(100 * 1024)));
  }

  #[test]
  fn test_parse_equal() {
    assert_eq!(SizeFilter::parse("=0"), Some(SizeFilter::Equal(0)));
    assert_eq!(SizeFilter::parse("=1K"), Some(SizeFilter::Equal(1024)));
  }

  #[test]
  fn test_parse_exact_without_operator() {
    assert_eq!(SizeFilter::parse("1024"), Some(SizeFilter::Equal(1024)));
    assert_eq!(SizeFilter::parse("1K"), Some(SizeFilter::Equal(1024)));
  }

  #[test]
  fn test_parse_range() {
    assert_eq!(
      SizeFilter::parse("1M-10M"),
      Some(SizeFilter::Range(1024 * 1024, 10 * 1024 * 1024))
    );
    assert_eq!(
      SizeFilter::parse("100K-1M"),
      Some(SizeFilter::Range(100 * 1024, 1024 * 1024))
    );
    assert_eq!(SizeFilter::parse("0-100"), Some(SizeFilter::Range(0, 100)));
  }

  #[test]
  fn test_parse_invalid() {
    assert_eq!(SizeFilter::parse(""), None);
    assert_eq!(SizeFilter::parse("  "), None);
    assert_eq!(SizeFilter::parse(">"), None);
    assert_eq!(SizeFilter::parse("<"), None);
    assert_eq!(SizeFilter::parse("="), None);
    assert_eq!(SizeFilter::parse("abc"), None);
  }

  // --- SizeFilter::matches tests ---

  #[test]
  fn test_matches_greater_than() {
    let filter = SizeFilter::GreaterThan(1000);
    assert!(!filter.matches(999));
    assert!(!filter.matches(1000));
    assert!(filter.matches(1001));
    assert!(filter.matches(2000));
  }

  #[test]
  fn test_matches_less_than() {
    let filter = SizeFilter::LessThan(1000);
    assert!(filter.matches(0));
    assert!(filter.matches(999));
    assert!(!filter.matches(1000));
    assert!(!filter.matches(1001));
  }

  #[test]
  fn test_matches_equal() {
    let filter = SizeFilter::Equal(1000);
    assert!(!filter.matches(999));
    assert!(filter.matches(1000));
    assert!(!filter.matches(1001));
  }

  #[test]
  fn test_matches_range() {
    let filter = SizeFilter::Range(100, 200);
    assert!(!filter.matches(99));
    assert!(filter.matches(100));
    assert!(filter.matches(150));
    assert!(filter.matches(200));
    assert!(!filter.matches(201));
  }

  #[test]
  fn test_matches_empty_files() {
    let filter = SizeFilter::Equal(0);
    assert!(filter.matches(0));
    assert!(!filter.matches(1));
  }

  // --- format_size_short tests ---

  #[test]
  fn test_format_size_short_bytes() {
    assert_eq!(format_size_short(0), "0B");
    assert_eq!(format_size_short(100), "100B");
    assert_eq!(format_size_short(1023), "1023B");
  }

  #[test]
  fn test_format_size_short_kilobytes() {
    assert_eq!(format_size_short(1024), "1K");
    assert_eq!(format_size_short(2048), "2K");
    assert_eq!(format_size_short(100 * 1024), "100K");
  }

  #[test]
  fn test_format_size_short_megabytes() {
    assert_eq!(format_size_short(1024 * 1024), "1M");
    assert_eq!(format_size_short(10 * 1024 * 1024), "10M");
  }

  #[test]
  fn test_format_size_short_gigabytes() {
    assert_eq!(format_size_short(1024 * 1024 * 1024), "1G");
  }
}
