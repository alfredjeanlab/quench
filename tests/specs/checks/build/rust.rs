// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Rust build specs.
//!
//! Reference: docs/specs/checks/build.md

#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::prelude::*;

// =============================================================================
// TARGET DETECTION SPECS
// =============================================================================

/// Spec: docs/specs/checks/build.md#targets
///
/// > Rust: `[[bin]]` in Cargo.toml
#[test]
fn build_detects_bin_from_cargo_toml() {
    let temp = Project::empty();
    temp.config(
        r#"
[check.build]
check = "error"
"#,
    );
    temp.file(
        "Cargo.toml",
        r#"
[package]
name = "testpkg"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "myapp"
path = "src/main.rs"
"#,
    );
    temp.file("src/main.rs", "fn main() {}");

    // Build the release binary
    std::process::Command::new("cargo")
        .args(["build", "--release"])
        .current_dir(temp.path())
        .output()
        .expect("cargo build should succeed");

    let result = check("build")
        .pwd(temp.path())
        .args(&["--ci"])
        .json()
        .passes();

    let metrics = result.require("metrics");
    let size = metrics.get("size").and_then(|v| v.as_object());
    assert!(size.is_some(), "should have size metrics");
    assert!(
        size.unwrap().contains_key("myapp"),
        "should detect myapp target"
    );
}

/// Spec: docs/specs/checks/build.md#targets
///
/// > Uses package name when `src/main.rs` exists (default binary)
#[test]
fn build_detects_default_binary_from_main_rs() {
    let temp = Project::empty();
    temp.config(
        r#"
[check.build]
check = "error"
"#,
    );
    temp.file(
        "Cargo.toml",
        r#"
[package]
name = "defaultbin"
version = "0.1.0"
edition = "2021"
"#,
    );
    temp.file("src/main.rs", "fn main() {}");

    // Build the release binary
    std::process::Command::new("cargo")
        .args(["build", "--release"])
        .current_dir(temp.path())
        .output()
        .expect("cargo build should succeed");

    let result = check("build")
        .pwd(temp.path())
        .args(&["--ci"])
        .json()
        .passes();

    let metrics = result.require("metrics");
    let size = metrics.get("size").and_then(|v| v.as_object());
    assert!(size.is_some(), "should have size metrics");
    assert!(
        size.unwrap().contains_key("defaultbin"),
        "should detect defaultbin from package name"
    );
}

/// Spec: docs/specs/checks/build.md#targets
///
/// > Handles multiple `[[bin]]` entries
#[test]
fn build_detects_multiple_bins() {
    let temp = Project::empty();
    temp.config(
        r#"
[check.build]
check = "error"
"#,
    );
    temp.file(
        "Cargo.toml",
        r#"
[package]
name = "multibin"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "myapp"
path = "src/bin/myapp.rs"

[[bin]]
name = "myserver"
path = "src/bin/myserver.rs"
"#,
    );
    temp.file("src/bin/myapp.rs", "fn main() {}");
    temp.file("src/bin/myserver.rs", "fn main() {}");

    // Build the release binaries
    std::process::Command::new("cargo")
        .args(["build", "--release"])
        .current_dir(temp.path())
        .output()
        .expect("cargo build should succeed");

    let result = check("build")
        .pwd(temp.path())
        .args(&["--ci"])
        .json()
        .passes();

    let metrics = result.require("metrics");
    let size = metrics.get("size").and_then(|v| v.as_object());
    assert!(size.is_some(), "should have size metrics");
    let size_obj = size.unwrap();
    assert!(size_obj.contains_key("myapp"), "should detect myapp target");
    assert!(
        size_obj.contains_key("myserver"),
        "should detect myserver target"
    );
}

// =============================================================================
// SIZE MEASUREMENT SPECS
// =============================================================================

