# Markdown Preview

**Branch:** `feat/markdown-preview`

## Overview
Show rendered markdown in preview pane.

## Tasks
- [x] Add `pulldown-cmark` dependency
- [x] Detect markdown files (.md, .markdown)
- [x] Parse markdown to terminal-friendly format
- [x] Render headings, lists, code blocks, emphasis
- [x] Syntax highlighting in code blocks
- [x] Toggle between raw and rendered
- [x] Handle links (show URL)

## Architecture Notes
- Convert markdown AST to styled spans
- Reuse syntax highlighting from code preview
- Consider: `termimad` for terminal markdown rendering

## Keybindings
| Key | Action |
|-----|--------|
| `m` | Toggle raw/rendered markdown |
| Standard scroll | Scroll content |

## Dependencies
```toml
pulldown-cmark = "0.13"
```

## Testing
- [x] Headings render with style
- [x] Code blocks have highlighting
- [x] Lists render correctly
- [x] Toggle preserves scroll position

## Workflow
- Mark tasks with `[x]` as you complete them
- Run `cargo test` and `cargo clippy` before committing
- Once all tasks and tests pass, create a PR to `main`
