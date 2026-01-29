# BATS/Shell Test Coverage via kcov

## Overview

Add coverage collection to the BATS test runner. When `targets` is specified in the suite config, resolve targets and collect coverage:
- **Glob patterns** (e.g., `scripts/*.sh`) → Shell script coverage via kcov
- **Build target names** (e.g., `myapp`) → Rust binary coverage via instrumented build

All infrastructure exists—this is primarily integration work connecting existing components.

## Project Structure

```
crates/cli/src/checks/tests/runners/
├── bats.rs          # MODIFY: Add coverage collection after test execution
├── kcov.rs          # EXISTS: collect_shell_coverage(), kcov_available()
├── targets.rs       # EXISTS: resolve_targets(), shell_script_files()
├── instrumented.rs  # EXISTS: build_instrumented(), collect_instrumented_coverage()
├── coverage.rs      # EXISTS: CoverageResult, collect_rust_coverage()
├── result.rs        # EXISTS: TestRunResult.with_collected_coverage()
└── mod.rs           # MODIFY: May need to pass Config to BatsRunner

tests/specs/checks/tests/
└── coverage.rs      # MODIFY: Remove #[ignore] from 4 Phase 940 tests
```

## Dependencies

- **kcov**: Required for shell script coverage (system tool, optional)
- **llvm-cov**: Required for Rust binary coverage (cargo install cargo-llvm-cov)
- **Existing crate deps**: `globset` (already used in targets.rs)

## Implementation Phases

### Phase 1: Pass Config to BatsRunner

**Goal**: Enable target resolution by passing project Config to the runner.

**Changes**:

1. Update `RunnerContext` in `mod.rs` to include Config reference:
```rust
pub struct RunnerContext<'a> {
    pub root: &'a Path,
    pub ci_mode: bool,
    pub collect_coverage: bool,
    pub config: &'a Config,  // ADD: For target resolution
}
```

2. Update `run_suites()` in `suite.rs` to pass config:
```rust
let runner_ctx = RunnerContext {
    root: ctx.root,
    ci_mode: ctx.ci_mode,
    collect_coverage: ctx.ci_mode,
    config: &ctx.config,  // ADD
};
```

**Verification**: `cargo build --all` passes, existing tests pass.

### Phase 2: Shell Coverage via kcov

**Goal**: Collect shell script coverage when targets contain glob patterns.

**Changes to `bats.rs`**:

1. Import coverage modules:
```rust
use super::{
    collect_shell_coverage, kcov_available,
    resolve_targets, shell_script_files,
};
```

2. After test execution, collect shell coverage:
```rust
fn run(&self, config: &TestSuiteConfig, ctx: &RunnerContext) -> TestRunResult {
    // ... existing test execution code ...

    let mut result = parse_tap_output(&stdout, total_time);

    // Collect shell coverage if requested and targets specified
    if ctx.collect_coverage && !config.targets.is_empty() {
        if let Some(shell_coverage) = collect_bats_shell_coverage(config, ctx) {
            result = result.with_collected_coverage(shell_coverage, "shell");
        }
    }

    result
}

/// Collect shell script coverage for BATS tests via kcov.
fn collect_bats_shell_coverage(
    config: &TestSuiteConfig,
    ctx: &RunnerContext,
) -> Option<CoverageResult> {
    if !kcov_available() {
        return None;
    }

    // Resolve targets to find shell scripts
    let resolved = resolve_targets(&config.targets, ctx.config, ctx.root).ok()?;
    let scripts = shell_script_files(&resolved);

    if scripts.is_empty() {
        return None;
    }

    // Build bats command for kcov to wrap
    let test_path = config.path.as_deref().unwrap_or("tests/");
    let test_command = vec!["bats".to_string(), test_path.to_string()];

    let coverage = collect_shell_coverage(&scripts, &test_command, ctx.root);
    if coverage.success {
        Some(coverage)
    } else {
        None
    }
}
```

**Verification**:
- Unit test: mock kcov output parsing
- Remove `#[ignore]` from `bats_runner_collects_shell_coverage_via_kcov`
- Run: `cargo test bats_runner_collects_shell_coverage`

### Phase 3: Rust Binary Coverage via Instrumented Build

**Goal**: Collect Rust binary coverage when targets contain build target names.

**Changes to `bats.rs`**:

1. Import instrumented build modules:
```rust
use super::{
    build_instrumented, collect_instrumented_coverage, coverage_env,
    rust_binary_names,
};
```

2. Add Rust binary coverage collection:
```rust
/// Collect Rust binary coverage for BATS tests via llvm-cov.
fn collect_bats_rust_coverage(
    config: &TestSuiteConfig,
    ctx: &RunnerContext,
) -> Option<CoverageResult> {
    // Resolve targets to find Rust binaries
    let resolved = resolve_targets(&config.targets, ctx.config, ctx.root).ok()?;
    let binaries = rust_binary_names(&resolved);

    if binaries.is_empty() {
        return None;
    }

    // Build instrumented binaries
    let build_result = build_instrumented(&binaries, ctx.root)?;

    // Run bats with coverage environment
    let test_path = config.path.as_deref().unwrap_or("tests/");
    let env = coverage_env(&build_result);

    let mut cmd = Command::new("bats");
    cmd.arg(test_path);
    cmd.current_dir(ctx.root);
    cmd.envs(env);
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let output = cmd.output().ok()?;
    if !output.status.success() {
        return None;
    }

    // Collect coverage from profraw files
    collect_instrumented_coverage(&build_result, ctx.root)
}
```

