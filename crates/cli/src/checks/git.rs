// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Git check implementation.
//!
//! Validates commit message format and git-related conventions.
//! Skips if not in a git repository.

use std::process::Command;

use crate::check::{Check, CheckContext, CheckResult};

/// The git check validates commit message format.
pub struct GitCheck;

impl Check for GitCheck {
    fn name(&self) -> &'static str {
        "git"
    }

    fn description(&self) -> &'static str {
        "Commit message format"
    }

    fn run(&self, ctx: &CheckContext) -> CheckResult {
        // Check if we're in a git repository
        let output = Command::new("git")
            .arg("rev-parse")
            .arg("--git-dir")
            .current_dir(ctx.root)
            .output();

        match output {
            Ok(out) if out.status.success() => {
                // We're in a git repo - stub for now (not yet implemented)
                CheckResult::stub(self.name())
            }
            _ => {
                // Not a git repo - skip
                CheckResult::skipped(self.name(), "Not a git repository")
            }
        }
    }

    fn default_enabled(&self) -> bool {
        false
    }
}

#[cfg(test)]
#[path = "git_tests.rs"]
mod tests;
