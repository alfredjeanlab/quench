//! Behavioral specs for environment variables.
//!
//! Tests that quench correctly handles:
//! - NO_COLOR (disables color output per no-color.org)
//! - COLOR (forces color output)
//! - QUENCH_LOG (enables debug/trace logging)
//! - Unknown QUENCH_* vars (silently ignored)
//!
//! Reference: docs/specs/02-config.md#environment-variables
//! Reference: docs/specs/03-output.md#colorization

#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::prelude::*;

// =============================================================================
// ENVIRONMENT VARIABLE SPECS
// =============================================================================

/// Spec: docs/specs/03-output.md#colorization
///
/// > NO_COLOR=1 disables color output
#[test]
fn env_no_color_disables_color() {
    let temp = Project::empty();
    temp.config(MINIMAL_CONFIG);
    temp.file("test.rs", "fn main() {}\n");

    let output = quench_cmd()
        .arg("check")
        .current_dir(temp.path())
        .env("NO_COLOR", "1")
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    // ANSI escape codes start with \x1b[
    assert!(
        !stdout.contains("\x1b["),
        "output should not contain ANSI codes"
    );
}

/// Spec: docs/specs/02-config.md#environment-variables
///
/// > QUENCH_LOG enables debug logging to stderr
#[test]
fn env_log_enables_debug() {
    let temp = Project::empty();
    temp.config(MINIMAL_CONFIG);

    quench_cmd()
        .arg("check")
        .current_dir(temp.path())
        .env("QUENCH_LOG", "debug")
        .assert()
        .success()
        .stderr(predicates::str::contains("DEBUG").or(predicates::str::contains("debug")));
}

/// Spec: docs/specs/02-config.md#environment-variables
///
/// > QUENCH_LOG=trace enables trace logging
#[test]
fn env_log_trace_level() {
    let temp = Project::empty();
    temp.config(MINIMAL_CONFIG);

    quench_cmd()
        .arg("check")
        .current_dir(temp.path())
        .env("QUENCH_LOG", "trace")
        .assert()
        .success()
        .stderr(predicates::str::contains("TRACE").or(predicates::str::contains("trace")));
}

/// Spec: docs/specs/02-config.md#environment-variables
///
/// > Unknown QUENCH_* environment variables are silently ignored
#[test]
fn env_unknown_vars_ignored() {
    quench_cmd()
        .arg("--help")
        .env("QUENCH_UNKNOWN_VAR", "some_value")
        .assert()
        .success(); // Should not error on unknown env vars
}
