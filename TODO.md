# Dual-Pane Split View

**Branch:** `feat/dual-pane`

## Overview
Norton Commander style side-by-side navigation for easy copy/move.

## Tasks
- [x] Add second `FileTree` instance
- [x] Track active pane (left/right)
- [x] Split layout: two tree panes
- [x] `Tab` switches active pane
- [ ] Copy/move uses inactive pane as destination
- [ ] Sync navigation option (both panes follow)
- [x] Toggle dual-pane mode on/off
- [x] Preview pane behavior in dual mode

## Architecture Notes
- `App` holds `Vec<FileTree>` or struct with two trees
- Active pane index determines which receives input
- Resize handles between panes
- Consider: tabbed panes vs fixed two?

## Keybindings
| Key | Action |
|-----|--------|
| `Tab` | Switch active pane |
| `F6` | Toggle dual-pane mode |

## Testing
- [x] Both panes navigate independently
- [x] Tab switches focus correctly
- [ ] Copy to other pane works
- [x] Toggle mode preserves state

## Workflow
- Mark tasks with `[x]` as you complete them
- Run `cargo test` and `cargo clippy` before committing
- Once all tasks and tests pass, create a PR to `main`
