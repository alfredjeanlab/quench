//! Size-gated file reading.
//!
//! Selects read strategy based on file size:
//! - < 10MB: Direct read into buffer
//! - > 10MB: Rejected with error
//!
//! Note: Memory-mapped I/O support is disabled due to workspace lint
//! forbidding unsafe code. This can be revisited in the future if the
//! lint policy changes.

use std::fs::File;
use std::io::Read;
use std::path::Path;

use crate::error::{Error, Result};

/// Size at which to warn about large files (1MB).
pub const LARGE_FILE_WARN: u64 = 1024 * 1024;

/// Maximum file size to read (10MB).
pub const MAX_FILE_SIZE: u64 = 10 * 1024 * 1024;

/// File content with metadata.
#[derive(Debug)]
pub struct FileContent {
    /// The file content as bytes.
    pub bytes: Vec<u8>,

    /// File size in bytes.
    pub size: u64,
}

/// Read strategy used for a file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadStrategy {
    /// Direct read into buffer.
    Direct,

    /// Skipped due to size (> 10MB).
    Skipped,
}

impl ReadStrategy {
    /// Determine the read strategy for a file of the given size.
    pub fn for_size(size: u64) -> Self {
        if size > MAX_FILE_SIZE {
            ReadStrategy::Skipped
        } else {
            ReadStrategy::Direct
        }
    }
}

/// Size-gated file reader.
pub struct FileReader {
    /// Maximum file size to read.
    max_size: u64,
}

impl Default for FileReader {
    fn default() -> Self {
        Self {
            max_size: MAX_FILE_SIZE,
        }
    }
}

impl FileReader {
    /// Create a new file reader with default thresholds.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a reader with custom max size.
    pub fn with_max_size(max_size: u64) -> Self {
        Self { max_size }
    }

    /// Read a file, checking size limits.
    ///
    /// Returns `Err(FileTooLarge)` for files exceeding max_size.
    pub fn read(&self, path: &Path) -> Result<FileContent> {
        let metadata = std::fs::metadata(path).map_err(|e| Error::Io {
            path: path.to_path_buf(),
            source: e,
        })?;

        let size = metadata.len();

        // Check size before reading
        if size > self.max_size {
            return Err(Error::FileTooLarge {
                path: path.to_path_buf(),
                size,
                max_size: self.max_size,
            });
        }

        // Report large files (1MB - 10MB)
        if size > LARGE_FILE_WARN {
            tracing::info!(
                path = %path.display(),
                size_mb = size as f64 / 1_000_000.0,
                "Reading large file"
            );
        }

        let bytes = self.read_direct(path, size)?;

        Ok(FileContent { bytes, size })
    }

    /// Read file directly into buffer.
    fn read_direct(&self, path: &Path, size: u64) -> Result<Vec<u8>> {
        let mut file = File::open(path).map_err(|e| Error::Io {
            path: path.to_path_buf(),
            source: e,
        })?;

        let mut buffer = Vec::with_capacity(size as usize);
        file.read_to_end(&mut buffer).map_err(|e| Error::Io {
            path: path.to_path_buf(),
            source: e,
        })?;

        Ok(buffer)
    }

    /// Check if a file should be read based on size.
    pub fn should_read(&self, path: &Path) -> Result<ReadStrategy> {
        let metadata = std::fs::metadata(path).map_err(|e| Error::Io {
            path: path.to_path_buf(),
            source: e,
        })?;

        Ok(ReadStrategy::for_size(metadata.len()))
    }
}

#[cfg(test)]
#[path = "reader_tests.rs"]
mod tests;
