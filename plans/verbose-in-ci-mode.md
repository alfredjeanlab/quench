# Plan: Verbose Output in CI Mode

## Status

**IMPLEMENTED** ✅ (Commit: 46503d5)

**Enhancements** ✅ (Follow-up):
- Added auto-detected suite detection source logging
- Added comprehensive unit tests (6 tests)
- Added behavioral/integration tests (9 tests in tests/specs/verbose.rs)
- Removed `[verbose]` prefix for cleaner output

## Overview

Add comprehensive verbose output to quench that activates **automatically in `--ci` mode** (always, not configurable) or when `--verbose` is explicitly passed. The verbose output surfaces diagnostic information about configuration resolution, file discovery, test suite execution, ratchet baselines, git commit ranges, and total wall time — giving CI operators and developers full visibility into what quench is doing and why.

## Project Structure

Key files to modify:

```
crates/cli/src/
├── cli.rs                          # Add --verbose flag
├── cmd_check.rs                    # Main orchestration: wire verbose output throughout
├── verbose.rs                      # NEW: Verbose output helper (VerboseLogger)
├── checks/
│   ├── tests/
│   │   ├── mod.rs                  # Log configured/auto-detected suites
│   │   └── suite.rs                # Log before/after each suite execution
│   └── git/mod.rs                  # Log commit list being validated
├── git.rs                          # Surface ratchet base search details
└── lib.rs                          # pub mod verbose
tests/
└── specs/                          # Behavioral tests for verbose output
```

## Dependencies

No new external dependencies. Uses existing `std::time::Instant` for wall-time measurement and `eprintln!` for stderr output (consistent with existing debug/timing output).

## Implementation Phases

### Phase 1: Add `--verbose` Flag and `VerboseLogger`

**Goal:** Establish the verbose infrastructure so subsequent phases have a clean API to use.

1. **Add `--verbose` to `CheckArgs`** in `cli.rs`:

```rust
/// Show verbose diagnostic output (always enabled in --ci mode)
#[arg(long)]
pub verbose: bool,
```

2. **Create `crates/cli/src/verbose.rs`** with a `VerboseLogger` struct:

```rust
use std::io::Write;

/// Verbose output logger. Writes to stderr with a `[verbose]` prefix.
/// All output is conditional on verbose mode being enabled.
pub struct VerboseLogger {
    enabled: bool,
}

impl VerboseLogger {
    pub fn new(enabled: bool) -> Self {
        Self { enabled }
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Print a verbose line to stderr.
    pub fn log(&self, msg: &str) {
        if self.enabled {
            eprintln!("[verbose] {}", msg);
        }
    }

    /// Print a verbose section header.
    pub fn section(&self, title: &str) {
        if self.enabled {
            eprintln!("[verbose] === {} ===", title);
        }
    }
}
```

3. **Wire into `cmd_check.rs`**: Create `VerboseLogger` from `args.ci || args.verbose` and pass it through the execution flow.

4. **Replace `debug_logging()` calls**: Migrate existing `debug_logging()` env-var checks in `cmd_check.rs` to use `VerboseLogger`. Keep the `QUENCH_DEBUG` env var as a fallback (it should also enable verbose).

**Milestone:** `quench check --verbose` produces `[verbose]` prefixed output to stderr. `--ci` implicitly enables it.

### Phase 2: Configuration and File Discovery Output (items a, b)

**Goal:** Output resolved source/test/exclude rules and file counts.

1. **(a) Source, tests, and exclude rules** — After config is loaded and language detection completes, log:
   - Global `project.source`, `project.tests`, `project.exclude` patterns
   - Per-language overrides when they differ from defaults (e.g., `rust.source`, `rust.tests`, `rust.exclude`)
   - Per-check customizations (e.g., `check.tests.commit.test_patterns`, `check.tests.commit.source_patterns`, `check.tests.commit.exclude`)
   - The detected project language

   Example output:
   ```
   [verbose] === Configuration ===
   [verbose] Config: quench.toml
   [verbose] Language: rust
   [verbose] project.source: (default)
   [verbose] project.tests: **/tests/**, **/test/**, **/*_test.*, **/*_tests.*, **/*.test.*, **/*.spec.*
   [verbose] project.exclude: target
   [verbose] rust.source: **/*.rs
   [verbose] rust.tests: **/tests/**, **/test/**/*.rs, **/benches/**, **/*_test.rs, **/*_tests.rs
   [verbose] rust.exclude: target/**
   [verbose] check.tests.commit.source_patterns: src/**/*
   [verbose] check.tests.commit.test_patterns: tests/**/*, test/**/*, ...
   [verbose] check.tests.commit.exclude: **/mod.rs, **/lib.rs, **/main.rs, **/generated/**
   ```

