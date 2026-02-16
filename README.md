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
- **Rendered markdown preview** with styled headings, lists, code blocks, and links
- **JSON/TOML pretty-printing** with formatted/raw view toggle (`P`)
- **Image preview** in supported terminals (Kitty graphics protocol)
- **Hex dump** for binary files
- **Directory summaries** with file counts and sizes
- **Fuzzy search/filter** across file names
- **File management** — cut, copy, paste, delete, rename, new file/dir, chmod
- **Yank path** to clipboard
- **Open with system default** — press Enter on a file to open with the default app
- **Open-with picker** — press `o` to choose from detected editors/IDEs
- **Shell integrations** - drop into `$EDITOR`, `$SHELL`, or Claude Code
- **Git status highlighting** — modified (yellow), staged (green), untracked (red), conflicted (bright red) with parent directory propagation
- **Git branch display** in header with ahead/behind counts and summary stats
- **Git diff preview** — view uncommitted changes with colored +/- lines and hunk navigation
- **.gitignore-aware** hidden file toggling
- **Custom ignore patterns** via glob syntax (e.g., `*.log`, `node_modules`)
- **Resizable panes** with adjustable tree/preview ratio
- **Dual-pane mode** — Norton Commander style side-by-side navigation (F6 to toggle, Tab to switch)
- **Preview cache** with LRU eviction and debounced loading
- **Favorites** — save directories, jump to them from a picker overlay
- **Breadcrumb navigation** — clickable path segments in header to jump to parent directories, keyboard nav with `g1`-`g9`
- **Home shortcut** — jump to home directory with `~` or `gh`
- **Directory history** — back/forward navigation through visited directories with `-`/`+`
- **Configurable keybindings** via TOML config file
- **Live config reload** — changes to `config.toml`, `apps.toml`, and `favorites` are picked up automatically without restarting
- **File metadata panel** — size, modified time, permissions, line count, git commit history
- **Image EXIF data** — camera model, ISO, exposure for photos with embedded metadata
- **Archive browsing** — preview contents of ZIP, TAR, TAR.GZ, TAR.BZ2, TAR.XZ files with file listing
- **Archive extraction** — extract archives to current directory, with optional delete after extract

## Keybindings

### Normal mode

| Key | Action |
|---|---|
| `j` / `↓` | Move down |
| `k` / `↑` | Move up |
| `h` / `←` | Collapse directory / go to parent |
| `l` / `→` | Expand directory / select file |
| `Space` | Toggle expand/collapse directory |
| `Enter` | Open file / enter directory |
| `o` | Open with... (picker) |
| `J` / `PageDown` | Scroll preview down |
| `K` / `PageUp` | Scroll preview up |
| `P` | Toggle formatted/raw view (JSON/TOML) |
| `gg` | Go to top |
| `G` | Go to bottom |
| `/` | Start search |
| `.` | Toggle hidden files |
| `I` | Toggle custom ignore patterns |
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
| `C` | Open Claude Code (alt mode — inverse of `claude_yolo` config) |
| `s` | Open `$SHELL` in current directory |
| `ø` | Shrink tree pane |
| `æ` | Grow tree pane |
| `~` / `gh` | Go to home directory |
| `f` | Open favorites picker |
| `F` | Add current directory to favorites |
| `x` | Extract archive to current directory |
| `X` | Extract archive and delete after success |
| `Ctrl+p` | Change file permissions (chmod) |
| `-` | Go back in directory history |
| `+` | Go forward in directory history |
| `m` | Toggle raw/rendered markdown preview |
| `Tab` | Switch active pane (dual-pane mode) |
| `F6` | Toggle dual-pane mode |
| `d` | Show git diff for current file |
| `n` | Jump to next diff hunk |
| `N` | Jump to previous diff hunk |
| `i` | Show file properties |
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
| `h` | Go to home directory (`gh`) |
| `1`-`9` | Jump to nth breadcrumb segment |
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

### Favorites mode

| Key | Action |
|---|---|
| `j` / `↓` | Move down |
| `k` / `↑` | Move up |
| `Enter` | Navigate to selected favorite |
| `a` | Add current directory |
| `d` / `Delete` | Remove selected favorite |
| `Esc` | Close picker |

### Open with mode

| Key | Action |
|---|---|
| `j` / `↓` | Move down |
| `k` / `↑` | Move up |
| `Enter` | Open with selected app |
| `q` / `Esc` | Close picker |

### Properties mode

| Key | Action |
|---|---|
| `i` | Close properties |
| `q` | Close properties |
| `Esc` | Close properties |

### Help mode

| Key | Action |
|---|---|
| `?` | Close help |
| `Esc` | Close help |

