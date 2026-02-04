# Directory History

**Branch:** `feat/directory-history`

## Overview
Back/forward navigation through visited directory locations.

## Tasks
- [x] Add history stack to `App` or `FileTree`
- [x] Push to history on directory change
- [x] `-` (or `Alt+Left`) goes back
- [x] `Alt+Right` goes forward
- [ ] Show history in status or popup
- [x] Limit history size
- [x] Skip duplicates in sequence

## Architecture Notes
- Two stacks: back and forward
- On navigate: push current to back, clear forward
- On back: push current to forward, pop back
- On forward: push current to back, pop forward

## Keybindings
| Key | Action |
|-----|--------|
| `-` | Go back |
| `+` | Go forward |
| `Alt+h` | Show history list |

## Testing
- [x] Back returns to previous dir
- [x] Forward after back works
- [x] New navigation clears forward
- [x] History respects size limit

## Workflow
- Mark tasks with `[x]` as you complete them
- Run `cargo test` and `cargo clippy` before committing
- Once all tasks and tests pass, create a PR to `main`
