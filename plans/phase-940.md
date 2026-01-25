# Phase 940: Test Runners - Coverage Targets

## Overview

Implement coverage target resolution and collection for test suites. The `targets` field specifies what code a test suite exercises for coverage collection. Target names resolve to either Rust binary targets (via llvm-cov instrumentation) or shell script patterns (via kcov). Coverage data is merged across suites testing the same language.

## Project Structure

```
crates/cli/src/checks/tests/
├── mod.rs                    # Update: coverage aggregation across suites
└── runners/
    ├── mod.rs                # Update: add TargetResolution type
    ├── coverage.rs           # Expand: target resolution, kcov integration
    ├── coverage_tests.rs     # Expand: unit tests
    ├── targets.rs            # NEW: target resolution logic
    ├── targets_tests.rs      # NEW: unit tests
    ├── kcov.rs               # NEW: kcov execution and parsing
    ├── kcov_tests.rs         # NEW: unit tests
    ├── instrumented.rs       # NEW: instrumented Rust binary building
    └── instrumented_tests.rs # NEW: unit tests

tests/specs/checks/tests/
└── coverage.rs               # Update: enable ignored specs

tests/fixtures/
├── coverage-rust-binary/     # NEW: Rust project with binary for bats tests
├── coverage-shell/           # NEW: Shell scripts with bats tests
└── coverage-merged/          # NEW: Multiple suites covering same language
```

## Dependencies

- **kcov**: Optional external tool for shell coverage (gracefully skipped if unavailable)
- **cargo-llvm-cov**: Existing dependency for Rust coverage

No new Rust dependencies required.

## Implementation Phases

### Phase 1: Target Resolution Framework

Create the target resolution module to classify targets as Rust binaries or shell scripts.

**Target Resolution Rules:**
1. **Build target name** (no glob characters): Look up in `[rust].targets` or Cargo.toml `[[bin]]` entries
2. **Glob pattern** (contains `*`, `?`, `[`): Match against `[shell].source` patterns

```rust
// crates/cli/src/checks/tests/runners/targets.rs

/// Resolved coverage target with collection strategy.
#[derive(Debug, Clone)]
pub enum ResolvedTarget {
    /// Rust binary target, collected via instrumented build + llvm-cov
    RustBinary {
        name: String,
        /// Path to binary (determined during build)
        binary_path: Option<PathBuf>,
    },
    /// Shell scripts, collected via kcov
    ShellScripts {
        pattern: String,
        /// Resolved file paths
        files: Vec<PathBuf>,
    },
}

/// Resolve a target string to a concrete coverage target.
pub fn resolve_target(
    target: &str,
    config: &Config,
    root: &Path,
) -> Result<ResolvedTarget, String> {
    if is_glob_pattern(target) {
        resolve_shell_pattern(target, config, root)
    } else {
        resolve_rust_binary(target, config, root)
    }
}

fn is_glob_pattern(s: &str) -> bool {
    s.contains('*') || s.contains('?') || s.contains('[')
}
```

**Files:**
- `crates/cli/src/checks/tests/runners/targets.rs`
- `crates/cli/src/checks/tests/runners/targets_tests.rs`

**Verification:** Unit tests for pattern detection, Rust binary lookup, shell glob resolution.

### Phase 2: Rust Binary Target Resolution

Implement Rust binary lookup from config and Cargo.toml.

**Resolution Order:**
1. Check `[rust].targets` array in quench.toml
2. Parse Cargo.toml for `[[bin]]` entries
3. Check default binary (package name)

