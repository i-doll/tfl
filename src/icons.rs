use ratatui::style::Color;

pub struct FileIcon {
  pub glyph: &'static str,
  pub color: Color,
}

pub fn file_icon(name: &str, is_dir: bool, expanded: bool, is_symlink: bool) -> FileIcon {
  // Priority 1: Symlinks
  if is_symlink {
    return FileIcon { glyph: "\u{f0c1} ", color: Color::Indexed(176) }; //
  }

  // Priority 2: Directories
  if is_dir {
    return if expanded {
      FileIcon { glyph: "\u{f115} ", color: Color::Indexed(75) } //
    } else {
      FileIcon { glyph: "\u{f114} ", color: Color::Indexed(75) } //
    };
  }

  // Priority 3: Exact filename matches
  let lower = name.to_lowercase();
  match lower.as_str() {
    "dockerfile" | "dockerfile.dev" | "dockerfile.prod" | "containerfile" =>
      return FileIcon { glyph: "\u{f308} ", color: Color::Indexed(39) },  //
    "makefile" | "gnumakefile" | "justfile" =>
      return FileIcon { glyph: "\u{f085} ", color: Color::Indexed(248) }, //
    ".gitignore" | ".gitmodules" | ".gitattributes" =>
      return FileIcon { glyph: "\u{f1d3} ", color: Color::Indexed(208) }, //
    "license" | "license.md" | "license.txt" | "licence" | "licence.md" =>
      return FileIcon { glyph: "\u{f0219} ", color: Color::Indexed(185) }, // 󰈙
    ".env" | ".env.local" | ".env.development" | ".env.production" =>
      return FileIcon { glyph: "\u{f0084} ", color: Color::Indexed(185) }, // 󰂄
    _ => {}
  }

  // Priority 4: Extension match
  let ext = name.rsplit('.').next().unwrap_or("").to_lowercase();

  // Lock files (check before extension to catch Cargo.lock, package-lock.json, etc.)
  if name.ends_with(".lock") {
    return FileIcon { glyph: "\u{f023} ", color: Color::Indexed(248) }; //
  }

  match ext.as_str() {
    // Rust
    "rs" => FileIcon { glyph: "\u{e7a8} ", color: Color::Indexed(208) }, //
    // Config
    "toml" => FileIcon { glyph: "\u{e6b2} ", color: Color::Indexed(150) }, //
    "json" | "jsonc" | "json5" => FileIcon { glyph: "\u{e60b} ", color: Color::Indexed(185) }, //
    "yaml" | "yml" => FileIcon { glyph: "\u{e6a8} ", color: Color::Indexed(150) }, //
    "xml" | "xsl" | "xslt" => FileIcon { glyph: "\u{e619} ", color: Color::Indexed(208) }, //
    "ini" | "cfg" | "conf" => FileIcon { glyph: "\u{e615} ", color: Color::Indexed(150) }, //
    // Markdown / Docs
    "md" | "mdx" => FileIcon { glyph: "\u{e73e} ", color: Color::Indexed(74) }, //
    "txt" => FileIcon { glyph: "\u{f0f6} ", color: Color::Indexed(252) }, //
    "pdf" => FileIcon { glyph: "\u{f1c1} ", color: Color::Indexed(167) }, //
    // Python
    "py" | "pyw" | "pyi" => FileIcon { glyph: "\u{e73c} ", color: Color::Indexed(114) }, //
    // JavaScript / TypeScript
    "js" | "mjs" | "cjs" => FileIcon { glyph: "\u{e74e} ", color: Color::Indexed(185) }, //
    "ts" | "mts" | "cts" => FileIcon { glyph: "\u{e628} ", color: Color::Indexed(74) }, //
    "tsx" => FileIcon { glyph: "\u{e7ba} ", color: Color::Indexed(74) }, //
    "jsx" => FileIcon { glyph: "\u{e7ba} ", color: Color::Indexed(74) }, //
    // Go
    "go" => FileIcon { glyph: "\u{e627} ", color: Color::Indexed(74) }, //
    // Shell
    "sh" | "bash" | "zsh" | "fish" => FileIcon { glyph: "\u{e795} ", color: Color::Indexed(114) }, //
    // Web
    "html" | "htm" => FileIcon { glyph: "\u{e736} ", color: Color::Indexed(208) }, //
    "css" | "scss" | "sass" | "less" => FileIcon { glyph: "\u{e749} ", color: Color::Indexed(74) }, //
    "svg" => FileIcon { glyph: "\u{f1c5} ", color: Color::Indexed(185) }, //
    // C / C++
    "c" => FileIcon { glyph: "\u{e61e} ", color: Color::Indexed(74) }, //
    "cpp" | "cc" | "cxx" | "c++" => FileIcon { glyph: "\u{e61d} ", color: Color::Indexed(74) }, //
    "h" | "hpp" | "hxx" => FileIcon { glyph: "\u{e61e} ", color: Color::Indexed(140) }, //
    // Java / JVM
    "java" => FileIcon { glyph: "\u{e738} ", color: Color::Indexed(167) }, //
    "kt" | "kts" => FileIcon { glyph: "\u{e634} ", color: Color::Indexed(140) }, //
    // Ruby
    "rb" => FileIcon { glyph: "\u{e739} ", color: Color::Indexed(167) }, //
    // Swift
    "swift" => FileIcon { glyph: "\u{e755} ", color: Color::Indexed(208) }, //
    // Zig
    "zig" => FileIcon { glyph: "\u{e6a9} ", color: Color::Indexed(208) }, //
    // Images
    "png" | "jpg" | "jpeg" | "gif" | "bmp" | "ico" | "webp" | "tiff" | "tif" | "avif" =>
      FileIcon { glyph: "\u{f1c5} ", color: Color::Indexed(139) }, //
    // Audio
    "mp3" | "flac" | "ogg" | "wav" | "aac" | "m4a" =>
      FileIcon { glyph: "\u{f001} ", color: Color::Indexed(139) }, //
    // Video
    "mp4" | "mkv" | "avi" | "mov" | "webm" | "flv" =>
      FileIcon { glyph: "\u{f03d} ", color: Color::Indexed(139) }, //
    // Archives
    "zip" | "tar" | "gz" | "bz2" | "xz" | "7z" | "rar" | "zst" =>
      FileIcon { glyph: "\u{f1c6} ", color: Color::Indexed(185) }, //
    // SQL
    "sql" => FileIcon { glyph: "\u{e706} ", color: Color::Indexed(74) }, //
    // Docker compose
    "dockerignore" => FileIcon { glyph: "\u{f308} ", color: Color::Indexed(39) }, //
    // Nix
    "nix" => FileIcon { glyph: "\u{f313} ", color: Color::Indexed(74) }, //
    // Lua
    "lua" => FileIcon { glyph: "\u{e620} ", color: Color::Indexed(74) }, //
    // Elixir
    "ex" | "exs" => FileIcon { glyph: "\u{e62d} ", color: Color::Indexed(140) }, //
    // Default
    _ => FileIcon { glyph: "\u{f016} ", color: Color::Indexed(252) }, //
  }
}

