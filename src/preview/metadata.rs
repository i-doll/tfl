use std::fs::Metadata;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::Path;
use std::time::SystemTime;

use crate::git::{GitCommit, GitRepo};

#[derive(Debug, Clone)]
pub struct FileMetadata {
  pub size: u64,
  pub modified: Option<SystemTime>,
  pub created: Option<SystemTime>,
  pub permissions: Option<u32>,
  pub owner: Option<String>,
  pub group: Option<String>,
  pub line_count: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct ImageMetadata {
  pub width: u32,
  pub height: u32,
  pub aspect_ratio: String,
  pub exif: Option<ExifData>,
}

#[derive(Debug, Clone)]
pub struct ExifData {
  pub camera: Option<String>,
  pub exposure: Option<String>,
  pub iso: Option<String>,
}

pub fn get_file_metadata(path: &Path) -> Option<FileMetadata> {
  let meta = std::fs::metadata(path).ok()?;
  Some(FileMetadata {
    size: meta.len(),
    modified: meta.modified().ok(),
    created: meta.created().ok(),
    permissions: Some(meta.permissions().mode()),
    owner: get_owner(&meta),
    group: get_group(&meta),
    line_count: None,
  })
}

pub fn get_file_metadata_with_lines(path: &Path, line_count: usize) -> Option<FileMetadata> {
  let mut meta = get_file_metadata(path)?;
  meta.line_count = Some(line_count);
  Some(meta)
}

fn get_owner(meta: &Metadata) -> Option<String> {
  let uid = meta.uid();
  // Try to get username from /etc/passwd
  if let Ok(content) = std::fs::read_to_string("/etc/passwd") {
    for line in content.lines() {
      let parts: Vec<&str> = line.split(':').collect();
      if parts.len() >= 3
        && let Ok(id) = parts[2].parse::<u32>()
        && id == uid
      {
        return Some(parts[0].to_string());
      }
    }
  }
  Some(uid.to_string())
}

fn get_group(meta: &Metadata) -> Option<String> {
  let gid = meta.gid();
  // Try to get group name from /etc/group
  if let Ok(content) = std::fs::read_to_string("/etc/group") {
    for line in content.lines() {
      let parts: Vec<&str> = line.split(':').collect();
      if parts.len() >= 3
        && let Ok(id) = parts[2].parse::<u32>()
        && id == gid
      {
        return Some(parts[0].to_string());
      }
    }
  }
  Some(gid.to_string())
}

pub fn get_image_metadata(path: &Path) -> Option<ImageMetadata> {
  let img = image::image_dimensions(path).ok()?;
  let (width, height) = img;
  let aspect_ratio = calculate_aspect_ratio(width, height);
  let exif = get_exif_data(path);

  Some(ImageMetadata { width, height, aspect_ratio, exif })
}

fn get_exif_data(path: &Path) -> Option<ExifData> {
  let file = std::fs::File::open(path).ok()?;
  let mut buf_reader = std::io::BufReader::new(&file);
  let exif_reader = exif::Reader::new();
  let exif = exif_reader.read_from_container(&mut buf_reader).ok()?;

  let camera = exif
    .get_field(exif::Tag::Model, exif::In::PRIMARY)
    .map(|f| f.display_value().to_string().trim_matches('"').to_string());

  let exposure = exif.get_field(exif::Tag::ExposureTime, exif::In::PRIMARY).map(|f| {
    let val = f.display_value().to_string();
    let val = val.trim_matches('"');
    // Clean up exposure time - round fractional denominators
    format_exposure(val)
  });

  let iso = exif
    .get_field(exif::Tag::PhotographicSensitivity, exif::In::PRIMARY)
    .map(|f| format!("ISO {}", f.display_value()));

  if camera.is_none() && exposure.is_none() && iso.is_none() {
    return None;
  }

  Some(ExifData { camera, exposure, iso })
}

pub fn format_permissions(mode: u32) -> String {
  let mut result = String::with_capacity(9);

  // Owner permissions
  result.push(if mode & 0o400 != 0 { 'r' } else { '-' });
  result.push(if mode & 0o200 != 0 { 'w' } else { '-' });
  result.push(if mode & 0o100 != 0 { 'x' } else { '-' });

  // Group permissions
  result.push(if mode & 0o040 != 0 { 'r' } else { '-' });
  result.push(if mode & 0o020 != 0 { 'w' } else { '-' });
  result.push(if mode & 0o010 != 0 { 'x' } else { '-' });

  // Others permissions
  result.push(if mode & 0o004 != 0 { 'r' } else { '-' });
  result.push(if mode & 0o002 != 0 { 'w' } else { '-' });
  result.push(if mode & 0o001 != 0 { 'x' } else { '-' });

  result
}

pub fn format_size(bytes: u64) -> String {
  const KB: u64 = 1024;
  const MB: u64 = KB * 1024;
  const GB: u64 = MB * 1024;

  if bytes >= GB {
    format!("{:.1} GB", bytes as f64 / GB as f64)
  } else if bytes >= MB {
    format!("{:.1} MB", bytes as f64 / MB as f64)
  } else if bytes >= KB {
    format!("{:.1} KB", bytes as f64 / KB as f64)
  } else {
    format!("{bytes} B")
  }
}

pub fn format_time(time: SystemTime) -> String {
  let now = SystemTime::now();
  let Ok(duration) = now.duration_since(time) else {
    return "future".to_string();
  };

  let secs = duration.as_secs();
  let minute = 60;
  let hour = minute * 60;
  let day = hour * 24;
  let week = day * 7;
  let month = day * 30;
  let year = day * 365;

  if secs < minute {
    "now".to_string()
  } else if secs < hour {
    let m = secs / minute;
    format!("{m}m ago")
  } else if secs < day {
    let h = secs / hour;
    format!("{h}h ago")
  } else if secs < week {
    let d = secs / day;
    format!("{d}d ago")
  } else if secs < month {
    let w = secs / week;
    format!("{w}w ago")
  } else if secs < year {
    let m = secs / month;
    format!("{m}mo ago")
  } else {
    let y = secs / year;
    format!("{y}y ago")
  }
}

pub fn calculate_aspect_ratio(width: u32, height: u32) -> String {
  if width == 0 || height == 0 {
    return "N/A".to_string();
  }

  let g = gcd(width, height);
  let w = width / g;
  let h = height / g;

  // Common aspect ratios
  let common_ratios = [
    (16, 9, "16:9"),
    (4, 3, "4:3"),
    (3, 2, "3:2"),
    (1, 1, "1:1"),
    (21, 9, "21:9"),
    (2, 1, "2:1"),
    (5, 4, "5:4"),
    (9, 16, "9:16"),
    (3, 4, "3:4"),
  ];

  // Check for common ratios with some tolerance
  let ratio = width as f64 / height as f64;
  for (rw, rh, name) in common_ratios {
    let expected = rw as f64 / rh as f64;
    if (ratio - expected).abs() < 0.02 {
      return name.to_string();
    }
  }

  // Fallback to reduced ratio
  format!("{w}:{h}")
}

fn gcd(a: u32, b: u32) -> u32 {
  if b == 0 { a } else { gcd(b, a % b) }
}

fn format_exposure(val: &str) -> String {
  // Handle formats like "1/33.3333334 s" -> "1/33 s"
  if let Some(slash_pos) = val.find('/') {
    let prefix = &val[..=slash_pos];
    let rest = &val[slash_pos + 1..];
    // Find where the number ends (at space or end of string)
    let num_end = rest.find(' ').unwrap_or(rest.len());
    let num_str = &rest[..num_end];
    let suffix = &rest[num_end..];

    // Try to parse as float and round
    if let Ok(num) = num_str.parse::<f64>() {
      let rounded = num.round() as i64;
      return format!("{prefix}{rounded}{suffix}");
    }
  }
  val.to_string()
}

pub fn get_git_commits(git_repo: Option<&GitRepo>, path: &Path, limit: usize) -> Vec<GitCommit> {
  git_repo.map(|r| r.get_file_commits(path, limit)).unwrap_or_default()
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::fs;

  #[test]
  fn test_format_permissions() {
    assert_eq!(format_permissions(0o755), "rwxr-xr-x");
    assert_eq!(format_permissions(0o644), "rw-r--r--");
    assert_eq!(format_permissions(0o700), "rwx------");
    assert_eq!(format_permissions(0o000), "---------");
    assert_eq!(format_permissions(0o777), "rwxrwxrwx");
  }

  #[test]
  fn test_format_size() {
    assert_eq!(format_size(0), "0 B");
    assert_eq!(format_size(500), "500 B");
    assert_eq!(format_size(1024), "1.0 KB");
    assert_eq!(format_size(1536), "1.5 KB");
    assert_eq!(format_size(1024 * 1024), "1.0 MB");
    assert_eq!(format_size(1024 * 1024 * 1024), "1.0 GB");
  }

  #[test]
  fn test_calculate_aspect_ratio() {
    assert_eq!(calculate_aspect_ratio(1920, 1080), "16:9");
    assert_eq!(calculate_aspect_ratio(1280, 720), "16:9");
    assert_eq!(calculate_aspect_ratio(800, 600), "4:3");
    assert_eq!(calculate_aspect_ratio(1500, 1000), "3:2");
    assert_eq!(calculate_aspect_ratio(1000, 1000), "1:1");
    assert_eq!(calculate_aspect_ratio(1080, 1920), "9:16");
  }

  #[test]
  fn test_gcd() {
    assert_eq!(gcd(16, 9), 1);
    assert_eq!(gcd(1920, 1080), 120);
    assert_eq!(gcd(100, 50), 50);
    assert_eq!(gcd(48, 18), 6);
  }

  #[test]
  fn test_format_exposure() {
    assert_eq!(format_exposure("1/33.3333334 s"), "1/33 s");
    assert_eq!(format_exposure("1/30 s"), "1/30 s");
    assert_eq!(format_exposure("1/250"), "1/250");
    assert_eq!(format_exposure("2 s"), "2 s");
  }

  #[test]
  fn test_get_file_metadata() {
    let dir = std::env::temp_dir().join("tfl_test_meta");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let file = dir.join("test.txt");
    fs::write(&file, "hello world").unwrap();

    let meta = get_file_metadata(&file).unwrap();
    assert_eq!(meta.size, 11);
    assert!(meta.modified.is_some());
    assert!(meta.permissions.is_some());

    let _ = fs::remove_dir_all(&dir);
  }

  #[test]
  fn test_format_time_now() {
    let now = SystemTime::now();
    assert_eq!(format_time(now), "now");
  }
}
