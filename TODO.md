# Regex Search

**Branch:** `feat/regex-search`

## Overview
Pattern matching in file search using regular expressions.

## Tasks
- [x] Add `regex` crate dependency
- [x] Toggle regex mode in search
- [x] Compile and validate regex pattern
- [x] Match against file names
- [x] Highlight matches in results
- [x] Show regex errors inline
- [x] Case sensitivity toggle

## Architecture Notes
- Extend existing search/filter infrastructure
- Lazy regex compilation (on pattern change)
- Consider: content search with regex too?

## Keybindings
| Key | Action |
|-----|--------|
| `/` | Start search |
| `Ctrl+r` | Toggle regex mode |
| `Ctrl+i` | Toggle case sensitivity |

## Dependencies
```toml
regex = "1.10"
```

## Testing
- [x] Simple regex matches files
- [x] Complex patterns work
- [x] Invalid regex shows error
- [x] Case toggle works

## Workflow
- Mark tasks with `[x]` as you complete them
- Run `cargo test` and `cargo clippy` before committing
- Once all tasks and tests pass, create a PR to `main`
