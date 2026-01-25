//! Behavioral specs for suite timeout configuration.
//!
//! Reference: docs/specs/checks/tests.md#timeout

#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::prelude::*;

/// Spec: docs/specs/checks/tests.md#timeout
///
/// > Suite timeout kills slow tests and reports failure.
#[test]
fn bats_runner_respects_timeout() {
    let temp = Project::empty();
    temp.config(
        r#"
[[check.tests.suite]]
runner = "bats"
path = "tests"
timeout = "100ms"
"#,
    );
    // Create a bats test that hangs
    temp.file(
        "tests/slow.bats",
        r#"
#!/usr/bin/env bats

@test "hangs forever" {
    sleep 60
}
"#,
    );

    check("tests")
        .pwd(temp.path())
        .args(&["--ci"])
        .fails()
        .stdout_has("timed out");
}

/// Spec: Suite without timeout runs normally.
#[test]
fn bats_runner_without_timeout_succeeds() {
    let temp = Project::empty();
    temp.config(
        r#"
[[check.tests.suite]]
runner = "bats"
path = "tests"
"#,
    );
    temp.file(
        "tests/quick.bats",
        r#"
#!/usr/bin/env bats

@test "quick test" {
    [ 1 -eq 1 ]
}
"#,
    );

    check("tests").pwd(temp.path()).args(&["--ci"]).passes();
}

/// Spec: Custom runner respects timeout configuration.
#[test]
fn custom_runner_respects_timeout() {
    let temp = Project::empty();
    temp.config(
        r#"
[[check.tests.suite]]
runner = "custom"
command = "sleep 60"
timeout = "100ms"
"#,
    );

    check("tests")
        .pwd(temp.path())
        .args(&["--ci"])
        .fails()
        .stdout_has("timed out");
}

/// Spec: Custom runner without timeout completes normally.
#[test]
fn custom_runner_without_timeout_succeeds() {
    let temp = Project::empty();
    temp.config(
        r#"
[[check.tests.suite]]
runner = "custom"
command = "echo 'done'"
"#,
    );

    check("tests").pwd(temp.path()).args(&["--ci"]).passes();
}