/// Spec: docs/specs/checks/build.md#metrics
///
/// > `size`: Output file size
#[test]
fn build_measures_binary_size() {
    let temp = Project::empty();
    temp.config(
        r#"
[check.build]
check = "error"
"#,
    );
    temp.file(
        "Cargo.toml",
        r#"
[package]
name = "sizetest"
version = "0.1.0"
edition = "2021"
"#,
    );
    temp.file("src/main.rs", "fn main() { println!(\"Hello\"); }");

    // Build release binary first
    std::process::Command::new("cargo")
        .args(["build", "--release"])
        .current_dir(temp.path())
        .output()
        .expect("cargo build should succeed");

    let result = check("build")
        .pwd(temp.path())
        .args(&["--ci"])
        .json()
        .passes();

    let metrics = result.require("metrics");
    let size = metrics
        .get("size")
        .and_then(|v| v.get("sizetest"))
        .and_then(|v| v.as_u64());

    assert!(size.is_some(), "should measure binary size");
    assert!(size.unwrap() > 0, "size should be non-zero");
}

/// Spec: docs/specs/checks/build.md#json-output
///
/// > JSON metrics structure includes size object
#[test]
fn build_size_in_json_metrics() {
    let temp = Project::empty();
    temp.config(
        r#"
[check.build]
check = "error"
"#,
    );
    temp.file(
        "Cargo.toml",
        r#"
[package]
name = "jsontest"
version = "0.1.0"
edition = "2021"
"#,
    );
    temp.file("src/main.rs", "fn main() {}");

    std::process::Command::new("cargo")
        .args(["build", "--release"])
        .current_dir(temp.path())
        .output()
        .expect("cargo build should succeed");

    let result = check("build")
        .pwd(temp.path())
        .args(&["--ci"])
        .json()
        .passes();

    let metrics = result.require("metrics");

    // Verify JSON structure
    assert!(metrics.get("size").is_some(), "should have size object");
    assert!(metrics.get("time").is_some(), "should have time object");

    let size = metrics.get("size").unwrap();
    assert!(size.is_object(), "size should be an object");
}

/// Spec: docs/specs/checks/build.md#language-specific-behavior
///
/// > Rust: Size measurement uses Release binary
#[test]
fn build_requires_release_binary() {
    let temp = Project::empty();
    temp.config(
        r#"
[check.build]
check = "error"
"#,
    );
    temp.file(
        "Cargo.toml",
        r#"
[package]
name = "releasetest"
version = "0.1.0"
edition = "2021"
"#,
    );
    temp.file("src/main.rs", "fn main() {}");

    // Only build debug, not release
    std::process::Command::new("cargo")
        .args(["build"])
        .current_dir(temp.path())
        .output()
        .expect("cargo build should succeed");

    let result = check("build")
        .pwd(temp.path())
        .args(&["--ci"])
        .json()
        .passes();

    // Without release binary, check returns stub (no metrics) or empty size metrics
    let metrics = result.get("metrics");

    // If metrics exists, check that size is empty or missing the target
    if let Some(m) = metrics
        && !m.is_null()
    {
        let size = m.get("size").and_then(|v| v.as_object());
        if let Some(size_obj) = size {
            // If there's a size object, it shouldn't have the target
            // (since only debug was built, not release)
            assert!(
                !size_obj.contains_key("releasetest")
                    || size_obj
                        .get("releasetest")
                        .and_then(|v| v.as_u64())
                        .is_none(),
                "should not measure debug binary"
            );
        }
    }
    // If metrics is None or null, that's also acceptable (stub result)
}

// =============================================================================
// SIZE THRESHOLD VIOLATION SPECS
// =============================================================================

