use std::fs::{self, Metadata};
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::Path;
use std::time::SystemTime;

/// File properties for display in the properties panel
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct FileProperties {
  pub path: String,
  pub size: u64,
  pub size_human: String,
  pub permissions_octal: String,
  pub permissions_rwx: String,
  pub owner: String,
  pub group: String,
  pub created: Option<String>,
  pub modified: Option<String>,
  pub accessed: Option<String>,
  pub file_type: String,
  pub mime_type: Option<String>,
  pub symlink_target: Option<String>,
  pub is_dir: bool,
  pub is_symlink: bool,
}

impl FileProperties {
  pub fn from_path(path: &Path) -> Option<Self> {
    let symlink_meta = fs::symlink_metadata(path).ok()?;
    let is_symlink = symlink_meta.is_symlink();
    let symlink_target = if is_symlink {
      fs::read_link(path).ok().map(|t| t.to_string_lossy().to_string())
    } else {
      None
    };

    // For symlinks, get the target's metadata for most properties
    let meta = fs::metadata(path).ok().unwrap_or_else(|| symlink_meta.clone());
    let is_dir = meta.is_dir();

    let size = if is_dir { 0 } else { meta.len() };
    let size_human = format_size(size);

    let mode = meta.permissions().mode();
    let permissions_octal = format!("{:04o}", mode & 0o7777);
    let permissions_rwx = format_rwx(mode);

    let owner = resolve_user(meta.uid());
    let group = resolve_group(meta.gid());

    let created = meta.created().ok().and_then(format_time);
    let modified = meta.modified().ok().and_then(format_time);
    let accessed = meta.accessed().ok().and_then(format_time);

    let file_type = determine_file_type(path, &meta, is_symlink);
    let mime_type = if !is_dir {
      infer::get_from_path(path).ok().flatten().map(|t| t.mime_type().to_string())
    } else {
      None
    };

    Some(FileProperties {
      path: path.to_string_lossy().to_string(),
      size,
      size_human,
      permissions_octal,
      permissions_rwx,
      owner,
      group,
      created,
      modified,
      accessed,
      file_type,
      mime_type,
      symlink_target,
      is_dir,
      is_symlink,
    })
  }
}

fn format_size(bytes: u64) -> String {
  const KB: u64 = 1024;
  const MB: u64 = KB * 1024;
  const GB: u64 = MB * 1024;
  const TB: u64 = GB * 1024;

  if bytes >= TB {
    format!("{:.2} TB", bytes as f64 / TB as f64)
  } else if bytes >= GB {
    format!("{:.2} GB", bytes as f64 / GB as f64)
  } else if bytes >= MB {
    format!("{:.2} MB", bytes as f64 / MB as f64)
  } else if bytes >= KB {
    format!("{:.2} KB", bytes as f64 / KB as f64)
  } else {
    format!("{} B", bytes)
  }
}

fn format_rwx(mode: u32) -> String {
  let mut result = String::with_capacity(10);

  // File type indicator
  if mode & 0o170000 == 0o120000 {
    result.push('l'); // symlink
  } else if mode & 0o170000 == 0o040000 {
    result.push('d'); // directory
  } else if mode & 0o170000 == 0o100000 {
    result.push('-'); // regular file
  } else {
    result.push('?');
  }

  // Owner permissions
  result.push(if mode & 0o400 != 0 { 'r' } else { '-' });
  result.push(if mode & 0o200 != 0 { 'w' } else { '-' });
  result.push(if mode & 0o100 != 0 {
    if mode & 0o4000 != 0 { 's' } else { 'x' }
  } else if mode & 0o4000 != 0 {
    'S'
  } else {
    '-'
  });

  // Group permissions
  result.push(if mode & 0o040 != 0 { 'r' } else { '-' });
  result.push(if mode & 0o020 != 0 { 'w' } else { '-' });
  result.push(if mode & 0o010 != 0 {
    if mode & 0o2000 != 0 { 's' } else { 'x' }
  } else if mode & 0o2000 != 0 {
    'S'
  } else {
    '-'
  });

  // Other permissions
  result.push(if mode & 0o004 != 0 { 'r' } else { '-' });
  result.push(if mode & 0o002 != 0 { 'w' } else { '-' });
  result.push(if mode & 0o001 != 0 {
    if mode & 0o1000 != 0 { 't' } else { 'x' }
  } else if mode & 0o1000 != 0 {
    'T'
  } else {
    '-'
  });

  result
}

