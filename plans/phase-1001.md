# Phase 1001: Build Check - Behavioral Specs

## Overview

Implement behavioral specifications (black-box tests) for the `build` check. The build check measures binary sizes and build times for compiled targets, generating violations when thresholds are exceeded. This is a CI-only check that skips in fast mode.

The existing implementation in `crates/cli/src/checks/build/mod.rs` provides basic metrics collection but lacks threshold enforcement and violation generation. This phase adds behavioral specs that drive the completion of violation logic.

## Project Structure

```
quench/
├── tests/
│   ├── specs.rs                      # Add: build module registration
│   ├── specs/
│   │   └── checks/
│   │       └── build.rs              # New: behavioral specs
│   └── fixtures/
│       └── build/                    # New: test fixtures
│           ├── rust-binary/          # Basic Rust binary project
│           ├── rust-multi-bin/       # Multiple [[bin]] targets
│           ├── rust-oversized/       # Binary exceeds size_max
│           └── rust-slow-build/      # Build exceeds time threshold
└── crates/cli/src/checks/build/
    └── mod.rs                        # Extend: threshold & violation logic
```

## Dependencies

**Existing:**
- `toml` - Cargo.toml parsing (already used)
- `serde_json` - JSON metrics output (already used)

**Test fixtures require:**
- Minimal Cargo.toml with `[[bin]]` sections
- Compiled binaries for size measurement (pre-built or built during test)

## Implementation Phases

### Phase 1: Test Infrastructure Setup

**Goal:** Create the spec file and register it in the test harness.

**Files:**
- `tests/specs/checks/build.rs` - Create with module header
- `tests/specs.rs` - Add module registration

**Code:**
```rust
// tests/specs/checks/build.rs
//! Behavioral specs for the build check.
//!
//! Tests that quench correctly:
//! - Detects binary targets from Cargo.toml
//! - Measures binary sizes
//! - Generates violations for size/time thresholds
//!
//! Reference: docs/specs/checks/build.md

#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::prelude::*;
```

```rust
// Add to tests/specs.rs
#[path = "specs/checks/build.rs"]
mod checks_build;
```

**Verification:**
```bash
cargo test --test specs -- checks_build --list
```

### Phase 2: Target Detection Specs

**Goal:** Verify binary target detection from Cargo.toml.

**Specs:**
1. `build_detects_bin_from_cargo_toml` - Detects `[[bin]]` section
2. `build_detects_default_binary_from_main_rs` - Uses package name when `src/main.rs` exists
3. `build_detects_multiple_bins` - Handles multiple `[[bin]]` entries

**Fixtures:**
```
tests/fixtures/build/rust-binary/
├── Cargo.toml           # [[bin]] name = "myapp"
├── quench.toml          # [check.build] enabled
└── src/main.rs          # fn main() {}

tests/fixtures/build/rust-multi-bin/
├── Cargo.toml           # [[bin]] myapp, [[bin]] myserver
├── quench.toml
└── src/
    ├── bin/
    │   ├── myapp.rs
    │   └── myserver.rs
```

**Spec Pattern:**
```rust
/// Spec: docs/specs/checks/build.md#targets
///
/// > Rust: `[[bin]]` in Cargo.toml
#[test]
fn build_detects_bin_from_cargo_toml() {
    let result = check("build")
        .on("build/rust-binary")
        .args(&["--ci"])
        .json()
        .passes();

    let metrics = result.require("metrics");
    let size = metrics.get("size").and_then(|v| v.as_object());
    assert!(size.is_some(), "should have size metrics");
    assert!(size.unwrap().contains_key("myapp"), "should detect myapp target");
}
```

**Verification:**
```bash
cargo test --test specs build_detects
```

### Phase 3: Size Measurement Specs

**Goal:** Verify binary size measurement and reporting.

**Specs:**
1. `build_measures_binary_size` - Reports size in bytes
2. `build_size_in_json_metrics` - Proper JSON structure
3. `build_requires_release_binary` - Measures `target/release/` not debug

**Fixture modification:** Pre-build release binary or use `Project::cargo()` helper.

**Spec Pattern:**
```rust
/// Spec: docs/specs/checks/build.md#metrics
///
/// > `size`: Output file size
#[test]
fn build_measures_binary_size() {
    let temp = Project::cargo("size_test");
    temp.config(r#"
[check.build]
check = "error"
"#);

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
        .and_then(|v| v.get("size_test"))
        .and_then(|v| v.as_u64());

    assert!(size.is_some(), "should measure binary size");
    assert!(size.unwrap() > 0, "size should be non-zero");
}
```

### Phase 4: Size Threshold Violations

**Goal:** Verify size_max threshold enforcement.

**Specs:**
1. `build_size_exceeded_generates_violation` - Over threshold fails
2. `build_size_under_threshold_passes` - Under threshold passes
3. `build_per_target_size_max` - Per-target override works

**Config patterns:**
```toml
# Global threshold
[check.build]
size_max = "1 MB"

# Per-target override
[check.build.target.myapp]
size_max = "500 KB"
```

**Fixture:**
```
tests/fixtures/build/rust-oversized/
├── Cargo.toml
├── quench.toml          # size_max = "100 bytes"
└── src/main.rs          # Minimal binary (will exceed tiny threshold)
```

**Spec Pattern:**
```rust
/// Spec: docs/specs/checks/build.md#configuration
///
/// > size_max = "10 MB" (Global default)
#[test]
fn build_size_exceeded_generates_violation() {
    let temp = Project::cargo("oversized");
    temp.config(r#"
[check.build]
check = "error"
size_max = "100 bytes"  # Impossibly small
"#);

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
    assert!(v.get("threshold").is_some(), "violation should include threshold");
}
```