/// Spec: docs/specs/checks/build.md#configuration
///
/// > size_max = "10 MB" (Global default)
#[test]
fn build_size_exceeded_generates_violation() {
    let temp = Project::empty();
    temp.config(
        r#"
[check.build]
check = "error"
size_max = "100 bytes"
"#,
    );
    temp.file(
        "Cargo.toml",
        r#"
[package]
name = "oversized"
version = "0.1.0"
edition = "2021"
"#,
    );
    temp.file("src/main.rs", "fn main() { println!(\"Hello world\"); }");

    // Build release
    std::process::Command::new("cargo")
        .args(["build", "--release"])
        .current_dir(temp.path())
        .output()
        .expect("build should succeed");

    let result = check("build")
        .pwd(temp.path())
        .args(&["--ci"])
        .json()
        .fails();

    assert!(result.has_violation("size_exceeded"));

    let v = result.require_violation("size_exceeded");
    assert!(v.get("target").is_some(), "violation should include target");
    assert!(v.get("value").is_some(), "violation should include value");
    assert!(
        v.get("threshold").is_some(),
        "violation should include threshold"
    );
}

/// Spec: docs/specs/checks/build.md#configuration
///
/// > Binary under threshold passes
#[test]
fn build_size_under_threshold_passes() {
    let temp = Project::empty();
    temp.config(
        r#"
[check.build]
check = "error"
size_max = "100 MB"
"#,
    );
    temp.file(
        "Cargo.toml",
        r#"
[package]
name = "smallbin"
version = "0.1.0"
edition = "2021"
"#,
    );
    temp.file("src/main.rs", "fn main() {}");

    std::process::Command::new("cargo")
        .args(["build", "--release"])
        .current_dir(temp.path())
        .output()
        .expect("build should succeed");

    let result = check("build")
        .pwd(temp.path())
        .args(&["--ci"])
        .json()
        .passes();

    assert!(
        !result.has_violation("size_exceeded"),
        "should not have size violation"
    );
}

/// Spec: docs/specs/checks/build.md#configuration
///
/// > [check.build.target.myapp] size_max = "5 MB" - Per-target override
#[test]
fn build_per_target_size_max() {
    let temp = Project::empty();
    temp.config(
        r#"
[check.build]
check = "error"
size_max = "100 MB"

[check.build.target.pertarget]
size_max = "100 bytes"
"#,
    );
    temp.file(
        "Cargo.toml",
        r#"
[package]
name = "pertarget"
version = "0.1.0"
edition = "2021"
"#,
    );
    temp.file("src/main.rs", "fn main() { println!(\"Hello\"); }");

    std::process::Command::new("cargo")
        .args(["build", "--release"])
        .current_dir(temp.path())
        .output()
        .expect("build should succeed");

    let result = check("build")
        .pwd(temp.path())
        .args(&["--ci"])
        .json()
        .fails();

    assert!(
        result.has_violation("size_exceeded"),
        "per-target threshold should trigger violation"
    );
}

// =============================================================================
// BUILD TIME SPECS
// =============================================================================

/// Spec: docs/specs/checks/build.md#metrics
///
/// > `time_cold`: Clean build time
#[test]
#[ignore = "Slow: requires full build cycle"]
fn build_measures_cold_build_time() {
    let temp = Project::empty();
    temp.config(
        r#"
[check.build]
check = "error"

[ratchet]
build_time_cold = true
"#,
    );
    temp.file(
        "Cargo.toml",
        r#"
[package]
name = "timetest"
version = "0.1.0"
edition = "2021"
"#,
    );
    temp.file("src/main.rs", "fn main() {}");

    let result = check("build")
        .pwd(temp.path())
        .args(&["--ci"])
        .json()
        .passes();

    let metrics = result.require("metrics");
    let time = metrics.get("time").and_then(|v| v.as_object());
    assert!(time.is_some(), "should have time metrics");
    assert!(
        time.unwrap().get("cold").is_some(),
        "should have cold build time"
    );
}

