# File Properties Viewer

**Branch:** `feat/file-properties`

## Overview
Show permissions, size, timestamps, owner/group in a properties panel.

## Tasks
- [x] Add `Action::ShowProperties`
- [x] Create properties popup/panel widget
- [x] Display metadata:
  - [x] Full path
  - [x] Size (human-readable)
  - [x] Permissions (octal + rwx)
  - [x] Owner/group (names if resolvable)
  - [x] Created/modified/accessed times
  - [x] File type / MIME type
  - [x] Symlink target (if applicable)
- [x] `i` or `Enter` on info keybind
- [x] Close with `Esc` or `q`

## Architecture Notes
- Use `std::fs::metadata()` and `std::os::unix::fs::MetadataExt`
- `users` crate for owner/group name resolution
- Modal popup overlaying main UI

## Keybindings
| Key | Action |
|-----|--------|
| `i` | Show properties |
| `Esc`/`q` | Close properties |

## Dependencies
```toml
users = "0.11"
```

## Testing
- [x] Properties show for file
- [x] Properties show for directory
- [x] Symlink shows target
- [x] Popup closes correctly

## Workflow
- Mark tasks with `[x]` as you complete them
- Run `cargo test` and `cargo clippy` before committing
- Once all tasks and tests pass, create a PR to `main`
