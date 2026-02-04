# Chmod UI

**Branch:** `feat/chmod-ui`

## Overview
Change file permissions interactively from the UI.

## Tasks
- [x] Add `Action::Chmod`
- [x] Create chmod dialog/popup
- [x] Display current permissions (rwx grid)
- [x] Toggle individual bits with keyboard
- [x] Octal input mode
- [x] Apply to single file or selection
- [x] Recursive option for directories
- [x] Show preview of changes

## Architecture Notes
- Use `std::fs::set_permissions()`
- `std::os::unix::fs::PermissionsExt` for mode
- Modal UI with live preview

## Keybindings
| Key | Action |
|-----|--------|
| `Ctrl+p` | Open chmod dialog |
| `r/w/x` | Toggle bits in dialog |
| `Enter` | Apply changes |

## Testing
- [x] Change file permissions
- [x] Octal input works
- [x] Recursive chmod on directory
- [x] Cancel reverts preview

## Workflow
- Mark tasks with `[x]` as you complete them
- Run `cargo test` and `cargo clippy` before committing
- Once all tasks and tests pass, create a PR to `main`
