# JSON/TOML Pretty Print

**Branch:** `feat/json-toml-preview`

## Overview
Formatted structured data preview with syntax highlighting.

## Tasks
- [x] Detect JSON/TOML files
- [x] Parse and pretty-print JSON
- [x] Parse and pretty-print TOML
- [x] Syntax highlighting for structure
- [ ] Collapsible sections (optional)
- [x] Show parse errors gracefully
- [x] Handle large files (truncate/paginate)

## Architecture Notes
- Use `serde_json` for JSON formatting
- Use `toml` crate for TOML
- Integrate with existing syntax highlighting
- Consider: YAML support too?

## Keybindings
| Key | Action |
|-----|--------|
| `P` | Toggle formatted/raw |
| Standard scroll | Scroll content |

## Dependencies
```toml
serde_json = "1.0"
toml = "0.8"
```

## Testing
- [x] JSON pretty-prints correctly
- [x] TOML pretty-prints correctly
- [x] Invalid JSON shows error
- [x] Large files handled gracefully

## Workflow
- Mark tasks with `[x]` as you complete them
- Run `cargo test` and `cargo clippy` before committing
- Once all tasks and tests pass, create a PR to `main`
