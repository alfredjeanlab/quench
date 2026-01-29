// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Verbose output logger for diagnostic information.
//!
//! Writes diagnostic output to stderr. Enabled automatically
//! in `--ci` mode, or explicitly with `--verbose` or `QUENCH_DEBUG=1`.

/// Verbose output logger. Writes to stderr when enabled.
/// All output is conditional on verbose mode being enabled.
pub struct VerboseLogger {
    enabled: bool,
}

impl VerboseLogger {
    pub fn new(enabled: bool) -> Self {
        Self { enabled }
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Print a verbose line to stderr (indented as content under a section).
    pub fn log(&self, msg: &str) {
        if self.enabled {
            eprintln!("  {}", msg);
        }
    }

    /// Print a verbose section header.
    pub fn section(&self, title: &str) {
        if self.enabled {
            eprintln!("\n{}:", title);
        }
    }
}

#[cfg(test)]
#[path = "verbose_tests.rs"]
mod tests;
