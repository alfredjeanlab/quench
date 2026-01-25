// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Centralized file reading with size-based strategy.
//!
// Allow unsafe_code for memory-mapped I/O (required by memmap2).
// Safety justification:
// 1. File handle is valid (just opened)
// 2. We don't mutate the mapped memory
// 3. Stale data on concurrent modification is acceptable for linting
#![allow(unsafe_code)]
//!
//! Per docs/specs/20-performance.md:
//! - < 64KB: Direct read into buffer
//! - >= 64KB: Memory-mapped I/O

use std::fs::{self, File};
use std::io;
use std::path::Path;

use memmap2::Mmap;

use crate::file_size::MMAP_THRESHOLD;

/// Content of a file, either owned or memory-mapped.
pub enum FileContent {
    /// Small file read into memory.
    Owned(String),
    /// Large file memory-mapped.
    Mapped(MappedContent),
}

/// Memory-mapped file content with UTF-8 validation.
pub struct MappedContent {
    mmap: Mmap,
}

impl MappedContent {
    /// Get content as string slice.
    /// Returns None if content is not valid UTF-8.
    pub fn as_str(&self) -> Option<&str> {
        std::str::from_utf8(&self.mmap).ok()
    }
}

impl FileContent {
    /// Read file using appropriate strategy based on size.
    pub fn read(path: &Path) -> io::Result<Self> {
        let meta = fs::metadata(path)?;
        let size = meta.len();

        if size < MMAP_THRESHOLD {
            // Small file: direct read
            let content = fs::read_to_string(path)?;
            Ok(FileContent::Owned(content))
        } else {
            // Large file: memory-map
            let file = File::open(path)?;
            // SAFETY: File handle is valid (just opened), we don't mutate the mapped memory,
            // and stale data on concurrent modification is acceptable for linting.
            let mmap = unsafe { Mmap::map(&file)? };
            Ok(FileContent::Mapped(MappedContent { mmap }))
        }
    }

    /// Get content as string slice.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            FileContent::Owned(s) => Some(s),
            FileContent::Mapped(m) => m.as_str(),
        }
    }
}

#[cfg(test)]
#[path = "file_reader_tests.rs"]
mod tests;