pub fn file_name_color(name: &str, is_dir: bool, is_symlink: bool) -> Color {
  if is_dir {
    return Color::Indexed(75); // blue
  }
  if is_symlink {
    return Color::Indexed(176); // purple
  }

  let ext = name.rsplit('.').next().unwrap_or("").to_lowercase();

  match ext.as_str() {
    // Rust
    "rs" => Color::Indexed(208), // orange
    // Config
    "toml" | "json" | "jsonc" | "json5" | "yaml" | "yml" | "xml" | "ini" | "cfg" | "conf" =>
      Color::Indexed(150), // green-ish
    // Scripts
    "py" | "pyw" | "pyi" | "sh" | "bash" | "zsh" | "fish" | "rb" | "lua" | "ex" | "exs" =>
      Color::Indexed(114), // green
    // Web
    "js" | "mjs" | "cjs" | "ts" | "mts" | "cts" | "tsx" | "jsx" | "html" | "htm" | "css"
    | "scss" | "sass" | "less" | "go" | "md" | "mdx" | "sql" =>
      Color::Indexed(74), // teal
    // Media
    "png" | "jpg" | "jpeg" | "gif" | "bmp" | "ico" | "webp" | "tiff" | "tif" | "avif"
    | "mp3" | "flac" | "ogg" | "wav" | "aac" | "m4a"
    | "mp4" | "mkv" | "avi" | "mov" | "webm" | "flv"
    | "svg" => Color::Indexed(139), // muted purple
    // Default
    _ => Color::Indexed(252), // light gray
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  // ── file_icon() tests ──

  #[test]
  fn test_dir_expanded_icon() {
    let icon = file_icon("src", true, true, false);
    assert_eq!(icon.glyph, "\u{f115} ");
  }

  #[test]
  fn test_dir_collapsed_icon() {
    let icon = file_icon("src", true, false, false);
    assert_eq!(icon.glyph, "\u{f114} ");
  }

  #[test]
  fn test_symlink_icon() {
    let icon = file_icon("link", false, false, true);
    assert_eq!(icon.glyph, "\u{f0c1} ");
    assert_eq!(icon.color, Color::Indexed(176));
  }

  #[test]
  fn test_rust_file_icon() {
    let icon = file_icon("main.rs", false, false, false);
    assert_eq!(icon.glyph, "\u{e7a8} ");
  }

  #[test]
  fn test_toml_icon() {
    let icon = file_icon("Cargo.toml", false, false, false);
    assert_eq!(icon.glyph, "\u{e6b2} ");
  }

  #[test]
  fn test_json_icon() {
    let icon = file_icon("package.json", false, false, false);
    assert_eq!(icon.glyph, "\u{e60b} ");
  }

  #[test]
  fn test_yaml_icon() {
    let icon = file_icon("config.yml", false, false, false);
    assert_eq!(icon.glyph, "\u{e6a8} ");
  }

  #[test]
  fn test_markdown_icon() {
    let icon = file_icon("README.md", false, false, false);
    assert_eq!(icon.glyph, "\u{e73e} ");
  }

  #[test]
  fn test_python_icon() {
    let icon = file_icon("script.py", false, false, false);
    assert_eq!(icon.glyph, "\u{e73c} ");
  }

  #[test]
  fn test_javascript_icon() {
    let icon = file_icon("index.js", false, false, false);
    assert_eq!(icon.glyph, "\u{e74e} ");
  }

  #[test]
  fn test_typescript_icon() {
    let icon = file_icon("app.ts", false, false, false);
    assert_eq!(icon.glyph, "\u{e628} ");
  }

  #[test]
  fn test_go_icon() {
    let icon = file_icon("main.go", false, false, false);
    assert_eq!(icon.glyph, "\u{e627} ");
  }

  #[test]
  fn test_shell_icon() {
    let icon = file_icon("build.sh", false, false, false);
    assert_eq!(icon.glyph, "\u{e795} ");
  }

  #[test]
  fn test_image_icon() {
    let icon = file_icon("photo.png", false, false, false);
    assert_eq!(icon.glyph, "\u{f1c5} ");
  }

  #[test]
  fn test_archive_icon() {
    let icon = file_icon("backup.zip", false, false, false);
    assert_eq!(icon.glyph, "\u{f1c6} ");
  }

  #[test]
  fn test_default_icon() {
    let icon = file_icon("unknown.xyz", false, false, false);
    assert_eq!(icon.glyph, "\u{f016} ");
  }

  #[test]
  fn test_exact_name_dockerfile() {
    let icon = file_icon("Dockerfile", false, false, false);
    assert_eq!(icon.glyph, "\u{f308} ");
  }

  #[test]
  fn test_exact_name_makefile() {
    let icon = file_icon("Makefile", false, false, false);
    assert_eq!(icon.glyph, "\u{f085} ");
  }

  #[test]
  fn test_exact_name_gitignore() {
    let icon = file_icon(".gitignore", false, false, false);
    assert_eq!(icon.glyph, "\u{f1d3} ");
  }

  #[test]
  fn test_exact_name_license() {
    let icon = file_icon("LICENSE", false, false, false);
    assert_eq!(icon.glyph, "\u{f0219} ");
  }

  #[test]
  fn test_exact_name_env() {
    let icon = file_icon(".env", false, false, false);
    assert_eq!(icon.glyph, "\u{f0084} ");
  }

  #[test]
  fn test_lock_file() {
    let icon = file_icon("Cargo.lock", false, false, false);
    assert_eq!(icon.glyph, "\u{f023} ");
  }

  #[test]
  fn test_dir_icon_color_is_blue() {
    let icon = file_icon("src", true, false, false);
    assert_eq!(icon.color, Color::Indexed(75));
  }

  #[test]
  fn test_icon_priority_symlink_over_extension() {
    let icon = file_icon("main.rs", false, false, true);
    assert_eq!(icon.glyph, "\u{f0c1} ");
    assert_eq!(icon.color, Color::Indexed(176));
  }

  // ── file_name_color() tests ──

  #[test]
  fn test_dir_name_color() {
    assert_eq!(file_name_color("src", true, false), Color::Indexed(75));
  }

  #[test]
  fn test_symlink_name_color() {
    assert_eq!(file_name_color("link", false, true), Color::Indexed(176));
  }

  #[test]
  fn test_rust_name_color() {
    assert_eq!(file_name_color("main.rs", false, false), Color::Indexed(208));
  }

  #[test]
  fn test_config_name_color() {
    assert_eq!(file_name_color("Cargo.toml", false, false), Color::Indexed(150));
    assert_eq!(file_name_color("package.json", false, false), Color::Indexed(150));
    assert_eq!(file_name_color("config.yaml", false, false), Color::Indexed(150));
  }

  #[test]
  fn test_default_name_color() {
    assert_eq!(file_name_color("unknown.xyz", false, false), Color::Indexed(252));
  }
}
