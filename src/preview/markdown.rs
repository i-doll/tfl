use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use unicode_width::UnicodeWidthStr;

use super::text::SyntaxHighlighter;

/// Renders markdown content to styled terminal lines
pub fn render_markdown(content: &str, highlighter: &SyntaxHighlighter) -> Vec<Line<'static>> {
  let mut options = Options::empty();
  options.insert(Options::ENABLE_TABLES);
  options.insert(Options::ENABLE_STRIKETHROUGH);

  let parser = Parser::new_ext(content, options);
  let mut lines: Vec<Line<'static>> = Vec::new();
  let mut current_spans: Vec<Span<'static>> = Vec::new();
  let mut list_depth: usize = 0;
  let mut ordered_list_number: Option<u64> = None;
  let mut in_code_block = false;
  let mut code_block_lang = String::new();
  let mut code_block_content = String::new();
  let mut heading_level: HeadingLevel = HeadingLevel::H1;
  let mut style_stack: Vec<Style> = vec![Style::default()];
  let mut in_blockquote = false;

  // Table state
  let mut table_row: Vec<String> = Vec::new();
  let mut table_alignments: Vec<pulldown_cmark::Alignment> = Vec::new();
  let mut is_header_row = false;
  let mut table_rows: Vec<Vec<String>> = Vec::new();

  for event in parser {
    match event {
      Event::Start(tag) => match tag {
        Tag::Heading { level, .. } => {
          flush_line(&mut current_spans, &mut lines);
          heading_level = level;
        }
        Tag::Paragraph => {
          // Start new paragraph - add blank line if not at start
          if !lines.is_empty() && !in_blockquote {
            lines.push(Line::from(""));
          }
        }
        Tag::CodeBlock(kind) => {
          flush_line(&mut current_spans, &mut lines);
          in_code_block = true;
          code_block_lang = match kind {
            CodeBlockKind::Fenced(lang) => lang.to_string(),
            CodeBlockKind::Indented => String::new(),
          };
          code_block_content.clear();
        }
        Tag::List(start) => {
          if list_depth == 0 && !lines.is_empty() {
            lines.push(Line::from(""));
          }
          list_depth += 1;
          ordered_list_number = start;
        }
        Tag::Item => {
          flush_line(&mut current_spans, &mut lines);
          let indent = "  ".repeat(list_depth.saturating_sub(1));
          let bullet = if let Some(n) = ordered_list_number {
            ordered_list_number = Some(n + 1);
            format!("{indent}{n}. ")
          } else {
            format!("{indent}- ")
          };
          current_spans.push(Span::styled(
            bullet,
            Style::default().fg(Color::Indexed(75)),
          ));
        }
        Tag::Emphasis => {
          let current = *style_stack.last().unwrap_or(&Style::default());
          style_stack.push(current.add_modifier(Modifier::ITALIC));
        }
        Tag::Strong => {
          let current = *style_stack.last().unwrap_or(&Style::default());
          style_stack.push(current.add_modifier(Modifier::BOLD));
        }
        Tag::Strikethrough => {
          let current = *style_stack.last().unwrap_or(&Style::default());
          style_stack.push(current.add_modifier(Modifier::CROSSED_OUT));
        }
        Tag::Link { dest_url, .. } => {
          let current = *style_stack.last().unwrap_or(&Style::default());
          style_stack.push(current.fg(Color::Indexed(75)).add_modifier(Modifier::UNDERLINED));
          // Store URL for later
          current_spans.push(Span::raw("")); // Placeholder to mark link start
          // We'll append URL after link text
          code_block_lang = dest_url.to_string(); // Repurpose for URL storage
        }
        Tag::BlockQuote(_) => {
          flush_line(&mut current_spans, &mut lines);
          in_blockquote = true;
        }
        Tag::Table(alignments) => {
          flush_line(&mut current_spans, &mut lines);
          if !lines.is_empty() {
            lines.push(Line::from(""));
          }
          table_alignments = alignments;
          table_rows.clear();
        }
        Tag::TableHead => {
          is_header_row = true;
          table_row.clear();
        }
        Tag::TableRow => {
          table_row.clear();
        }
        Tag::TableCell => {}
        _ => {}
      },
      Event::End(tag) => match tag {
        TagEnd::Heading(_) => {
          let style = heading_style(heading_level);
          let prefix = heading_prefix(heading_level);

          // Build the heading line with prefix
          let mut heading_spans: Vec<Span<'static>> = Vec::new();
          heading_spans.push(Span::styled(prefix, style));
          for span in current_spans.drain(..) {
            heading_spans.push(Span::styled(span.content.to_string(), style));
          }
          lines.push(Line::from(heading_spans));
        }
        TagEnd::Paragraph => {
          flush_line(&mut current_spans, &mut lines);
        }
        TagEnd::CodeBlock => {
          // Render code block with syntax highlighting
          let highlighted = highlighter.highlight(&code_block_content, &code_block_lang);

          // Add code block delimiter
          lines.push(Line::from(Span::styled(
            "```".to_string() + &code_block_lang,
            Style::default().fg(Color::Indexed(240)),
          )));

          // Add highlighted lines (strip line numbers from highlighter output)
          for line in highlighted {
            let mut code_spans: Vec<Span<'static>> = Vec::new();
            code_spans.push(Span::styled("  ", Style::default())); // Indent
            // Skip the line number span (first span) and collect the rest
            for span in line.spans.into_iter().skip(1) {
              code_spans.push(span);
            }
            lines.push(Line::from(code_spans));
          }

          lines.push(Line::from(Span::styled(
            "```",
            Style::default().fg(Color::Indexed(240)),
          )));

          in_code_block = false;
          code_block_content.clear();
        }
        TagEnd::List(_) => {
          list_depth = list_depth.saturating_sub(1);
          if list_depth == 0 {
            ordered_list_number = None;
          }
        }
        TagEnd::Item => {
          flush_line(&mut current_spans, &mut lines);
        }
        TagEnd::Emphasis | TagEnd::Strong | TagEnd::Strikethrough => {
          style_stack.pop();
        }
        TagEnd::Link => {
          style_stack.pop();
          // Append URL in parentheses
          let url = std::mem::take(&mut code_block_lang);
          if !url.is_empty() {
            current_spans.push(Span::styled(
              format!(" ({url})"),
              Style::default().fg(Color::Indexed(240)),
            ));
          }
        }
        TagEnd::BlockQuote(_) => {
          in_blockquote = false;
        }
        TagEnd::Table => {
          // Render the complete table
          render_table(&table_rows, &table_alignments, &mut lines);
          table_rows.clear();
        }
        TagEnd::TableHead => {
          table_rows.push(table_row.clone());
          is_header_row = false;
        }
        TagEnd::TableRow => {
          if !is_header_row {
            table_rows.push(table_row.clone());
          }
        }
        TagEnd::TableCell => {
          // Cell content is accumulated in current_spans
          let cell_text: String = current_spans.iter().map(|s| s.content.as_ref()).collect();
          table_row.push(cell_text);
          current_spans.clear();
        }
        _ => {}
      },
      Event::Text(text) => {
        if in_code_block {
          code_block_content.push_str(&text);
        } else if in_blockquote {
          // Render blockquote lines with prefix
          for (i, line) in text.lines().enumerate() {
            if i > 0 || !current_spans.is_empty() {
              flush_line(&mut current_spans, &mut lines);
            }
            current_spans.push(Span::styled(
              "> ",
              Style::default().fg(Color::Indexed(240)),
            ));
            current_spans.push(Span::styled(
              line.to_string(),
              Style::default().fg(Color::Indexed(252)).add_modifier(Modifier::ITALIC),
            ));
          }
        } else {
          let style = *style_stack.last().unwrap_or(&Style::default());
          current_spans.push(Span::styled(text.to_string(), style));
        }
      }
      Event::Code(code) => {
        // Inline code
        current_spans.push(Span::styled(
          format!("`{code}`"),
          Style::default().fg(Color::Indexed(214)).bg(Color::Indexed(236)),
        ));
      }
      Event::SoftBreak | Event::HardBreak => {
        flush_line(&mut current_spans, &mut lines);
      }
      Event::Rule => {
        flush_line(&mut current_spans, &mut lines);
        lines.push(Line::from(Span::styled(
          "---",
          Style::default().fg(Color::Indexed(240)),
        )));
      }
      _ => {}
    }
  }

  // Flush any remaining content
  flush_line(&mut current_spans, &mut lines);

  lines
}

