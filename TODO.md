# Duplicate File

**Branch:** `feat/duplicate-file`

## Overview
Quick duplicate without copy/paste workflow.

## Tasks
- [x] Add `Action::Duplicate`
- [x] Generate default name: `file_copy.ext` or `file (1).ext`
- [x] Prompt for new name (pre-filled)
- [x] Handle name conflicts
- [x] Duplicate directories recursively
- [ ] Show progress for large files/dirs

## Architecture Notes
- Use `fs::copy()` for files
- Recursive copy for directories
- Same async pattern as other file ops

## Keybindings
| Key | Action |
|-----|--------|
| `D` | Duplicate (if not used for delete) |
| `Ctrl+d` | Duplicate |

## Testing
- [x] Duplicate file creates copy
- [x] Duplicate dir copies recursively
- [x] Name prompt allows editing
- [x] Conflicts handled gracefully

## Workflow
- Mark tasks with `[x]` as you complete them
- Run `cargo test` and `cargo clippy` before committing
- Once all tasks and tests pass, create a PR to `main`
