//! Filesystem utilities.

use std::io;
use std::path::Path;

/// Atomically write `contents` to `path`.
///
/// Writes to a temporary sibling file first, then renames it over the target.
/// A crash mid-write leaves the previous file intact rather than a truncated
/// or corrupt one — important for `.afk` state files that the loop re-reads
/// every iteration.
pub fn write_atomic(path: &Path, contents: &[u8]) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("state");
    let tmp_path = path.with_file_name(format!(".{file_name}.tmp-{}", std::process::id()));

    let result = (|| {
        std::fs::write(&tmp_path, contents)?;
        std::fs::rename(&tmp_path, path)
    })();

    if result.is_err() {
        let _ = std::fs::remove_file(&tmp_path);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_write_atomic_creates_file() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("state.json");
        write_atomic(&path, b"{\"a\":1}").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "{\"a\":1}");
    }

    #[test]
    fn test_write_atomic_overwrites() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("state.json");
        write_atomic(&path, b"first").unwrap();
        write_atomic(&path, b"second").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "second");
    }

    #[test]
    fn test_write_atomic_creates_parents() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("nested/dir/state.json");
        write_atomic(&path, b"data").unwrap();
        assert!(path.exists());
    }

    #[test]
    fn test_write_atomic_no_temp_left_behind() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("state.json");
        write_atomic(&path, b"data").unwrap();
        let entries: Vec<_> = std::fs::read_dir(temp.path()).unwrap().collect();
        assert_eq!(entries.len(), 1);
    }
}
