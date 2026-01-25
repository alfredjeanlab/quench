// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! File size thresholds and utilities.
//!
//! Per docs/specs/20-performance.md:
//! - < 64KB: Direct read into buffer
//! - 64KB - 1MB: Memory-mapped, full processing
//! - 1MB - 10MB: Memory-mapped, report as oversized
//! - > 10MB: Skip with warning, don't read

/// Maximum file size to process (10MB).
/// Files larger than this are skipped with a warning.
pub const MAX_FILE_SIZE: u64 = 10 * 1024 * 1024;

/// Soft limit for "oversized" reporting (1MB).
/// Files between SOFT_LIMIT and MAX_FILE_SIZE are processed
/// but may be reported as potential violations by size-aware checks.
pub const SOFT_LIMIT_SIZE: u64 = 1024 * 1024;

/// Threshold for memory-mapped I/O (64KB).
/// Files smaller than this are read directly into buffer.
pub const MMAP_THRESHOLD: u64 = 64 * 1024;

/// File size classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileSizeClass {
    /// < 64KB - direct read
    Small,
    /// 64KB - 1MB - mmap, full processing
    Normal,
    /// 1MB - 10MB - mmap, may report oversized
    Oversized,
    /// > 10MB - skip entirely
    TooLarge,
}

impl FileSizeClass {
    /// Classify a file by size.
    pub fn from_size(size: u64) -> Self {
        if size > MAX_FILE_SIZE {
            FileSizeClass::TooLarge
        } else if size > SOFT_LIMIT_SIZE {
            FileSizeClass::Oversized
        } else if size > MMAP_THRESHOLD {
            FileSizeClass::Normal
        } else {
            FileSizeClass::Small
        }
    }
}

/// Format file size for human-readable output.
///
/// If `spaced` is true, adds a space between number and unit (e.g., "1.0 MB").
/// Otherwise, no space (e.g., "1.0MB").
pub fn human_size(bytes: u64, spaced: bool) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    let space = if spaced { " " } else { "" };

    if bytes >= MB {
        format!("{:.1}{space}MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1}{space}KB", bytes as f64 / KB as f64)
    } else {
        format!("{}{space}B", bytes)
    }
}

#[cfg(test)]
#[path = "file_size_tests.rs"]
mod tests;
