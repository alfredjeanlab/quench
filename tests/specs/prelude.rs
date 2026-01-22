//! Test helpers for behavioral specifications.
//!
//! Provides high-level DSL for testing quench CLI behavior.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

pub use assert_cmd::prelude::*;
pub use predicates;
pub use predicates::prelude::PredicateBooleanExt;
use std::process::Command;

/// Returns a Command configured to run the quench binary
pub fn quench_cmd() -> Command {
    Command::new(assert_cmd::cargo::cargo_bin!("quench"))
}

/// High-level check builder (expanded in later phases)
#[allow(dead_code)] // KEEP UNTIL: Phase 010 fixtures
pub struct CheckBuilder {
    check_name: String,
    fixture: Option<String>,
    json: bool,
}

#[allow(dead_code)] // KEEP UNTIL: Phase 010 fixtures
impl CheckBuilder {
    pub fn new(name: &str) -> Self {
        Self {
            check_name: name.to_string(),
            fixture: None,
            json: false,
        }
    }

    pub fn on(mut self, fixture: &str) -> Self {
        self.fixture = Some(fixture.to_string());
        self
    }

    pub fn json(mut self) -> Self {
        self.json = true;
        self
    }
}

/// Create a check builder for the named check
#[allow(dead_code)] // KEEP UNTIL: Phase 010 fixtures
pub fn check(name: &str) -> CheckBuilder {
    CheckBuilder::new(name)
}

/// Get path to a test fixture directory
#[allow(dead_code)] // KEEP UNTIL: Phase 010 fixtures
pub fn fixture(name: &str) -> std::path::PathBuf {
    let manifest_dir =
        std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR should be set");
    std::path::PathBuf::from(manifest_dir)
        .parent()
        .expect("parent should exist")
        .parent()
        .expect("grandparent should exist")
        .join("tests")
        .join("fixtures")
        .join(name)
}
