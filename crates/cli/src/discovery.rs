// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Config file discovery.
//!
//! Walks from the current directory up to the git root looking for quench.toml.

use std::path::{Path, PathBuf};

/// Find quench.toml starting from `start_dir` and walking up to git root.
pub fn find_config(start_dir: &Path) -> Option<PathBuf> {
    let mut current = start_dir.to_path_buf();

    loop {
        let config_path = current.join("quench.toml");
        if config_path.exists() {
            return Some(config_path);
        }

        // Stop at git root
        if current.join(".git").exists() {
            return None;
        }

        // Move up one directory
        match current.parent() {
            Some(parent) => current = parent.to_path_buf(),
            None => return None,
        }
    }
}

#[cfg(test)]
#[path = "discovery_tests.rs"]
mod tests;
