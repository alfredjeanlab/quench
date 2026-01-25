# Phase 1399: Timing Mode

**Root Feature:** `quench-588b`

## Overview

Add a `--timing` flag to display performance breakdown during check execution. Shows phase durations (discovery, checking, output), per-check timing, file counts, and cache statistics. Outputs to stderr for text format, included in JSON object for JSON output.

## Project Structure

Files to modify:
```
crates/cli/src/
├── cli.rs              # Add --timing flag to CheckArgs
├── main.rs             # Instrument phases, format timing output
├── runner.rs           # Add per-check timing to CheckRunner
├── check.rs            # Add timing field to CheckResult
├── timing.rs           # NEW: Timing types and helpers
└── output/
    └── json.rs         # Add timing to JSON output structure
```

## Dependencies

No new external dependencies. Uses:
- `std::time::{Instant, Duration}` - timing measurement
- Existing `serde` - JSON serialization

## Implementation Phases

### Phase 1: CLI Flag & Timing Types

Add the `--timing` flag and define timing data structures.

**crates/cli/src/cli.rs** - Add flag to CheckArgs (after line 98):
```rust
/// Show timing breakdown (phases, per-check, cache stats)
#[arg(long)]
pub timing: bool,
```

**crates/cli/src/timing.rs** - New file:
```rust
//! Timing data structures for --timing flag.

use std::collections::HashMap;
use std::time::Duration;
use serde::Serialize;

/// Phase timing breakdown.
#[derive(Debug, Clone, Default, Serialize)]
pub struct PhaseTiming {
    /// File discovery time.
    pub discovery_ms: u64,
    /// Check execution time.
    pub checking_ms: u64,
    /// Output formatting time.
    pub output_ms: u64,
    /// Total elapsed time.
    pub total_ms: u64,
}

/// Complete timing information.
#[derive(Debug, Clone, Default, Serialize)]
pub struct TimingInfo {
    /// Phase breakdown.
    #[serde(flatten)]
    pub phases: PhaseTiming,
    /// Number of files scanned.
    pub files: usize,
    /// Cache hits.
    pub cache_hits: usize,
    /// Per-check timing (check name -> milliseconds).
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub checks: HashMap<String, u64>,
}

impl PhaseTiming {
    /// Format as text output lines for stderr.
    pub fn format_text(&self) -> String {
        format!(
            "discovery: {}ms\nchecking: {}ms\noutput: {}ms\ntotal: {}ms",
            self.discovery_ms, self.checking_ms, self.output_ms, self.total_ms
        )
    }
}

impl TimingInfo {
    /// Format cache statistics line.
    pub fn format_cache(&self, misses: usize) -> String {
        let total = self.cache_hits + misses;
        if total == 0 {
            "cache: 0/0".to_string()
        } else {
            format!("cache: {}/{}", self.cache_hits, total)
        }
    }
}
```

**crates/cli/src/lib.rs** - Add module:
```rust
pub mod timing;
```

### Phase 2: Per-Check Timing in Runner

Modify `CheckRunner` to measure each check's execution time and return it.

**crates/cli/src/check.rs** - Add duration field to CheckResult (after line 347):
```rust
/// Execution duration (for --timing flag).
#[serde(skip_serializing_if = "Option::is_none")]
pub duration_ms: Option<u64>,
```

Update all `CheckResult` constructors to initialize `duration_ms: None`.

**crates/cli/src/runner.rs** - Wrap check execution with timing:
```rust
use std::time::Instant;

// In run() method, wrap check.run():
let check_start = Instant::now();
let mut result = match std::panic::catch_unwind(...) {
    Ok(result) => result,
    Err(_) => CheckResult::skipped(...),
};
result.duration_ms = Some(check_start.elapsed().as_millis() as u64);
```

Apply the same pattern in `run_uncached()`.

### Phase 3: Phase Instrumentation in main.rs

Wrap major execution phases with timing measurement.

**crates/cli/src/main.rs** - Add timing instrumentation:
```rust
use std::time::Instant;
use quench::timing::TimingInfo;

fn run_check(cli: &Cli, args: &CheckArgs) -> anyhow::Result<ExitCode> {
    let total_start = Instant::now();

    // ... existing config loading ...

    // === Discovery Phase ===
    let discovery_start = Instant::now();

    // Walker setup and file discovery (existing code ~lines 250-272)
    let files = walker.walk(&root);

    let discovery_ms = discovery_start.elapsed().as_millis() as u64;

    // === Checking Phase ===
    let checking_start = Instant::now();

    // Runner execution (existing code ~line 377)
    let check_results = runner.run(checks, &files, &config, &root);

    let checking_ms = checking_start.elapsed().as_millis() as u64;

    // ... existing cache persistence ...

    // === Build timing info before output ===
    let timing_info = if args.timing {
        let stats = cache.as_ref().map(|c| c.stats());
        Some(TimingInfo {
            phases: PhaseTiming {
                discovery_ms,
                checking_ms,
                output_ms: 0, // Updated after output
                total_ms: 0,  // Updated after output
            },
            files: files.len(),
            cache_hits: stats.as_ref().map(|s| s.hits).unwrap_or(0),
            checks: check_results.iter()
                .filter_map(|r| r.duration_ms.map(|d| (r.name.clone(), d)))
                .collect(),
        })
    } else {
        None
    };

    // === Output Phase ===
    let output_start = Instant::now();

    // ... existing output formatting ...

    let output_ms = output_start.elapsed().as_millis() as u64;
    let total_ms = total_start.elapsed().as_millis() as u64;

    // === Print timing to stderr ===
    if let Some(mut info) = timing_info {
        info.phases.output_ms = output_ms;
        info.phases.total_ms = total_ms;

        // Text output goes to stderr
        if !matches!(args.output, OutputFormat::Json) {
            eprintln!("{}", info.phases.format_text());
            // Per-check timing
            for result in &check_results {
                if let Some(ms) = result.duration_ms {
                    eprintln!("{}: {}ms", result.name, ms);
                }
            }
            // File and cache stats
            eprintln!("files: {}", info.files);
            let misses = cache.as_ref().map(|c| c.stats().misses).unwrap_or(0);
            eprintln!("{}", info.format_cache(misses));
        }
    }
```

