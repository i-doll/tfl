# Trash Support

**Branch:** `feat/trash-support`

## Overview
Move to trash instead of permanent delete, with option to restore.

## Tasks
- [x] Add `trash` crate dependency
- [x] Replace `fs::remove_*` with `trash::delete()`
- [x] Config option: `use_trash: bool` (default true)
- [x] Shift+D for permanent delete (bypass trash)
- [x] Update delete confirmation message
- [ ] Consider: trash listing/restore UI? (out of scope for this PR)

## Architecture Notes
- `trash` crate handles cross-platform trash
- Fallback to permanent delete if trash unavailable
- Async delete for large directories

## Keybindings
| Key | Action |
|-----|--------|
| `d` | Move to trash |
| `D` | Permanent delete |

## Dependencies
```toml
trash = "5"
```

## Testing
- [x] File moves to system trash
- [x] Directory moves to trash
- [x] Permanent delete bypasses trash
- [x] Config toggle works

## Workflow
- Mark tasks with `[x]` as you complete them
- Run `cargo test` and `cargo clippy` before committing
- Once all tasks and tests pass, create a PR to `main`
