# Symlink Creation

**Branch:** `feat/symlink-creation`

## Overview
Create symbolic links from the UI.

## Tasks
- [x] Add `Action::NewSymlinkStart`
- [x] Prompt for link name/location
- [x] Option: symlink at current location pointing to selected
- [ ] Option: symlink elsewhere pointing to selected
- [x] Show symlink indicator in tree (existing)
- [x] Handle existing file conflicts

## Architecture Notes
- Use `std::os::unix::fs::symlink()`
- Two modes: "link here" vs "link to here"
- Consider relative vs absolute paths

## Keybindings
| Key | Action |
|-----|--------|
| `Ctrl+l` | Create symlink |
| `L` | Create symlink (alternate) |

## Testing
- [x] Create symlink to file
- [x] Create symlink to directory
- [x] Symlink appears with indicator
- [x] Conflict handling works

## Workflow
- Mark tasks with `[x]` as you complete them
- Run `cargo test` and `cargo clippy` before committing
- Once all tasks and tests pass, create a PR to `main`
