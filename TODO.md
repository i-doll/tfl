# Git Diff Preview

**Branch:** `feat/git-diff-preview`

## Overview
Show file diffs for modified files in preview pane.

## Tasks
- [x] Detect modified files (git status)
- [x] Generate diff for preview
- [x] Syntax highlight diff output
- [x] Show added/removed lines with color
- [x] Line number gutter
- [x] Navigate between hunks
- [ ] Toggle unified/split diff view

## Architecture Notes
- Use `git2` for diff generation
- Integrate with existing preview system
- Cache diffs like other previews

## Keybindings
| Key | Action |
|-----|--------|
| `d` | Show diff (on modified file) |
| `]c` | Next hunk |
| `[c` | Previous hunk |

## Dependencies
```toml
git2 = "0.18"
```

## Testing
- [x] Diff shows for modified file
- [x] Colors distinguish +/-
- [x] Hunk navigation works
- [x] Unmodified file shows message

## Workflow
- Mark tasks with `[x]` as you complete them
- Run `cargo test` and `cargo clippy` before committing
- Once all tasks and tests pass, create a PR to `main`
