//! Shared unit test utilities.
//!
//! Provides common helpers for unit tests in the cli crate.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;
use tempfile::TempDir;

/// Creates a temp directory with a minimal quench.toml.
pub fn temp_project() -> TempDir {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("quench.toml"), "version = 1\n").unwrap();
    dir
}

/// Creates a temp directory with custom config content.
pub fn temp_project_with_config(config: &str) -> TempDir {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("quench.toml"), config).unwrap();
    dir
}

/// Creates a directory tree from a list of (path, content) pairs.
///
/// Parent directories are created automatically.
///
/// # Example
///
/// ```ignore
/// let tmp = temp_project();
/// create_tree(tmp.path(), &[
///     ("src/lib.rs", "fn main() {}"),
///     ("src/test.rs", "fn test() {}"),
/// ]);
/// ```
pub fn create_tree(root: &Path, files: &[(&str, &str)]) {
    for (path, content) in files {
        let full_path = root.join(path);
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(full_path, content).unwrap();
    }
}