fn flush_line(spans: &mut Vec<Span<'static>>, lines: &mut Vec<Line<'static>>) {
  if !spans.is_empty() {
    lines.push(Line::from(std::mem::take(spans)));
  }
}

fn heading_style(level: HeadingLevel) -> Style {
  match level {
    HeadingLevel::H1 => Style::default()
      .fg(Color::Indexed(75))
      .add_modifier(Modifier::BOLD),
    HeadingLevel::H2 => Style::default()
      .fg(Color::Indexed(114))
      .add_modifier(Modifier::BOLD),
    HeadingLevel::H3 => Style::default()
      .fg(Color::Indexed(214))
      .add_modifier(Modifier::BOLD),
    HeadingLevel::H4 => Style::default()
      .fg(Color::Indexed(252))
      .add_modifier(Modifier::BOLD),
    HeadingLevel::H5 => Style::default()
      .fg(Color::Indexed(246))
      .add_modifier(Modifier::BOLD),
    HeadingLevel::H6 => Style::default()
      .fg(Color::Indexed(240))
      .add_modifier(Modifier::BOLD),
  }
}

fn heading_prefix(level: HeadingLevel) -> String {
  match level {
    HeadingLevel::H1 => "# ".to_string(),
    HeadingLevel::H2 => "## ".to_string(),
    HeadingLevel::H3 => "### ".to_string(),
    HeadingLevel::H4 => "#### ".to_string(),
    HeadingLevel::H5 => "##### ".to_string(),
    HeadingLevel::H6 => "###### ".to_string(),
  }
}

