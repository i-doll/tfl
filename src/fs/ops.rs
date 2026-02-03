use std::io;
use std::path::{Path, PathBuf};

/// Returns a unique destination path by appending `_copy`, `_copy2`, etc.
/// if the path already exists.
pub fn unique_dest_path(dest: &Path) -> PathBuf {
  if !dest.exists() {
    return dest.to_path_buf();
  }

  let stem = dest
    .file_stem()
    .map(|s| s.to_string_lossy().to_string())
    .unwrap_or_default();
  let ext = dest.extension().map(|e| e.to_string_lossy().to_string());
  let parent = dest.parent().unwrap_or(Path::new("."));

  let make_name = |suffix: &str| -> PathBuf {
    match &ext {
      Some(e) => parent.join(format!("{stem}{suffix}.{e}")),
      None => parent.join(format!("{stem}{suffix}")),
    }
  };

  let first = make_name("_copy");
  if !first.exists() {
    return first;
  }

  let mut n = 2u32;
  loop {
    let candidate = make_name(&format!("_copy{n}"));
    if !candidate.exists() {
      return candidate;
    }
    n += 1;
  }
}

/// Copy a file or directory to `dest`. For directories, copies recursively.
pub fn copy_path(source: &Path, dest: &Path) -> io::Result<()> {
  if source.is_dir() {
    copy_dir_recursive(source, dest)
  } else {
    std::fs::copy(source, dest)?;
    Ok(())
  }
}

/// Recursively copy a directory and all its contents.
pub fn copy_dir_recursive(source: &Path, dest: &Path) -> io::Result<()> {
  std::fs::create_dir_all(dest)?;
  for entry in std::fs::read_dir(source)? {
    let entry = entry?;
    let src_path = entry.path();
    let dst_path = dest.join(entry.file_name());
    if src_path.is_dir() {
      copy_dir_recursive(&src_path, &dst_path)?;
    } else {
      std::fs::copy(&src_path, &dst_path)?;
    }
  }
  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::fs;
  use std::sync::atomic::{AtomicU32, Ordering};

  static COUNTER: AtomicU32 = AtomicU32::new(0);

  fn test_dir(prefix: &str) -> PathBuf {
    let id = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!("tfl_ops_{prefix}_{id}_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
  }

  #[test]
  fn test_unique_dest_path_no_conflict() {
    let dir = test_dir("no_conflict");
    let dest = dir.join("foo.txt");
    assert_eq!(unique_dest_path(&dest), dest);
    let _ = fs::remove_dir_all(&dir);
  }

  #[test]
  fn test_unique_dest_path_with_extension() {
    let dir = test_dir("with_ext");
    let dest = dir.join("foo.txt");
    fs::write(&dest, "").unwrap();
    let result = unique_dest_path(&dest);
    assert_eq!(result, dir.join("foo_copy.txt"));
    let _ = fs::remove_dir_all(&dir);
  }

  #[test]
  fn test_unique_dest_path_without_extension() {
    let dir = test_dir("no_ext");
    let dest = dir.join("foo");
    fs::write(&dest, "").unwrap();
    let result = unique_dest_path(&dest);
    assert_eq!(result, dir.join("foo_copy"));
    let _ = fs::remove_dir_all(&dir);
  }

  #[test]
  fn test_unique_dest_path_incrementing() {
    let dir = test_dir("incr");
    let dest = dir.join("foo.txt");
    fs::write(&dest, "").unwrap();
    fs::write(dir.join("foo_copy.txt"), "").unwrap();
    let result = unique_dest_path(&dest);
    assert_eq!(result, dir.join("foo_copy2.txt"));

    fs::write(dir.join("foo_copy2.txt"), "").unwrap();
    let result = unique_dest_path(&dest);
    assert_eq!(result, dir.join("foo_copy3.txt"));
    let _ = fs::remove_dir_all(&dir);
  }

  #[test]
  fn test_copy_path_file() {
    let dir = test_dir("copy_file");
    let src = dir.join("src.txt");
    let dst = dir.join("dst.txt");
    fs::write(&src, "hello").unwrap();
    copy_path(&src, &dst).unwrap();
    assert_eq!(fs::read_to_string(&dst).unwrap(), "hello");
    let _ = fs::remove_dir_all(&dir);
  }

  #[test]
  fn test_copy_dir_recursive() {
    let dir = test_dir("copy_dir");
    let src = dir.join("src_dir");
    fs::create_dir_all(src.join("sub")).unwrap();
    fs::write(src.join("a.txt"), "aaa").unwrap();
    fs::write(src.join("sub").join("b.txt"), "bbb").unwrap();

    let dst = dir.join("dst_dir");
    copy_path(&src, &dst).unwrap();

    assert!(dst.join("a.txt").exists());
    assert!(dst.join("sub").join("b.txt").exists());
    assert_eq!(fs::read_to_string(dst.join("a.txt")).unwrap(), "aaa");
    assert_eq!(fs::read_to_string(dst.join("sub").join("b.txt")).unwrap(), "bbb");
    let _ = fs::remove_dir_all(&dir);
  }
}
