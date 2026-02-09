// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Stub check implementation for unimplemented checks.

use crate::check::{Check, CheckContext, CheckResult};

/// A stub check that always passes.
/// Used for checks not yet implemented.
pub struct StubCheck {
    name: &'static str,
    description: &'static str,
    default_enabled: bool,
}

impl StubCheck {
    pub fn new(name: &'static str, description: &'static str, default_enabled: bool) -> Self {
        Self { name, description, default_enabled }
    }
}

impl Check for StubCheck {
    fn name(&self) -> &'static str {
        self.name
    }

    fn description(&self) -> &'static str {
        self.description
    }

    fn run(&self, _ctx: &CheckContext) -> CheckResult {
        // Stub checks always pass (no implementation yet)
        CheckResult::stub(self.name)
    }

    fn default_enabled(&self) -> bool {
        self.default_enabled
    }
}

#[cfg(test)]
#[path = "stub_tests.rs"]
mod tests;
