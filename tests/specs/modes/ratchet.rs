//! Ratchet behavioral specifications.
//!
//! Tests baseline file I/O and ratchet comparison behavior.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::prelude::*;
use std::fs;

const CLAUDE_MD: &str =
    "# Project\n\n## Directory Structure\n\nLayout.\n\n## Landing the Plane\n\n- Done\n";
const CARGO_TOML: &str = "[package]\nname = \"test\"\nversion = \"0.1.0\"\nedition = \"2024\"\n";

// Config that enables ratcheting with escapes in count mode (not comment mode)
// High threshold (100) ensures the escapes check passes, letting ratchet do the enforcement
const RATCHET_CONFIG: &str = r#"
version = 1

[ratchet]
check = "error"
escapes = true

[[check.escapes.patterns]]
name = "unsafe"
pattern = "unsafe"
action = "count"
threshold = 100
"#;

const RATCHET_OFF_CONFIG: &str = r#"
version = 1

[ratchet]
check = "off"
escapes = true

[[check.escapes.patterns]]
name = "unsafe"
pattern = "unsafe"
action = "count"
threshold = 100
"#;

/// Spec: docs/specs/04-ratcheting.md#no-baseline
///
/// > Without a baseline file, ratchet check passes (nothing to compare against).
/// > Suggests creating one with --fix in verbose mode.
#[test]
fn no_baseline_passes() {
    let temp = Project::empty();
    temp.config(RATCHET_CONFIG);
    temp.file("CLAUDE.md", CLAUDE_MD);
    temp.file("Cargo.toml", CARGO_TOML);
    temp.file("src/lib.rs", "fn main() {}");

    cli().pwd(temp.path()).passes();
}

/// Spec: docs/specs/04-ratcheting.md#no-baseline-verbose
///
/// > In verbose mode, suggests creating baseline with --fix.
#[test]
fn no_baseline_verbose_suggests_fix() {
    let temp = Project::empty();
    temp.config(RATCHET_CONFIG);
    temp.file("CLAUDE.md", CLAUDE_MD);
    temp.file("Cargo.toml", CARGO_TOML);
    temp.file("src/lib.rs", "fn main() {}");

    // Uses quench_cmd() directly for debug output
    quench_cmd()
        .args(["check"])
        .current_dir(temp.path())
        .env("QUENCH_DEBUG", "1")
        .assert()
        .success()
        .stderr(predicates::str::contains("No baseline found"));
}

/// Spec: docs/specs/04-ratcheting.md#fix-creates-baseline
///
/// > --fix creates initial baseline when none exists.
#[test]
fn fix_creates_baseline() {
    let temp = Project::empty();
    temp.config(RATCHET_CONFIG);
    temp.file("CLAUDE.md", CLAUDE_MD);
    temp.file("Cargo.toml", CARGO_TOML);
    temp.file("src/lib.rs", "fn f() { unsafe {} }");

    quench_cmd()
        .args(["check", "--fix"])
        .current_dir(temp.path())
        .assert()
        .success()
        .stderr(predicates::str::contains("created initial baseline"));

    assert!(
        temp.path().join(".quench/baseline.json").exists(),
        "baseline file should be created"
    );

    // Verify baseline contains the escape count
    let baseline_content = fs::read_to_string(temp.path().join(".quench/baseline.json")).unwrap();
    assert!(
        baseline_content.contains("\"unsafe\""),
        "baseline should track unsafe escapes"
    );
}

/// Spec: docs/specs/04-ratcheting.md#regression-fails
///
/// > Ratchet check fails when current metrics exceed baseline.
#[test]
fn regression_fails() {
    let temp = Project::empty();
    temp.config(RATCHET_CONFIG);
    temp.file("CLAUDE.md", CLAUDE_MD);
    temp.file("Cargo.toml", CARGO_TOML);

    // Create baseline with 1 unsafe
    fs::create_dir_all(temp.path().join(".quench")).unwrap();
    fs::write(
        temp.path().join(".quench/baseline.json"),
        r#"{
  "version": 1,
  "updated": "2026-01-20T00:00:00Z",
  "metrics": {
    "escapes": {
      "source": { "unsafe": 1 }
    }
  }
}"#,
    )
    .unwrap();

    // Source has 2 unsafe blocks on different lines -> regression
    temp.file("src/lib.rs", "fn f() {\n    unsafe {}\n    unsafe {}\n}");

    cli()
        .pwd(temp.path())
        .fails()
        .stdout_has("escapes.unsafe: 2 (max: 1 from baseline)");
}

/// Spec: docs/specs/04-ratcheting.md#same-value-passes
///
/// > Ratchet check passes when current equals baseline (no regression).
#[test]
fn same_value_passes() {
    let temp = Project::empty();
    temp.config(RATCHET_CONFIG);
    temp.file("CLAUDE.md", CLAUDE_MD);
    temp.file("Cargo.toml", CARGO_TOML);

    // Create baseline with 2 unsafe
    fs::create_dir_all(temp.path().join(".quench")).unwrap();
    fs::write(
        temp.path().join(".quench/baseline.json"),
        r#"{
  "version": 1,
  "updated": "2026-01-20T00:00:00Z",
  "metrics": {
    "escapes": {
      "source": { "unsafe": 2 }
    }
  }
}"#,
    )
    .unwrap();

    // Source has 2 unsafe blocks on different lines -> same, passes
    temp.file("src/lib.rs", "fn f() {\n    unsafe {}\n    unsafe {}\n}");

    cli().pwd(temp.path()).passes();
}