2. **(b) File count** — After file discovery completes, log the count:

   ```
   [verbose] === Discovery ===
   [verbose] Scanned 142 files (3 errors, 0 symlink loops, 0 skipped >10MB)
   ```

**Milestone:** Running `quench check --verbose` shows config resolution and file counts.

### Phase 3: Test Suite Logging (items c, e)

**Goal:** Log configured/auto-detected test suites and per-suite execution details.

1. **(c) Configured test suites** — Before running suites, log the list:

   ```
   [verbose] === Test Suites ===
   [verbose] Configured suites: cargo (tests), bats (shell-tests)
   ```

   Or for auto-detected:
   ```
   [verbose] === Test Suites ===
   [verbose] Auto-detected suites:
   [verbose]   cargo (detected: Cargo.toml with [lib] and tests/ directory)
   [verbose]   bats (detected: tests/*.bats files found)
   ```

2. **(e) Before/after each suite** — In `run_single_suite()` in `suite.rs`, log:

   Before:
   ```
   [verbose] Running suite: cargo ...
   [verbose]   command: cargo test --all --color=never
   ```

   After:
   ```
   [verbose] Suite "cargo" completed: exit_code=0, 47 tests, 1234ms
   ```

   Or on failure:
   ```
   [verbose] Suite "cargo" completed: exit_code=1, 47 tests (3 failing), 1234ms
   ```

**Implementation notes:**
- The `VerboseLogger` needs to be passed through `RunnerContext` to `run_single_suite()`. Add a `verbose: bool` field to `RunnerContext`.
- Each `TestRunner::run()` already returns `TestRunResult` with pass/fail and timing. The verbose logging wraps the existing call.
- To get the command string, add a `fn command_line(&self, config: &TestSuiteConfig, ctx: &RunnerContext) -> String` method to `TestRunner` trait (or just format it in `run_single_suite` from the suite config).

### Phase 4: Ratchet Baseline Logging (item f)

**Goal:** Surface which branch/baseline/commit is used for ratcheting, or explain why none was found.

In `cmd_check.rs`, after ratchet resolution:

```
[verbose] === Ratchet ===
[verbose] Mode: git notes
[verbose] Base branch: main (auto-detected)
[verbose] Ratchet base: abc1234 (merge-base of HEAD and origin/main)
[verbose] Baseline: loaded from git notes for abc1234
```

Or when not found:
```
[verbose] === Ratchet ===
[verbose] Mode: git notes
[verbose] Base branch: main (auto-detected)
[verbose] Ratchet base: abc1234
[verbose] Baseline: not found (searched: refs/notes/quench for abc1234)
```

Or when ratchet is off:
```
[verbose] === Ratchet ===
[verbose] Ratchet check: off
```

**Implementation:**
- Modify `find_ratchet_base()` in `git.rs` to return more context: which branches were tried, which succeeded, the strategy used. Either return a richer struct or add a separate verbose-aware wrapper in `cmd_check.rs`.
- The `detect_base_branch()` function already tries `main` → `master` → `origin/main` → `origin/master`. Log each attempt.

### Phase 5: Git Commit Logging (items g)

**Goal:** Output oneline descriptions of commits included in git enforcement / ratchet comparison.

1. **(g) Commit list** — After obtaining the commit range (in `cmd_check.rs` or the git check), log each commit:

   ```
   [verbose] === Commits ===
   [verbose] Commits since main (3):
   [verbose]   abc1234 feat(cli): add verbose flag
   [verbose]   def5678 fix(tests): correct timeout handling
   [verbose]   ghi9012 chore: update dependencies
   ```

   This requires accessing `get_commits_since()` from `git.rs` at the verbose logging point. Since the git check and the ratchet comparison should use the same commit set, compute this once and log it.