/// Spec: docs/specs/checks/build.md#metrics
///
/// > `time_hot`: Incremental build time
#[test]
#[ignore = "Slow: requires full build cycle"]
fn build_measures_hot_build_time() {
    let temp = Project::empty();
    temp.config(
        r#"
[check.build]
check = "error"

[ratchet]
build_time_hot = true
"#,
    );
    temp.file(
        "Cargo.toml",
        r#"
[package]
name = "hottest"
version = "0.1.0"
edition = "2021"
"#,
    );
    temp.file("src/main.rs", "fn main() {}");

    // First do a release build so there's something to do incrementally
    std::process::Command::new("cargo")
        .args(["build", "--release"])
        .current_dir(temp.path())
        .output()
        .expect("cargo build should succeed");

    let result = check("build")
        .pwd(temp.path())
        .args(&["--ci"])
        .json()
        .passes();

    let metrics = result.require("metrics");
    let time = metrics.get("time").and_then(|v| v.as_object());
    assert!(time.is_some(), "should have time metrics");
    assert!(
        time.unwrap().get("hot").is_some(),
        "should have hot build time"
    );
}

/// Spec: docs/specs/checks/build.md#configuration
///
/// > time_cold_max = "60s" - threshold for cold build
#[test]
fn build_time_cold_exceeded_generates_violation() {
    let temp = Project::empty();
    temp.config(
        r#"
[check.build]
check = "error"
time_cold_max = "1ms"

[ratchet]
build_time_cold = true
"#,
    );
    temp.file(
        "Cargo.toml",
        r#"
[package]
name = "slowbuild"
version = "0.1.0"
edition = "2021"
"#,
    );
    temp.file("src/main.rs", "fn main() {}");

    let result = check("build")
        .pwd(temp.path())
        .args(&["--ci"])
        .json()
        .fails();

    assert!(result.has_violation("time_cold_exceeded"));
}

/// Spec: docs/specs/checks/build.md#configuration
///
/// > time_hot_max = "5s" - threshold for hot build
#[test]
fn build_time_hot_exceeded_generates_violation() {
    let temp = Project::empty();
    temp.config(
        r#"
[check.build]
check = "error"
time_hot_max = "1ms"

[ratchet]
build_time_hot = true
"#,
    );
    temp.file(
        "Cargo.toml",
        r#"
[package]
name = "slowhot"
version = "0.1.0"
edition = "2021"
"#,
    );
    temp.file("src/main.rs", "fn main() {}");

    // Pre-build so hot build is meaningful
    std::process::Command::new("cargo")
        .args(["build", "--release"])
        .current_dir(temp.path())
        .output()
        .expect("cargo build should succeed");

    let result = check("build")
        .pwd(temp.path())
        .args(&["--ci"])
        .json()
        .fails();

    assert!(result.has_violation("time_hot_exceeded"));
}

// =============================================================================
// VIOLATION TYPE COVERAGE SPECS
// =============================================================================

/// Spec: docs/specs/checks/build.md#json-output
///
/// > Violation types: `size_exceeded`
#[test]
fn build_violation_type_is_size_exceeded() {
    let temp = Project::empty();
    temp.config(
        r#"
[check.build]
check = "error"
size_max = "100 bytes"
"#,
    );
    temp.file(
        "Cargo.toml",
        r#"
[package]
name = "sizeviol"
version = "0.1.0"
edition = "2021"
"#,
    );
    temp.file("src/main.rs", "fn main() { println!(\"Hello\"); }");

    std::process::Command::new("cargo")
        .args(["build", "--release"])
        .current_dir(temp.path())
        .output()
        .expect("build should succeed");

    let result = check("build")
        .pwd(temp.path())
        .args(&["--ci"])
        .json()
        .fails();

    let v = result.require_violation("size_exceeded");
    assert_eq!(
        v.get("type").and_then(|v| v.as_str()),
        Some("size_exceeded")
    );
}

/// Spec: docs/specs/checks/build.md#json-output
///
/// > Violation types: `missing_target`
#[test]
fn build_violation_type_is_missing_target() {
    let temp = Project::empty();
    temp.config(
        r#"
[check.build]
check = "error"
targets = ["nonexistent"]
"#,
    );
    temp.file(
        "Cargo.toml",
        r#"
[package]
name = "missingtest"
version = "0.1.0"
edition = "2021"
"#,
    );
    temp.file("src/lib.rs", "pub fn foo() {}");

    let result = check("build")
        .pwd(temp.path())
        .args(&["--ci"])
        .json()
        .fails();

    assert!(result.has_violation("missing_target"));
}