/// Spec: docs/specs/04-ratcheting.md#improvement-passes
///
/// > Ratchet check passes when current is better than baseline.
#[test]
fn improvement_passes() {
    let temp = Project::empty();
    temp.config(RATCHET_CONFIG);
    temp.file("CLAUDE.md", CLAUDE_MD);
    temp.file("Cargo.toml", CARGO_TOML);

    // Create baseline with 5 unsafe
    fs::create_dir_all(temp.path().join(".quench")).unwrap();
    fs::write(
        temp.path().join(".quench/baseline.json"),
        r#"{
  "version": 1,
  "updated": "2026-01-20T00:00:00Z",
  "metrics": {
    "escapes": {
      "source": { "unsafe": 5 }
    }
  }
}"#,
    )
    .unwrap();

    // Source has 2 unsafe blocks on different lines -> improvement from 5 to 2, passes
    temp.file("src/lib.rs", "fn f() {\n    unsafe {}\n    unsafe {}\n}");

    cli().pwd(temp.path()).passes();
}

/// Spec: docs/specs/04-ratcheting.md#fix-updates-baseline
///
/// > --fix updates baseline when metrics improve.
#[test]
fn fix_updates_baseline_on_improvement() {
    let temp = Project::empty();
    temp.config(RATCHET_CONFIG);
    temp.file("CLAUDE.md", CLAUDE_MD);
    temp.file("Cargo.toml", CARGO_TOML);

    // Create baseline with 5 unsafe
    fs::create_dir_all(temp.path().join(".quench")).unwrap();
    fs::write(
        temp.path().join(".quench/baseline.json"),
        r#"{
  "version": 1,
  "updated": "2026-01-20T00:00:00Z",
  "metrics": {
    "escapes": {
      "source": { "unsafe": 5 }
    }
  }
}"#,
    )
    .unwrap();

    // Source has 2 unsafe blocks on different lines -> improvement from 5 to 2
    temp.file("src/lib.rs", "fn f() {\n    unsafe {}\n    unsafe {}\n}");

    quench_cmd()
        .args(["check", "--fix"])
        .current_dir(temp.path())
        .assert()
        .success()
        .stderr(predicates::str::contains("5 -> 2 (new ceiling)"));

    // Verify baseline was updated
    let baseline_content = fs::read_to_string(temp.path().join(".quench/baseline.json")).unwrap();
    assert!(
        baseline_content.contains("\"unsafe\": 2"),
        "baseline should be updated to new value"
    );
}

/// Spec: docs/specs/04-ratcheting.md#fix-message-variants
///
/// > --fix reports "baseline synced" when no improvements detected.
#[test]
fn fix_baseline_synced_message() {
    let temp = Project::empty();
    temp.config(RATCHET_CONFIG);
    temp.file("CLAUDE.md", CLAUDE_MD);
    temp.file("Cargo.toml", CARGO_TOML);

    // Create baseline with 2 unsafe
    fs::create_dir_all(temp.path().join(".quench")).unwrap();
    fs::write(
        temp.path().join(".quench/baseline.json"),
        r#"{
  "version": 1,
  "updated": "2026-01-20T00:00:00Z",
  "metrics": {
    "escapes": {
      "source": { "unsafe": 2 }
    }
  }
}"#,
    )
    .unwrap();

    // Source has 2 unsafe -> no improvement, just sync
    temp.file("src/lib.rs", "fn f() {\n    unsafe {}\n    unsafe {}\n}");

    quench_cmd()
        .args(["check", "--fix"])
        .current_dir(temp.path())
        .assert()
        .success()
        .stderr(predicates::str::contains("baseline synced"));
}

/// Spec: docs/specs/04-ratcheting.md#fix-message-variants
///
/// > --fix reports "updated baseline" with improvement details when metrics improve.
#[test]
fn fix_baseline_updated_with_improvements() {
    let temp = Project::empty();
    temp.config(RATCHET_CONFIG);
    temp.file("CLAUDE.md", CLAUDE_MD);
    temp.file("Cargo.toml", CARGO_TOML);

    // Create baseline with 5 unsafe
    fs::create_dir_all(temp.path().join(".quench")).unwrap();
    fs::write(
        temp.path().join(".quench/baseline.json"),
        r#"{
  "version": 1,
  "updated": "2026-01-20T00:00:00Z",
  "metrics": {
    "escapes": {
      "source": { "unsafe": 5 }
    }
  }
}"#,
    )
    .unwrap();

    // Source has 2 unsafe -> improvement
    temp.file("src/lib.rs", "fn f() {\n    unsafe {}\n    unsafe {}\n}");

    quench_cmd()
        .args(["check", "--fix"])
        .current_dir(temp.path())
        .assert()
        .success()
        .stderr(predicates::str::contains("updated baseline"))
        .stderr(predicates::str::contains("5 -> 2"));
}

/// Spec: docs/specs/04-ratcheting.md#ratchet-disabled
///
/// > check = "off" disables ratchet checking entirely.
#[test]
fn ratchet_disabled_with_check_off() {
    let temp = Project::empty();
    temp.config(RATCHET_OFF_CONFIG);
    temp.file("CLAUDE.md", CLAUDE_MD);
    temp.file("Cargo.toml", CARGO_TOML);

    // Create baseline with 1 unsafe
    fs::create_dir_all(temp.path().join(".quench")).unwrap();
    fs::write(
        temp.path().join(".quench/baseline.json"),
        r#"{
  "version": 1,
  "updated": "2026-01-20T00:00:00Z",
  "metrics": {
    "escapes": {
      "source": { "unsafe": 1 }
    }
  }
}"#,
    )
    .unwrap();

    // Source has 10 unsafe blocks -> would be regression, but check is off
    temp.file(
        "src/lib.rs",
        "fn f() { unsafe {} unsafe {} unsafe {} unsafe {} unsafe {} unsafe {} unsafe {} unsafe {} unsafe {} unsafe {} }",
    );

    cli().pwd(temp.path()).passes();
}
