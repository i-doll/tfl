//! Structured data (JSON/TOML) pretty-printing with syntax highlighting.

/// Result of attempting to format structured data.
pub enum FormatResult {
  /// Successfully formatted content with the given extension for highlighting.
  Formatted { content: String, extension: String },
  /// Failed to parse the content; returns an error message.
  #[allow(dead_code)]
  Error(String),
}

/// Detects if a file extension indicates structured data (JSON or TOML).
pub fn is_structured_data(extension: &str) -> bool {
  matches!(extension.to_lowercase().as_str(), "json" | "toml")
}

/// Pretty-prints JSON content with 2-space indentation.
pub fn format_json(content: &str) -> FormatResult {
  match serde_json::from_str::<serde_json::Value>(content) {
    Ok(value) => {
      match serde_json::to_string_pretty(&value) {
        Ok(formatted) => FormatResult::Formatted {
          content: formatted,
          extension: "json".to_string(),
        },
        Err(e) => FormatResult::Error(format!("JSON serialization error: {e}")),
      }
    }
    Err(e) => FormatResult::Error(format!("JSON parse error: {e}")),
  }
}

/// Pretty-prints TOML content.
pub fn format_toml(content: &str) -> FormatResult {
  match content.parse::<toml::Table>() {
    Ok(table) => {
      match toml::to_string_pretty(&table) {
        Ok(formatted) => FormatResult::Formatted {
          content: formatted,
          extension: "toml".to_string(),
        },
        Err(e) => FormatResult::Error(format!("TOML serialization error: {e}")),
      }
    }
    Err(e) => FormatResult::Error(format!("TOML parse error: {e}")),
  }
}

/// Formats content based on file extension.
/// Returns None if the extension is not a structured data format.
pub fn format_structured(content: &str, extension: &str) -> Option<FormatResult> {
  match extension.to_lowercase().as_str() {
    "json" => Some(format_json(content)),
    "toml" => Some(format_toml(content)),
    _ => None,
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_is_structured_json() {
    assert!(is_structured_data("json"));
    assert!(is_structured_data("JSON"));
    assert!(is_structured_data("Json"));
  }

  #[test]
  fn test_is_structured_toml() {
    assert!(is_structured_data("toml"));
    assert!(is_structured_data("TOML"));
    assert!(is_structured_data("Toml"));
  }

  #[test]
  fn test_is_structured_other() {
    assert!(!is_structured_data("txt"));
    assert!(!is_structured_data("rs"));
    assert!(!is_structured_data("md"));
    assert!(!is_structured_data(""));
  }

  #[test]
  fn test_format_json_valid() {
    let input = r#"{"name":"test","count":42}"#;
    let result = format_json(input);
    match result {
      FormatResult::Formatted { content, extension } => {
        assert_eq!(extension, "json");
        // Should be pretty-printed with newlines
        assert!(content.contains('\n'));
        // Should preserve values
        assert!(content.contains("\"name\""));
        assert!(content.contains("\"test\""));
        assert!(content.contains("42"));
      }
      FormatResult::Error(e) => panic!("Expected Formatted, got Error: {e}"),
    }
  }

  #[test]
  fn test_format_json_nested() {
    let input = r#"{"outer":{"inner":"value"},"array":[1,2,3]}"#;
    let result = format_json(input);
    match result {
      FormatResult::Formatted { content, .. } => {
        // Should have proper indentation
        assert!(content.contains("  "));
        assert!(content.contains("\"outer\""));
        assert!(content.contains("\"inner\""));
      }
      FormatResult::Error(e) => panic!("Expected Formatted, got Error: {e}"),
    }
  }

  #[test]
  fn test_format_json_invalid() {
    let input = r#"{"name": incomplete"#;
    let result = format_json(input);
    match result {
      FormatResult::Error(msg) => {
        assert!(msg.contains("JSON parse error"));
      }
      FormatResult::Formatted { .. } => panic!("Expected Error, got Formatted"),
    }
  }

  #[test]
  fn test_format_json_empty_object() {
    let input = "{}";
    let result = format_json(input);
    assert!(matches!(result, FormatResult::Formatted { .. }));
  }

  #[test]
  fn test_format_json_array() {
    let input = r#"[1, 2, 3]"#;
    let result = format_json(input);
    match result {
      FormatResult::Formatted { content, .. } => {
        assert!(content.contains("1"));
        assert!(content.contains("2"));
        assert!(content.contains("3"));
      }
      FormatResult::Error(e) => panic!("Expected Formatted, got Error: {e}"),
    }
  }

  #[test]
  fn test_format_toml_valid() {
    let input = r#"name = "test"
count = 42"#;
    let result = format_toml(input);
    match result {
      FormatResult::Formatted { content, extension } => {
        assert_eq!(extension, "toml");
        assert!(content.contains("name"));
        assert!(content.contains("test"));
        assert!(content.contains("42"));
      }
      FormatResult::Error(e) => panic!("Expected Formatted, got Error: {e}"),
    }
  }

  #[test]
  fn test_format_toml_with_sections() {
    let input = r#"[package]
name = "my-app"
version = "0.1.0"

[dependencies]
serde = "1.0""#;
    let result = format_toml(input);
    match result {
      FormatResult::Formatted { content, .. } => {
        assert!(content.contains("[package]"));
        assert!(content.contains("[dependencies]"));
      }
      FormatResult::Error(e) => panic!("Expected Formatted, got Error: {e}"),
    }
  }

  #[test]
  fn test_format_toml_invalid() {
    let input = r#"[package
name = incomplete"#;
    let result = format_toml(input);
    match result {
      FormatResult::Error(msg) => {
        assert!(msg.contains("TOML parse error"));
      }
      FormatResult::Formatted { .. } => panic!("Expected Error, got Formatted"),
    }
  }

  #[test]
  fn test_format_structured_json() {
    let result = format_structured(r#"{"a":1}"#, "json");
    assert!(result.is_some());
    assert!(matches!(result.unwrap(), FormatResult::Formatted { .. }));
  }

  #[test]
  fn test_format_structured_toml() {
    let result = format_structured("key = \"value\"", "toml");
    assert!(result.is_some());
    assert!(matches!(result.unwrap(), FormatResult::Formatted { .. }));
  }

  #[test]
  fn test_format_structured_unknown() {
    let result = format_structured("some content", "txt");
    assert!(result.is_none());
  }
}
