# Sort Preference Persistence

**Branch:** `feat/sort-persistence`

## Overview
Save preferred sort order across sessions.

## Tasks
- [x] Add sort fields to config file
- [ ] Save sort field and order on change
- [x] Load sort preference on startup
- [ ] Per-directory sort (optional)
- [x] Global default sort

## Architecture Notes
- Store in main config: `default_sort_field`, `default_sort_order`
- Optional: `.tfl.toml` per-directory overrides
- Apply on `reload_config()` path

## Config Format
```toml
[sort]
field = "name"  # name, size, date, type
order = "asc"   # asc, desc
dirs_first = true
```

## Testing
- [x] Sort persists after restart
- [x] Config change applies immediately
- [x] Invalid values use defaults

## Workflow
- Mark tasks with `[x]` as you complete them
- Run `cargo test` and `cargo clippy` before committing
- Once all tasks and tests pass, create a PR to `main`