3. Integrate into `run()`:
```rust
if ctx.collect_coverage && !config.targets.is_empty() {
    // Shell coverage
    if let Some(shell_coverage) = collect_bats_shell_coverage(config, ctx) {
        result = result.with_collected_coverage(shell_coverage, "shell");
    }

    // Rust binary coverage
    if let Some(rust_coverage) = collect_bats_rust_coverage(config, ctx) {
        result = result.with_collected_coverage(rust_coverage, "rust");
    }
}
```

**Verification**:
- Remove `#[ignore]` from `bats_runner_collects_rust_binary_coverage`
- Run: `cargo test bats_runner_collects_rust_binary_coverage`

### Phase 4: Coverage Merging and Edge Cases

**Goal**: Handle multiple suites, empty targets, and coverage aggregation.

**Changes**:

1. **Empty targets handling**: Already implicit—`resolve_targets(&[])` returns empty, no coverage collected.

2. **Coverage merging**: Already handled in `AggregatedCoverage::merge_shell()` and `merge_rust()`.

3. Verify aggregation is called correctly in `mod.rs` or check execution.

**Verification**:
- Remove `#[ignore]` from `multiple_suite_coverages_merged`
- Remove `#[ignore]` from `suite_with_empty_targets_skips_coverage`
- Run: `cargo test --test specs coverage`

### Phase 5: Documentation and Cleanup

**Goal**: Ensure all tests pass, update any docs.

**Tasks**:
1. Run full test suite: `make check`
2. Verify Phase 940 tests all pass
3. Bump `CACHE_VERSION` in `crates/cli/src/cache.rs` if check output format changed
4. Update any error messages for clarity

## Key Implementation Details

### kcov Command Structure

kcov wraps the test command and instruments shell scripts:
```bash
kcov --include-path scripts/ target/kcov/ bats tests/
```

Output: `target/kcov/<executable>/cobertura.xml` with Cobertura format:
```xml
<coverage line-rate="0.75">
  <class filename="scripts/helper.sh" line-rate="0.80">
```

### Instrumented Rust Binary Pattern

1. Build with coverage instrumentation:
```bash
RUSTFLAGS="-C instrument-coverage" cargo build --release
```

2. Set profile output path:
```bash
LLVM_PROFILE_FILE="target/coverage/%p-%m.profraw"
```

3. Run tests (bats executes the instrumented binary)

4. Merge profraw files:
```bash
llvm-profdata merge -sparse target/coverage/*.profraw -o merged.profdata
```

5. Generate report:
```bash
llvm-cov report --instr-profile=merged.profdata target/release/myapp
```

### Coverage Data Flow

```
TestSuiteConfig.targets
       │
       ▼
resolve_targets() → ResolvedTarget[]
       │
       ├─ ShellScripts { files } → collect_shell_coverage() → CoverageResult
       │                                    │
       └─ RustBinary { name }   → build_instrumented() + collect_instrumented_coverage()
                                            │
                                            ▼
                              TestRunResult.with_collected_coverage()
                                            │
                                            ▼
                              SuiteResult.coverage HashMap
                                            │
                                            ▼
                              AggregatedCoverage.merge_*()
                                            │
                                            ▼
                              Final metrics: { "shell": 75.0, "rust": 82.0 }
```

### Error Handling

- **kcov not available**: Skip shell coverage silently (optional tool)
- **llvm-cov not available**: Skip Rust binary coverage silently
- **No matching targets**: Skip coverage (explicit `targets = []` case)
- **Coverage collection fails**: Log warning, continue without coverage

## Verification Plan

### Unit Tests

Add to `crates/cli/src/checks/tests/runners/bats_tests.rs`:

```rust
#[test]
fn collect_bats_shell_coverage_no_kcov() {
    // Verify graceful skip when kcov unavailable
}

#[test]
fn collect_bats_shell_coverage_empty_targets() {
    // Verify no coverage attempted with targets = []
}
```

### Integration Tests (Phase 940 specs)

All four tests in `tests/specs/checks/tests/coverage.rs`:

1. `bats_runner_collects_shell_coverage_via_kcov` - shell scripts via kcov
2. `bats_runner_collects_rust_binary_coverage` - Rust binary via llvm-cov
3. `multiple_suite_coverages_merged` - coverage aggregation across suites
4. `suite_with_empty_targets_skips_coverage` - targets = [] behavior

### Manual Verification

```bash
# Full test suite
make check

# Specific coverage tests
cargo test --test specs coverage

# With verbose output
QUENCH_DEBUG=1 cargo test --test specs bats
```

### CI Verification

Ensure CI has:
- kcov installed (for shell coverage tests)
- cargo-llvm-cov installed (for Rust binary coverage tests)
- Skip tests gracefully if tools unavailable
