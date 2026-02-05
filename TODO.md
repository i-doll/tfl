# Hidden File Toggle Persistence

**Branch:** `feat/hidden-persistence`

## Overview
Remember show/hide hidden files state across sessions.

## Tasks
- [x] Add `show_hidden` to config
- [x] Save state on toggle
- [x] Load state on startup
- [ ] Per-directory option (optional)
- [x] Sync with config reload

## Architecture Notes
- Store in main config: `show_hidden = false`
- Update on `.` keypress
- Consider: auto-save vs explicit save?

## Config Format
```toml
[display]
show_hidden = false
```

## Testing
- [x] Hidden state persists
- [x] Toggle updates config
- [x] Reload applies setting

## Workflow
- Mark tasks with `[x]` as you complete them
- Run `cargo test` and `cargo clippy` before committing
- Once all tasks and tests pass, create a PR to `main`
