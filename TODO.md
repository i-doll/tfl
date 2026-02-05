# PDF Preview

**Branch:** `feat/pdf-preview`

## Overview
Extract and display text from PDF files in preview pane.

## Tasks
- [x] Add `pdf-extract` or `lopdf` dependency
- [x] Detect PDF files by extension/magic
- [x] Extract text content from PDF
- [x] Display in preview pane with scrolling
- [x] Handle multi-page PDFs
- [x] Show page numbers/navigation
- [x] Fallback message for encrypted/image PDFs

## Architecture Notes
- Integrate with `PreviewState` and cache
- Async text extraction (can be slow)
- Consider: `pdftotext` external command as fallback?

## Keybindings
| Key | Action |
|-----|--------|
| `n/p` | Next/prev page (in preview) |
| Standard scroll | Scroll text |

## Dependencies
```toml
pdf-extract = "0.7"
# or
lopdf = "0.31"
```

## Testing
- [x] PDF text displays in preview
- [x] Multi-page navigation works
- [x] Large PDFs don't block UI
- [x] Encrypted PDFs show message

## Workflow
- Mark tasks with `[x]` as you complete them
- Run `cargo test` and `cargo clippy` before committing
- Once all tasks and tests pass, create a PR to `main`
