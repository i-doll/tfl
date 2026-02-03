# tfl

A terminal file explorer with vim-style navigation and rich file previews, built with [ratatui](https://ratatui.rs).

```
┌──────────────────────────────────────────────────────────┐
│  ~/projects/my-app                                       │
├──────────────────┬───────────────────────────────────────┤
│  alpha_dir/      │ fn main() {                           │
│  beta_dir/       │     let config = Config::load();      │
│ > src/           │     let app = App::new(config);       │
│    main.rs       │     app.run();                        │
│    lib.rs        │ }                                     │
│  Cargo.toml      │                                       │
│  README.md       │                                       │
├──────────────────┴───────────────────────────────────────┤
│  main.rs | 234 B | rs | 12 lines                3/7      │
└──────────────────────────────────────────────────────────┘
```

## Features

- **Vim-style navigation** with `hjkl`, `gg`/`G`, and search with `/`
- **Syntax-highlighted text preview** via syntect
- **Image preview** in supported terminals (Kitty graphics protocol)
- **Hex dump** for binary files
- **Directory summaries** with file counts and sizes
- **Fuzzy search/filter** across file names
- **File management** — cut, copy, paste, delete, rename, new file/dir
- **Yank path** to clipboard
- **Shell integrations** - drop into `$EDITOR`, `$SHELL`, or Claude Code
- **Git status highlighting** — modified (yellow), staged (green), untracked (red), conflicted (bright red) with parent directory propagation
- **Git branch display** in header with ahead/behind counts and summary stats
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
| `Space` | Toggle expand/collapse directory |
| `Enter` | Enter directory |
| `J` | Scroll preview down |
| `K` | Scroll preview up |
| `gg` | Go to top |
| `G` | Go to bottom |
| `/` | Start search |
| `.` | Toggle hidden files |
| `y` | Yank path to clipboard |
| `Ctrl+c` | Copy file/dir to clipboard |
| `Ctrl+x` | Cut file/dir to clipboard |
| `Ctrl+v` | Paste from clipboard |
| `Delete` | Delete file/dir (y/N confirm) |
| `r` / `F2` | Rename file/dir |
| `a` | Create new file |
| `A` | Create new directory |
| `e` | Open file in `$EDITOR` |
| `c` | Open Claude Code in current directory |
| `s` | Open `$SHELL` in current directory |
| `ø` | Shrink tree pane |
| `æ` | Grow tree pane |
| `?` | Show help |
| `q` / `Esc` | Quit |

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

### Prompt mode (rename, new file, new dir)

| Key | Action |
|---|---|
| Characters | Type name |
| `Enter` | Confirm |
| `Esc` | Cancel |
| `Backspace` | Delete character |

### Delete confirmation

| Key | Action |
|---|---|
| `y` | Confirm delete |
| Any other | Cancel |

### Help mode

| Key | Action |
|---|---|
| `?` | Close help |
| `Esc` | Close help |

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
tfl [options] [path]
tfl --init
tfl -a ~/projects
tfl --help
```

If no path is given, opens the current directory.

| Flag | Description |
|---|---|
| `-a`, `--all` | Show hidden files |
| `--init` | Write default config to `~/.config/tfl/config.toml` |
| `-h`, `--help` | Print help message |
| `-V`, `--version` | Print version |

## Configuration

tfl loads configuration from `$XDG_CONFIG_HOME/tfl/config.toml` (defaults to `~/.config/tfl/config.toml`). General settings are optional — unspecified values keep their defaults. Key sections (`[keys.normal]`, `[keys.g_prefix]`) **replace** the defaults entirely when present, so include all bindings you want. Use `tfl --init` to get a starting config with all defaults.

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
enter = "enter_dir"
"shift+j" = "scroll_preview_down"
"shift+k" = "scroll_preview_up"
"." = "toggle_hidden"
"shift+g" = "go_to_bottom"
g = "g_press"
"/" = "search_start"
y = "yank_path"
e = "open_editor"
c = "open_claude"
s = "open_shell"
q = "quit"
esc = "quit"
delete = "delete_file"
"ctrl+x" = "cut_file"
"ctrl+v" = "paste"
"ctrl+c" = "copy_file"
r = "rename_start"
f2 = "rename_start"
a = "new_file_start"
"shift+a" = "new_dir_start"
"ø" = "shrink_tree"
"æ" = "grow_tree"
"?" = "toggle_help"

[keys.g_prefix]
g = "go_to_top"
```

### Key format

- Single characters: `j`, `q`, `.`, `/`, `ø`
- Uppercase / shift: `"shift+j"` or `"J"` (equivalent)
- Ctrl combos: `"ctrl+c"`
- Named keys: `enter`, `space`, `esc`, `up`, `down`, `left`, `right`, `backspace`, `delete`, `tab`
- Function keys: `f1` through `f24`

### Available actions

`quit`, `move_up`, `move_down`, `move_left`, `move_right`, `toggle_expand`, `enter_dir`, `scroll_preview_up`, `scroll_preview_down`, `toggle_hidden`, `go_to_top`, `go_to_bottom`, `search_start`, `yank_path`, `open_editor`, `open_claude`, `open_shell`, `shrink_tree`, `grow_tree`, `g_press`, `toggle_help`, `cut_file`, `copy_file`, `paste`, `delete_file`, `rename_start`, `new_file_start`, `new_dir_start`, `none`

Use `"none"` to unbind a key (e.g., `q = "none"`).

Search and prompt mode keys are not configurable (they handle text input).

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
    ops.rs         Filesystem helpers (copy, unique path)
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
    help.rs        Floating help overlay with keybinding reference
```
