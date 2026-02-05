use std::io::{Read, Seek};
use std::path::Path;

use flate2::read::GzDecoder;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use tar::Archive as TarArchive;
use zip::ZipArchive;

/// Represents an entry within an archive
#[derive(Debug, Clone)]
pub struct ArchiveEntry {
  pub name: String,
  pub size: u64,
  pub is_dir: bool,
  /// Compressed size (for future use in displaying compression ratio)
  #[allow(dead_code)]
  pub compressed_size: Option<u64>,
}

/// Detect if a path is a supported archive type
pub fn is_archive(path: &Path) -> bool {
  let ext = path.extension()
    .and_then(|e| e.to_str())
    .map(|s| s.to_lowercase());

  matches!(ext.as_deref(), Some("zip" | "tar" | "gz" | "tgz"))
}

/// Get the archive type from extension
pub fn archive_type(path: &Path) -> Option<&'static str> {
  let name = path.file_name()?.to_str()?;
  let ext = path.extension().and_then(|e| e.to_str()).map(|s| s.to_lowercase());

  // Check for .tar.gz first
  if name.ends_with(".tar.gz") || ext.as_deref() == Some("tgz") {
    return Some("tar.gz");
  }

  match ext.as_deref() {
    Some("zip") => Some("zip"),
    Some("tar") => Some("tar"),
    Some("gz") => Some("gz"),
    _ => None,
  }
}

/// List contents of a ZIP file
pub fn list_zip(path: &Path) -> Result<Vec<ArchiveEntry>, String> {
  let file = std::fs::File::open(path)
    .map_err(|e| format!("Failed to open archive: {e}"))?;

  let mut archive = ZipArchive::new(file)
    .map_err(|e| format!("Invalid ZIP archive: {e}"))?;

  let mut entries = Vec::new();
  for i in 0..archive.len() {
    let file = archive.by_index(i)
      .map_err(|e| format!("Failed to read entry: {e}"))?;

    entries.push(ArchiveEntry {
      name: file.name().to_string(),
      size: file.size(),
      is_dir: file.is_dir(),
      compressed_size: Some(file.compressed_size()),
    });
  }

  Ok(entries)
}

/// List contents of a TAR file
pub fn list_tar<R: Read>(reader: R) -> Result<Vec<ArchiveEntry>, String> {
  let mut archive = TarArchive::new(reader);
  let mut entries = Vec::new();

  for entry_result in archive.entries()
    .map_err(|e| format!("Failed to read tar: {e}"))? {
    let entry = entry_result
      .map_err(|e| format!("Failed to read entry: {e}"))?;

    let path = entry.path()
      .map_err(|e| format!("Invalid path in archive: {e}"))?;

    entries.push(ArchiveEntry {
      name: path.to_string_lossy().to_string(),
      size: entry.size(),
      is_dir: entry.header().entry_type().is_dir(),
      compressed_size: None,
    });
  }

  Ok(entries)
}

/// List contents of a TAR.GZ file
pub fn list_tar_gz(path: &Path) -> Result<Vec<ArchiveEntry>, String> {
  let file = std::fs::File::open(path)
    .map_err(|e| format!("Failed to open archive: {e}"))?;

  let decoder = GzDecoder::new(file);
  list_tar(decoder)
}

/// List contents of a plain TAR file
pub fn list_tar_file(path: &Path) -> Result<Vec<ArchiveEntry>, String> {
  let file = std::fs::File::open(path)
    .map_err(|e| format!("Failed to open archive: {e}"))?;

  list_tar(file)
}

/// List archive contents based on detected type
pub fn list_archive(path: &Path) -> Result<Vec<ArchiveEntry>, String> {
  match archive_type(path) {
    Some("zip") => list_zip(path),
    Some("tar.gz") => list_tar_gz(path),
    Some("tar") => list_tar_file(path),
    Some("gz") => {
      // Plain .gz file - try to decompress and read as tar
      list_tar_gz(path)
    }
    _ => Err("Unsupported archive format".to_string()),
  }
}

/// Format file size for display
fn format_size(size: u64) -> String {
  if size < 1024 {
    format!("{size} B")
  } else if size < 1024 * 1024 {
    format!("{:.1} KB", size as f64 / 1024.0)
  } else if size < 1024 * 1024 * 1024 {
    format!("{:.1} MB", size as f64 / (1024.0 * 1024.0))
  } else {
    format!("{:.2} GB", size as f64 / (1024.0 * 1024.0 * 1024.0))
  }
}

