use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use bzip2::read::BzDecoder;
use flate2::read::GzDecoder;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use tar::Archive as TarArchive;

use crate::theme::Theme;
use xz2::read::XzDecoder;
use zip::ZipArchive;

/// Detect if a path is a supported archive type
pub fn is_archive(path: &Path) -> bool {
  let name = path.file_name()
    .and_then(|n| n.to_str())
    .map(|s| s.to_lowercase())
    .unwrap_or_default();

  // Check for compound extensions first
  if name.ends_with(".tar.gz") || name.ends_with(".tar.bz2") || name.ends_with(".tar.xz") {
    return true;
  }

  let ext = path.extension()
    .and_then(|e| e.to_str())
    .map(|s| s.to_lowercase());

  matches!(ext.as_deref(), Some("zip" | "tar" | "gz" | "tgz" | "tbz2" | "txz"))
}

/// Get the archive type from extension
pub fn archive_type(path: &Path) -> Option<&'static str> {
  let name = path.file_name()?.to_str()?.to_lowercase();
  let ext = path.extension().and_then(|e| e.to_str()).map(|s| s.to_lowercase());

  // Check for compound extensions first
  if name.ends_with(".tar.gz") || ext.as_deref() == Some("tgz") {
    return Some("tar.gz");
  }
  if name.ends_with(".tar.bz2") || ext.as_deref() == Some("tbz2") {
    return Some("tar.bz2");
  }
  if name.ends_with(".tar.xz") || ext.as_deref() == Some("txz") {
    return Some("tar.xz");
  }

  match ext.as_deref() {
    Some("zip") => Some("zip"),
    Some("tar") => Some("tar"),
    Some("gz") => Some("gz"),
    Some("bz2") => Some("bz2"),
    Some("xz") => Some("xz"),
    _ => None,
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

/// Render a simple archive summary (without reading contents)
pub fn render_archive_summary(archive_type: &str, file_size: u64, theme: &Theme) -> Vec<Line<'static>> {
  let format_name = match archive_type {
    "zip" => "ZIP",
    "tar" => "TAR",
    "tar.gz" => "TAR.GZ",
    "tar.bz2" => "TAR.BZ2",
    "tar.xz" => "TAR.XZ",
    "gz" => "GZIP",
    "bz2" => "BZIP2",
    "xz" => "XZ",
    _ => "Archive",
  };

  vec![
    Line::from(""),
    Line::from(vec![
      Span::styled(
        format!("  {format_name} archive"),
        Style::default().fg(theme.info),
      ),
    ]),
    Line::from(""),
    Line::from(vec![
      Span::styled("  Size: ", Style::default().fg(theme.text_dim)),
      Span::styled(format_size(file_size), Style::default().fg(theme.warning)),
    ]),
    Line::from(""),
    Line::from(vec![
      Span::styled(
        "  Press x to extract",
        Style::default().fg(theme.text_dim),
      ),
    ]),
  ]
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

/// Extract TAR.BZ2 archive
pub fn extract_tar_bz2(path: &Path, dest_dir: &Path) -> Result<(), String> {
  let file = std::fs::File::open(path)
    .map_err(|e| format!("Failed to open archive: {e}"))?;

  let decoder = BzDecoder::new(file);
  extract_tar(decoder, dest_dir)
}

/// Extract TAR.XZ archive
pub fn extract_tar_xz(path: &Path, dest_dir: &Path) -> Result<(), String> {
  let file = std::fs::File::open(path)
    .map_err(|e| format!("Failed to open archive: {e}"))?;

  let decoder = XzDecoder::new(file);
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
    Some("tar.bz2") => extract_tar_bz2(path, dest_dir),
    Some("tar.xz") => extract_tar_xz(path, dest_dir),
    Some("tar") => extract_tar_file(path, dest_dir),
    Some("gz") => extract_tar_gz(path, dest_dir),
    Some("bz2") => extract_tar_bz2(path, dest_dir),
    Some("xz") => extract_tar_xz(path, dest_dir),
    _ => Err("Unsupported archive format".to_string()),
  }
}

/// Compress files/directories into an archive
pub fn compress_to_archive(paths: &[PathBuf], dest: &Path, format: &str) -> Result<(), String> {
  match format {
    "zip" => compress_zip(paths, dest),
    "tar.gz" => compress_tar(paths, dest, "gz"),
    "tar.bz2" => compress_tar(paths, dest, "bz2"),
    "tar.xz" => compress_tar(paths, dest, "xz"),
    _ => Err(format!("Unsupported format: {format}")),
  }
}

