//! Check registry and discovery.
//!
//! All 8 built-in checks are registered here:
//! - cloc: Lines of code, file size limits (enabled by default)
//! - escapes: Escape hatch detection (enabled by default)
//! - agents: CLAUDE.md, .cursorrules validation (enabled by default)
//! - docs: File refs, specs validation (enabled by default)
//! - tests: Test correlation (enabled by default)
//! - git: Commit message format (disabled by default)
//! - build: Binary/bundle size + build time (disabled by default)
//! - license: License header validation (disabled by default)

pub mod cloc;
pub mod git;
pub mod stub;

use std::sync::Arc;

use crate::check::Check;

/// All registered check names in canonical order.
pub const CHECK_NAMES: &[&str] = &[
    "cloc", "escapes", "agents", "docs", "tests", "git", "build", "license",
];

/// Checks enabled by default in fast mode.
pub const DEFAULT_ENABLED: &[&str] = &["cloc", "escapes", "agents", "docs", "tests"];

/// Create all registered checks.
pub fn all_checks() -> Vec<Arc<dyn Check>> {
    vec![
        Arc::new(cloc::ClocCheck),
        Arc::new(stub::StubCheck::new(
            "escapes",
            "Escape hatch detection",
            true,
        )),
        Arc::new(stub::StubCheck::new(
            "agents",
            "Agent file validation",
            true,
        )),
        Arc::new(stub::StubCheck::new(
            "docs",
            "Documentation validation",
            true,
        )),
        Arc::new(stub::StubCheck::new("tests", "Test correlation", true)),
        Arc::new(git::GitCheck),
        Arc::new(stub::StubCheck::new("build", "Build metrics", false)),
        Arc::new(stub::StubCheck::new("license", "License headers", false)),
    ]
}

/// Get a check by name.
pub fn get_check(name: &str) -> Option<Arc<dyn Check>> {
    all_checks().into_iter().find(|c| c.name() == name)
}

/// Filter checks based on enabled/disabled flags.
///
/// Semantics:
/// - No flags: run ALL 8 checks
/// - `--<check>`: run ONLY specified checks
/// - `--no-<check>`: run all EXCEPT specified checks
pub fn filter_checks(enabled: &[String], disabled: &[String]) -> Vec<Arc<dyn Check>> {
    let all = all_checks();

    if !enabled.is_empty() {
        // Explicit enable: only run specified checks
        all.into_iter()
            .filter(|c| enabled.iter().any(|e| e == c.name()))
            .collect()
    } else {
        // Default mode: run all checks minus disabled
        all.into_iter()
            .filter(|c| !disabled.iter().any(|d| d == c.name()))
            .collect()
    }
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