```rust
// crates/cli/src/checks/tests/runners/targets.rs

fn resolve_rust_binary(
    name: &str,
    config: &Config,
    root: &Path,
) -> Result<ResolvedTarget, String> {
    // 1. Check [rust].targets
    if let Some(rust_config) = config.rust.as_ref() {
        if rust_config.targets.contains(&name.to_string()) {
            return Ok(ResolvedTarget::RustBinary {
                name: name.to_string(),
                binary_path: None,
            });
        }
    }

    // 2. Parse Cargo.toml for [[bin]] entries
    let cargo_path = root.join("Cargo.toml");
    if cargo_path.exists() {
        let cargo = parse_cargo_toml(&cargo_path)?;
        if cargo.has_binary(name) {
            return Ok(ResolvedTarget::RustBinary {
                name: name.to_string(),
                binary_path: None,
            });
        }
    }

    Err(format!("unknown target: {name} (not a Rust binary or glob pattern)"))
}

/// Minimal Cargo.toml parsing for binary detection.
#[derive(Deserialize)]
struct CargoToml {
    package: Option<CargoPackage>,
    bin: Option<Vec<CargoBin>>,
}

#[derive(Deserialize)]
struct CargoPackage {
    name: String,
}

#[derive(Deserialize)]
struct CargoBin {
    name: String,
}
```

**Verification:** Unit tests with sample Cargo.toml parsing.

### Phase 3: Shell Script Pattern Resolution

Implement glob pattern matching against shell source files.

```rust
// crates/cli/src/checks/tests/runners/targets.rs

fn resolve_shell_pattern(
    pattern: &str,
    config: &Config,
    root: &Path,
) -> Result<ResolvedTarget, String> {
    // Get shell source patterns (defaults: ["**/*.sh", "**/*.bash"])
    let source_patterns = config.shell_source_patterns();

    // Expand the target pattern
    let glob = glob::glob(&root.join(pattern).to_string_lossy())
        .map_err(|e| format!("invalid glob pattern: {e}"))?;

    let mut files = Vec::new();
    for entry in glob.flatten() {
        // Verify file matches a shell source pattern
        if matches_any_pattern(&entry, &source_patterns, root) {
            files.push(entry);
        }
    }

    if files.is_empty() {
        return Err(format!("no shell scripts match pattern: {pattern}"));
    }

    Ok(ResolvedTarget::ShellScripts {
        pattern: pattern.to_string(),
        files,
    })
}
```

**Verification:** Unit tests for glob expansion and shell pattern matching.

### Phase 4: Instrumented Binary Building

Build Rust binaries with LLVM coverage instrumentation for external test execution.

**Strategy:**
- Use `cargo build` with `RUSTFLAGS="-C instrument-coverage"`
- Set `LLVM_PROFILE_FILE` to capture coverage during test execution
- Merge profiles with `llvm-profdata merge` after tests complete

```rust
// crates/cli/src/checks/tests/runners/instrumented.rs

use std::path::{Path, PathBuf};
use std::process::Command;

/// Build context for instrumented binaries.
pub struct InstrumentedBuild {
    /// Directory for coverage profiles
    pub profile_dir: PathBuf,
    /// Built binary paths by target name
    pub binaries: HashMap<String, PathBuf>,
}

/// Build Rust binaries with coverage instrumentation.
pub fn build_instrumented(
    targets: &[String],
    root: &Path,
) -> Result<InstrumentedBuild, String> {
    let profile_dir = root.join("target").join("quench-coverage");
    std::fs::create_dir_all(&profile_dir)
        .map_err(|e| format!("failed to create profile dir: {e}"))?;

    // Build with instrumentation
    let mut cmd = Command::new("cargo");
    cmd.arg("build");
    for target in targets {
        cmd.args(["--bin", target]);
    }
    cmd.env("RUSTFLAGS", "-C instrument-coverage")
       .env("LLVM_PROFILE_FILE", profile_dir.join("%p-%m.profraw"))
       .current_dir(root)
       .stdout(Stdio::piped())
       .stderr(Stdio::piped());

    let output = cmd.output()
        .map_err(|e| format!("failed to build instrumented binaries: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("instrumented build failed:\n{}", truncate(&stderr, 500)));
    }

    // Locate built binaries
    let mut binaries = HashMap::new();
    for target in targets {
        let binary_path = root.join("target").join("debug").join(target);
        if binary_path.exists() {
            binaries.insert(target.clone(), binary_path);
        }
    }

    Ok(InstrumentedBuild { profile_dir, binaries })
}

/// Merge coverage profiles and generate report.
pub fn collect_instrumented_coverage(
    build: &InstrumentedBuild,
    root: &Path,
) -> CoverageResult {
    // Merge .profraw files
    // Run llvm-cov export with binary
    // Parse JSON output (reuse existing parse_llvm_cov_json)
}
```

