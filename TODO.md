# Multi-file Selection & Batch Operations

**Branch:** `feat/multi-file-selection`

## Overview
Select multiple files with space, operate on all selected files at once.

## Tasks
- [x] Add `selected: bool` field to `FileEntry`
- [x] Track selected entries in `FileTree` (or `App`)
- [x] `Space` toggles selection on current entry
- [x] Visual indicator for selected files (highlight, marker)
- [x] `V` enters visual selection mode (range select)
- [x] Batch operations work on selection if non-empty:
  - [x] Delete all selected
  - [x] Move/copy all selected
  - [x] Yank all selected paths
- [x] `Esc` or keybind to clear selection
- [x] Selection count in status bar

## Architecture Notes
- Selection state lives alongside flat vec tree
- Batch ops iterate `entries.iter().filter(|e| e.selected)`
- Visual mode tracks anchor index for range selection

## Keybindings
| Key | Action |
|-----|--------|
| `Space` | Toggle selection |
| `V` | Visual selection mode |
| `Tab` | Toggle expand (moved from Space) |
| `Esc` | Clear selection (in visual mode) |

## Testing
- [x] Toggle selection on single file
- [x] Visual mode range selection
- [x] Batch delete with confirmation
- [x] Selection persists across tree navigation

## Workflow
- Mark tasks with `[x]` as you complete them
- Run `cargo test` and `cargo clippy` before committing
- Once all tasks and tests pass, create a PR to `main`
