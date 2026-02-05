# Find by Date

**Branch:** `feat/find-by-date`

## Overview
Filter files by modification time.

## Tasks
- [x] Add date filter mode
- [x] Parse date expressions (relative and absolute)
- [x] Filter by modified time
- [x] Support created/accessed time
- [x] Combine with other filters
- [x] Show matching time in results

## Architecture Notes
- Use `chrono` for date parsing
- Relative: `today`, `yesterday`, `7d`, `1w`, `1m`
- Absolute: `2024-01-15`, `>2024-01-01`

## Keybindings
| Key | Action |
|-----|--------|
| `Ctrl+d` | Date filter mode |
| Enter | Apply filter |
| Esc | Clear filter |

## Date Expression Examples
- `today` - modified today
- `7d` - last 7 days
- `>2024-01-01` - after date
- `<1w` - older than 1 week

## Dependencies
```toml
chrono = "0.4"
```

## Testing
- [x] Relative dates work
- [x] Absolute dates work
- [x] Date ranges work
- [x] Combined filters work

## Workflow
- Mark tasks with `[x]` as you complete them
- Run `cargo test` and `cargo clippy` before committing
- Once all tasks and tests pass, create a PR to `main`