// =============================================================================
// CI-ONLY ENFORCEMENT SPECS
// =============================================================================

/// Spec: docs/specs/checks/build.md#purpose
///
/// > CI-only. This check only runs in `--ci` mode.
#[test]
fn build_skips_without_ci_flag() {
    let temp = Project::empty();
    temp.config(
        r#"
[check.build]
check = "error"
"#,
    );
    temp.file(
        "Cargo.toml",
        r#"
[package]
name = "citest"
version = "0.1.0"
edition = "2021"
"#,
    );
    temp.file("src/main.rs", "fn main() {}");

    // Without --ci flag, build check should return stub/skipped
    let result = check("build").pwd(temp.path()).json().passes();

    // In non-CI mode, should return stub (no metrics)
    let metrics = result.get("metrics");
    assert!(
        metrics.is_none() || metrics.unwrap().is_null(),
        "build check should not collect metrics without --ci"
    );
}

/// Spec: docs/specs/checks/build.md#purpose
///
/// > Build check runs in CI mode
#[test]
fn build_runs_with_ci_flag() {
    let temp = Project::empty();
    temp.config(
        r#"
[check.build]
check = "error"
"#,
    );
    temp.file(
        "Cargo.toml",
        r#"
[package]
name = "ciruntest"
version = "0.1.0"
edition = "2021"
"#,
    );
    temp.file("src/main.rs", "fn main() {}");

    std::process::Command::new("cargo")
        .args(["build", "--release"])
        .current_dir(temp.path())
        .output()
        .expect("cargo build should succeed");

    let result = check("build")
        .pwd(temp.path())
        .args(&["--ci"])
        .json()
        .passes();

    let metrics = result.get("metrics");
    assert!(
        metrics.is_some() && !metrics.unwrap().is_null(),
        "build check should collect metrics with --ci"
    );
}

// =============================================================================
// ADVICE MESSAGE SPECS
// =============================================================================

/// Spec: docs/specs/checks/build.md#fail-threshold-exceeded
///
/// > size_exceeded advice: "Reduce binary size. Check for unnecessary dependencies."
#[test]
fn build_size_exceeded_has_correct_advice() {
    let temp = Project::empty();
    temp.config(
        r#"
[check.build]
check = "error"
size_max = "100 bytes"
"#,
    );
    temp.file(
        "Cargo.toml",
        r#"
[package]
name = "advicetest"
version = "0.1.0"
edition = "2021"
"#,
    );
    temp.file("src/main.rs", "fn main() { println!(\"Hello\"); }");

    std::process::Command::new("cargo")
        .args(["build", "--release"])
        .current_dir(temp.path())
        .output()
        .expect("build should succeed");

    let result = check("build")
        .pwd(temp.path())
        .args(&["--ci"])
        .json()
        .fails();

    let v = result.require_violation("size_exceeded");
    let advice = v.get("advice").and_then(|a| a.as_str()).unwrap();
    assert_eq!(
        advice,
        "Reduce binary size. Check for unnecessary dependencies."
    );
}

// =============================================================================
// TEXT OUTPUT FORMAT SPECS
// =============================================================================

/// Spec: docs/specs/checks/build.md#text-output
///
/// > Size violations show human-readable sizes
#[test]
fn build_size_exceeded_text_output() {
    let temp = Project::empty();
    temp.config(
        r#"
[check.build]
check = "error"
size_max = "100 bytes"
"#,
    );
    temp.file(
        "Cargo.toml",
        r#"
[package]
name = "texttest"
version = "0.1.0"
edition = "2021"
"#,
    );
    temp.file("src/main.rs", "fn main() { println!(\"Hello\"); }");

    std::process::Command::new("cargo")
        .args(["build", "--release"])
        .current_dir(temp.path())
        .output()
        .expect("build should succeed");

    // Verify text output contains human-readable size
    check("build")
        .pwd(temp.path())
        .args(&["--ci"])
        .fails()
        .stdout_has("texttest:")
        .stdout_has("(max: 100 B)");
}