### Phase 5: Build Time Specs

**Goal:** Verify cold and hot build time measurement.

**Specs:**
1. `build_measures_cold_build_time` - Reports time_cold in seconds
2. `build_measures_hot_build_time` - Reports time_hot in seconds
3. `build_time_cold_exceeded_generates_violation` - Over threshold fails
4. `build_time_hot_exceeded_generates_violation` - Over threshold fails

**Note:** Build time specs are inherently slow. Mark with `#[ignore]` for fast test runs, run in CI with `--include-ignored`.

**Spec Pattern:**
```rust
/// Spec: docs/specs/checks/build.md#metrics
///
/// > `time_cold`: Clean build time
#[test]
#[ignore = "Slow: requires full build cycle"]
fn build_measures_cold_build_time() {
    let temp = Project::cargo("time_test");
    temp.config(r#"
[check.build]
check = "error"
[ratchet]
build_time_cold = true
"#);

    let result = check("build")
        .pwd(temp.path())
        .args(&["--ci"])
        .json()
        .passes();

    let metrics = result.require("metrics");
    let time = metrics.get("time").and_then(|v| v.as_object());
    assert!(time.is_some());
    assert!(time.unwrap().get("cold").is_some());
}
```

### Phase 6: Violation Type Coverage

**Goal:** Verify all documented violation types are correctly used.

**Specs:**
1. `build_violation_type_size_exceeded` - For binary over size_max
2. `build_violation_type_time_cold_exceeded` - For cold build over threshold
3. `build_violation_type_time_hot_exceeded` - For hot build over threshold
4. `build_violation_type_missing_target` - For configured target not found

**Spec Pattern:**
```rust
/// Spec: docs/specs/checks/build.md#json-output
///
/// > Violation types: `size_exceeded`, `time_cold_exceeded`,
/// > `time_hot_exceeded`, `missing_target`
#[test]
fn build_violation_type_is_size_exceeded() {
    // ... setup with size threshold ...

    let v = result.require_violation("size_exceeded");
    assert_eq!(
        v.get("type").and_then(|v| v.as_str()),
        Some("size_exceeded")
    );
}

#[test]
fn build_violation_type_is_missing_target() {
    let temp = Project::empty();
    temp.config(r#"
[check.build]
check = "error"
targets = ["nonexistent"]
"#);
    temp.file("Cargo.toml", r#"
[package]
name = "test"
version = "0.1.0"
"#);

    let result = check("build")
        .pwd(temp.path())
        .args(&["--ci"])
        .json()
        .fails();

    assert!(result.has_violation("missing_target"));
}
```

## Key Implementation Details

### Violation Structure

Build violations follow the standard structure from `crates/cli/src/check.rs`:

```rust
Violation {
    file: None,  // Build violations are not file-specific
    line: None,
    violation_type: "size_exceeded".to_string(),
    target: Some("myapp".to_string()),  // Build-specific field
    value: Some(5347737),               // Actual size in bytes
    threshold: Some(5242880),           // Configured max (5 MB)
    advice: "Reduce binary size. Check for unnecessary dependencies.".to_string(),
    ..Default::default()
}
```

### Size Parsing

The config should support human-readable size formats:
- `"5 MB"` → 5,242,880 bytes
- `"500 KB"` → 512,000 bytes
- `5242880` → literal bytes

### CI-Only Enforcement

Build check must only run in `--ci` mode:

```rust
fn run(&self, ctx: &CheckContext) -> CheckResult {
    if !ctx.ci_mode {
        return CheckResult::stub(self.name());
    }
    // ... actual implementation
}
```

### Advice Messages

Per-violation-type advice from spec:

| Violation Type | Advice |
|----------------|--------|
| `size_exceeded` | "Reduce binary size. Check for unnecessary dependencies." |
| `time_cold_exceeded` | "Build time exceeds threshold. Consider incremental compilation or dependency reduction." |
| `time_hot_exceeded` | "Incremental build is slow. Check for unnecessary recompilation triggers." |
| `missing_target` | "Configured build target not found. Verify target exists and builds successfully." |

## Verification Plan

### Unit Tests

Run specs in isolation:
```bash
# Fast specs only (skips build time tests)
cargo test --test specs -- checks_build

# All specs including slow ones
cargo test --test specs -- checks_build --include-ignored
```

### Integration Verification

```bash
# Verify on quench itself
quench check --build --ci

# Verify JSON output structure
quench check --build --ci -o json | jq '.checks[] | select(.name == "build")'
```

### Checklist

- [ ] `tests/specs/checks/build.rs` created with all specs
- [ ] `tests/specs.rs` updated with module registration
- [ ] Test fixtures created under `tests/fixtures/build/`
- [ ] All violation types have corresponding specs
- [ ] Specs reference `docs/specs/checks/build.md` sections
- [ ] Slow tests marked with `#[ignore]`
- [ ] `make check` passes

### Exit Criteria

All the following specs pass (or are marked `#[ignore = "TODO: ..."]` for future phases):

1. `build_detects_bin_from_cargo_toml` - Target detection
2. `build_measures_binary_size` - Size measurement
3. `build_size_exceeded_generates_violation` - Size threshold
4. `build_measures_cold_build_time` - Cold time measurement
5. `build_measures_hot_build_time` - Hot time measurement
6. `build_time_over_threshold_generates_violation` - Time threshold
7. `build_violation_type_coverage` - All violation types documented
