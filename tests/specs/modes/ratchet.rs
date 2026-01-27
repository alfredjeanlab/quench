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
// Uses file-based baseline (not git notes) since these tests don't have git repos
const RATCHET_CONFIG: &str = r#"
version = 1

[git]
baseline = ".quench/baseline.json"

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

[git]
baseline = ".quench/baseline.json"

[ratchet]
check = "off"
escapes = true

[[check.escapes.patterns]]
name = "unsafe"
pattern = "unsafe"
action = "count"
threshold = 100
"#;

// File-based ratchet config for tests that don't use git
const RATCHET_FILE_CONFIG: &str = r#"
version = 1

[git]
baseline = ".quench/baseline.json"

[ratchet]
check = "error"
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
    temp.config(RATCHET_FILE_CONFIG);
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
    temp.config(RATCHET_FILE_CONFIG);
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
    temp.config(RATCHET_FILE_CONFIG);
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
    temp.config(RATCHET_FILE_CONFIG);
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
    temp.config(RATCHET_FILE_CONFIG);
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
    temp.config(RATCHET_FILE_CONFIG);
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
    temp.config(RATCHET_FILE_CONFIG);
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
    temp.config(RATCHET_FILE_CONFIG);
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

// =============================================================================
// Coverage Ratcheting Specs
// =============================================================================

/// Spec: docs/specs/04-ratcheting.md#coverage
///
/// > Coverage can't drop below baseline minus tolerance.
#[test]
#[ignore = "TODO: Phase 1202 - Coverage ratcheting requires tests check with coverage metrics"]
fn coverage_regression_fails() {
    // This spec requires the tests check to produce coverage metrics, which
    // depends on the language adapter supporting coverage. Once implemented,
    // the spec should verify:
    // - Baseline has coverage: 80%
    // - Current has coverage: 75%
    // - Output shows: coverage.total: 75.0% (min: 80.0% from baseline)
}

/// Spec: docs/specs/04-ratcheting.md#tolerance
///
/// > Coverage within tolerance passes.
#[test]
#[ignore = "TODO: Phase 1202 - Coverage ratcheting requires tests check with coverage metrics"]
fn coverage_within_tolerance_passes() {
    // This spec requires:
    // - Config with coverage_tolerance = 0.05 (5%)
    // - Baseline has coverage: 80%
    // - Current has coverage: 76% (within 5% tolerance)
    // - Check should pass
}

// =============================================================================
// Binary Size Ratcheting Specs
// =============================================================================

/// Spec: docs/specs/04-ratcheting.md#binary-size
///
/// > Binary size can't exceed baseline plus tolerance.
#[test]
#[ignore = "TODO: Phase 1215 - Binary size ratcheting requires build check with size metrics"]
fn binary_size_regression_fails() {
    // This spec requires the build check to produce binary size metrics.
    // Once implemented, the spec should verify:
    // - Baseline has binary_size: 1MB
    // - Current has binary_size: 1.5MB
    // - Output shows: binary_size.target: 1.5MB (max: 1MB from baseline)
}

// =============================================================================
// Stale Baseline Specs
// =============================================================================

const RATCHET_STALE_CONFIG: &str = r#"
version = 1

[git]
baseline = ".quench/baseline.json"

[ratchet]
check = "error"
escapes = true
stale_days = 30

[[check.escapes.patterns]]
name = "unsafe"
pattern = "unsafe"
action = "count"
threshold = 100
"#;

/// Spec: docs/specs/04-ratcheting.md#stale-baseline
///
/// > Warns when baseline is older than stale_days threshold.
#[test]
fn stale_baseline_warns() {
    let temp = Project::empty();
    temp.config(RATCHET_STALE_CONFIG);
    temp.file("CLAUDE.md", CLAUDE_MD);
    temp.file("Cargo.toml", CARGO_TOML);

    // Create baseline that is 45 days old
    fs::create_dir_all(temp.path().join(".quench")).unwrap();
    fs::write(
        temp.path().join(".quench/baseline.json"),
        r#"{
  "version": 1,
  "updated": "2025-12-01T00:00:00Z",
  "metrics": {
    "escapes": {
      "source": { "unsafe": 1 }
    }
  }
}"#,
    )
    .unwrap();

    // Source matches baseline
    temp.file("src/lib.rs", "fn f() { unsafe {} }");

    // Check passes but warns about stale baseline
    quench_cmd()
        .args(["check"])
        .current_dir(temp.path())
        .assert()
        .success()
        .stderr(
            predicates::str::contains("baseline is").and(predicates::str::contains("days old")),
        );
}

// =============================================================================
// Warn Level Specs
// =============================================================================

const RATCHET_WARN_CONFIG: &str = r#"
version = 1

[git]
baseline = ".quench/baseline.json"

[ratchet]
check = "warn"
escapes = true

[[check.escapes.patterns]]
name = "unsafe"
pattern = "unsafe"
action = "count"
threshold = 100
"#;

