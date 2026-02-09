// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

use std::path::PathBuf;

/// Quench error types
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Configuration file not found or invalid
    #[error("config error: {message}")]
    Config { message: String, path: Option<PathBuf> },

    /// Invalid command-line arguments
    #[error("argument error: {0}")]
    Argument(String),

    /// File I/O error
    #[error("io error: {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// Internal error (bug)
    #[error("internal error: {0}")]
    Internal(String),

    /// File exceeds maximum size limit.
    #[error("file too large: {} ({} bytes, max: {} bytes)", .path.display(), .size, .max_size)]
    FileTooLarge { path: PathBuf, size: u64, max_size: u64 },

    /// Walker error.
    #[error("walk error: {message}")]
    Walk { message: String },
}

/// Result type using quench Error
pub type Result<T> = std::result::Result<T, Error>;

/// Exit codes per CLI spec
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ExitCode {
    /// All checks passed
    Success = 0,
    /// One or more checks failed
    CheckFailed = 1,
    /// Configuration or argument error
    ConfigError = 2,
    /// Internal error
    InternalError = 3,
}

impl From<&Error> for ExitCode {
    fn from(err: &Error) -> Self {
        match err {
            Error::Config { .. } | Error::Argument(_) => ExitCode::ConfigError,
            Error::Io { .. } => ExitCode::InternalError,
            Error::Internal(_) => ExitCode::InternalError,
            Error::FileTooLarge { .. } => ExitCode::CheckFailed,
            Error::Walk { .. } => ExitCode::InternalError,
        }
    }
}

#[cfg(test)]
#[path = "error_tests.rs"]
mod tests;