**Verification:** Integration test with fixture Rust project.

### Phase 5: kcov Integration for Shell Scripts

Implement kcov execution for shell script coverage.

**Strategy:**
- Run tests via `kcov --include-path=<scripts> <output-dir> bats ...`
- Parse kcov's Cobertura XML output
- Aggregate per-file line coverage

```rust
// crates/cli/src/checks/tests/runners/kcov.rs

use std::path::{Path, PathBuf};
use std::process::Command;

/// Check if kcov is available.
pub fn kcov_available() -> bool {
    Command::new("kcov")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

/// Collect shell script coverage via kcov.
pub fn collect_shell_coverage(
    scripts: &[PathBuf],
    test_command: &str,
    root: &Path,
) -> CoverageResult {
    if !kcov_available() {
        return CoverageResult::skipped();
    }

    let start = Instant::now();
    let output_dir = root.join("target").join("kcov");
    std::fs::create_dir_all(&output_dir)?;

    // Build include paths
    let include_paths: Vec<_> = scripts.iter()
        .filter_map(|p| p.parent())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();

    let include_arg = include_paths.iter()
        .map(|p| p.to_string_lossy())
        .collect::<Vec<_>>()
        .join(",");

    // Run kcov wrapping the test command
    let mut cmd = Command::new("kcov");
    cmd.arg("--include-path")
       .arg(&include_arg)
       .arg(&output_dir)
       .args(test_command.split_whitespace())
       .current_dir(root);

    let output = cmd.output()?;
    let duration = start.elapsed();

    if !output.status.success() {
        return CoverageResult::failed(duration, "kcov failed");
    }

    // Parse Cobertura XML output
    parse_kcov_output(&output_dir, duration)
}

/// Parse kcov Cobertura XML output.
fn parse_kcov_output(output_dir: &Path, duration: Duration) -> CoverageResult {
    let xml_path = output_dir.join("cobertura.xml");
    if !xml_path.exists() {
        return CoverageResult::failed(duration, "kcov output not found");
    }

    let xml = std::fs::read_to_string(&xml_path)?;
    // Parse XML for line-rate attributes per file
    // Return CoverageResult with per-file coverage
}
```

**Verification:** Integration test with bats fixture exercising shell scripts.

### Phase 6: Coverage Merging Across Suites

Aggregate coverage data when multiple suites test the same language.

**Merge Strategy:**
- Rust: Merge LLVM profiles before generating report
- Shell: Merge kcov results by taking max coverage per line

```rust
// crates/cli/src/checks/tests/mod.rs

/// Aggregated coverage across all test suites.
#[derive(Debug, Default)]
pub struct AggregatedCoverage {
    pub rust: Option<CoverageResult>,
    pub shell: Option<CoverageResult>,
    // Future: python, javascript, go
}

impl AggregatedCoverage {
    /// Merge coverage from a suite into the aggregate.
    pub fn merge(&mut self, suite_coverage: SuiteCoverage) {
        if let Some(rust) = suite_coverage.rust {
            self.rust = Some(match self.rust.take() {
                Some(existing) => merge_coverage_results(existing, rust),
                None => rust,
            });
        }
        // Similar for shell
    }
}

/// Merge two coverage results by taking max coverage per file.
fn merge_coverage_results(a: CoverageResult, b: CoverageResult) -> CoverageResult {
    let mut files = a.files;
    for (path, coverage) in b.files {
        files.entry(path)
            .and_modify(|existing| *existing = existing.max(coverage))
            .or_insert(coverage);
    }

    // Recalculate overall percentage from merged files
    let total_coverage = if files.is_empty() {
        None
    } else {
        Some(files.values().sum::<f64>() / files.len() as f64)
    };

    CoverageResult {
        success: a.success && b.success,
        error: a.error.or(b.error),
        duration: a.duration + b.duration,
        line_coverage: total_coverage,
        files,
    }
}
```

