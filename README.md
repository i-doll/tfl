# tfl

A terminal file explorer with vim-style navigation and rich file previews, built with [ratatui](https://ratatui.rs).

```
┌──────────────────────────────────────────────────────────┐
│  ~/projects/my-app                                       │
├──────────────────┬───────────────────────────────────────┤
│  alpha_dir/     │ fn main() {                           │
│  beta_dir/      │     let config = Config::load();      │
│ > src/           │     let app = App::new(config);       │
│    main.rs      │     app.run();                        │
│    lib.rs       │ }                                     │
│  Cargo.toml     │                                       │
│  README.md      │                                       │
├──────────────────┴───────────────────────────────────────┤
│  main.rs | 234 B | rs | 12 lines              3/7      │
└──────────────────────────────────────────────────────────┘
```

## Features

- **Vim-style navigation** with `hjkl`, `gg`/`G`, and search with `/`
- **Syntax-highlighted text preview** via syntect
- **Image preview** in supported terminals (Kitty graphics protocol)
- **Hex dump** for binary files
- **Directory summaries** with file counts and sizes
- **Fuzzy search/filter** across file names
- **Yank path** to clipboard
- **Shell integrations** - drop into `$EDITOR`, `$SHELL`, or Claude Code
- **.gitignore-aware** hidden file toggling
- **Resizable panes** with adjustable tree/preview ratio
- **Preview cache** with LRU eviction and debounced loading

## Keybindings

### Normal mode

| Key | Action |
|---|---|
| `j` / `↓` | Move down |
| `k` / `↑` | Move up |
| `h` / `←` | Collapse directory / go to parent |
| `l` / `→` | Expand directory / select file |
| `Space` / `Enter` | Toggle expand directory |
| `J` | Scroll preview down |
| `K` | Scroll preview up |
| `gg` | Go to top |
| `G` | Go to bottom |
| `/` | Start search |
| `.` | Toggle hidden files |
| `y` | Yank path to clipboard |
| `e` | Open file in `$EDITOR` |
| `c` | Open Claude Code in current directory |
| `s` | Open `$SHELL` in current directory |
| `ø` | Shrink tree pane |
| `æ` | Grow tree pane |
| `q` / `Esc` | Quit |
| `Ctrl+c` | Quit |

### Search mode

| Key | Action |
|---|---|
| Characters | Filter file list |
| `Enter` | Confirm search |
| `Esc` | Cancel search |
| `Backspace` | Delete character |

### g-prefix mode

| Key | Action |
|---|---|
| `g` | Go to top (`gg`) |
| Any other | Cancel |

## Dependencies

| Crate | Purpose |
|---|---|
| `ratatui` | Terminal UI framework |
| `crossterm` | Terminal backend (input, raw mode, alternate screen) |
| `ratatui-image` | Image rendering via Kitty graphics protocol |
| `image` | Image decoding |
| `syntect` | Syntax highlighting for text preview |
| `infer` | MIME type detection for binary vs text |
| `ignore` | .gitignore-aware file filtering |
| `anyhow` / `thiserror` | Error handling |
| `unicode-width` | Accurate column width for Unicode strings |
| `clipboard-anywhere` | Cross-platform clipboard (yank path) |

## Build

```sh
cargo build --release
```

The binary will be at `target/release/tfl`.

## Usage

```sh
tfl [path]
```

If no path is given, opens the current directory.

## Module structure

```
src/
  main.rs          Entry point, terminal setup, event loop
  app.rs           Application state, action dispatch, suspend/resume
  action.rs        Action enum (all possible user actions)
  event.rs         Event loop, key mapping, input modes
  config.rs        Constants (ratios, tick rate)
  fs/
    entry.rs       FileEntry struct (path, metadata, depth)
    tree.rs        FileTree: flat vec, expand/collapse, sort, reload
  preview/
    mod.rs         PreviewState: cache, debounce, type detection
    text.rs        Syntax-highlighted text preview
    image.rs       Async image loading (Kitty protocol)
    hex.rs         Hex dump for binary files
    directory.rs   Directory summary (file counts, sizes)
  ui/
    mod.rs         Layout: header, tree/preview split, status bar
    file_tree.rs   Tree pane rendering with indent/icons
    preview.rs     Preview pane rendering (text, image, hex)
    status_bar.rs  Status bar: search input, file info, position
```
