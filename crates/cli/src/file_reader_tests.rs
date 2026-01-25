// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use std::io::Write;
use tempfile::NamedTempFile;

#[test]
fn reads_small_file_directly() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, "hello world").unwrap();

    let content = FileContent::read(file.path()).unwrap();
    assert!(matches!(content, FileContent::Owned(_)));
    assert_eq!(content.as_str(), Some("hello world\n"));
}

#[test]
fn reads_large_file_with_mmap() {
    let mut file = NamedTempFile::new().unwrap();
    // Write more than MMAP_THRESHOLD bytes
    let data = "x".repeat(65 * 1024);
    write!(file, "{}", data).unwrap();

    let content = FileContent::read(file.path()).unwrap();
    assert!(matches!(content, FileContent::Mapped(_)));
    assert_eq!(content.as_str().map(|s| s.len()), Some(65 * 1024));
}

#[test]
fn returns_none_for_invalid_utf8() {
    let mut file = NamedTempFile::new().unwrap();
    // Write more than MMAP_THRESHOLD bytes with invalid UTF-8
    let mut data = vec![b'x'; 65 * 1024];
    data[1000] = 0xFF; // Invalid UTF-8 byte
    file.write_all(&data).unwrap();

    let content = FileContent::read(file.path()).unwrap();
    assert!(content.as_str().is_none());
}