fn resolve_user(uid: u32) -> String {
  users::get_user_by_uid(uid)
    .map(|u| u.name().to_string_lossy().to_string())
    .unwrap_or_else(|| uid.to_string())
}

fn resolve_group(gid: u32) -> String {
  users::get_group_by_gid(gid)
    .map(|g| g.name().to_string_lossy().to_string())
    .unwrap_or_else(|| gid.to_string())
}

fn format_time(time: SystemTime) -> Option<String> {
  let duration = time.duration_since(SystemTime::UNIX_EPOCH).ok()?;
  let secs = duration.as_secs() as i64;

  // Convert to broken-down time manually (simplified UTC)
  let days_since_epoch = secs / 86400;
  let time_of_day = secs % 86400;
  let hours = time_of_day / 3600;
  let minutes = (time_of_day % 3600) / 60;
  let seconds = time_of_day % 60;

  // Calculate year/month/day from days since epoch (1970-01-01)
  let (year, month, day) = days_to_ymd(days_since_epoch);

  Some(format!(
    "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
    year, month, day, hours, minutes, seconds
  ))
}

fn days_to_ymd(mut days: i64) -> (i64, u32, u32) {
  // Days since 1970-01-01
  let mut year = 1970;

  loop {
    let days_in_year = if is_leap_year(year) { 366 } else { 365 };
    if days < days_in_year {
      break;
    }
    days -= days_in_year;
    year += 1;
  }

  let days_in_months: [u32; 12] = if is_leap_year(year) {
    [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
  } else {
    [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
  };

  let mut month = 1;
  for &days_in_month in &days_in_months {
    if days < days_in_month as i64 {
      break;
    }
    days -= days_in_month as i64;
    month += 1;
  }

  (year, month, (days + 1) as u32)
}

fn is_leap_year(year: i64) -> bool {
  (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

fn determine_file_type(path: &Path, meta: &Metadata, is_symlink: bool) -> String {
  if is_symlink {
    return "Symbolic link".to_string();
  }

  if meta.is_dir() {
    return "Directory".to_string();
  }

  // Try to determine from extension first
  if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
    let ext_lower = ext.to_lowercase();
    let type_name = match ext_lower.as_str() {
      // Text/code
      "rs" => "Rust source",
      "py" => "Python source",
      "js" => "JavaScript source",
      "ts" => "TypeScript source",
      "jsx" => "JSX source",
      "tsx" => "TSX source",
      "html" | "htm" => "HTML document",
      "css" => "CSS stylesheet",
      "json" => "JSON data",
      "yaml" | "yml" => "YAML data",
      "toml" => "TOML config",
      "xml" => "XML document",
      "md" | "markdown" => "Markdown document",
      "txt" => "Text file",
      "sh" | "bash" | "zsh" => "Shell script",
      "c" => "C source",
      "cpp" | "cc" | "cxx" => "C++ source",
      "h" | "hpp" => "C/C++ header",
      "go" => "Go source",
      "java" => "Java source",
      "rb" => "Ruby source",
      "php" => "PHP source",
      "swift" => "Swift source",
      "kt" | "kts" => "Kotlin source",
      "lua" => "Lua source",
      "vim" => "Vim script",
      "el" => "Emacs Lisp",
      "sql" => "SQL",
      // Images
      "png" => "PNG image",
      "jpg" | "jpeg" => "JPEG image",
      "gif" => "GIF image",
      "svg" => "SVG image",
      "webp" => "WebP image",
      "bmp" => "BMP image",
      "ico" => "Icon",
      "tiff" | "tif" => "TIFF image",
      // Audio/video
      "mp3" => "MP3 audio",
      "wav" => "WAV audio",
      "ogg" => "Ogg audio",
      "flac" => "FLAC audio",
      "mp4" => "MP4 video",
      "mkv" => "Matroska video",
      "avi" => "AVI video",
      "mov" => "QuickTime video",
      "webm" => "WebM video",
      // Archives
      "zip" => "ZIP archive",
      "tar" => "TAR archive",
      "gz" | "gzip" => "Gzip archive",
      "bz2" => "Bzip2 archive",
      "xz" => "XZ archive",
      "7z" => "7-Zip archive",
      "rar" => "RAR archive",
      // Documents
      "pdf" => "PDF document",
      "doc" => "Word document",
      "docx" => "Word document",
      "xls" => "Excel spreadsheet",
      "xlsx" => "Excel spreadsheet",
      "ppt" => "PowerPoint",
      "pptx" => "PowerPoint",
      "odt" => "OpenDocument text",
      "ods" => "OpenDocument spreadsheet",
      // Other
      "exe" => "Windows executable",
      "dll" => "Windows library",
      "so" => "Shared library",
      "dylib" => "macOS library",
      "a" => "Static library",
      "o" => "Object file",
      "lock" => "Lock file",
      "log" => "Log file",
      _ => return format!("{} file", ext.to_uppercase()),
    };
    return type_name.to_string();
  }

  // Check for special filenames
  if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
    match name {
      "Makefile" | "makefile" | "GNUmakefile" => return "Makefile".to_string(),
      "Dockerfile" => return "Dockerfile".to_string(),
      "Cargo.toml" => return "Cargo manifest".to_string(),
      "Cargo.lock" => return "Cargo lock file".to_string(),
      "package.json" => return "npm package".to_string(),
      "package-lock.json" => return "npm lock file".to_string(),
      "tsconfig.json" => return "TypeScript config".to_string(),
      ".gitignore" => return "Git ignore rules".to_string(),
      ".gitattributes" => return "Git attributes".to_string(),
      ".editorconfig" => return "EditorConfig".to_string(),
      "LICENSE" | "LICENSE.md" | "LICENSE.txt" => return "License file".to_string(),
      "README" | "README.md" | "README.txt" => return "Readme".to_string(),
      ".bashrc" | ".zshrc" | ".profile" => return "Shell config".to_string(),
      _ => {}
    }
  }

  "File".to_string()
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::fs;
  use std::os::unix::fs::symlink;

  fn setup_test_dir() -> std::path::PathBuf {
    use std::sync::atomic::{AtomicU32, Ordering};
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let id = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!("tfl_props_test_{id}_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
  }

  fn cleanup_test_dir(dir: &std::path::PathBuf) {
    let _ = fs::remove_dir_all(dir);
  }

  #[test]
  fn test_format_size_bytes() {
    assert_eq!(format_size(0), "0 B");
    assert_eq!(format_size(100), "100 B");
    assert_eq!(format_size(1023), "1023 B");
  }

  #[test]
  fn test_format_size_kilobytes() {
    assert_eq!(format_size(1024), "1.00 KB");
    assert_eq!(format_size(1536), "1.50 KB");
  }

  #[test]
  fn test_format_size_megabytes() {
    assert_eq!(format_size(1024 * 1024), "1.00 MB");
    assert_eq!(format_size(1024 * 1024 * 5), "5.00 MB");
  }

  #[test]
  fn test_format_size_gigabytes() {
    assert_eq!(format_size(1024 * 1024 * 1024), "1.00 GB");
  }

  #[test]
  fn test_format_rwx_regular_file() {
    // -rw-r--r-- = 0o100644
    let mode = 0o100644;
    assert_eq!(format_rwx(mode), "-rw-r--r--");
  }

  #[test]
  fn test_format_rwx_executable() {
    // -rwxr-xr-x = 0o100755
    let mode = 0o100755;
    assert_eq!(format_rwx(mode), "-rwxr-xr-x");
  }

  #[test]
  fn test_format_rwx_directory() {
    // drwxr-xr-x = 0o040755
    let mode = 0o040755;
    assert_eq!(format_rwx(mode), "drwxr-xr-x");
  }

  #[test]
  fn test_format_rwx_symlink() {
    // lrwxrwxrwx = 0o120777
    let mode = 0o120777;
    assert_eq!(format_rwx(mode), "lrwxrwxrwx");
  }

  #[test]
  fn test_format_rwx_setuid() {
    // -rwsr-xr-x = 0o104755
    let mode = 0o104755;
    assert_eq!(format_rwx(mode), "-rwsr-xr-x");
  }

  #[test]
  fn test_format_rwx_sticky() {
    // drwxrwxrwt = 0o041777
    let mode = 0o041777;
    assert_eq!(format_rwx(mode), "drwxrwxrwt");
  }

  #[test]
  fn test_properties_for_regular_file() {
    let dir = setup_test_dir();
    let file = dir.join("test.txt");
    fs::write(&file, "hello world").unwrap();

    let props = FileProperties::from_path(&file).unwrap();
    assert_eq!(props.size, 11);
    assert_eq!(props.size_human, "11 B");
    assert!(!props.is_dir);
    assert!(!props.is_symlink);
    assert_eq!(props.symlink_target, None);
    assert_eq!(props.file_type, "Text file");

    cleanup_test_dir(&dir);
  }

  #[test]
  fn test_properties_for_directory() {
    let dir = setup_test_dir();

    let props = FileProperties::from_path(&dir).unwrap();
    assert!(props.is_dir);
    assert!(!props.is_symlink);
    assert_eq!(props.file_type, "Directory");

    cleanup_test_dir(&dir);
  }

  #[test]
  fn test_properties_for_symlink() {
    let dir = setup_test_dir();
    let file = dir.join("real.txt");
    fs::write(&file, "content").unwrap();
    let link = dir.join("link.txt");
    symlink(&file, &link).unwrap();

    let props = FileProperties::from_path(&link).unwrap();
    assert!(props.is_symlink);
    assert_eq!(props.symlink_target, Some(file.to_string_lossy().to_string()));
    assert_eq!(props.file_type, "Symbolic link");

    cleanup_test_dir(&dir);
  }

  #[test]
  fn test_properties_file_type_rust() {
    let dir = setup_test_dir();
    let file = dir.join("main.rs");
    fs::write(&file, "fn main() {}").unwrap();

    let props = FileProperties::from_path(&file).unwrap();
    assert_eq!(props.file_type, "Rust source");

    cleanup_test_dir(&dir);
  }

  #[test]
  fn test_properties_file_type_makefile() {
    let dir = setup_test_dir();
    let file = dir.join("Makefile");
    fs::write(&file, "all:\n\techo hello").unwrap();

    let props = FileProperties::from_path(&file).unwrap();
    assert_eq!(props.file_type, "Makefile");

    cleanup_test_dir(&dir);
  }

  #[test]
  fn test_properties_owner_and_group_resolved() {
    let dir = setup_test_dir();
    let file = dir.join("test.txt");
    fs::write(&file, "test").unwrap();

    let props = FileProperties::from_path(&file).unwrap();
    // Owner and group should be resolved to names (not numeric IDs)
    // unless the UID/GID doesn't exist in the system
    assert!(!props.owner.is_empty());
    assert!(!props.group.is_empty());

    cleanup_test_dir(&dir);
  }

  #[test]
  fn test_properties_timestamps_present() {
    let dir = setup_test_dir();
    let file = dir.join("test.txt");
    fs::write(&file, "test").unwrap();

    let props = FileProperties::from_path(&file).unwrap();
    assert!(props.modified.is_some());
    assert!(props.accessed.is_some());
    // created may be None on some filesystems

    cleanup_test_dir(&dir);
  }

  #[test]
  fn test_properties_nonexistent_file() {
    let props = FileProperties::from_path(Path::new("/nonexistent/file.txt"));
    assert!(props.is_none());
  }

  #[test]
  fn test_days_to_ymd_epoch() {
    let (year, month, day) = days_to_ymd(0);
    assert_eq!((year, month, day), (1970, 1, 1));
  }

  #[test]
  fn test_days_to_ymd_leap_year() {
    // 2000-03-01 is a leap year
    // Days from 1970-01-01 to 2000-03-01
    // = 30 years + leap days + 31 (Jan) + 29 (Feb leap)
    let (year, month, day) = days_to_ymd(11017);
    assert_eq!(year, 2000);
    assert_eq!(month, 3);
    assert_eq!(day, 1);
  }
}