/// Spec: docs/specs/04-ratcheting.md#warn-level
///
/// > check = "warn" reports regressions but exits 0.
#[test]
fn warn_level_reports_but_passes() {
    let temp = Project::empty();
    temp.config(RATCHET_WARN_CONFIG);
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

    // Source has 3 unsafe blocks -> regression, but warn level
    temp.file(
        "src/lib.rs",
        "fn f() {\n    unsafe {}\n    unsafe {}\n    unsafe {}\n}",
    );

    // Should pass (exit 0) but show warning
    quench_cmd()
        .args(["check"])
        .current_dir(temp.path())
        .assert()
        .success()
        .stdout(predicates::str::contains("ratchet: WARN"))
        .stdout(predicates::str::contains(
            "escapes.unsafe: 3 (max: 1 from baseline)",
        ));
}

// =============================================================================
// Per-Package Ratcheting Specs
// =============================================================================

/// Spec: docs/specs/04-ratcheting.md#per-package
///
/// > Per-package ratcheting respects package-specific settings.
#[test]
#[ignore = "TODO: Phase 1225 - Per-package ratcheting"]
fn per_package_coverage_ratchet() {
    // This spec requires:
    // - Workspace with multiple packages
    // - Config with per-package ratchet settings
    // - Coverage varies by package
    // - Only ratcheted packages fail on regression
}

// =============================================================================
// Git Notes Baseline Specs
// =============================================================================

/// Spec: docs/specs/04-ratcheting.md#baseline-storage
///
/// > Git notes is the default baseline source
#[test]
#[ignore = "TODO: Phase 2 - Git notes as default baseline"]
fn ratchet_reads_baseline_from_git_notes_by_default() {
    // Setup: create project with git history and notes
    // Run: quench check (no --save-notes flag)
    // Assert: compares against notes, not file
}

/// Spec: docs/specs/04-ratcheting.md#baseline-storage
///
/// > Baseline falls back to file when notes unavailable
#[test]
#[ignore = "TODO: Phase 2 - Git notes as default baseline"]
fn ratchet_falls_back_to_file_when_no_notes() {
    // Setup: project with baseline.json but no notes
    // Run: quench check
    // Assert: uses file baseline
}

/// Spec: docs/specs/04-ratcheting.md#baseline-storage
///
/// > --no-notes disables git notes entirely
#[test]
fn no_notes_flag_uses_file_only() {
    let temp = Project::empty();
    temp.config(
        r#"
[git]
baseline = ".quench/baseline.json"

[ratchet]
check = "error"
escapes = true

[[check.escapes.patterns]]
name = "unsafe"
pattern = "unsafe"
action = "count"
threshold = 100
"#,
    );
    temp.file("CLAUDE.md", CLAUDE_MD);
    temp.file("Cargo.toml", CARGO_TOML);
    temp.file("src/lib.rs", "fn main() {}");

    // Create baseline file with 0 unsafe
    fs::create_dir_all(temp.path().join(".quench")).unwrap();
    fs::write(
        temp.path().join(".quench/baseline.json"),
        r#"{"version":1,"updated":"2026-01-20T00:00:00Z","metrics":{"escapes":{"source":{"unsafe":0}}}}"#,
    )
    .unwrap();

    git_init(&temp);
    git_initial_commit(&temp);

    // Add note with higher value (would fail ratchet if used)
    git_add_note(
        &temp,
        r#"{"version":1,"updated":"2026-01-20T00:00:00Z","metrics":{"escapes":{"source":{"unsafe":10}}}}"#,
    );

    // --no-notes should use file baseline, not notes
    // Use --no-git since CLAUDE.md doesn't have Commits section
    cli()
        .pwd(temp.path())
        .args(&["--no-notes", "--no-git"])
        .passes();
}

/// Spec: docs/specs/04-ratcheting.md#baseline-storage
///
/// > --base <REF> uses baseline from that commit's note for ratchet comparison
#[test]
fn base_ref_uses_baseline_from_that_commit() {
    let temp = Project::empty();
    temp.config(RATCHET_CONFIG);
    temp.file("CLAUDE.md", CLAUDE_MD);
    temp.file("Cargo.toml", CARGO_TOML);
    temp.file("src/lib.rs", "fn main() {}");

    git_init(&temp);
    git_initial_commit(&temp);

    // Add note with baseline metrics to first commit
    git_add_note(
        &temp,
        r#"{"version":1,"updated":"2026-01-20T00:00:00Z","metrics":{"escapes":{"source":{"unsafe":0}}}}"#,
    );

    // Create second commit
    git_commit(&temp, "feat: second commit");

    // Using --base HEAD~1 should use the baseline from the first commit
    // Use --no-git since CLAUDE.md doesn't have Commits section
    cli()
        .pwd(temp.path())
        .args(&["--base", "HEAD~1", "--no-git"])
        .passes();
}
