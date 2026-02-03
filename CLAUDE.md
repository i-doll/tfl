# Agent instructions for tfl

## Git commits

- Conventional commits: `feat(module): description`, `fix(preview): description`, `chore(deps): description`
- Scope should be the module/area affected: `tree`, `preview`, `ui`, `event`, `app`, `deps`, `config`
- Imperative mood in subject line
- No co-authored-by, no agent attribution, no bot markers

## Git branches

- Prefix with type: `feat/`, `fix/`, `chore/`, `refactor/`, `docs/`
- Examples: `feat/image-preview`, `fix/scroll-overflow`, `chore/update-deps`

## README maintenance

When adding or changing features, keybinds, or preview types, update README.md:
- Keybindings table (all 3 modes)
- Features list
- Dependencies table (if new crates added)
- Module structure (if new files added)

## Code style

- Rust 2024 edition
- 2-space indentation
- `use` grouping order: std, external crates, crate-internal
- Dirs-first sorting in file tree (case-insensitive)

## Testing

- Run `cargo test` after changes
- Run `cargo clippy` before committing
- Never run dev servers or build commands — ask the user

## Architecture

- **Flat vec tree**: `FileTree.entries` is a flat `Vec<FileEntry>` with depth tracking, not a recursive tree. Expand inserts children after parent; collapse drains them.
- **Action-based event system**: `Event` → `map_key()` → `Action` → `App::update()`. All key handling goes through the action enum.
- **Preview cache/debounce**: `PreviewState` has an LRU cache (10 entries) and 80ms debounce. Images load asynchronously via `mpsc`.
- **Suspend/resume**: Editor, Claude, and shell integrations drop the terminal, spawn the process, then restore.

## Norwegian keyboard

- `ø` shrinks tree pane, `æ` grows tree pane (not `[`/`]`)