### Chmod mode

| Key | Action |
|---|---|
| `r` / `w` / `x` | Toggle owner read/write/execute |
| `R` / `W` / `X` | Toggle group read/write/execute |
| `4` / `2` / `1` | Toggle others read/write/execute |
| `Tab` | Toggle octal input mode |
| `d` | Toggle recursive (directories only) |
| `Enter` | Apply changes |
| `Esc` / `q` | Cancel |

## Dependencies

| Crate | Purpose |
|---|---|
| `ratatui` | Terminal UI framework |
| `crossterm` | Terminal backend (input, raw mode, alternate screen) |
| `ratatui-image` | Image rendering via Kitty graphics protocol |
| `image` | Image decoding |
| `syntect` | Syntax highlighting for text preview |
| `pulldown-cmark` | Markdown parsing for rendered preview |
| `infer` | MIME type detection for binary vs text |
| `ignore` | .gitignore-aware file filtering |
| `globset` | Glob pattern matching for custom ignore rules |
| `anyhow` / `thiserror` | Error handling |
| `unicode-width` | Accurate column width for Unicode strings |
| `clipboard-anywhere` | Cross-platform clipboard (yank path) |
| `serde` | Serialization/deserialization for config |
| `serde_json` | JSON parsing and pretty-printing |
| `toml` | TOML config file parsing and pretty-printing |
| `dirs` | XDG config directory resolution |
| `open` | Open files with system default application |
| `notify` | OS-native file watching for live config reload |
| `git2` | Native Git repository operations (status, commits, branch info) |
| `kamadak-exif` | EXIF metadata extraction from images |
| `zip` | ZIP archive reading and extraction |
| `tar` | TAR archive reading and extraction |
| `flate2` | GZIP decompression for tar.gz files |
| `bzip2` | BZIP2 decompression for tar.bz2 files |
| `xz2` | XZ/LZMA decompression for tar.xz files |
| `users` | Resolve UID/GID to user/group names |

## Installation

### From source

```sh
just install
```

Or without just:

```sh
cargo install --path .
```

### Pre-built binaries

