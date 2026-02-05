# Git Operations

**Branch:** `feat/git-operations`

## Overview
Stage, unstage, and commit from the UI.

## Tasks
- [x] Detect if in git repo
- [x] Show git status indicators in tree
- [x] Stage file(s) with keybind
- [x] Unstage file(s) with keybind
- [x] Commit dialog with message input
- [x] Discard changes (with confirmation)
- [x] Status bar shows branch/dirty state

## Architecture Notes
- Use `git2` crate for git operations
- Async operations for large repos
- Integrate with multi-file selection for batch stage

## Keybindings
| Key | Action |
|-----|--------|
| `ga` | Stage file |
| `gu` | Unstage file |
| `gc` | Commit dialog |
| `gd` | Discard changes |

## Dependencies
```toml
git2 = "0.18"
```

## Testing
- [x] Stage single file
- [x] Unstage file
- [x] Commit with message
- [x] Discard shows confirmation
- [x] Non-repo gracefully handles

## Workflow
- Mark tasks with `[x]` as you complete them
- Run `cargo test` and `cargo clippy` before committing
- Once all tasks and tests pass, create a PR to `main`