fn compress_zip(paths: &[PathBuf], dest: &Path) -> Result<(), String> {
  let file = std::fs::File::create(dest)
    .map_err(|e| format!("Failed to create archive: {e}"))?;
  let mut zip = zip::ZipWriter::new(file);
  let options = zip::write::SimpleFileOptions::default()
    .compression_method(zip::CompressionMethod::Deflated);

  for path in paths {
    if path.is_dir() {
      zip_add_dir_recursive(&mut zip, path, path, options)
        .map_err(|e| format!("Failed to add directory: {e}"))?;
    } else {
      let name = path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("file");
      zip.start_file(name, options)
        .map_err(|e| format!("Failed to add file: {e}"))?;
      let data = std::fs::read(path)
        .map_err(|e| format!("Failed to read file: {e}"))?;
      zip.write_all(&data)
        .map_err(|e| format!("Failed to write file: {e}"))?;
    }
  }

  zip.finish().map_err(|e| format!("Failed to finalize archive: {e}"))?;
  Ok(())
}

fn zip_add_dir_recursive(
  zip: &mut zip::ZipWriter<std::fs::File>,
  base: &Path,
  path: &Path,
  options: zip::write::SimpleFileOptions,
) -> Result<(), String> {
  let base_parent = base.parent().unwrap_or(base);
  for entry in std::fs::read_dir(path)
    .map_err(|e| format!("Failed to read directory: {e}"))?
  {
    let entry = entry.map_err(|e| format!("Failed to read entry: {e}"))?;
    let entry_path = entry.path();
    let rel = entry_path.strip_prefix(base_parent)
      .unwrap_or(&entry_path);
    let name = rel.to_string_lossy().to_string();

    if entry_path.is_dir() {
      zip.add_directory(&name, options)
        .map_err(|e| format!("Failed to add directory: {e}"))?;
      zip_add_dir_recursive(zip, base, &entry_path, options)?;
    } else {
      zip.start_file(&name, options)
        .map_err(|e| format!("Failed to add file: {e}"))?;
      let data = std::fs::read(&entry_path)
        .map_err(|e| format!("Failed to read file: {e}"))?;
      zip.write_all(&data)
        .map_err(|e| format!("Failed to write file: {e}"))?;
    }
  }
  Ok(())
}

fn compress_tar(paths: &[PathBuf], dest: &Path, compression: &str) -> Result<(), String> {
  let file = std::fs::File::create(dest)
    .map_err(|e| format!("Failed to create archive: {e}"))?;

  match compression {
    "gz" => {
      let encoder = flate2::write::GzEncoder::new(file, flate2::Compression::default());
      compress_tar_inner(paths, encoder)?;
    }
    "bz2" => {
      let encoder = bzip2::write::BzEncoder::new(file, bzip2::Compression::default());
      compress_tar_inner(paths, encoder)?;
    }
    "xz" => {
      let encoder = xz2::write::XzEncoder::new(file, 6);
      compress_tar_inner(paths, encoder)?;
    }
    _ => return Err(format!("Unsupported compression: {compression}")),
  }

  Ok(())
}

