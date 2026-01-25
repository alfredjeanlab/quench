// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Stub runner for unimplemented runner types.

use super::{RunnerContext, TestRunResult, TestRunner};
use crate::config::TestSuiteConfig;

/// Stub runner that always skips.
///
/// Used as a placeholder for runners that haven't been implemented yet.
pub struct StubRunner {
    name: &'static str,
}

impl StubRunner {
    /// Create a new stub runner with the given name.
    pub fn new(name: &'static str) -> Self {
        Self { name }
    }
}

impl TestRunner for StubRunner {
    fn name(&self) -> &'static str {
        self.name
    }

    fn available(&self, _ctx: &RunnerContext) -> bool {
        false
    }

    fn run(&self, _config: &TestSuiteConfig, _ctx: &RunnerContext) -> TestRunResult {
        TestRunResult::skipped(format!("{} runner not yet implemented", self.name))
    }
}
