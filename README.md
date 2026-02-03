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
- **Configurable keybindings** via TOML config file

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
| `serde` | Serialization/deserialization for config |
| `toml` | TOML config file parsing |
| `dirs` | XDG config directory resolution |

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

## Configuration

tfl loads configuration from `$XDG_CONFIG_HOME/tfl/config.toml` (defaults to `~/.config/tfl/config.toml`). All fields are optional — unspecified values keep their defaults.

```toml
[general]
tree_ratio = 30       # initial tree pane width (percentage, default 30)
tick_rate_ms = 100    # event loop tick rate in ms (default 100)

[keys.normal]
j = "move_down"
k = "move_up"
h = "move_left"
l = "move_right"
down = "move_down"
up = "move_up"
left = "move_left"
right = "move_right"
space = "toggle_expand"
enter = "toggle_expand"
"shift+j" = "scroll_preview_down"
"shift+k" = "scroll_preview_up"
"." = "toggle_hidden"
"shift+g" = "go_to_bottom"
g = "g_press"
"/" = "search_start"
y = "yank_path"
e = "open_editor"
c = "open_claude"
"ctrl+c" = "quit"
s = "open_shell"
q = "quit"
esc = "quit"
ø = "shrink_tree"
æ = "grow_tree"

[keys.g_prefix]
g = "go_to_top"
```

### Key format

- Single characters: `j`, `q`, `.`, `/`, `ø`
- Uppercase / shift: `"shift+j"` or `"J"` (equivalent)
- Ctrl combos: `"ctrl+c"`
- Named keys: `enter`, `space`, `esc`, `up`, `down`, `left`, `right`, `backspace`, `tab`

### Available actions

`quit`, `move_up`, `move_down`, `move_left`, `move_right`, `toggle_expand`, `scroll_preview_up`, `scroll_preview_down`, `toggle_hidden`, `go_to_top`, `go_to_bottom`, `search_start`, `yank_path`, `open_editor`, `open_claude`, `open_shell`, `shrink_tree`, `grow_tree`, `g_press`, `none`

Use `"none"` to unbind a key (e.g., `q = "none"`).

Search mode keys are not configurable (they handle text input).

## Module structure

```
src/
  main.rs          Entry point, terminal setup, event loop
  app.rs           Application state, action dispatch, suspend/resume
  action.rs        Action enum (all possible user actions)
  event.rs         Event loop, key mapping, input modes
  config.rs        Config loading, key binding parsing, defaults
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
