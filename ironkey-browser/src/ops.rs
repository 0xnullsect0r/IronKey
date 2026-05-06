//! File operations: copy, move, rename, delete.

use anyhow::{Context, Result};
use std::path::Path;

/// Copy `src` to `dst`, preserving metadata where possible.
pub fn copy_file(src: &Path, dst: &Path) -> Result<u64> {
    let bytes = std::fs::copy(src, dst)
        .with_context(|| format!("Copying {} to {}", src.display(), dst.display()))?;
    Ok(bytes)
}

/// Move (rename) `src` to `dst`. Falls back to copy+delete across filesystems.
pub fn move_file(src: &Path, dst: &Path) -> Result<()> {
    if std::fs::rename(src, dst).is_err() {
        // Cross-device move: copy then delete
        copy_file(src, dst)?;
        std::fs::remove_file(src)
            .with_context(|| format!("Deleting source file {}", src.display()))?;
    }
    Ok(())
}

/// Rename `src` to a new name in the same directory.
pub fn rename_file(src: &Path, new_name: &str) -> Result<std::path::PathBuf> {
    let parent = src.parent().unwrap_or(Path::new("/"));
    let dst = parent.join(new_name);
    std::fs::rename(src, &dst)
        .with_context(|| format!("Renaming {} to {}", src.display(), dst.display()))?;
    Ok(dst)
}

/// Delete a file or empty directory.
///
/// For safety, this does NOT recursively delete directories.
/// Use `delete_dir_recursive` explicitly for directories.
pub fn delete_file(path: &Path) -> Result<()> {
    if path.is_dir() {
        std::fs::remove_dir(path)
            .with_context(|| format!("Removing directory {} (must be empty)", path.display()))?;
    } else {
        std::fs::remove_file(path)
            .with_context(|| format!("Removing file {}", path.display()))?;
    }
    Ok(())
}

/// Recursively delete a directory and all its contents.
pub fn delete_dir_recursive(path: &Path) -> Result<()> {
    std::fs::remove_dir_all(path)
        .with_context(|| format!("Removing directory tree {}", path.display()))
}

/// Create a new directory at `path`.
pub fn create_directory(path: &Path) -> Result<()> {
    std::fs::create_dir(path)
        .with_context(|| format!("Creating directory {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn copy_and_delete() {
        let dir = tempfile_dir();
        let src = dir.join("src.txt");
        let dst = dir.join("dst.txt");
        let mut f = std::fs::File::create(&src).unwrap();
        f.write_all(b"hello ironkey").unwrap();
        drop(f);

        let bytes = copy_file(&src, &dst).unwrap();
        assert_eq!(bytes, 13);
        assert!(dst.exists());

        delete_file(&src).unwrap();
        assert!(!src.exists());

        delete_file(&dst).unwrap();
    }

    #[test]
    fn rename_file_test() {
        let dir = tempfile_dir();
        let src = dir.join("old.txt");
        std::fs::write(&src, b"data").unwrap();

        let new_path = rename_file(&src, "new.txt").unwrap();
        assert!(new_path.exists());
        assert!(!src.exists());
        delete_file(&new_path).unwrap();
    }

    fn tempfile_dir() -> std::path::PathBuf {
        let dir = std::path::PathBuf::from("/tmp/ironkey_test");
        std::fs::create_dir_all(&dir).ok();
        dir
    }
}
