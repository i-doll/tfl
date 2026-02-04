# Sorting Options

**Branch:** `feat/sorting-options`

## Overview
Sort by name/size/date/type with toggle for ascending/descending.

## Tasks
- [x] Add `SortField` enum: `Name`, `Size`, `Date`, `Type`
- [x] Add `SortOrder` enum: `Ascending`, `Descending`
- [x] Store current sort in `FileTree` or `App`
- [x] Implement sorting in `expand_dir()` / tree building
- [x] Keybinding to cycle sort field
- [x] Keybinding to toggle sort order
- [x] Show current sort in status bar or header
- [x] Maintain dirs-first regardless of sort field

## Architecture Notes
- Sort applies when building/expanding tree
- Re-sort on toggle (rebuild visible portion)
- Consider: per-directory sort memory?

## Keybindings
| Key | Action |
|-----|--------|
| `s` | Cycle sort field |
| `S` | Toggle ascending/descending |

Note: `open_shell` moved from `s` to `$` to accommodate sort keybindings.

## Testing
- [x] Sort by each field type
- [x] Toggle order reverses list
- [x] Dirs remain first after sort
- [x] Sort persists through navigation

## Workflow
- Mark tasks with `[x]` as you complete them
- Run `cargo test` and `cargo clippy` before committing
- Once all tasks and tests pass, create a PR to `main`
