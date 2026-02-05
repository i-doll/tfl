# Find by Size

**Branch:** `feat/find-by-size`

## Overview
Filter files by size range.

## Tasks
- [x] Add size filter mode
- [x] Parse size expressions (e.g., `>1M`, `<100K`, `1M-10M`)
- [x] Filter tree entries by size
- [x] Show matching files
- [x] Combine with name search
- [x] Human-readable size display

## Architecture Notes
- Filter applies to flat tree entries
- Parse units: B, K, M, G
- Support operators: `<`, `>`, `=`, ranges

## Keybindings
| Key | Action |
|-----|--------|
| `Ctrl+s` | Size filter mode |
| Enter | Apply filter |
| Esc | Clear filter |

## Size Expression Examples
- `>1M` - larger than 1 MB
- `<100K` - smaller than 100 KB
- `1M-10M` - between 1 and 10 MB
- `=0` - empty files

## Testing
- [x] Greater than filter works
- [x] Less than filter works
- [x] Range filter works
- [x] Combined with name filter

## Workflow
- Mark tasks with `[x]` as you complete them
- Run `cargo test` and `cargo clippy` before committing
- Once all tasks and tests pass, create a PR to `main`
