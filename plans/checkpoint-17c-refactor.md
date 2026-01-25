# Checkpoint 17C: Refactor - Performance

**Plan:** `checkpoint-17c-refactor`
**Root Feature:** `quench-performance`
**Depends On:** Checkpoint 17B (Performance Validation)

## Overview

Address the blocking issue from checkpoint 17B (large file skip not implemented) and refactor the codebase for better maintainability while preserving validated performance characteristics.

**Blocking Issue from 17B:**
- Files >10MB are NOT skipped with a warning as required by `docs/specs/20-performance.md`
- `Error::FileTooLarge` type exists in `error.rs` but is never constructed
- Must implement size-gated file reading before files are processed

**Refactoring Goals:**
- Extract large file filtering from walker to make skip behavior explicit
- Modularize `main.rs` to improve maintainability (currently 652 lines)
- Consolidate file size threshold constants

**Performance Constraints:**
- Must maintain cold run < 500ms on 50K LOC
- Must maintain warm run < 100ms on 50K LOC
- No changes to caching behavior (validated in 17B)

## Project Structure

```
quench/
├── crates/cli/src/
│   ├── main.rs             # REFACTOR: Extract command handlers
│   ├── cmd_check.rs        # NEW: Check command handler
│   ├── cmd_report.rs       # NEW: Report command handler
│   ├── walker.rs           # MODIFY: Add file size filtering
│   ├── file_size.rs        # NEW: File size constants and utilities
│   ├── error.rs            # EXISTING: Already has FileTooLarge
│   └── runner.rs           # VERIFY: Receives filtered files
├── tests/
│   ├── specs/
│   │   └── performance/
│   │       └── large_files.rs  # NEW: Large file skip tests
│   └── fixtures/
│       └── large-file/     # NEW: Test fixture for >10MB handling
└── reports/
    └── checkpoint-17-performance.md  # UPDATE: Mark criterion 3 as PASS
```

## Dependencies

No new external dependencies. Uses existing infrastructure:
- `ignore = "0.4"` - File walking (already has metadata)
- `tracing = "0.1"` - Warning output
- `criterion = "0.5"` - Benchmarking (existing)

## Implementation Phases

### Phase 1: Define File Size Constants

**Goal:** Consolidate file size thresholds into a single, well-documented module.

**Create:** `crates/cli/src/file_size.rs`

```rust
// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! File size thresholds and utilities.
//!
//! Per docs/specs/20-performance.md:
//! - < 64KB: Direct read into buffer
//! - 64KB - 1MB: Memory-mapped, full processing
//! - 1MB - 10MB: Memory-mapped, report as oversized
//! - > 10MB: Skip with warning, don't read

/// Maximum file size to process (10MB).
/// Files larger than this are skipped with a warning.
pub const MAX_FILE_SIZE: u64 = 10 * 1024 * 1024;

/// Soft limit for "oversized" reporting (1MB).
/// Files between SOFT_LIMIT and MAX_FILE_SIZE are processed
/// but may be reported as potential violations by size-aware checks.
pub const SOFT_LIMIT_SIZE: u64 = 1024 * 1024;

/// Threshold for memory-mapped I/O (64KB).
/// Files smaller than this are read directly into buffer.
pub const MMAP_THRESHOLD: u64 = 64 * 1024;

/// File size classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileSizeClass {
    /// < 64KB - direct read
    Small,
    /// 64KB - 1MB - mmap, full processing
    Normal,
    /// 1MB - 10MB - mmap, may report oversized
    Oversized,
    /// > 10MB - skip entirely
    TooLarge,
}

impl FileSizeClass {
    /// Classify a file by size.
    pub fn from_size(size: u64) -> Self {
        if size > MAX_FILE_SIZE {
            FileSizeClass::TooLarge
        } else if size > SOFT_LIMIT_SIZE {
            FileSizeClass::Oversized
        } else if size > MMAP_THRESHOLD {
            FileSizeClass::Normal
        } else {
            FileSizeClass::Small
        }
    }
}

/// Format file size for human-readable output.
pub fn human_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;

    if bytes >= MB {
        format!("{:.1}MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1}KB", bytes as f64 / KB as f64)
    } else {
        format!("{}B", bytes)
    }
}
```

