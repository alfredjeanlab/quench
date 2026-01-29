# Plan: Fix Build Checks

## Overview

Enable the four ignored behavioral specs that test build-check and ratchet integration:

| Test | Phase | Blocker |
|------|-------|---------|
| `modes_ratchet::coverage_regression_fails` | 1202 | Needs fixture producing coverage metrics + baseline with coverage |
| `modes_ratchet::coverage_within_tolerance_passes` | 1202 | Same |
| `modes_ratchet::binary_size_regression_fails` | 1215 | Needs fixture producing build metrics + baseline with binary_size |
| `cli_ci_mode::ci_mode_enables_build_check` | — | Needs fixture with buildable binary; both fast/CI paths currently return stub |

All underlying logic (ratchet comparison, metric extraction, baseline I/O) is already implemented and unit-tested. The gap is purely at the **behavioral spec level**: the specs need fixtures that produce real metrics so the ratchet comparison path is exercised end-to-end.

## Project Structure

Files touched:

```
tests/
├── specs/
│   ├── modes/ratchet.rs        # Enable 3 ignored tests, add fixture setup
│   └── cli/ci_mode.rs          # Enable 1 ignored test, add fixture setup
└── fixtures/
    └── rust-build/             # NEW: minimal Rust binary project for build check
        ├── quench.toml
        ├── CLAUDE.md
        ├── Cargo.toml
        └── src/main.rs
```

No changes to implementation code (`crates/cli/src/`). This is test-only work.

## Dependencies

- `cargo llvm-cov` — already used by existing coverage specs; must be installed in CI
- Rust toolchain — needed to compile the binary fixture for build check
- No new crate dependencies

## Implementation Phases

### Phase 1: Coverage Ratchet Behavioral Specs (2 tests)

**Goal:** Enable `coverage_regression_fails` and `coverage_within_tolerance_passes`.

**Approach:** Use `Project::cargo()` to create a temp project with a test suite that produces coverage. Write a baseline JSON with coverage metrics, then run the CLI with ratchet enabled and assert on output.

**Key pattern** (following existing `regression_fails` test for escapes):

```rust
const COVERAGE_RATCHET_CONFIG: &str = r#"
version = 1

[git]
baseline = ".quench/baseline.json"

[ratchet]
check = "error"
coverage = true

[[check.tests.suite]]
runner = "cargo"
coverage = true
"#;

#[test]
fn coverage_regression_fails() {
    let temp = Project::cargo("cov_test");
    temp.config(COVERAGE_RATCHET_CONFIG);

    // Baseline claims 95% coverage — actual coverage will be ~50%
    fs::create_dir_all(temp.path().join(".quench")).unwrap();
    fs::write(
        temp.path().join(".quench/baseline.json"),
        r#"{
  "version": 1,
  "updated": "2026-01-20T00:00:00Z",
  "metrics": {
    "coverage": { "total": 0.95 }
  }
}"#,
    ).unwrap();

    cli()
        .pwd(temp.path())
        .args(&["--ci"])
        .fails()
        .stdout_has("coverage.total:")
        .stdout_has("(min:")
        .stdout_has("from baseline)");
}
```

For `coverage_within_tolerance_passes`, set `coverage_tolerance = 0.50` (50 percentage points) so the actual coverage (~50%) falls within tolerance of the 95% baseline.

**Verify:** `cargo test --test specs coverage_regression_fails` and `cargo test --test specs coverage_within_tolerance_passes`.

**Risk:** `cargo llvm-cov` must be available. The existing `tests/specs/checks/tests/coverage.rs` specs already depend on it, so this is not a new requirement. The test needs `--ci` to trigger coverage collection.

### Phase 2: Binary Size Ratchet Behavioral Spec (1 test)

**Goal:** Enable `binary_size_regression_fails`.

**Approach:** Create a `rust-build` fixture (or use `Project::cargo()` inline) with `src/main.rs`. The build check runs `cargo build --release`, which produces a binary in `target/release/`. We write a baseline with a very small `binary_size` (e.g., 1 byte) so the actual binary always exceeds it.

Since the build check is CI-only (`default_enabled() = false`, returns stub without `--ci`), the test must use `--ci --build`.

```rust
const BINARY_SIZE_RATCHET_CONFIG: &str = r#"
version = 1

[git]
baseline = ".quench/baseline.json"

[ratchet]
check = "error"
binary_size = true

[check.build]
targets = ["binsize_test"]
"#;

#[test]
fn binary_size_regression_fails() {
    let temp = Project::empty();
    temp.config(BINARY_SIZE_RATCHET_CONFIG);
    temp.file("CLAUDE.md", CLAUDE_MD);
    temp.file("Cargo.toml",
        "[package]\nname = \"binsize_test\"\nversion = \"0.1.0\"\nedition = \"2021\"\n");
    temp.file("src/main.rs", "fn main() { println!(\"hello\"); }");

    // Baseline claims binary is 1 byte — real binary will be much larger
    fs::create_dir_all(temp.path().join(".quench")).unwrap();
    fs::write(
        temp.path().join(".quench/baseline.json"),
        r#"{
  "version": 1,
  "updated": "2026-01-20T00:00:00Z",
  "metrics": {
    "binary_size": { "binsize_test": 1 }
  }
}"#,
    ).unwrap();

    cli()
        .pwd(temp.path())
        .args(&["--ci", "--build"])
        .fails()
        .stdout_has("binary_size.binsize_test:")
        .stdout_has("(max:")
        .stdout_has("from baseline)");
}
```