fn compress_tar_inner<W: Write>(paths: &[PathBuf], writer: W) -> Result<(), String> {
  let mut builder = tar::Builder::new(writer);

  for path in paths {
    let name = path.file_name()
      .and_then(|n| n.to_str())
      .unwrap_or("file");
    if path.is_dir() {
      builder.append_dir_all(name, path)
        .map_err(|e| format!("Failed to add directory: {e}"))?;
    } else {
      builder.append_path_with_name(path, name)
        .map_err(|e| format!("Failed to add file: {e}"))?;
    }
  }

  builder.into_inner()
    .map_err(|e| format!("Failed to finalize archive: {e}"))?;
  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::fs;
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
    assert!(is_archive(Path::new("test.tar.bz2")));
    assert!(is_archive(Path::new("test.tbz2")));
    assert!(is_archive(Path::new("test.tar.xz")));
    assert!(is_archive(Path::new("test.txz")));
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
    assert_eq!(archive_type(Path::new("test.tar.bz2")), Some("tar.bz2"));
    assert_eq!(archive_type(Path::new("test.tbz2")), Some("tar.bz2"));
    assert_eq!(archive_type(Path::new("test.bz2")), Some("bz2"));
    assert_eq!(archive_type(Path::new("test.tar.xz")), Some("tar.xz"));
    assert_eq!(archive_type(Path::new("test.txz")), Some("tar.xz"));
    assert_eq!(archive_type(Path::new("test.xz")), Some("xz"));
    assert_eq!(archive_type(Path::new("test.txt")), None);
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
      let options = zip::write::SimpleFileOptions::default();

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
  fn test_extract_tar_bz2() {
    let dir = test_dir("extract_tar_bz2");
    let tar_bz2_path = dir.join("test.tar.bz2");
    let extract_dir = dir.join("extracted");
    fs::create_dir_all(&extract_dir).unwrap();

    // Create a test TAR.BZ2 file
    {
      let file = fs::File::create(&tar_bz2_path).unwrap();
      let encoder = bzip2::write::BzEncoder::new(file, bzip2::Compression::default());
      let mut builder = tar::Builder::new(encoder);

      let data = b"Hello TAR.BZ2";
      let mut header = tar::Header::new_gnu();
      header.set_size(data.len() as u64);
      header.set_mode(0o644);
      header.set_cksum();
      builder.append_data(&mut header, "hello.txt", &data[..]).unwrap();

      builder.into_inner().unwrap().finish().unwrap();
    }

    extract_tar_bz2(&tar_bz2_path, &extract_dir).unwrap();

    assert!(extract_dir.join("hello.txt").exists());
    assert_eq!(fs::read_to_string(extract_dir.join("hello.txt")).unwrap(), "Hello TAR.BZ2");

    cleanup_dir(&dir);
  }

  #[test]
  fn test_extract_tar_xz() {
    let dir = test_dir("extract_tar_xz");
    let tar_xz_path = dir.join("test.tar.xz");
    let extract_dir = dir.join("extracted");
    fs::create_dir_all(&extract_dir).unwrap();

    // Create a test TAR.XZ file
    {
      let file = fs::File::create(&tar_xz_path).unwrap();
      let encoder = xz2::write::XzEncoder::new(file, 6);
      let mut builder = tar::Builder::new(encoder);

      let data = b"Hello TAR.XZ";
      let mut header = tar::Header::new_gnu();
      header.set_size(data.len() as u64);
      header.set_mode(0o644);
      header.set_cksum();
      builder.append_data(&mut header, "hello.txt", &data[..]).unwrap();

      builder.into_inner().unwrap().finish().unwrap();
    }

    extract_tar_xz(&tar_xz_path, &extract_dir).unwrap();

    assert!(extract_dir.join("hello.txt").exists());
    assert_eq!(fs::read_to_string(extract_dir.join("hello.txt")).unwrap(), "Hello TAR.XZ");

    cleanup_dir(&dir);
  }

  #[test]
  fn test_format_size() {
    assert_eq!(format_size(100), "100 B");
    assert_eq!(format_size(1024), "1.0 KB");
    assert_eq!(format_size(1024 * 1024), "1.0 MB");
    assert_eq!(format_size(1024 * 1024 * 1024), "1.00 GB");
  }

  #[test]
  fn test_compress_zip() {
    let dir = test_dir("compress_zip");
    fs::write(dir.join("file1.txt"), "hello").unwrap();
    fs::write(dir.join("file2.txt"), "world").unwrap();

    let dest = dir.join("archive.zip");
    compress_to_archive(
      &[dir.join("file1.txt"), dir.join("file2.txt")],
      &dest,
      "zip",
    ).unwrap();

    assert!(dest.exists());
    let extract_dir = dir.join("extracted");
    fs::create_dir_all(&extract_dir).unwrap();
    extract_zip(&dest, &extract_dir).unwrap();
    assert_eq!(fs::read_to_string(extract_dir.join("file1.txt")).unwrap(), "hello");
    assert_eq!(fs::read_to_string(extract_dir.join("file2.txt")).unwrap(), "world");

    cleanup_dir(&dir);
  }

  #[test]
  fn test_compress_tar_gz() {
    let dir = test_dir("compress_tar_gz");
    fs::write(dir.join("data.txt"), "compressed content").unwrap();

    let dest = dir.join("archive.tar.gz");
    compress_to_archive(
      &[dir.join("data.txt")],
      &dest,
      "tar.gz",
    ).unwrap();

    assert!(dest.exists());
    let extract_dir = dir.join("extracted");
    fs::create_dir_all(&extract_dir).unwrap();
    extract_tar_gz(&dest, &extract_dir).unwrap();
    assert_eq!(fs::read_to_string(extract_dir.join("data.txt")).unwrap(), "compressed content");

    cleanup_dir(&dir);
  }

  #[test]
  fn test_compress_directory() {
    let dir = test_dir("compress_dir");
    let subdir = dir.join("mydir");
    fs::create_dir_all(&subdir).unwrap();
    fs::write(subdir.join("inner.txt"), "inside").unwrap();

    let dest = dir.join("archive.tar.gz");
    compress_to_archive(
      &[subdir],
      &dest,
      "tar.gz",
    ).unwrap();

    assert!(dest.exists());
    let extract_dir = dir.join("extracted");
    fs::create_dir_all(&extract_dir).unwrap();
    extract_tar_gz(&dest, &extract_dir).unwrap();
    assert!(extract_dir.join("mydir").join("inner.txt").exists());
    assert_eq!(fs::read_to_string(extract_dir.join("mydir").join("inner.txt")).unwrap(), "inside");

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
      let options = zip::write::SimpleFileOptions::default();
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
