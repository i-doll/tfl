# Archive Browsing & Extract

**Branch:** `feat/archive-extract`

## Overview
View contents of zip/tar/gz files and extract them.

## Tasks
- [x] Add `zip` and `tar` crate dependencies
- [x] Detect archive files by extension
- [x] List archive contents in preview pane
- [x] Show file sizes and paths within archive
- [ ] Extract single file from archive
- [x] Extract entire archive
- [x] Extract and delete archive option
- [ ] Handle nested archives

## Architecture Notes
- Preview shows file listing (like `unzip -l`)
- Extract to current directory or specified location
- Async extraction for large archives
- Extract-and-delete: confirm before deleting, only delete on successful extraction

## Keybindings
| Key | Action |
|-----|--------|
| `x` | Extract archive |
| `X` | Extract and delete archive |
| `Enter` | Extract selected file (in preview) |

## Dependencies
```toml
zip = "0.6"
tar = "0.4"
flate2 = "1.0"
```

## Testing
- [x] ZIP contents display
- [x] TAR.GZ contents display
- [ ] Single file extraction works
- [x] Full extraction works
- [x] Extract and delete removes archive after success

## Workflow
- Mark tasks with `[x]` as you complete them
- Run `cargo test` and `cargo clippy` before committing
- Once all tasks and tests pass, create a PR to `main`