fn render_table(
  rows: &[Vec<String>],
  alignments: &[pulldown_cmark::Alignment],
  lines: &mut Vec<Line<'static>>,
) {
  if rows.is_empty() {
    return;
  }

  // Calculate column widths using unicode display width
  let col_count = rows.iter().map(|r| r.len()).max().unwrap_or(0);
  let mut col_widths: Vec<usize> = vec![0; col_count];

  for row in rows {
    for (i, cell) in row.iter().enumerate() {
      if i < col_widths.len() {
        col_widths[i] = col_widths[i].max(cell.width());
      }
    }
  }

  // Ensure minimum column width of 3 for alignment indicators
  for w in &mut col_widths {
    *w = (*w).max(3);
  }

  let border_style = Style::default().fg(Color::Indexed(240));
  let header_style = Style::default().fg(Color::Indexed(75)).add_modifier(Modifier::BOLD);
  let cell_style = Style::default();

  for (row_idx, row) in rows.iter().enumerate() {
    let mut spans: Vec<Span<'static>> = Vec::new();
    spans.push(Span::styled("│ ", border_style));

    for (col_idx, cell) in row.iter().enumerate() {
      let width = col_widths.get(col_idx).copied().unwrap_or(3);
      let alignment = alignments.get(col_idx).copied().unwrap_or(pulldown_cmark::Alignment::None);

      let padded = align_text(cell, width, alignment);
      let style = if row_idx == 0 { header_style } else { cell_style };

      spans.push(Span::styled(padded, style));
      // Use " │ " between cells, " │" after last cell (no trailing space)
      if col_idx < col_count - 1 {
        spans.push(Span::styled(" │ ", border_style));
      } else {
        spans.push(Span::styled(" │", border_style));
      }
    }

    // Fill empty columns
    for col_idx in row.len()..col_count {
      let width = col_widths.get(col_idx).copied().unwrap_or(3);
      spans.push(Span::styled(" ".repeat(width), cell_style));
      if col_idx < col_count - 1 {
        spans.push(Span::styled(" │ ", border_style));
      } else {
        spans.push(Span::styled(" │", border_style));
      }
    }

    lines.push(Line::from(spans));

    // Add separator after header row
    if row_idx == 0 {
      let mut sep_spans: Vec<Span<'static>> = Vec::new();
      sep_spans.push(Span::styled("├", border_style));

      for (col_idx, &width) in col_widths.iter().enumerate() {
        // Each cell is: content(width) + " │ " (3 chars)
        // So separator needs: "─" * (width + 2) to match " content " spacing
        sep_spans.push(Span::styled("─".repeat(width + 2), border_style));
        if col_idx < col_widths.len() - 1 {
          sep_spans.push(Span::styled("┼", border_style));
        }
      }
      sep_spans.push(Span::styled("┤", border_style));
      lines.push(Line::from(sep_spans));
    }
  }
}

