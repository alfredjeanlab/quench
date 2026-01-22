//! Parallel check runner with error recovery.
//!
//! Runs checks in parallel using rayon, isolating errors so one
//! check failure doesn't prevent other checks from running.

use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;

use rayon::prelude::*;

use crate::check::{Check, CheckContext, CheckResult};
use crate::config::Config;
use crate::walker::WalkedFile;

/// Configuration for the check runner.
pub struct RunnerConfig {
    /// Maximum violations before early termination (None = unlimited).
    pub limit: Option<usize>,
}

/// The check runner executes multiple checks in parallel.
pub struct CheckRunner {
    config: RunnerConfig,
}

impl CheckRunner {
    pub fn new(config: RunnerConfig) -> Self {
        Self { config }
    }

    /// Run all provided checks and return results.
    ///
    /// Checks run in parallel. Errors are isolated - one check failing
    /// doesn't prevent other checks from running.
    pub fn run(
        &self,
        checks: Vec<Arc<dyn Check>>,
        files: &[WalkedFile],
        config: &Config,
        root: &Path,
    ) -> Vec<CheckResult> {
        let violation_count = AtomicUsize::new(0);

        // Run checks in parallel
        let results: Vec<CheckResult> = checks
            .into_par_iter()
            .map(|check| {
                let ctx = CheckContext {
                    root,
                    files,
                    config,
                    limit: self.config.limit,
                    violation_count: &violation_count,
                };

                // Catch panics to ensure error isolation
                match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| check.run(&ctx))) {
                    Ok(result) => result,
                    Err(_) => {
                        // Check panicked - return skipped result
                        CheckResult::skipped(
                            check.name(),
                            "Internal error: check panicked".to_string(),
                        )
                    }
                }
            })
            .collect();

        // Sort results by canonical check order for consistent output
        let mut sorted = results;
        sorted.sort_by_key(|r| {
            crate::checks::CHECK_NAMES
                .iter()
                .position(|&n| n == r.name)
                .unwrap_or(usize::MAX)
        });

        sorted
    }

    /// Check if early termination is needed based on violation count.
    pub fn should_terminate(&self, violation_count: usize) -> bool {
        if let Some(limit) = self.config.limit {
            violation_count >= limit
        } else {
            false
        }
    }
}

#[cfg(test)]
#[path = "runner_tests.rs"]
mod tests;
