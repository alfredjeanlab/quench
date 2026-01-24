//! Behavioral specs for config validation.
//!
//! Tests that quench correctly handles:
//! - Unknown config keys (errors)
//! - Unknown nested keys (errors)
//! - Valid config (no errors)
//!
//! Reference: docs/specs/02-config.md#validation

#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::prelude::*;

// =============================================================================
// CONFIG VALIDATION SPECS
// =============================================================================

/// Spec: docs/specs/02-config.md#validation
///
/// > Unknown keys are errors
#[test]
fn unknown_config_key_fails() {
    let temp = Project::empty();
    temp.config(
        r#"version = 1
unknown_key = true

[check.agents]
required = []
"#,
    );

    quench_cmd()
        .arg("check")
        .current_dir(temp.path())
        .assert()
        .failure()
        .stderr(predicates::str::contains("unknown field"));
}

/// Spec: docs/specs/02-config.md#validation
///
/// > Unknown nested keys are errors
#[test]
fn unknown_nested_config_key_fails() {
    let temp = Project::empty();
    temp.config(&format!(
        r#"{MINIMAL_CONFIG}
[check.unknown]
field = "value"
"#
    ));

    quench_cmd()
        .arg("check")
        .current_dir(temp.path())
        .assert()
        .failure()
        .stderr(predicates::str::contains("unknown field"));
}

/// Spec: docs/specs/02-config.md#validation
///
/// > Valid config produces no errors
#[test]
fn valid_config_no_errors() {
    let temp = Project::empty();
    temp.config(MINIMAL_CONFIG);

    quench_cmd()
        .arg("check")
        .current_dir(temp.path())
        .assert()
        .success()
        .stderr(predicates::str::is_empty());
}