**Output Format:**
```
tests: coverage 78.4%
  rust: 82.3% (cargo + bats)
  shell: 71.2% (bats)
```

**Verification:** Integration test with multiple suites, verify merged percentages.

## Key Implementation Details

### Target Resolution Flow

```
targets = ["myapp", "scripts/*.sh"]
         ↓
    ┌────┴────┐
    ↓         ↓
  myapp   scripts/*.sh
    ↓         ↓
  no glob    glob chars
    ↓         ↓
  Cargo.toml  [shell].source
  lookup      pattern match
    ↓         ↓
  RustBinary  ShellScripts
```

### Environment Variables for Instrumented Builds

```bash
# Set by quench during instrumented build
RUSTFLAGS="-C instrument-coverage"
LLVM_PROFILE_FILE="target/quench-coverage/%p-%m.profraw"

# Binary location (passed to tests)
QUENCH_COVERAGE_BINARY="target/debug/myapp"
```

### Coverage Tool Availability

Both coverage tools are optional:
- **cargo-llvm-cov**: Skipped if not installed (existing behavior)
- **kcov**: Skipped if not installed (new behavior)

The check does not fail if coverage tools are unavailable; coverage is simply not reported.

### Empty Targets Array

An explicit `targets = []` disables coverage for that suite (timing only):

```toml
[[check.tests.suite]]
runner = "bats"
path = "tests/smoke/"
targets = []  # No coverage collection
```

### Profile Cleanup

Coverage profiles are cleaned up after report generation to avoid accumulation:

```rust
fn cleanup_coverage_profiles(profile_dir: &Path) {
    if profile_dir.exists() {
        let _ = std::fs::remove_dir_all(profile_dir);
    }
}
```

## Verification Plan

### Unit Tests

1. **Target Resolution** (`targets_tests.rs`)
   - Glob pattern detection
   - Rust binary lookup from Cargo.toml
   - Shell pattern matching
   - Error cases (unknown target, no matches)

2. **kcov Parsing** (`kcov_tests.rs`)
   - Cobertura XML parsing
   - Per-file coverage extraction
   - Missing output handling

3. **Coverage Merging** (`mod_tests.rs` or `coverage_tests.rs`)
   - Same file in multiple suites
   - Different files merged correctly
   - Percentage recalculation

### Integration Tests

Update `tests/specs/checks/tests/coverage.rs` to enable ignored specs:

1. `bats_runner_collects_shell_coverage_via_kcov` - Shell coverage
2. `bats_runner_collects_rust_binary_coverage` - Instrumented binary
3. `multiple_suite_coverages_merged` - Coverage aggregation
4. `suite_with_empty_targets_skips_coverage` - Empty targets array

### Fixtures

1. **coverage-rust-binary/** - Rust CLI with main.rs exercised by bats tests
2. **coverage-shell/** - Shell scripts exercised by bats tests
3. **coverage-merged/** - Both cargo unit tests and bats CLI tests

### Manual Verification

1. Install kcov locally: `brew install kcov` (macOS) or `apt install kcov` (Linux)
2. Create project with shell scripts and bats tests
3. Run `quench check --tests --ci` and verify coverage output

## Commit Strategy

1. `feat(tests): add target resolution framework`
2. `feat(tests): implement Rust binary target resolution`
3. `feat(tests): implement shell script pattern resolution`
4. `feat(tests): add instrumented binary building for coverage`
5. `feat(tests): integrate kcov for shell coverage`
6. `feat(tests): merge coverage across test suites`