**Update:** `crates/cli/src/lib.rs` to export the module.

**Verification:**
```bash
cargo build
cargo test file_size
```

---

### Phase 2: Add Large File Filtering to Walker

**Goal:** Skip files >10MB during walking and emit warnings.

**Modify:** `crates/cli/src/walker.rs`

**Add to WalkStats:**
```rust
/// Files skipped due to size limit (>10MB).
pub files_skipped_size: usize,
```

**Add filtering logic in walk callbacks:**

```rust
// In parallel walker callback (after getting metadata)
let size = meta.as_ref().map(|m| m.len()).unwrap_or(0);

// Skip files exceeding size limit
if size > crate::file_size::MAX_FILE_SIZE {
    tracing::warn!(
        "skipping {} ({} > 10MB limit)",
        entry.path().display(),
        crate::file_size::human_size(size)
    );
    files_skipped_size.fetch_add(1, Ordering::Relaxed);
    return WalkState::Continue;
}
```

Apply same logic to sequential walker.

**Update WalkedFile:**
```rust
/// File size classification for processing hints.
pub size_class: FileSizeClass,
```

**Verification:**
```bash
cargo test walker
# Create 15MB temp file, verify it's skipped with warning
```

---

### Phase 3: Add Large File Test Fixtures and Specs

**Goal:** Behavioral tests for large file handling.

**Create fixture:** `tests/fixtures/large-file/`

```bash
# Generate fixture (done in test setup, not committed)
mkdir -p tests/fixtures/large-file/src
# 15MB file - will be generated at test time
```

**Create spec:** `tests/specs/performance/large_files.rs`

```rust
//! Large file handling behavioral tests.
//!
//! Verifies that files >10MB are skipped per docs/specs/20-performance.md.

use crate::prelude::*;

/// Files over 10MB are skipped with a warning, not processed.
#[test]
fn large_file_skipped_with_warning() {
    // Create temp 15MB file
    let fixture = TempFixture::new();
    fixture.write_large_file("src/huge.rs", 15 * 1024 * 1024);

    cli()
        .on(fixture.path())
        .args(&["check", "--verbose"])
        .succeeds()
        .stderr_has("skipping src/huge.rs")
        .stderr_has("> 10MB limit");
}

/// Large file is not counted in check violations.
#[test]
fn large_file_not_in_violations() {
    let fixture = TempFixture::new();
    // Create huge file that would normally trigger cloc violation
    fixture.write_large_file("src/huge.rs", 15 * 1024 * 1024);

    cli()
        .on(fixture.path())
        .args(&["check", "-o", "json"])
        .succeeds()
        .stdout_has(r#""files_skipped":"#);
}

/// Files just under 10MB are still processed.
#[test]
fn file_under_10mb_processed() {
    let fixture = TempFixture::new();
    // 9.9MB file should be processed
    fixture.write_large_file("src/big.rs", 9_900_000);

    // Should trigger cloc violation for oversized file
    cli()
        .on(fixture.path())
        .args(&["check"])
        .fails()
        .stdout_has("file_too_large");
}
```

**Verification:**
```bash
cargo test --test specs large_files
```

---

### Phase 4: Extract Check Command Handler

**Goal:** Extract `run_check()` from `main.rs` into `cmd_check.rs`.

**Create:** `crates/cli/src/cmd_check.rs`

The function is ~520 lines. Extract it with these logical sections:

```rust
// crates/cli/src/cmd_check.rs

//! Check command implementation.

use std::sync::Arc;
use std::time::Instant;
// ... imports ...

/// Configuration derived from CLI args and config file.
pub(crate) struct CheckContext {
    pub root: PathBuf,
    pub config: Config,
    pub walker_config: WalkerConfig,
    pub runner_config: RunnerConfig,
    pub output_format: OutputFormat,
    pub timing: bool,
    pub verbose: bool,
}

/// Run the check command.
pub fn run(cli: &Cli, args: &CheckArgs) -> anyhow::Result<ExitCode> {
    let total_start = Instant::now();

    // Phase 1: Validate arguments
    validate_args(args)?;

    // Phase 2: Build context
    let ctx = build_context(cli, args)?;

    // Phase 3: Discovery phase
    let (files, discovery_ms) = discover_files(&ctx, args)?;

    // Phase 4: Checking phase
    let (output, checking_ms, cache) = run_checks(&ctx, &files, args)?;

    // Phase 5: Ratchet phase
    let ratchet_result = handle_ratchet(&ctx, &output, args)?;

    // Phase 6: Output phase
    let exit_code = write_output(&ctx, &output, ratchet_result, args, total_start)?;

    Ok(exit_code)
}

fn validate_args(args: &CheckArgs) -> anyhow::Result<()> {
    if args.dry_run && !args.fix {
        eprintln!("--dry-run requires --fix");
        return Ok(ExitCode::ConfigError);
    }
    // ... other validations
    Ok(())
}

// ... helper functions for each phase ...
```

**Update main.rs:**
```rust
mod cmd_check;
// ...
Some(Command::Check(args)) => cmd_check::run(&cli, args),
```

**Verification:**
```bash
cargo build
cargo test --all
```

---

### Phase 5: Extract Report Command Handler

**Goal:** Extract `run_report()` from `main.rs` into `cmd_report.rs`.

**Create:** `crates/cli/src/cmd_report.rs`

```rust
// crates/cli/src/cmd_report.rs

//! Report command implementation.

use std::io::Write;
// ... imports ...

/// Run the report command.
pub fn run(cli: &Cli, args: &ReportArgs) -> anyhow::Result<()> {
    let cwd = std::env::current_dir()?;

    // Load config
    let config = load_config(cli, &cwd)?;

    // Determine baseline path
    let baseline_path = args
        .baseline
        .clone()
        .unwrap_or_else(|| cwd.join(&config.git.baseline));

    // Parse output target
    let (format, file_path) = args.output_target();

    // Validate flags
    if args.compact && !matches!(format, OutputFormat::Json) {
        eprintln!("warning: --compact only applies to JSON output, ignoring");
    }

    // Load and format report
    let baseline = Baseline::load(&baseline_path)?;
    write_report(format, baseline.as_ref(), args, file_path)?;

    Ok(())
}
```

**Update main.rs:**
```rust
mod cmd_report;
// ...
Some(Command::Report(args)) => {
    cmd_report::run(&cli, args)?;
    Ok(ExitCode::Success)
}
```

**Resulting main.rs should be ~60-80 lines** (down from 652):
- init_logging()
- main()
- run() - dispatches to command modules

**Verification:**
```bash
cargo build
cargo test --all
make check
```

---

### Phase 6: Update Validation Report and Final Verification

**Goal:** Complete checkpoint 17 by updating the validation report and running full verification.

**Update:** `reports/checkpoint-17-performance.md`

Change criterion 3 status from FAIL to PASS:

```markdown
### 3. Large File Handling (>10MB skipped with warning)

| Test | Expected | Actual | Status |
|------|----------|--------|--------|
| 15MB file skipped | Warning emitted, file not processed | Warning emitted, file skipped | **PASS** |
| File not processed | No violations from file | File excluded from results | **PASS** |

**Evidence:**
- `MAX_FILE_SIZE = 10 * 1024 * 1024` constant in `file_size.rs`
- Size check in walker before sending file to channel
- Warning logged via `tracing::warn!`
- Behavioral test: `tests/specs/performance/large_files.rs`
```