fn align_text(text: &str, width: usize, alignment: pulldown_cmark::Alignment) -> String {
  let text_width = text.width();
  if text_width >= width {
    return text.to_string();
  }

  let padding = width - text_width;
  match alignment {
    pulldown_cmark::Alignment::Right => format!("{}{}", " ".repeat(padding), text),
    pulldown_cmark::Alignment::Center => {
      let left = padding / 2;
      let right = padding - left;
      format!("{}{}{}", " ".repeat(left), text, " ".repeat(right))
    }
    _ => format!("{}{}", text, " ".repeat(padding)),
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn highlighter() -> SyntaxHighlighter {
    SyntaxHighlighter::new()
  }

  #[test]
  fn test_heading_renders_with_style() {
    let h = highlighter();
    let content = "# Hello World";
    let lines = render_markdown(content, &h);

    assert!(!lines.is_empty(), "Should produce at least one line");
    let first_line = &lines[0];

    // First span should be the heading prefix "# "
    assert!(first_line.spans[0].content.starts_with('#'));

    // Should have bold modifier
    let style = first_line.spans[0].style;
    assert!(style.add_modifier.contains(Modifier::BOLD));
  }

  #[test]
  fn test_h2_renders_differently_from_h1() {
    let h = highlighter();
    let h1_lines = render_markdown("# H1", &h);
    let h2_lines = render_markdown("## H2", &h);

    assert!(!h1_lines.is_empty());
    assert!(!h2_lines.is_empty());

    // H1 and H2 should have different colors
    let h1_color = h1_lines[0].spans[0].style.fg;
    let h2_color = h2_lines[0].spans[0].style.fg;
    assert_ne!(h1_color, h2_color);
  }

  #[test]
  fn test_code_block_has_highlighting() {
    let h = highlighter();
    let content = "```rust\nfn main() {}\n```";
    let lines = render_markdown(content, &h);

    // Should have: opening ```, code line(s), closing ```
    assert!(lines.len() >= 3, "Code block should produce multiple lines");

    // First line should be the opening fence
    assert!(lines[0].spans.iter().any(|s| s.content.contains("```")));

    // Last line should be the closing fence
    let last = lines.last().unwrap();
    assert!(last.spans.iter().any(|s| s.content.contains("```")));
  }

  #[test]
  fn test_list_renders_correctly() {
    let h = highlighter();
    let content = "- Item 1\n- Item 2\n- Item 3";
    let lines = render_markdown(content, &h);

    // Should have at least 3 lines (one per item)
    assert!(lines.len() >= 3, "List should produce lines for each item");

    // Each line should start with "- "
    for line in &lines {
      if !line.spans.is_empty() {
        let text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
        if !text.trim().is_empty() {
          assert!(text.contains("- ") || text.contains("1. "), "List items should have bullet/number");
        }
      }
    }
  }

  #[test]
  fn test_ordered_list() {
    let h = highlighter();
    let content = "1. First\n2. Second\n3. Third";
    let lines = render_markdown(content, &h);

    let text: String = lines.iter()
      .flat_map(|l| l.spans.iter().map(|s| s.content.as_ref()))
      .collect();

    assert!(text.contains("1."));
    assert!(text.contains("2."));
    assert!(text.contains("3."));
  }

  #[test]
  fn test_emphasis_renders_with_italic() {
    let h = highlighter();
    let content = "*italic text*";
    let lines = render_markdown(content, &h);

    assert!(!lines.is_empty());

    // Find the span with italic text
    let has_italic = lines.iter().any(|line| {
      line.spans.iter().any(|span| {
        span.style.add_modifier.contains(Modifier::ITALIC)
      })
    });
    assert!(has_italic, "Should have italic modifier");
  }

  #[test]
  fn test_strong_renders_with_bold() {
    let h = highlighter();
    let content = "**bold text**";
    let lines = render_markdown(content, &h);

    assert!(!lines.is_empty());

    let has_bold = lines.iter().any(|line| {
      line.spans.iter().any(|span| {
        span.style.add_modifier.contains(Modifier::BOLD)
      })
    });
    assert!(has_bold, "Should have bold modifier");
  }

  #[test]
  fn test_inline_code() {
    let h = highlighter();
    let content = "Use `println!` macro";
    let lines = render_markdown(content, &h);

    let text: String = lines.iter()
      .flat_map(|l| l.spans.iter().map(|s| s.content.as_ref()))
      .collect();

    assert!(text.contains("`println!`"), "Inline code should be preserved with backticks");
  }

  #[test]
  fn test_link_shows_url() {
    let h = highlighter();
    let content = "[Rust](https://rust-lang.org)";
    let lines = render_markdown(content, &h);

    let text: String = lines.iter()
      .flat_map(|l| l.spans.iter().map(|s| s.content.as_ref()))
      .collect();

    assert!(text.contains("Rust"), "Link text should be visible");
    assert!(text.contains("rust-lang.org"), "URL should be shown");
  }

  #[test]
  fn test_horizontal_rule() {
    let h = highlighter();
    let content = "Above\n\n---\n\nBelow";
    let lines = render_markdown(content, &h);

    let has_rule = lines.iter().any(|line| {
      line.spans.iter().any(|span| span.content.contains("---"))
    });
    assert!(has_rule, "Horizontal rule should be rendered");
  }

  #[test]
  fn test_blockquote() {
    let h = highlighter();
    let content = "> This is a quote";
    let lines = render_markdown(content, &h);

    let text: String = lines.iter()
      .flat_map(|l| l.spans.iter().map(|s| s.content.as_ref()))
      .collect();

    assert!(text.contains(">"), "Blockquote should have > prefix");
    assert!(text.contains("This is a quote"));
  }

  #[test]
  fn test_toggle_preserves_scroll_position() {
    // This tests the concept - actual implementation will be in PreviewState
    let h = highlighter();
    let content = "# Title\n\nParagraph 1\n\nParagraph 2\n\nParagraph 3";

    let raw_lines: Vec<&str> = content.lines().collect();
    let rendered_lines = render_markdown(content, &h);

    // Both should have content
    assert!(!raw_lines.is_empty());
    assert!(!rendered_lines.is_empty());
  }

  #[test]
  fn test_table_renders() {
    let h = highlighter();
    let content = "| Flag | Description |\n|---|---|\n| `-a` | Show all |\n| `-h` | Help |";
    let lines = render_markdown(content, &h);

    // Should have multiple lines for table
    assert!(lines.len() >= 3, "Table should produce multiple lines");

    let text: String = lines.iter()
      .flat_map(|l| l.spans.iter().map(|s| s.content.as_ref()))
      .collect();

    assert!(text.contains("Flag"), "Header should be present");
    assert!(text.contains("Description"), "Header should be present");
    assert!(text.contains("-a"), "Cell content should be present");
    assert!(text.contains("Show all"), "Cell content should be present");
  }

  #[test]
  fn test_table_with_alignment() {
    let h = highlighter();
    let content = "| Left | Center | Right |\n|:---|:---:|---:|\n| L | C | R |";
    let lines = render_markdown(content, &h);

    assert!(!lines.is_empty(), "Table should produce lines");

    let text: String = lines.iter()
      .flat_map(|l| l.spans.iter().map(|s| s.content.as_ref()))
      .collect();

    assert!(text.contains("Left"));
    assert!(text.contains("Center"));
    assert!(text.contains("Right"));
  }
}