**Implementation:**
- In `cmd_check.rs`, after `base_branch` is resolved and before checks run, call `get_commits_since()` to get the commit list.
- Log each commit's short hash + subject line.
- Pass this commit list to relevant checks if they need it (currently `git/mod.rs` calls `get_commits_to_check()` internally, which calls the same function).

### Phase 6: Total Wall Time and Polish (item h)

**Goal:** Output total wall time and finalize output formatting.

1. **(h) Total wall time** — At the very end of `cmd_check.rs`, after all output is done:

   ```
   [verbose] === Summary ===
   [verbose] Total wall time: 4.72s
   ```

2. **Ensure verbose output doesn't interfere with JSON mode** — When `--output json`, verbose output still goes to stderr (which is already the case since we use `eprintln!`). This is correct behavior: JSON goes to stdout, verbose diagnostics go to stderr.

3. **Ensure verbose output doesn't duplicate `--timing`** — The `--timing` flag shows phase breakdowns. Verbose should complement, not duplicate. If both are active, let `--timing` handle its own section; verbose handles the diagnostic sections (config, suites, ratchet, commits).

**Milestone:** Full verbose output works end-to-end in `--ci` mode. All items (a) through (h) are implemented.

## Key Implementation Details

### Output goes to stderr

All verbose output uses `eprintln!` (stderr), consistent with existing timing and debug output. This ensures stdout remains clean for machine-parseable output (JSON mode).

### Output format

~~Using `[verbose]` prefix makes it easy to grep for verbose lines in CI logs, and clearly distinguishes diagnostic output from check results.~~

**IMPLEMENTED:** No prefix is used. Verbose output is distinguished by section headers (`=== Section Name ===`) and is only output when verbose mode is enabled. This avoids redundancy since all output in verbose mode is diagnostic.

### VerboseLogger propagation

The `VerboseLogger` needs to reach:
- `cmd_check.rs` (owns it, handles phases 2, 4, 5, 6)
- `run_suites()` / `run_single_suite()` (phase 3) — via `RunnerContext`
- Git commit listing (phase 5) — called directly from `cmd_check.rs`

The simplest approach: add `verbose: bool` to `RunnerContext` and `CheckContext`. Individual checks that need it can read it from context.

### Backward compatibility with QUENCH_DEBUG

The existing `QUENCH_DEBUG=1` env var should continue working. The `VerboseLogger` constructor should be:

```rust
let verbose = args.ci || args.verbose
    || std::env::var("QUENCH_DEBUG").is_ok_and(|v| v == "1" || v.eq_ignore_ascii_case("true"));
```

### Test runner command strings

To log the command being run (item e), each runner already constructs a `Command`. The simplest approach is to format the command in `run_single_suite()` based on the suite config:
- For built-in runners: `"{runner} test"` + relevant flags (e.g., `cargo test --all`)
- For custom runners: the literal `command` field from config

An alternative is to add a `fn describe(&self, config: &TestSuiteConfig) -> String` to the `TestRunner` trait that returns the command string without executing it.

## Verification Plan

### Unit tests

Add tests in `verbose_tests.rs`:
- `VerboseLogger::new(false)` produces no output
- `VerboseLogger::new(true)` produces prefixed output
- Verbose is enabled when `ci=true` regardless of `verbose` flag

### Integration/behavioral tests

Add spec tests in `tests/specs/`:
1. **`--verbose` flag produces verbose output** — Run `quench check --verbose` on a fixture project, assert stderr contains `[verbose]` lines
2. **`--ci` implicitly enables verbose** — Run `quench check --ci`, assert stderr contains `[verbose]` lines
3. **Normal mode has no verbose output** — Run `quench check`, assert stderr does NOT contain `[verbose]`
4. **JSON mode keeps stdout clean** — Run `quench check --ci --output json`, assert stdout is valid JSON, verbose output only on stderr
5. **Config rules are logged** — Assert stderr contains source/test/exclude patterns
6. **File count is logged** — Assert stderr contains file count
7. **Suite execution is logged** — On a project with test suites, assert before/after messages appear
8. **Wall time is logged** — Assert stderr contains "Total wall time"

### Manual verification

```bash
# Basic verbose
quench check --verbose

# CI mode (verbose implied)
quench check --ci

# JSON mode + verbose (verify stdout is clean JSON)
quench check --ci --output json 2>verbose.log
jq . < /dev/stdin  # verify JSON
grep '\[verbose\]' verbose.log  # verify verbose on stderr
```