### Phase 4: JSON Timing Output

Add timing field to JSON output structure.

**crates/cli/src/output/json.rs** - Extend CombinedOutput:
```rust
use crate::timing::TimingInfo;

#[derive(Debug, Serialize)]
struct CombinedOutput<'a> {
    timestamp: &'a str,
    passed: bool,
    checks: &'a [CheckResult],
    #[serde(skip_serializing_if = "Option::is_none")]
    ratchet: Option<RatchetOutput>,
    #[serde(skip_serializing_if = "Option::is_none")]
    timing: Option<&'a TimingInfo>,
}

impl<W: Write> JsonFormatter<W> {
    /// Write JSON output with optional ratchet and timing.
    pub fn write_with_timing(
        &mut self,
        output: &CheckOutput,
        ratchet: Option<&RatchetResult>,
        timing: Option<&TimingInfo>,
    ) -> std::io::Result<()> {
        let combined = CombinedOutput {
            timestamp: &output.timestamp,
            passed: output.passed && ratchet.as_ref().is_none_or(|r| r.passed),
            checks: &output.checks,
            ratchet: ratchet.map(Into::into),
            timing,
        };
        let json = serde_json::to_string_pretty(&combined).map_err(std::io::Error::other)?;
        writeln!(self.writer, "{}", json)
    }
}
```

**crates/cli/src/main.rs** - Update JSON output call:
```rust
OutputFormat::Json => {
    let mut formatter = JsonFormatter::new(std::io::stdout());
    if args.timing {
        formatter.write_with_timing(&output, ratchet_result.as_ref(), timing_info.as_ref())?;
    } else {
        formatter.write_with_ratchet(&output, ratchet_result.as_ref())?;
    }
}
```

### Phase 5: Edge Cases & Tests

Handle edge cases and enable behavioral specs.

**Edge cases to handle:**
1. `--timing --no-cache` - Show "cache: 0/N" (zero hits)
2. `--timing --config-only` - Skip timing output (no phases run)
3. `--timing` with failures - Still show timing after violations
4. Per-check timing only for enabled checks

**tests/specs/cli/timing.rs** - Remove `#[ignore]` from all tests:
- `timing_shows_phase_breakdown`
- `timing_phases_show_milliseconds`
- `timing_shows_per_check_breakdown`
- `timing_only_shows_enabled_checks`
- `timing_with_json_adds_timing_field`
- `timing_json_includes_phases`
- `timing_json_includes_per_check`
- `timing_shows_file_count`
- `timing_shows_cache_stats`
- `timing_json_includes_cache_stats`
- `timing_works_with_failures`
- `timing_no_cache_shows_zero_hits`
- `timing_config_only_shows_discovery`

## Key Implementation Details

### Timing Measurement Pattern

Use `std::time::Instant` for monotonic, high-resolution timing:
```rust
let start = Instant::now();
// ... work ...
let elapsed_ms = start.elapsed().as_millis() as u64;
```

### Output Destination

- **Text mode**: Timing goes to stderr (doesn't pollute stdout)
- **JSON mode**: Timing embedded in JSON object on stdout

### Cache Statistics

The cache already tracks hits/misses via `AtomicUsize`:
```rust
// In cache.rs
pub fn stats(&self) -> CacheStats {
    CacheStats {
        hits: self.hits.load(Ordering::Relaxed),
        misses: self.misses.load(Ordering::Relaxed),
        entries: self.inner.len(),
    }
}
```

### Per-Check Timing

Measured in `runner.rs` by wrapping `check.run()`. The duration is stored in `CheckResult.duration_ms` and collected after all checks complete.

## Verification Plan

### Unit Tests

1. **timing.rs** - Test `PhaseTiming::format_text()` output format
2. **timing.rs** - Test `TimingInfo::format_cache()` with various hit/miss values

### Behavioral Specs

Run all timing specs after implementation:
```bash
cargo test --test specs timing
```

Expected behavior per spec file `tests/specs/cli/timing.rs`:

| Test | Validates |
|------|-----------|
| `timing_shows_phase_breakdown` | discovery/checking/output/total labels present |
| `timing_phases_show_milliseconds` | "ms" suffix in output |
| `timing_shows_per_check_breakdown` | Individual check names with timing |
| `timing_only_shows_enabled_checks` | Respects check filters |
| `timing_with_json_adds_timing_field` | `"timing":` in JSON |
| `timing_json_includes_phases` | `discovery_ms`, `checking_ms`, `total_ms` |
| `timing_json_includes_per_check` | `"checks":` object |
| `timing_shows_file_count` | `files:` line |
| `timing_shows_cache_stats` | `cache:` line |
| `timing_no_cache_shows_zero_hits` | `cache: 0/` when --no-cache |

### Manual Verification

```bash
# Text output
cargo run -- check --timing

# JSON output
cargo run -- check --timing -o json | jq '.timing'

# With --no-cache
cargo run -- check --timing --no-cache

# Filter to single check
cargo run -- check --timing --cloc
```

### Performance Validation

Timing overhead should be negligible (<1ms). Verify by comparing:
```bash
time cargo run -- check
time cargo run -- check --timing
```
