//! Test helpers for behavioral specifications.
//!
//! Provides high-level DSL for testing quench CLI behavior.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

pub use assert_cmd::prelude::*;
pub use predicates;
pub use predicates::prelude::PredicateBooleanExt;
use std::path::Path;
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

/// Creates a temp directory with quench.toml (version = 1)
pub fn temp_project() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("quench.toml"), "version = 1\n").unwrap();
    dir
}

/// Run quench check with JSON output, return parsed JSON
#[allow(dead_code)] // KEEP UNTIL: More specs use this helper
pub fn check_json(dir: &Path) -> serde_json::Value {
    let output = quench_cmd()
        .args(["check", "-o", "json"])
        .current_dir(dir)
        .output()
        .unwrap();
    serde_json::from_slice(&output.stdout).unwrap()
}

/// Run quench check with args, return parsed JSON
pub fn check_json_with_args(dir: &Path, args: &[&str]) -> serde_json::Value {
    let mut cmd_args = vec!["check", "-o", "json"];
    cmd_args.extend(args);
    let output = quench_cmd()
        .args(&cmd_args)
        .current_dir(dir)
        .output()
        .unwrap();
    serde_json::from_slice(&output.stdout).unwrap()
}

/// Extract check names from JSON output
pub fn check_names(json: &serde_json::Value) -> Vec<&str> {
    json.get("checks")
        .and_then(|v| v.as_array())
        .unwrap()
        .iter()
        .filter_map(|c| c.get("name").and_then(|n| n.as_str()))
        .collect()
}

/// Find a check by name in JSON output
pub fn find_check<'a>(json: &'a serde_json::Value, name: &str) -> &'a serde_json::Value {
    json.get("checks")
        .and_then(|v| v.as_array())
        .unwrap()
        .iter()
        .find(|c| c.get("name").and_then(|n| n.as_str()) == Some(name))
        .unwrap_or_else(|| panic!("check '{}' not found in output", name))
}
