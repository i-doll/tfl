# Git Blame

**Branch:** `feat/git-blame`

## Overview
Show blame information in preview pane.

## Tasks
- [x] Add blame view mode for files
- [x] Show commit hash per line
- [x] Show author per line
- [x] Show date per line
- [x] Color by commit (group same commits)
- [ ] Click/enter on line shows full commit
- [x] Toggle blame on/off

## Architecture Notes
- Use `git2` blame API
- Render alongside file content
- Cache blame data

## Keybindings
| Key | Action |
|-----|--------|
| `gb` | Toggle blame view |
| `Enter` | Show commit details |

## Dependencies
```toml
git2 = "0.18"
```

## Testing
- [x] Blame shows for tracked file
- [x] Colors group commits
- [ ] Commit details accessible
- [x] Non-tracked file shows message

## Workflow
- Mark tasks with `[x]` as you complete them
- Run `cargo test` and `cargo clippy` before committing
- Once all tasks and tests pass, create a PR to `main`
