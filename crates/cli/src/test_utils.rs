//! Shared unit test utilities.
//!
//! Provides common helpers for unit tests in the cli crate.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::io::Write;
use std::path::Path;
use tempfile::{NamedTempFile, TempDir};

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

/// Creates a temp file with the given content for testing.
///
/// Returns the NamedTempFile which keeps the file alive.
pub fn temp_file_with_content(content: &str) -> NamedTempFile {
    let mut file = NamedTempFile::new().unwrap();
    write!(file, "{}", content).unwrap();
    file.flush().unwrap();
    file
}

/// Creates a temp file with content using writeln! for each line.
///
/// Useful for tests that need explicit newlines.
pub fn temp_file_with_lines(lines: &[&str]) -> NamedTempFile {
    let mut file = NamedTempFile::new().unwrap();
    for line in lines {
        writeln!(file, "{}", line).unwrap();
    }
    file.flush().unwrap();
    file
}
