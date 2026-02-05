# Saved Searches

**Branch:** `feat/saved-searches`

## Overview
Save and recall common search patterns.

## Tasks
- [x] Define saved search structure (name, pattern, filters)
- [x] Store in config file
- [x] Save current search as named search
- [x] List saved searches
- [x] Quick-apply saved search
- [x] Edit/delete saved searches
- [x] Import/export searches

## Architecture Notes
- Store in `~/.config/tfl/searches.toml`
- Include: name, pattern, regex flag, size filter, date filter
- Quick access via numbered shortcuts or fuzzy find

## Keybindings
| Key | Action |
|-----|--------|
| `Ctrl+/` | List saved searches |
| `Ctrl+s` | Save current search |
| `1-9` | Quick apply saved search |

## Config Format
```toml
[[search]]
name = "Large files"
pattern = "*"
size = ">100M"

[[search]]
name = "Recent code"
pattern = "*.rs"
date = "7d"
```

## Testing
- [x] Save search persists
- [x] Load saved search applies all filters
- [x] Delete removes from config
- [x] Quick keys work

## Workflow
- Mark tasks with `[x]` as you complete them
- Run `cargo test` and `cargo clippy` before committing
- Once all tasks and tests pass, create a PR to `main`