**Verify:** `cargo test --test specs binary_size_regression_fails`.

**Risk:** This test compiles a Rust binary in a temp directory, which is slow (~10-30s). It requires `--ci` mode. Acceptable for CI; the test was already designed for CI-only execution. The build check itself skips build-time measurement unless `build_time_cold` or `build_time_hot` is enabled, so the test only measures binary size (fast path: just check file size after build).

### Phase 3: CI Mode Build Check Behavioral Spec (1 test)

**Goal:** Enable `ci_mode_enables_build_check`.

**Approach:** The test already has the correct logic written — it just needs a fixture that produces a non-stub build check result when `--ci` is passed. The `default_project()` helper creates a project without `src/main.rs`, so no binary targets are detected and the build check returns a stub even in CI mode (no metrics = stub, per `build/mod.rs:211-213`).

The fix: use a project with a `src/main.rs` so `get_rust_targets()` detects it and the build check measures its size.

```rust
#[test]
fn ci_mode_enables_build_check() {
    let temp = default_project();
    // Add a binary target so build check has something to measure
    temp.file("Cargo.toml",
        "[package]\nname = \"citest\"\nversion = \"0.1.0\"\nedition = \"2021\"\n");
    temp.file("src/main.rs", "fn main() {}");

    // Without --ci, build check returns stub
    let result = cli().pwd(temp.path()).args(&["--build"]).json().passes();
    let build = result
        .checks()
        .iter()
        .find(|c| c.get("name").and_then(|n| n.as_str()) == Some("build"))
        .expect("build check should exist");
    assert_eq!(
        build.get("stub").and_then(|s| s.as_bool()),
        Some(true),
        "build check should be a stub without --ci"
    );

    // With --ci, build check runs (no stub)
    let result = cli()
        .pwd(temp.path())
        .args(&["--ci", "--build"])
        .json()
        .passes();
    let build = result
        .checks()
        .iter()
        .find(|c| c.get("name").and_then(|n| n.as_str()) == Some("build"))
        .expect("build check should exist");
    assert_ne!(
        build.get("stub").and_then(|s| s.as_bool()),
        Some(true),
        "build check should run with --ci"
    );
}
```

**Verify:** `cargo test --test specs ci_mode_enables_build_check`.

**Risk:** Same as Phase 2 — requires a real `cargo build --release` in CI. The test verifies the stub/non-stub distinction.

### Phase 4: Verify and Clean Up

1. Run all four previously-ignored tests:
   ```bash
   cargo test --test specs -- coverage_regression_fails coverage_within_tolerance_passes binary_size_regression_fails ci_mode_enables_build_check
   ```

2. Run the full spec suite to confirm no regressions:
   ```bash
   cargo test --test specs
   ```

3. Run `make check` per CLAUDE.md landing checklist.

4. Check that no `CACHE_VERSION` bump is needed (no check logic changed, only tests).

## Key Implementation Details

### Output Format for Ratchet Failures

The text output formatter (`output/text.rs:418-434`) produces:

- **Coverage (higher is better):** `coverage.total: 75.0% (min: 80.0% from baseline)`
- **Binary size (lower is better):** `binary_size.target: 1500000 (max: 1000000 from baseline)`
- **Escapes (lower is better):** `escapes.unsafe: 2 (max: 1 from baseline)`

Coverage uses `min` threshold label; all others use `max`. The `format_value` function formats coverage as `{:.1}%` (multiplied by 100), binary size as integer, and times as `{:.1}s`.

### Baseline JSON Structure

```json
{
  "version": 1,
  "updated": "2026-01-20T00:00:00Z",
  "metrics": {
    "coverage": { "total": 0.80 },
    "binary_size": { "target_name": 1000000 },
    "escapes": { "source": { "unsafe": 3 } }
  }
}
```

Coverage is stored as a fraction (0.0-1.0), not a percentage.

### Build Check Stub Behavior

The build check returns `CheckResult::stub(name)` in two cases:
1. `ctx.ci_mode == false` (line 55-56)
2. `ci_mode == true` but no metrics collected and no violations (line 212-213)

Case 2 happens when no build targets are detected. The `ci_mode_enables_build_check` test must provide a project with detectable targets (e.g., `src/main.rs` + `Cargo.toml` with `[package]`).

### Coverage Collection Path

Coverage metrics flow: `CargoRunner::run()` → `collect_rust_coverage()` → `TestRunResult.coverage` → `SuiteResult.coverage` → `CheckResult.metrics["coverage"]` → `extract_coverage_metrics()` → `CurrentMetrics.coverage` → `compare()`.

The test suite config must have `coverage = true` and the CLI must be in `--ci` mode for coverage to be collected.

## Verification Plan

1. **Per-phase:** Each test is independently runnable via `cargo test --test specs -- <test_name>`
2. **Integration:** Full spec suite: `cargo test --test specs`
3. **Regression:** `make check` (fmt, clippy, all tests, build, audit, deny)
4. **No implementation changes:** Verify with `git diff crates/` showing empty — only test files changed
