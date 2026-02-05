use std::io;
use std::path::{Path, PathBuf};

/// Returns the tfl-specific trash directory, creating it if needed.
/// Uses ~/.local/share/tfl/trash on Linux/macOS.
pub fn trash_dir() -> io::Result<PathBuf> {
  let base = dirs::data_local_dir()
    .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "could not find local data directory"))?;
  let trash = base.join("tfl").join("trash");
  std::fs::create_dir_all(&trash)?;
  Ok(trash)
}

/// Move a file or directory to the tfl trash.
/// Returns the path where the file was moved to in the trash.
pub fn move_to_trash(path: &Path) -> io::Result<PathBuf> {
  let trash = trash_dir()?;
  let file_name = path.file_name().ok_or_else(|| {
    io::Error::new(io::ErrorKind::InvalidInput, "path has no filename")
  })?;

  // Create unique trash path using timestamp to avoid conflicts
  let timestamp = std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)
    .map(|d| d.as_millis())
    .unwrap_or(0);
  let trash_name = format!("{}.{}", timestamp, file_name.to_string_lossy());
  let trash_path = trash.join(&trash_name);

  // Try rename first, fall back to copy+delete for cross-filesystem moves
  if std::fs::rename(path, &trash_path).is_err() {
    copy_path(path, &trash_path)?;
    delete_path(path)?;
  }
  Ok(trash_path)
}

/// Restore a file from trash to its original location.
pub fn restore_from_trash(trash_path: &Path, original: &Path) -> io::Result<()> {
  // If original location already exists, find a unique path
  let dest = if original.exists() {
    unique_dest_path(original)
  } else {
    original.to_path_buf()
  };

  // Try rename first, fall back to copy+delete for cross-filesystem moves
  if std::fs::rename(trash_path, &dest).is_err() {
    copy_path(trash_path, &dest)?;
    delete_path(trash_path)?;
  }
  Ok(())
}

/// Delete a path (file or directory) permanently.
pub fn delete_path(path: &Path) -> io::Result<()> {
  if path.is_dir() {
    std::fs::remove_dir_all(path)
  } else {
    std::fs::remove_file(path)
  }
}

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

  #[test]
  fn test_trash_dir_created() {
    let trash = trash_dir().unwrap();
    assert!(trash.exists());
    assert!(trash.is_dir());
  }

  #[test]
  fn test_move_to_trash_file() {
    let dir = test_dir("trash_file");
    let file = dir.join("delete_me.txt");
    fs::write(&file, "goodbye").unwrap();
    assert!(file.exists());

    let trash_path = move_to_trash(&file).unwrap();
    assert!(!file.exists());
    assert!(trash_path.exists());
    assert_eq!(fs::read_to_string(&trash_path).unwrap(), "goodbye");

    // Cleanup
    let _ = fs::remove_file(&trash_path);
    let _ = fs::remove_dir_all(&dir);
  }

  #[test]
  fn test_move_to_trash_dir() {
    let dir = test_dir("trash_dir");
    let subdir = dir.join("delete_me_dir");
    fs::create_dir_all(&subdir).unwrap();
    fs::write(subdir.join("inner.txt"), "inner").unwrap();
    assert!(subdir.exists());

    let trash_path = move_to_trash(&subdir).unwrap();
    assert!(!subdir.exists());
    assert!(trash_path.exists());
    assert!(trash_path.join("inner.txt").exists());

    // Cleanup
    let _ = fs::remove_dir_all(&trash_path);
    let _ = fs::remove_dir_all(&dir);
  }

  #[test]
  fn test_restore_from_trash() {
    let dir = test_dir("restore_trash");
    let file = dir.join("restore_me.txt");
    fs::write(&file, "restore this").unwrap();

    let trash_path = move_to_trash(&file).unwrap();
    assert!(!file.exists());

    restore_from_trash(&trash_path, &file).unwrap();
    assert!(file.exists());
    assert!(!trash_path.exists());
    assert_eq!(fs::read_to_string(&file).unwrap(), "restore this");

    let _ = fs::remove_dir_all(&dir);
  }

  #[test]
  fn test_restore_conflict_gets_unique_name() {
    let dir = test_dir("restore_conflict");
    let file = dir.join("conflict.txt");
    fs::write(&file, "original").unwrap();

    let trash_path = move_to_trash(&file).unwrap();

    // Create a new file at the original location
    fs::write(&file, "new file").unwrap();

    // Restore should succeed with a unique name
    restore_from_trash(&trash_path, &file).unwrap();
    assert!(file.exists());
    assert!(dir.join("conflict_copy.txt").exists());

    let _ = fs::remove_dir_all(&dir);
  }

  #[test]
  fn test_delete_path_file() {
    let dir = test_dir("delete_file_perm");
    let file = dir.join("perm_delete.txt");
    fs::write(&file, "delete").unwrap();
    assert!(file.exists());

    delete_path(&file).unwrap();
    assert!(!file.exists());

    let _ = fs::remove_dir_all(&dir);
  }

  #[test]
  fn test_delete_path_dir() {
    let dir = test_dir("delete_dir_perm");
    let subdir = dir.join("perm_delete_dir");
    fs::create_dir_all(&subdir).unwrap();
    fs::write(subdir.join("inner.txt"), "inner").unwrap();
    assert!(subdir.exists());

    delete_path(&subdir).unwrap();
    assert!(!subdir.exists());

    let _ = fs::remove_dir_all(&dir);
  }
}
