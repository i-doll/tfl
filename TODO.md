# Undo/Redo

**Branch:** `feat/undo-redo`

## Overview
Undo accidental file operations (rename, move, delete from trash).

## Tasks
- [x] Define `UndoAction` enum for reversible operations
- [x] Implement undo stack in `App`
- [x] Track operations:
  - [x] Rename: store old/new path
  - [x] Move: store source/dest
  - [x] Delete (trash): store trash location
  - [x] Copy: store created path (undo = delete)
- [x] `u` triggers undo
- [x] `Ctrl+r` triggers redo
- [x] Show undo feedback in status bar
- [x] Limit stack size (config?)

## Architecture Notes
- Undo stack is `Vec<UndoAction>`
- Redo stack for undone actions
- Clear redo on new action
- Some ops may not be undoable (permanent delete)

## Keybindings
| Key | Action |
|-----|--------|
| `u` | Undo |
| `Ctrl+r` | Redo |

## Testing
- [x] Undo rename restores old name
- [x] Undo move returns file
- [x] Undo delete restores from trash
- [x] Redo re-applies action
- [x] Stack respects size limit

## Workflow
- Mark tasks with `[x]` as you complete them
- Run `cargo test` and `cargo clippy` before committing
- Once all tasks and tests pass, create a PR to `main`
