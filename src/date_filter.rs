use std::path::Path;
use std::time::SystemTime;

use chrono::{DateTime, Duration, Local, NaiveDate};

/// Represents a time type for filtering
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TimeType {
  #[default]
  Modified,
  Created,
  Accessed,
}

/// Represents a parsed date filter expression
#[derive(Debug, Clone, PartialEq)]
pub enum DateFilter {
  /// Match files from today
  Today,
  /// Match files from yesterday
  Yesterday,
  /// Match files within the last N days
  LastDays(u32),
  /// Match files within the last N weeks
  LastWeeks(u32),
  /// Match files within the last N months
  LastMonths(u32),
  /// Match files on an exact date
  ExactDate(NaiveDate),
  /// Match files after a date
  After(NaiveDate),
  /// Match files before a date
  Before(NaiveDate),
  /// Match files between two dates (inclusive)
  Between(NaiveDate, NaiveDate),
}

impl DateFilter {
  /// Parse a date expression string into a DateFilter
  pub fn parse(expr: &str) -> Option<Self> {
    let expr = expr.trim().to_lowercase();

    if expr.is_empty() {
      return None;
    }

    // Check for relative date keywords
    match expr.as_str() {
      "today" => return Some(DateFilter::Today),
      "yesterday" => return Some(DateFilter::Yesterday),
      _ => {}
    }

    // Check for relative duration patterns: 7d, 1w, 1m
    if let Some(filter) = Self::parse_relative_duration(&expr) {
      return Some(filter);
    }

    // Check for comparison operators: >2024-01-01, <2024-01-01
    if let Some(rest) = expr.strip_prefix('>') {
      let date_str = rest.trim();
      if let Some(date) = Self::parse_date(date_str) {
        return Some(DateFilter::After(date));
      }
    }

    if let Some(rest) = expr.strip_prefix('<') {
      let date_str = rest.trim();
      if let Some(date) = Self::parse_date(date_str) {
        return Some(DateFilter::Before(date));
      }
    }

    // Check for range: 2024-01-01..2024-01-31
    if let Some(pos) = expr.find("..") {
      let start_str = expr[..pos].trim();
      let end_str = expr[pos + 2..].trim();
      if let (Some(start), Some(end)) = (Self::parse_date(start_str), Self::parse_date(end_str)) {
        return Some(DateFilter::Between(start, end));
      }
    }

    // Try parsing as exact date
    if let Some(date) = Self::parse_date(&expr) {
      return Some(DateFilter::ExactDate(date));
    }

    None
  }

  /// Parse relative duration patterns like 7d, 1w, 1m
  fn parse_relative_duration(expr: &str) -> Option<DateFilter> {
    // Check for "older than" pattern: <7d, <1w, <1m
    if let Some(rest) = expr.strip_prefix('<') {
      let rest = rest.trim();
      if let Some((num, unit)) = Self::extract_duration(rest) {
        return match unit {
          'd' => Some(DateFilter::Before(Self::date_n_days_ago(num))),
          'w' => Some(DateFilter::Before(Self::date_n_days_ago(num * 7))),
          'm' => Some(DateFilter::Before(Self::date_n_months_ago(num))),
          _ => None,
        };
      }
    }

    // Standard duration pattern: 7d, 1w, 1m (means within last N units)
    if let Some((num, unit)) = Self::extract_duration(expr) {
      return match unit {
        'd' => Some(DateFilter::LastDays(num)),
        'w' => Some(DateFilter::LastWeeks(num)),
        'm' => Some(DateFilter::LastMonths(num)),
        _ => None,
      };
    }

    None
  }

  fn extract_duration(s: &str) -> Option<(u32, char)> {
    if s.is_empty() {
      return None;
    }
    let unit = s.chars().last()?;
    if !matches!(unit, 'd' | 'w' | 'm') {
      return None;
    }
    let num_str = &s[..s.len() - 1];
    let num: u32 = num_str.parse().ok()?;
    if num == 0 {
      return None;
    }
    Some((num, unit))
  }