Update conclusion:
```markdown
## Conclusion

**Checkpoint 17 validated.** All 6 criteria pass.
- Criterion 3 (large file handling) implemented in 17C refactor.
```

**Full verification:**
```bash
# 1. All tests pass
make check

# 2. Performance targets still met
cargo build --release
hyperfine --warmup 0 --runs 5 -i \
    --prepare 'rm -rf tests/fixtures/stress-monorepo/.quench' \
    './target/release/quench check tests/fixtures/stress-monorepo'

# 3. Large file handling works
mkdir -p /tmp/large-file-test/src
dd if=/dev/zero bs=1M count=15 | tr '\0' 'a' > /tmp/large-file-test/src/huge.txt
./target/release/quench check /tmp/large-file-test --verbose 2>&1 | grep -i "skipping"

# 4. Dogfooding
./target/release/quench check .
```

## Key Implementation Details

### File Size Check Location

The size check must happen in the walker, before files are sent through the channel. This ensures:
1. Large files never enter the processing pipeline
2. Cache is not populated with large file entries
3. Memory is never allocated for large file content

```rust
// In walker.rs - critical placement
let size = meta.as_ref().map(|m| m.len()).unwrap_or(0);

// MUST check before creating WalkedFile
if size > MAX_FILE_SIZE {
    tracing::warn!("skipping {} ({} > 10MB)", ...);
    return WalkState::Continue;  // Skip, don't send
}

// Only files under limit reach here
let walked = WalkedFile { ... };
tx.send(walked)?;
```

### Warning Output Format

Follow existing tracing patterns:
```rust
tracing::warn!(
    "skipping {} ({} > 10MB limit)",
    path.display(),
    human_size(size)
);
```

Output: `WARN skipping src/huge.rs (15.0MB > 10MB limit)`

### Statistics Tracking

Track skipped files for `--verbose` output:
```rust
if args.verbose {
    eprintln!("Scanned {} files, {} skipped (>10MB)",
        stats.files_found, stats.files_skipped_size);
}
```

### JSON Output Extension

Add `files_skipped` to JSON output for CI visibility:
```json
{
  "passed": true,
  "files_scanned": 5000,
  "files_skipped": 2,
  "checks": [ ... ]
}
```

## Verification Plan

### Phase 1 Verification
```bash
cargo build
cargo test file_size
```

### Phase 2 Verification
```bash
cargo test walker
# Manual: create 15MB file, verify warning
dd if=/dev/zero bs=1M count=15 > /tmp/huge.txt
./target/debug/quench check /tmp --verbose 2>&1 | grep -i skip
```

### Phase 3 Verification
```bash
cargo test --test specs large_files
```

### Phase 4-5 Verification
```bash
cargo build
cargo test --all
# Verify main.rs line count reduced
wc -l crates/cli/src/main.rs  # Should be ~60-80 lines
```

### Phase 6 (Final) Verification
```bash
make check

# Performance regression check
cargo build --release
hyperfine --warmup 1 --runs 10 \
    './target/release/quench check tests/fixtures/stress-monorepo'
# Must still be < 100ms warm

# Large file handling
# (commands from Phase 6 above)
```

## Exit Criteria

- [ ] `MAX_FILE_SIZE = 10MB` constant defined in `file_size.rs`
- [ ] Walker skips files >10MB with warning
- [ ] `WalkStats.files_skipped_size` tracks skipped count
- [ ] Behavioral tests for large file handling pass
- [ ] `main.rs` reduced to ~60-80 lines (command dispatch only)
- [ ] `cmd_check.rs` contains check command logic
- [ ] `cmd_report.rs` contains report command logic
- [ ] Performance targets maintained (warm < 100ms)
- [ ] `reports/checkpoint-17-performance.md` shows all criteria PASS
- [ ] `make check` passes
- [ ] Dogfooding works: `quench check .` passes
