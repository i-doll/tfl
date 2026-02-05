use std::path::Path;
use std::sync::mpsc;
use std::thread;

use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

/// Result of async PDF text extraction
pub enum PdfLoadResult {
  Loaded(PdfContent),
  Error(String),
}

/// Extracted content from a PDF file
#[derive(Clone)]
pub struct PdfContent {
  /// Text content per page
  pub pages: Vec<String>,
  /// Total number of pages
  pub page_count: usize,
  /// Whether the PDF might be encrypted or image-only
  pub is_text_available: bool,
}

impl PdfContent {
  /// Get a specific page's text (0-indexed)
  pub fn get_page(&self, page_num: usize) -> Option<&str> {
    self.pages.get(page_num).map(|s| s.as_str())
  }
}

/// Check if a file is a PDF by extension or magic bytes
pub fn is_pdf(path: &Path) -> bool {
  // Check extension first
  if let Some(ext) = path.extension()
    && ext.to_string_lossy().to_lowercase() == "pdf"
  {
    return true;
  }

  // Check magic bytes: %PDF-
  if let Ok(data) = std::fs::read(path)
    && data.len() >= 5 && &data[..5] == b"%PDF-"
  {
    return true;
  }

  false
}

/// Load PDF text content asynchronously
pub fn load_pdf_async(path: &Path) -> mpsc::Receiver<PdfLoadResult> {
  let (tx, rx) = mpsc::channel();
  let path = path.to_path_buf();

  thread::spawn(move || {
    let result = extract_pdf_text(&path);
    let _ = tx.send(result);
  });

  rx
}

/// Extract text from a PDF file
fn extract_pdf_text(path: &Path) -> PdfLoadResult {
  match pdf_extract::extract_text(path) {
    Ok(text) => {
      // Split text by form feed character (page break) or estimate pages
      let pages: Vec<String> = if text.contains('\x0C') {
        text.split('\x0C').map(|s| s.to_string()).collect()
      } else {
        // If no page breaks, treat as single page
        vec![text.clone()]
      };

      let page_count = pages.len();
      let is_text_available = !text.trim().is_empty();

      PdfLoadResult::Loaded(PdfContent {
        pages,
        page_count,
        is_text_available,
      })
    }
    Err(e) => {
      let msg = e.to_string();
      // Check for common error patterns
      if msg.contains("encrypted") || msg.contains("password") {
        PdfLoadResult::Error("PDF is encrypted or password-protected".to_string())
      } else {
        PdfLoadResult::Error(format!("Failed to extract PDF text: {msg}"))
      }
    }
  }
}

/// Render PDF text content as styled lines for preview
pub fn render_pdf_content(content: &PdfContent, current_page: usize) -> Vec<Line<'static>> {
  let mut lines = Vec::new();

  // Header line with page info
  let header = Line::from(vec![
    Span::styled(
      format!(" PDF - Page {}/{}", current_page + 1, content.page_count),
      Style::default().fg(Color::Indexed(75)),
    ),
  ]);
  lines.push(header);
  lines.push(Line::from("")); // Empty line after header

  if !content.is_text_available {
    lines.push(Line::from(Span::styled(
      " This PDF appears to contain only images or scanned content.",
      Style::default().fg(Color::Indexed(214)),
    )));
    lines.push(Line::from(Span::styled(
      " Text extraction is not available.",
      Style::default().fg(Color::Indexed(214)),
    )));
    return lines;
  }

  // Get current page text
  if let Some(page_text) = content.get_page(current_page) {
    for (line_num, line_text) in page_text.lines().enumerate() {
      let line_number = format!("{:>4} ", line_num + 1);
      lines.push(Line::from(vec![
        Span::styled(line_number, Style::default().fg(Color::DarkGray)),
        Span::raw(line_text.to_string()),
      ]));
    }
  }

  lines
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::fs;

  #[test]
  fn test_is_pdf_by_extension() {
    let dir = std::env::temp_dir().join("tfl_pdf_test_ext");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();

    let pdf_file = dir.join("test.pdf");
    fs::write(&pdf_file, "fake pdf content").unwrap();
    assert!(is_pdf(&pdf_file));

    let txt_file = dir.join("test.txt");
    fs::write(&txt_file, "text content").unwrap();
    assert!(!is_pdf(&txt_file));

    let _ = fs::remove_dir_all(&dir);
  }

  #[test]
  fn test_is_pdf_by_magic_bytes() {
    let dir = std::env::temp_dir().join("tfl_pdf_test_magic");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();

    // File without .pdf extension but with PDF magic bytes
    let pdf_file = dir.join("document");
    fs::write(&pdf_file, b"%PDF-1.4 fake pdf content").unwrap();
    assert!(is_pdf(&pdf_file));

    let non_pdf_file = dir.join("other");
    fs::write(&non_pdf_file, b"not a pdf").unwrap();
    assert!(!is_pdf(&non_pdf_file));

    let _ = fs::remove_dir_all(&dir);
  }

  #[test]
  fn test_pdf_content_get_page() {
    let content = PdfContent {
      pages: vec!["Page 1 text".to_string(), "Page 2 text".to_string()],
      page_count: 2,
      is_text_available: true,
    };

    assert_eq!(content.get_page(0), Some("Page 1 text"));
    assert_eq!(content.get_page(1), Some("Page 2 text"));
    assert_eq!(content.get_page(2), None);
  }

  #[test]
  fn test_render_pdf_content_with_text() {
    let content = PdfContent {
      pages: vec!["Line one\nLine two\nLine three".to_string()],
      page_count: 1,
      is_text_available: true,
    };

    let lines = render_pdf_content(&content, 0);

    // Should have header + empty line + 3 content lines = 5 lines
    assert_eq!(lines.len(), 5);

    // Check header contains page info
    let header_text: String = lines[0].spans.iter().map(|s| s.content.as_ref()).collect();
    assert!(header_text.contains("Page 1/1"));
  }

  #[test]
  fn test_render_pdf_content_image_only() {
    let content = PdfContent {
      pages: vec!["".to_string()],
      page_count: 1,
      is_text_available: false,
    };

    let lines = render_pdf_content(&content, 0);

    // Should have header + empty + 2 warning lines = 4 lines
    assert!(lines.len() >= 4);

    // Check for warning message
    let all_text: String = lines.iter()
      .flat_map(|l| l.spans.iter())
      .map(|s| s.content.as_ref())
      .collect();
    assert!(all_text.contains("images") || all_text.contains("scanned"));
  }

  #[test]
  fn test_render_pdf_multipage() {
    let content = PdfContent {
      pages: vec!["Page 1".to_string(), "Page 2".to_string(), "Page 3".to_string()],
      page_count: 3,
      is_text_available: true,
    };

    // Check page 2 (0-indexed as 1)
    let lines = render_pdf_content(&content, 1);
    let header_text: String = lines[0].spans.iter().map(|s| s.content.as_ref()).collect();
    assert!(header_text.contains("Page 2/3"));
  }

  #[test]
  fn test_load_pdf_async_nonexistent() {
    let rx = load_pdf_async(Path::new("/nonexistent/file.pdf"));
    let result = rx.recv().unwrap();
    assert!(matches!(result, PdfLoadResult::Error(_)));
  }
}