  fn date_n_days_ago(n: u32) -> NaiveDate {
    let now = Local::now();
    (now - Duration::days(n as i64)).date_naive()
  }

  fn date_n_months_ago(n: u32) -> NaiveDate {
    let now = Local::now();
    // Approximate months as 30 days
    (now - Duration::days((n * 30) as i64)).date_naive()
  }

  /// Parse a date string in YYYY-MM-DD format
  fn parse_date(s: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()
  }

  /// Check if a file matches this date filter
  pub fn matches(&self, path: &Path, time_type: TimeType) -> bool {
    let metadata = match path.metadata() {
      Ok(m) => m,
      Err(_) => return false,
    };

    let file_time = match time_type {
      TimeType::Modified => metadata.modified(),
      TimeType::Created => metadata.created(),
      TimeType::Accessed => metadata.accessed(),
    };

    let file_time = match file_time {
      Ok(t) => t,
      Err(_) => return false,
    };

    self.matches_time(file_time)
  }

  /// Check if a SystemTime matches this date filter
  pub fn matches_time(&self, time: SystemTime) -> bool {
    let datetime: DateTime<Local> = time.into();
    let file_date = datetime.date_naive();
    let today = Local::now().date_naive();

    match self {
      DateFilter::Today => file_date == today,
      DateFilter::Yesterday => file_date == today - Duration::days(1),
      DateFilter::LastDays(n) => {
        let cutoff = today - Duration::days(*n as i64);
        file_date >= cutoff
      }
      DateFilter::LastWeeks(n) => {
        let cutoff = today - Duration::days(*n as i64 * 7);
        file_date >= cutoff
      }
      DateFilter::LastMonths(n) => {
        let cutoff = today - Duration::days(*n as i64 * 30);
        file_date >= cutoff
      }
      DateFilter::ExactDate(date) => file_date == *date,
      DateFilter::After(date) => file_date > *date,
      DateFilter::Before(date) => file_date < *date,
      DateFilter::Between(start, end) => file_date >= *start && file_date <= *end,
    }
  }