Download from [GitHub Releases](https://github.com/i-doll/tfl/releases).

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
| `--init` | Write default `config.toml` and `apps.toml` to `~/.config/tfl/` |
| `-h`, `--help` | Print help message |
| `-V`, `--version` | Print version |

## Configuration

tfl loads configuration from `$XDG_CONFIG_HOME/tfl/config.toml` (defaults to `~/.config/tfl/config.toml`). General settings are optional — unspecified values keep their defaults. Key sections (`[keys.normal]`, `[keys.g_prefix]`) **replace** the defaults entirely when present, so include all bindings you want. Use `tfl --init` to generate both `config.toml` and `apps.toml` with all defaults as a starting point.

**Live reload:** Changes to `config.toml`, `apps.toml`, and `favorites` are detected automatically via OS-native file watchers (FSEvents on macOS, inotify on Linux). Keybindings, custom apps, and favorites update immediately — no restart required. Layout settings (`tree_ratio`, `tick_rate_ms`) are only applied at startup to preserve any manual adjustments during the session.

```toml
[general]
tree_ratio = 30       # initial tree pane width (percentage, default 30)
tick_rate_ms = 100    # event loop tick rate in ms (default 100)
claude_yolo = false   # if true, `c` launches Claude with --dangerously-skip-permissions (default false)

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
enter = "open_default"
o = "open_with"
"shift+j" = "scroll_preview_down"
"shift+k" = "scroll_preview_up"
pagedown = "scroll_preview_down"
pageup = "scroll_preview_up"
"." = "toggle_hidden"
"shift+g" = "go_to_bottom"
g = "g_press"
"/" = "search_start"
y = "yank_path"
e = "open_editor"
c = "open_claude"
"shift+c" = "open_claude_alt"
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
"~" = "go_home"
f = "favorites_open"
"shift+f" = "favorite_add"
"ctrl+p" = "chmod"
"shift+i" = "toggle_custom_ignore"
"-" = "history_back"
"+" = "history_forward"
m = "toggle_markdown_mode"
tab = "switch_pane"
f6 = "toggle_dual_pane"
i = "show_properties"

[keys.g_prefix]
g = "go_to_top"
h = "go_home"

[ignore]
patterns = [
  "*.log",
  "node_modules",
  "__pycache__",
  ".DS_Store",
]
use_gitignore = true   # respect .gitignore files (default true)
use_custom = true      # apply custom patterns (default true)
```

### Custom ignore patterns

The `[ignore]` section lets you filter files from the tree view using glob patterns. Patterns match against file and directory names (not paths). Common use cases:

- `*.log` — hide log files
- `node_modules` — hide npm dependencies
- `__pycache__` — hide Python cache
- `.DS_Store` — hide macOS metadata

Press `I` to toggle custom ignore patterns on/off. The `use_gitignore` option controls whether `.gitignore` files are respected (separate from hidden file toggling with `.`).

### Key format

- Single characters: `j`, `q`, `.`, `/`, `ø`
- Uppercase / shift: `"shift+j"` or `"J"` (equivalent)
- Ctrl combos: `"ctrl+c"`
- Named keys: `enter`, `space`, `esc`, `up`, `down`, `left`, `right`, `backspace`, `delete`, `tab`, `pageup`, `pagedown`
- Function keys: `f1` through `f24`

### Available actions

`quit`, `move_up`, `move_down`, `move_left`, `move_right`, `toggle_expand`, `enter_dir`, `open_default`, `open_with`, `scroll_preview_up`, `scroll_preview_down`, `toggle_hidden`, `toggle_custom_ignore`, `toggle_formatted`, `go_to_top`, `go_to_bottom`, `search_start`, `yank_path`, `open_editor`, `open_claude`, `open_claude_alt`, `open_shell`, `shrink_tree`, `grow_tree`, `g_press`, `toggle_help`, `toggle_markdown_mode`, `go_home`, `favorites_open`, `favorite_add`, `history_back`, `history_forward`, `cut_file`, `copy_file`, `paste`, `delete_file`, `rename_start`, `new_file_start`, `new_dir_start`, `extract_archive`, `extract_and_delete`, `chmod`, `switch_pane`, `toggle_dual_pane`, `show_diff`, `next_hunk`, `prev_hunk`, `show_properties`, `none`

Use `"none"` to unbind a key (e.g., `q = "none"`).

Search and prompt mode keys are not configurable (they handle text input).

### Custom apps (`apps.toml`)

The open-with picker (`o`) is configured via `~/.config/tfl/apps.toml`. Run `tfl --init` to generate it with all built-in apps as a starting point, then reorder, remove, or add entries as you like.

When `apps.toml` exists on disk, the hardcoded built-in app list is skipped entirely — only apps listed in the file are considered. Delete `apps.toml` to restore the default built-in list.

```toml
[[apps]]
name = "Kakoune"
command = "kak"
tui = true

[[apps]]
name = "Lite XL"
command = "lite-xl"

[[apps]]
name = "Pages"
macos_app = "Pages"    # macOS only — opens via `open -a "Pages"`
```

Each entry needs at least `command` or `macos_app`. The `tui` flag (default `false`) enables suspend/resume for terminal editors. The `opens_dir` flag (default `false`) adds a "open containing folder" variant for files. Only apps found on your system will appear in the picker.

## Module structure

```
src/
  main.rs          Entry point, terminal setup, event loop
  app.rs           Application state, action dispatch, suspend/resume
  action.rs        Action enum (all possible user actions)
  event.rs         Event loop, key mapping, input modes
  config.rs        Config loading, key binding parsing, defaults
  favorites.rs     Favorites persistence (load/save/add/remove)
  opener.rs        Open-with app detection and launching
  git.rs           Git operations via libgit2 (status, branch, commits)
  fs/
    entry.rs       FileEntry struct (path, metadata, depth)
    ops.rs         Filesystem helpers (copy, unique path)
    properties.rs  File properties extraction (permissions, owner, times)
    tree.rs        FileTree: flat vec, expand/collapse, sort, reload
  preview/
    mod.rs         PreviewState: cache, debounce, type detection
    archive.rs     Archive listing and extraction (ZIP, TAR, TAR.GZ, TAR.BZ2, TAR.XZ)
    diff.rs        Git diff generation and colored rendering
    text.rs        Syntax-highlighted text preview
    markdown.rs    Rendered markdown preview with styled elements
    structured.rs  JSON/TOML pretty-printing
    image.rs       Async image loading (Kitty protocol)
    hex.rs         Hex dump for binary files
    directory.rs   Directory summary (file counts, sizes)
    metadata.rs    File/image metadata extraction, formatting
  ui/
    mod.rs         Layout: header, tree/preview split, status bar
    breadcrumb.rs  Breadcrumb path parsing and click detection
    chmod.rs       Chmod dialog for changing file permissions
    favorites.rs   Favorites picker floating overlay
    open_with.rs   Open-with picker floating overlay
    properties.rs  File properties floating overlay
    file_tree.rs   Tree pane rendering with indent/icons
    preview.rs     Preview pane rendering (text, image, hex)
    status_bar.rs  Status bar: search input, file info, position
    help.rs        Floating help overlay with keybinding reference
```