/// Spec: docs/specs/checks/build.md#json-output
///
/// > Per-target breakdown in metrics.size
#[test]
fn build_json_per_target_breakdown() {
    let temp = Project::empty();
    temp.config(
        r#"
[check.build]
check = "error"
"#,
    );
    temp.file(
        "Cargo.toml",
        r#"
[package]
name = "multibin"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "app1"
path = "src/bin/app1.rs"

[[bin]]
name = "app2"
path = "src/bin/app2.rs"
"#,
    );
    temp.file("src/bin/app1.rs", "fn main() {}");
    temp.file("src/bin/app2.rs", "fn main() { println!(\"larger\"); }");

    std::process::Command::new("cargo")
        .args(["build", "--release"])
        .current_dir(temp.path())
        .output()
        .expect("build should succeed");

    let result = check("build")
        .pwd(temp.path())
        .args(&["--ci"])
        .json()
        .passes();

    let metrics = result.require("metrics");
    let size = metrics.get("size").and_then(|v| v.as_object()).unwrap();

    // Both targets should be present
    assert!(
        size.contains_key("app1"),
        "should have app1 in per-target breakdown"
    );
    assert!(
        size.contains_key("app2"),
        "should have app2 in per-target breakdown"
    );

    // Both should have non-zero sizes
    assert!(size.get("app1").and_then(|v| v.as_u64()).unwrap() > 0);
    assert!(size.get("app2").and_then(|v| v.as_u64()).unwrap() > 0);
}

/// Spec: docs/specs/checks/build.md#json-output
///
/// > Time metrics structure with cold and hot
#[test]
fn build_json_time_structure() {
    let temp = Project::empty();
    temp.config(
        r#"
[check.build]
check = "error"
"#,
    );
    temp.file(
        "Cargo.toml",
        r#"
[package]
name = "timestructure"
version = "0.1.0"
edition = "2021"
"#,
    );
    temp.file("src/main.rs", "fn main() {}");

    std::process::Command::new("cargo")
        .args(["build", "--release"])
        .current_dir(temp.path())
        .output()
        .expect("cargo build should succeed");

    let result = check("build")
        .pwd(temp.path())
        .args(&["--ci"])
        .json()
        .passes();

    let metrics = result.require("metrics");

    // Verify time object exists with cold and hot keys
    let time = metrics.get("time").unwrap();
    assert!(time.get("cold").is_some(), "time.cold should exist");
    assert!(time.get("hot").is_some(), "time.hot should exist");
}

/// Spec: docs/specs/checks/build.md#text-output
///
/// > Time violations show seconds with one decimal
#[test]
fn build_time_violation_text_format() {
    let temp = Project::empty();
    temp.config(
        r#"
[check.build]
check = "error"
time_cold_max = "1ms"

[ratchet]
build_time_cold = true
"#,
    );
    temp.file(
        "Cargo.toml",
        r#"
[package]
name = "timeformat"
version = "0.1.0"
edition = "2021"
"#,
    );
    temp.file("src/main.rs", "fn main() {}");

    check("build")
        .pwd(temp.path())
        .args(&["--ci"])
        .fails()
        .stdout_has("cold build:")
        .stdout_has("(max:");
}

/// Spec: docs/specs/checks/build.md#text-output
///
/// > Missing target shows target name
#[test]
fn build_missing_target_text_output() {
    let temp = Project::empty();
    temp.config(
        r#"
[check.build]
check = "error"
targets = ["nonexistent"]
"#,
    );
    temp.file(
        "Cargo.toml",
        r#"
[package]
name = "missingtext"
version = "0.1.0"
edition = "2021"
"#,
    );
    temp.file("src/lib.rs", "pub fn foo() {}");

    check("build")
        .pwd(temp.path())
        .args(&["--ci"])
        .fails()
        .stdout_has("target not found: nonexistent");
}