  /// Get a display string for the time that matched
  pub fn format_time(time: SystemTime) -> String {
    let datetime: DateTime<Local> = time.into();
    datetime.format("%Y-%m-%d %H:%M").to_string()
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use chrono::TimeZone;
  use std::fs;
  use std::time::Duration as StdDuration;

  // --- Parsing tests ---

  #[test]
  fn test_parse_today() {
    assert_eq!(DateFilter::parse("today"), Some(DateFilter::Today));
    assert_eq!(DateFilter::parse("TODAY"), Some(DateFilter::Today));
    assert_eq!(DateFilter::parse("  today  "), Some(DateFilter::Today));
  }

  #[test]
  fn test_parse_yesterday() {
    assert_eq!(DateFilter::parse("yesterday"), Some(DateFilter::Yesterday));
    assert_eq!(DateFilter::parse("YESTERDAY"), Some(DateFilter::Yesterday));
  }

  #[test]
  fn test_parse_relative_days() {
    assert_eq!(DateFilter::parse("7d"), Some(DateFilter::LastDays(7)));
    assert_eq!(DateFilter::parse("1d"), Some(DateFilter::LastDays(1)));
    assert_eq!(DateFilter::parse("30d"), Some(DateFilter::LastDays(30)));
  }

  #[test]
  fn test_parse_relative_weeks() {
    assert_eq!(DateFilter::parse("1w"), Some(DateFilter::LastWeeks(1)));
    assert_eq!(DateFilter::parse("2w"), Some(DateFilter::LastWeeks(2)));
    assert_eq!(DateFilter::parse("4w"), Some(DateFilter::LastWeeks(4)));
  }

  #[test]
  fn test_parse_relative_months() {
    assert_eq!(DateFilter::parse("1m"), Some(DateFilter::LastMonths(1)));
    assert_eq!(DateFilter::parse("3m"), Some(DateFilter::LastMonths(3)));
    assert_eq!(DateFilter::parse("12m"), Some(DateFilter::LastMonths(12)));
  }

  #[test]
  fn test_parse_exact_date() {
    let expected = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
    assert_eq!(DateFilter::parse("2024-01-15"), Some(DateFilter::ExactDate(expected)));
  }

  #[test]
  fn test_parse_after_date() {
    let expected = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
    assert_eq!(DateFilter::parse(">2024-01-01"), Some(DateFilter::After(expected)));
    assert_eq!(DateFilter::parse("> 2024-01-01"), Some(DateFilter::After(expected)));
  }

  #[test]
  fn test_parse_before_date() {
    let expected = NaiveDate::from_ymd_opt(2024, 6, 30).unwrap();
    assert_eq!(DateFilter::parse("<2024-06-30"), Some(DateFilter::Before(expected)));
  }

  #[test]
  fn test_parse_older_than_duration() {
    // <1w means older than 1 week
    if let Some(DateFilter::Before(_)) = DateFilter::parse("<1w") {
      // Pass - we just check it parses to Before
    } else {
      panic!("Expected DateFilter::Before for <1w");
    }

    if let Some(DateFilter::Before(_)) = DateFilter::parse("<7d") {
      // Pass
    } else {
      panic!("Expected DateFilter::Before for <7d");
    }
  }

  #[test]
  fn test_parse_date_range() {
    let start = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
    let end = NaiveDate::from_ymd_opt(2024, 1, 31).unwrap();
    assert_eq!(DateFilter::parse("2024-01-01..2024-01-31"), Some(DateFilter::Between(start, end)));
  }

  #[test]
  fn test_parse_empty() {
    assert_eq!(DateFilter::parse(""), None);
    assert_eq!(DateFilter::parse("   "), None);
  }

  #[test]
  fn test_parse_invalid() {
    assert_eq!(DateFilter::parse("foo"), None);
    assert_eq!(DateFilter::parse("0d"), None);
    assert_eq!(DateFilter::parse("abc123"), None);
    assert_eq!(DateFilter::parse("2024-13-01"), None); // Invalid month
  }

  // --- Matching tests ---

  #[test]
  fn test_matches_today() {
    let now = SystemTime::now();
    assert!(DateFilter::Today.matches_time(now));

    let yesterday = now - StdDuration::from_secs(24 * 60 * 60);
    assert!(!DateFilter::Today.matches_time(yesterday));
  }

  #[test]
  fn test_matches_yesterday() {
    let now = SystemTime::now();
    let yesterday = now - StdDuration::from_secs(24 * 60 * 60);

    assert!(!DateFilter::Yesterday.matches_time(now));
    assert!(DateFilter::Yesterday.matches_time(yesterday));
  }

  #[test]
  fn test_matches_last_days() {
    let now = SystemTime::now();
    let three_days_ago = now - StdDuration::from_secs(3 * 24 * 60 * 60);
    let ten_days_ago = now - StdDuration::from_secs(10 * 24 * 60 * 60);

    let filter = DateFilter::LastDays(7);
    assert!(filter.matches_time(now));
    assert!(filter.matches_time(three_days_ago));
    assert!(!filter.matches_time(ten_days_ago));
  }

  #[test]
  fn test_matches_last_weeks() {
    let now = SystemTime::now();
    let one_week_ago = now - StdDuration::from_secs(5 * 24 * 60 * 60);
    let three_weeks_ago = now - StdDuration::from_secs(21 * 24 * 60 * 60);

    let filter = DateFilter::LastWeeks(2);
    assert!(filter.matches_time(now));
    assert!(filter.matches_time(one_week_ago));
    assert!(!filter.matches_time(three_weeks_ago));
  }

  #[test]
  fn test_matches_exact_date() {
    let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
    let filter = DateFilter::ExactDate(date);

    // Create a SystemTime for that date
    let datetime = Local.with_ymd_and_hms(2024, 6, 15, 12, 0, 0).unwrap();
    let time: SystemTime = datetime.into();
    assert!(filter.matches_time(time));

    // Different date should not match
    let other_datetime = Local.with_ymd_and_hms(2024, 6, 16, 12, 0, 0).unwrap();
    let other_time: SystemTime = other_datetime.into();
    assert!(!filter.matches_time(other_time));
  }

  #[test]
  fn test_matches_after() {
    let date = NaiveDate::from_ymd_opt(2024, 6, 1).unwrap();
    let filter = DateFilter::After(date);

    let after_datetime = Local.with_ymd_and_hms(2024, 6, 15, 12, 0, 0).unwrap();
    let after_time: SystemTime = after_datetime.into();
    assert!(filter.matches_time(after_time));

    let before_datetime = Local.with_ymd_and_hms(2024, 5, 15, 12, 0, 0).unwrap();
    let before_time: SystemTime = before_datetime.into();
    assert!(!filter.matches_time(before_time));

    // Same date should not match (After is exclusive)
    let same_datetime = Local.with_ymd_and_hms(2024, 6, 1, 12, 0, 0).unwrap();
    let same_time: SystemTime = same_datetime.into();
    assert!(!filter.matches_time(same_time));
  }

  #[test]
  fn test_matches_before() {
    let date = NaiveDate::from_ymd_opt(2024, 6, 1).unwrap();
    let filter = DateFilter::Before(date);

    let before_datetime = Local.with_ymd_and_hms(2024, 5, 15, 12, 0, 0).unwrap();
    let before_time: SystemTime = before_datetime.into();
    assert!(filter.matches_time(before_time));

    let after_datetime = Local.with_ymd_and_hms(2024, 6, 15, 12, 0, 0).unwrap();
    let after_time: SystemTime = after_datetime.into();
    assert!(!filter.matches_time(after_time));
  }

  #[test]
  fn test_matches_between() {
    let start = NaiveDate::from_ymd_opt(2024, 6, 1).unwrap();
    let end = NaiveDate::from_ymd_opt(2024, 6, 30).unwrap();
    let filter = DateFilter::Between(start, end);

    let in_range_datetime = Local.with_ymd_and_hms(2024, 6, 15, 12, 0, 0).unwrap();
    let in_range_time: SystemTime = in_range_datetime.into();
    assert!(filter.matches_time(in_range_time));

    let start_datetime = Local.with_ymd_and_hms(2024, 6, 1, 12, 0, 0).unwrap();
    let start_time: SystemTime = start_datetime.into();
    assert!(filter.matches_time(start_time)); // Inclusive

    let end_datetime = Local.with_ymd_and_hms(2024, 6, 30, 12, 0, 0).unwrap();
    let end_time: SystemTime = end_datetime.into();
    assert!(filter.matches_time(end_time)); // Inclusive

    let out_of_range_datetime = Local.with_ymd_and_hms(2024, 7, 1, 12, 0, 0).unwrap();
    let out_of_range_time: SystemTime = out_of_range_datetime.into();
    assert!(!filter.matches_time(out_of_range_time));
  }

  #[test]
  fn test_format_time() {
    let datetime = Local.with_ymd_and_hms(2024, 6, 15, 14, 30, 0).unwrap();
    let time: SystemTime = datetime.into();
    let formatted = DateFilter::format_time(time);
    assert_eq!(formatted, "2024-06-15 14:30");
  }

  // --- File matching tests ---

  #[test]
  fn test_matches_file_modified() {
    let dir = std::env::temp_dir().join("tfl_date_filter_test");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();

    let file = dir.join("test.txt");
    fs::write(&file, "hello").unwrap();

    // File was just created, so it should match "today"
    assert!(DateFilter::Today.matches(&file, TimeType::Modified));

    let _ = fs::remove_dir_all(&dir);
  }

  #[test]
  fn test_time_type_default() {
    assert_eq!(TimeType::default(), TimeType::Modified);
  }
}
