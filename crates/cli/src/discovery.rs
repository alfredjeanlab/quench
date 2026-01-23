// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Config file discovery.
//!
//! Walks from the current directory up to the git root looking for quench.toml.

use std::path::{Path, PathBuf};

use crate::error::{Error, Result};

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

/// Resolve config path from CLI arg, env var, or discovery.
///
/// Priority:
/// 1. CLI flag `-C`/`--config` (handled by clap with env = "QUENCH_CONFIG")
/// 2. Discovery from current directory up to git root
/// 3. None (use defaults)
pub fn resolve_config(explicit: Option<&Path>, cwd: &Path) -> Result<Option<PathBuf>> {
    match explicit {
        Some(path) => {
            if path.exists() {
                Ok(Some(path.to_path_buf()))
            } else {
                Err(Error::Config {
                    message: format!("config file not found: {}", path.display()),
                    path: Some(path.to_path_buf()),
                })
            }
        }
        None => Ok(find_config(cwd)),
    }
}

#[cfg(test)]
#[path = "discovery_tests.rs"]
mod tests;