/// Render archive contents as styled lines for preview
pub fn render_archive_contents(entries: &[ArchiveEntry]) -> Vec<Line<'static>> {
  let mut lines = Vec::new();

  // Header
  lines.push(Line::from(vec![
    Span::styled(
      format!(" Archive contents ({} entries)", entries.len()),
      Style::default().fg(Color::Cyan),
    ),
  ]));
  lines.push(Line::from(""));

  // Column header
  lines.push(Line::from(vec![
    Span::styled(" Size", Style::default().fg(Color::DarkGray)),
    Span::raw("       "),
    Span::styled("Name", Style::default().fg(Color::DarkGray)),
  ]));
  lines.push(Line::from(vec![
    Span::styled(" ----", Style::default().fg(Color::DarkGray)),
    Span::raw("       "),
    Span::styled("----", Style::default().fg(Color::DarkGray)),
  ]));

  for entry in entries {
    let size_str = if entry.is_dir {
      "     -".to_string()
    } else {
      format!("{:>6}", format_size(entry.size))
    };

    let name_style = if entry.is_dir {
      Style::default().fg(Color::Blue)
    } else {
      Style::default()
    };

    let name = if entry.is_dir && !entry.name.ends_with('/') {
      format!("{}/", entry.name)
    } else {
      entry.name.clone()
    };

    lines.push(Line::from(vec![
      Span::styled(format!(" {size_str}"), Style::default().fg(Color::Yellow)),
      Span::raw("  "),
      Span::styled(name, name_style),
    ]));
  }

  lines
}

/// Extract a single file from a ZIP archive (for future single-file extraction feature)
#[allow(dead_code)]
pub fn extract_zip_file<R: Read + Seek>(
  archive: &mut ZipArchive<R>,
  file_name: &str,
  dest_dir: &Path,
) -> Result<std::path::PathBuf, String> {
  let mut file = archive.by_name(file_name)
    .map_err(|e| format!("File not found in archive: {e}"))?;

  let dest_path = dest_dir.join(file_name);

  // Create parent directories if needed
  if let Some(parent) = dest_path.parent() {
    std::fs::create_dir_all(parent)
      .map_err(|e| format!("Failed to create directory: {e}"))?;
  }

  if file.is_dir() {
    std::fs::create_dir_all(&dest_path)
      .map_err(|e| format!("Failed to create directory: {e}"))?;
  } else {
    let mut outfile = std::fs::File::create(&dest_path)
      .map_err(|e| format!("Failed to create file: {e}"))?;
    std::io::copy(&mut file, &mut outfile)
      .map_err(|e| format!("Failed to extract file: {e}"))?;
  }

  Ok(dest_path)
}

/// Extract entire ZIP archive
pub fn extract_zip(path: &Path, dest_dir: &Path) -> Result<(), String> {
  let file = std::fs::File::open(path)
    .map_err(|e| format!("Failed to open archive: {e}"))?;

  let mut archive = ZipArchive::new(file)
    .map_err(|e| format!("Invalid ZIP archive: {e}"))?;

  for i in 0..archive.len() {
    let mut file = archive.by_index(i)
      .map_err(|e| format!("Failed to read entry: {e}"))?;

    let outpath = dest_dir.join(file.name());

    if file.is_dir() {
      std::fs::create_dir_all(&outpath)
        .map_err(|e| format!("Failed to create directory: {e}"))?;
    } else {
      if let Some(parent) = outpath.parent() {
        std::fs::create_dir_all(parent)
          .map_err(|e| format!("Failed to create directory: {e}"))?;
      }
      let mut outfile = std::fs::File::create(&outpath)
        .map_err(|e| format!("Failed to create file: {e}"))?;
      std::io::copy(&mut file, &mut outfile)
        .map_err(|e| format!("Failed to extract file: {e}"))?;
    }
  }

  Ok(())
}

/// Extract entire TAR archive
pub fn extract_tar<R: Read>(reader: R, dest_dir: &Path) -> Result<(), String> {
  let mut archive = TarArchive::new(reader);
  archive.unpack(dest_dir)
    .map_err(|e| format!("Failed to extract tar: {e}"))
}

/// Extract TAR.GZ archive
pub fn extract_tar_gz(path: &Path, dest_dir: &Path) -> Result<(), String> {
  let file = std::fs::File::open(path)
    .map_err(|e| format!("Failed to open archive: {e}"))?;

  let decoder = GzDecoder::new(file);
  extract_tar(decoder, dest_dir)
}

/// Extract plain TAR archive
pub fn extract_tar_file(path: &Path, dest_dir: &Path) -> Result<(), String> {
  let file = std::fs::File::open(path)
    .map_err(|e| format!("Failed to open archive: {e}"))?;

  extract_tar(file, dest_dir)
}

