use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

const BYTES_PER_LINE: usize = 16;

pub fn hex_dump(data: &[u8]) -> Vec<Line<'static>> {
  let mut lines = Vec::new();

  for (i, chunk) in data.chunks(BYTES_PER_LINE).enumerate() {
    let offset = format!("{:08x}  ", i * BYTES_PER_LINE);
    let mut hex_part = String::new();
    let mut ascii_part = String::new();

    for (j, byte) in chunk.iter().enumerate() {
      hex_part.push_str(&format!("{byte:02x} "));
      if j == 7 {
        hex_part.push(' ');
      }
      ascii_part.push(if byte.is_ascii_graphic() || *byte == b' ' {
        *byte as char
      } else {
        '.'
      });
    }

    // Pad if chunk is shorter than BYTES_PER_LINE
    let missing = BYTES_PER_LINE - chunk.len();
    for _ in 0..missing {
      hex_part.push_str("   ");
    }
    if chunk.len() <= 8 {
      hex_part.push(' ');
    }

    let line = Line::from(vec![
      Span::styled(offset, Style::default().fg(Color::DarkGray)),
      Span::styled(hex_part, Style::default().fg(Color::Indexed(75))),
      Span::styled(" |".to_string(), Style::default().fg(Color::DarkGray)),
      Span::styled(ascii_part, Style::default().fg(Color::Indexed(150))),
      Span::styled("|".to_string(), Style::default().fg(Color::DarkGray)),
    ]);

    lines.push(line);
  }

  lines
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_hex_dump_empty() {
    let lines = hex_dump(&[]);
    assert!(lines.is_empty());
  }

  #[test]
  fn test_hex_dump_single_line() {
    let data = b"Hello, World!";
    let lines = hex_dump(data);
    assert_eq!(lines.len(), 1);
    // Offset should be 00000000
    assert!(lines[0].spans[0].content.starts_with("00000000"));
  }

  #[test]
  fn test_hex_dump_multiple_lines() {
    let data = vec![0u8; 32];
    let lines = hex_dump(&data);
    assert_eq!(lines.len(), 2);
    assert!(lines[1].spans[0].content.starts_with("00000010"));
  }

  #[test]
  fn test_hex_dump_ascii_display() {
    let data = b"AB\x00\xff";
    let lines = hex_dump(data);
    // The ascii part should show "AB.."
    let ascii = &lines[0].spans[3].content;
    assert!(ascii.starts_with("AB.."));
  }

  #[test]
  fn test_hex_dump_partial_line() {
    let data = vec![0x41u8; 5]; // "AAAAA"
    let lines = hex_dump(&data);
    assert_eq!(lines.len(), 1);
    // ASCII part should be "AAAAA"
    assert_eq!(lines[0].spans[3].content, "AAAAA");
  }
}