/// Extract archive based on detected type
pub fn extract_archive(path: &Path, dest_dir: &Path) -> Result<(), String> {
  match archive_type(path) {
    Some("zip") => extract_zip(path, dest_dir),
    Some("tar.gz") => extract_tar_gz(path, dest_dir),
    Some("tar") => extract_tar_file(path, dest_dir),
    Some("gz") => extract_tar_gz(path, dest_dir),
    _ => Err("Unsupported archive format".to_string()),
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::fs;
  use std::io::Write;
  use std::sync::atomic::{AtomicU32, Ordering};

  static COUNTER: AtomicU32 = AtomicU32::new(0);

  fn test_dir(prefix: &str) -> std::path::PathBuf {
    let id = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!("tfl_archive_{prefix}_{id}_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
  }

  fn cleanup_dir(dir: &std::path::PathBuf) {
    let _ = fs::remove_dir_all(dir);
  }

  #[test]
  fn test_is_archive_zip() {
    assert!(is_archive(Path::new("test.zip")));
    assert!(is_archive(Path::new("test.ZIP")));
    assert!(is_archive(Path::new("/path/to/archive.zip")));
  }

  #[test]
  fn test_is_archive_tar() {
    assert!(is_archive(Path::new("test.tar")));
    assert!(is_archive(Path::new("test.tar.gz")));
    assert!(is_archive(Path::new("test.tgz")));
    assert!(is_archive(Path::new("test.gz")));
  }

  #[test]
  fn test_is_archive_not_archive() {
    assert!(!is_archive(Path::new("test.txt")));
    assert!(!is_archive(Path::new("test.rs")));
    assert!(!is_archive(Path::new("test")));
  }

  #[test]
  fn test_archive_type_detection() {
    assert_eq!(archive_type(Path::new("test.zip")), Some("zip"));
    assert_eq!(archive_type(Path::new("test.tar")), Some("tar"));
    assert_eq!(archive_type(Path::new("test.tar.gz")), Some("tar.gz"));
    assert_eq!(archive_type(Path::new("test.tgz")), Some("tar.gz"));
    assert_eq!(archive_type(Path::new("test.gz")), Some("gz"));
    assert_eq!(archive_type(Path::new("test.txt")), None);
  }

  #[test]
  fn test_list_zip_contents() {
    let dir = test_dir("list_zip");
    let zip_path = dir.join("test.zip");

    // Create a test ZIP file
    {
      let file = fs::File::create(&zip_path).unwrap();
      let mut zip = zip::ZipWriter::new(file);
      let options = zip::write::FileOptions::default()
        .compression_method(zip::CompressionMethod::Stored);

      zip.start_file("hello.txt", options).unwrap();
      zip.write_all(b"Hello World").unwrap();

      zip.start_file("subdir/nested.txt", options).unwrap();
      zip.write_all(b"Nested content").unwrap();

      zip.finish().unwrap();
    }

    let entries = list_zip(&zip_path).unwrap();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].name, "hello.txt");
    assert_eq!(entries[0].size, 11); // "Hello World"
    assert!(!entries[0].is_dir);
    assert_eq!(entries[1].name, "subdir/nested.txt");

    cleanup_dir(&dir);
  }

  #[test]
  fn test_list_tar_contents() {
    let dir = test_dir("list_tar");
    let tar_path = dir.join("test.tar");

    // Create a test TAR file
    {
      let file = fs::File::create(&tar_path).unwrap();
      let mut builder = tar::Builder::new(file);

      // Add a file
      let data = b"Hello TAR";
      let mut header = tar::Header::new_gnu();
      header.set_size(data.len() as u64);
      header.set_mode(0o644);
      header.set_cksum();
      builder.append_data(&mut header, "hello.txt", &data[..]).unwrap();

      builder.finish().unwrap();
    }

    let entries = list_tar_file(&tar_path).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].name, "hello.txt");
    assert_eq!(entries[0].size, 9);

    cleanup_dir(&dir);
  }

  #[test]
  fn test_list_tar_gz_contents() {
    let dir = test_dir("list_tar_gz");
    let tar_gz_path = dir.join("test.tar.gz");

    // Create a test TAR.GZ file
    {
      let file = fs::File::create(&tar_gz_path).unwrap();
      let encoder = flate2::write::GzEncoder::new(file, flate2::Compression::default());
      let mut builder = tar::Builder::new(encoder);

      let data = b"Hello TAR.GZ";
      let mut header = tar::Header::new_gnu();
      header.set_size(data.len() as u64);
      header.set_mode(0o644);
      header.set_cksum();
      builder.append_data(&mut header, "hello.txt", &data[..]).unwrap();

      builder.into_inner().unwrap().finish().unwrap();
    }

    let entries = list_tar_gz(&tar_gz_path).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].name, "hello.txt");
    assert_eq!(entries[0].size, 12);

    cleanup_dir(&dir);
  }

  #[test]
  fn test_extract_zip() {
    let dir = test_dir("extract_zip");
    let zip_path = dir.join("test.zip");
    let extract_dir = dir.join("extracted");
    fs::create_dir_all(&extract_dir).unwrap();

    // Create a test ZIP file
    {
      let file = fs::File::create(&zip_path).unwrap();
      let mut zip = zip::ZipWriter::new(file);
      let options = zip::write::FileOptions::default();

      zip.start_file("hello.txt", options).unwrap();
      zip.write_all(b"Hello World").unwrap();

      zip.add_directory("subdir", options).unwrap();

      zip.start_file("subdir/nested.txt", options).unwrap();
      zip.write_all(b"Nested").unwrap();

      zip.finish().unwrap();
    }

    extract_zip(&zip_path, &extract_dir).unwrap();

    assert!(extract_dir.join("hello.txt").exists());
    assert!(extract_dir.join("subdir").join("nested.txt").exists());
    assert_eq!(fs::read_to_string(extract_dir.join("hello.txt")).unwrap(), "Hello World");

    cleanup_dir(&dir);
  }

  #[test]
  fn test_extract_tar_gz() {
    let dir = test_dir("extract_tar_gz");
    let tar_gz_path = dir.join("test.tar.gz");
    let extract_dir = dir.join("extracted");
    fs::create_dir_all(&extract_dir).unwrap();

    // Create a test TAR.GZ file
    {
      let file = fs::File::create(&tar_gz_path).unwrap();
      let encoder = flate2::write::GzEncoder::new(file, flate2::Compression::default());
      let mut builder = tar::Builder::new(encoder);

      let data = b"Hello TAR.GZ";
      let mut header = tar::Header::new_gnu();
      header.set_size(data.len() as u64);
      header.set_mode(0o644);
      header.set_cksum();
      builder.append_data(&mut header, "hello.txt", &data[..]).unwrap();

      builder.into_inner().unwrap().finish().unwrap();
    }

    extract_tar_gz(&tar_gz_path, &extract_dir).unwrap();

    assert!(extract_dir.join("hello.txt").exists());
    assert_eq!(fs::read_to_string(extract_dir.join("hello.txt")).unwrap(), "Hello TAR.GZ");

    cleanup_dir(&dir);
  }

  #[test]
  fn test_render_archive_contents() {
    let entries = vec![
      ArchiveEntry {
        name: "hello.txt".to_string(),
        size: 1024,
        is_dir: false,
        compressed_size: Some(512),
      },
      ArchiveEntry {
        name: "subdir".to_string(),
        size: 0,
        is_dir: true,
        compressed_size: None,
      },
    ];

    let lines = render_archive_contents(&entries);
    assert!(lines.len() > 4); // Header + separator + entries
  }

  #[test]
  fn test_format_size() {
    assert_eq!(format_size(100), "100 B");
    assert_eq!(format_size(1024), "1.0 KB");
    assert_eq!(format_size(1024 * 1024), "1.0 MB");
    assert_eq!(format_size(1024 * 1024 * 1024), "1.00 GB");
  }

  #[test]
  fn test_invalid_zip_returns_error() {
    let dir = test_dir("invalid_zip");
    let path = dir.join("invalid.zip");
    fs::write(&path, "not a zip file").unwrap();

    let result = list_zip(&path);
    assert!(result.is_err());

    cleanup_dir(&dir);
  }

  #[test]
  fn test_extract_and_delete_workflow() {
    // This tests the workflow for extract-and-delete
    // The actual deletion happens in App, but we verify extraction works
    let dir = test_dir("extract_delete");
    let zip_path = dir.join("test.zip");
    let extract_dir = dir.join("extracted");
    fs::create_dir_all(&extract_dir).unwrap();

    // Create ZIP
    {
      let file = fs::File::create(&zip_path).unwrap();
      let mut zip = zip::ZipWriter::new(file);
      let options = zip::write::FileOptions::default();
      zip.start_file("test.txt", options).unwrap();
      zip.write_all(b"test content").unwrap();
      zip.finish().unwrap();
    }

    // Extract
    assert!(extract_archive(&zip_path, &extract_dir).is_ok());

    // Verify contents before deletion
    assert!(extract_dir.join("test.txt").exists());

    // Now deletion would be safe (done by App)
    cleanup_dir(&dir);
  }
}
