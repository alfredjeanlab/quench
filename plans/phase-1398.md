# Phase 1398: Timing Mode - Specs

## Overview

Add `--timing` flag to `quench check` that displays performance timing breakdown. This phase focuses on writing behavioral specs (tests) first, with implementation to follow in a subsequent phase. The timing information helps developers understand where time is spent and identify performance bottlenecks.

Per docs/specs/01-cli.md line 107: `--timing` shows "timing breakdown (file walking, pattern matching, etc.)"

## Project Structure

```
tests/specs/
├── cli/
│   └── timing.rs          # NEW: Timing flag specs
└── fixtures/
    └── timing/            # NEW: Test fixtures
        ├── small-project/ # Few files, quick checks
        └── multi-check/   # Multiple checks enabled

crates/cli/src/
├── cli.rs                 # Add --timing flag (future)
├── main.rs                # Timing instrumentation (future)
├── output/
│   ├── mod.rs             # TimingOutput struct (future)
│   └── json.rs            # Timing in JSON output (future)
└── timing.rs              # NEW: Timing collection (future)
```

Key files to modify in future implementation:
- `crates/cli/src/cli.rs` - Add `--timing` flag to `CheckArgs`
- `crates/cli/src/main.rs` - Instrument phases with timing
- `crates/cli/src/output/mod.rs` - Add `TimingOutput` to output structs

## Dependencies

No new external dependencies. Uses:
- `std::time::Instant` for timing measurement
- Existing `serde` for JSON serialization

## Implementation Phases

### Phase 1: Spec File Setup

Create `tests/specs/cli/timing.rs` with module registration.

Update `tests/specs/cli/mod.rs`:
```rust
mod timing;
```

Create minimal fixture `tests/fixtures/timing/small-project/`:
```
small-project/
├── quench.toml
└── src/
    └── main.rs
```

`quench.toml`:
```toml
[check.cloc]
max_lines = 1000
```

`src/main.rs`:
```rust
fn main() {
    println!("Hello");
}
```

**Milestone:** Test module compiles and is discovered.

### Phase 2: Phase Breakdown Specs

Write specs for `--timing` showing phase breakdown.

```rust
// tests/specs/cli/timing.rs
//! Behavioral specs for --timing flag.
//!
//! Reference: docs/specs/01-cli.md (--timing flag)
//! Reference: docs/specs/20-performance.md (Performance Model)

use crate::prelude::*;

/// Spec: --timing shows phase breakdown (discovery, reading, checking, output)
///
/// Per docs/specs/20-performance.md:
/// "Total Time = File Discovery + File Reading + Pattern Matching + Aggregation"
#[test]
#[ignore = "TODO: Phase 1398 - implement --timing flag"]
fn timing_shows_phase_breakdown() {
    let temp = Project::empty();
    temp.config(r#"
        [check.cloc]
        max_lines = 1000
    "#);
    temp.file("src/main.rs", "fn main() {}");

    cli()
        .args(["--timing"])
        .on_temp(&temp)
        .passes()
        .stderr_has("discovery:")
        .stderr_has("checking:")
        .stderr_has("output:")
        .stderr_has("total:");
}

/// Spec: Phase breakdown shows millisecond timing
#[test]
#[ignore = "TODO: Phase 1398 - implement --timing flag"]
fn timing_phases_show_milliseconds() {
    let temp = Project::empty();
    temp.config(r#"
        [check.cloc]
        max_lines = 1000
    "#);
    temp.file("src/main.rs", "fn main() {}");

    cli()
        .args(["--timing"])
        .on_temp(&temp)
        .passes()
        .stderr_has("ms");
}
```

**Milestone:** Specs compile and are ignored with correct TODO message.

### Phase 3: Per-Check Timing Specs

Write specs for per-check timing breakdown.

```rust
/// Spec: --timing shows per-check timing
#[test]
#[ignore = "TODO: Phase 1398 - implement --timing flag"]
fn timing_shows_per_check_breakdown() {
    let temp = Project::empty();
    temp.config(r#"
        [check.cloc]
        max_lines = 1000

        [check.escapes]
        check = "error"
    "#);
    temp.file("src/main.rs", "fn main() {}");

    cli()
        .args(["--timing"])
        .on_temp(&temp)
        .passes()
        .stderr_has("cloc:")
        .stderr_has("escapes:");
}

/// Spec: Per-check timing only shows enabled checks
#[test]
#[ignore = "TODO: Phase 1398 - implement --timing flag"]
fn timing_only_shows_enabled_checks() {
    let temp = Project::empty();
    temp.config(r#"
        [check.cloc]
        max_lines = 1000

        [check.escapes]
        check = "off"
    "#);
    temp.file("src/main.rs", "fn main() {}");

    cli()
        .args(["--timing"])
        .on_temp(&temp)
        .passes()
        .stderr_has("cloc:")
        .stderr_lacks("escapes:");
}
```

**Milestone:** Per-check timing specs written and ignored.

### Phase 4: JSON Output Specs

Write specs for `--timing` with `-o json`.

```rust
/// Spec: --timing works with -o json (adds timing field)
#[test]
#[ignore = "TODO: Phase 1398 - implement --timing flag"]
fn timing_with_json_adds_timing_field() {
    let temp = Project::empty();
    temp.config(r#"
        [check.cloc]
        max_lines = 1000
    "#);
    temp.file("src/main.rs", "fn main() {}");

    cli()
        .args(["--timing", "-o", "json"])
        .on_temp(&temp)
        .passes()
        .stdout_has(r#""timing":"#);
}

/// Spec: JSON timing includes phase breakdown
#[test]
#[ignore = "TODO: Phase 1398 - implement --timing flag"]
fn timing_json_includes_phases() {
    let temp = Project::empty();
    temp.config(r#"
        [check.cloc]
        max_lines = 1000
    "#);
    temp.file("src/main.rs", "fn main() {}");

    cli()
        .args(["--timing", "-o", "json"])
        .on_temp(&temp)
        .passes()
        .stdout_has(r#""discovery_ms":"#)
        .stdout_has(r#""checking_ms":"#)
        .stdout_has(r#""total_ms":"#);
}

/// Spec: JSON timing includes per-check breakdown
#[test]
#[ignore = "TODO: Phase 1398 - implement --timing flag"]
fn timing_json_includes_per_check() {
    let temp = Project::empty();
    temp.config(r#"
        [check.cloc]
        max_lines = 1000
    "#);
    temp.file("src/main.rs", "fn main() {}");

    cli()
        .args(["--timing", "-o", "json"])
        .on_temp(&temp)
        .passes()
        .stdout_has(r#""checks":"#);
}
```

**Milestone:** JSON timing specs written and ignored.

### Phase 5: Cache Statistics Specs

Write specs for file count and cache hit rate.

```rust
/// Spec: --timing shows file count and cache hit rate
#[test]
#[ignore = "TODO: Phase 1398 - implement --timing flag"]
fn timing_shows_file_count() {
    let temp = Project::empty();
    temp.config(r#"
        [check.cloc]
        max_lines = 1000
    "#);
    temp.file("src/main.rs", "fn main() {}");
    temp.file("src/lib.rs", "pub fn hello() {}");

    cli()
        .args(["--timing"])
        .on_temp(&temp)
        .passes()
        .stderr_has("files:");
}

/// Spec: --timing shows cache statistics
#[test]
#[ignore = "TODO: Phase 1398 - implement --timing flag"]
fn timing_shows_cache_stats() {
    let temp = Project::empty();
    temp.config(r#"
        [check.cloc]
        max_lines = 1000
    "#);
    temp.file("src/main.rs", "fn main() {}");

    // First run - cold cache
    cli()
        .args(["--timing"])
        .on_temp(&temp)
        .passes()
        .stderr_has("cache:");

    // Second run - warm cache (should show hits)
    cli()
        .args(["--timing"])
        .on_temp(&temp)
        .passes()
        .stderr_has("cache:");
}

/// Spec: JSON timing includes cache statistics
#[test]
#[ignore = "TODO: Phase 1398 - implement --timing flag"]
fn timing_json_includes_cache_stats() {
    let temp = Project::empty();
    temp.config(r#"
        [check.cloc]
        max_lines = 1000
    "#);
    temp.file("src/main.rs", "fn main() {}");

    cli()
        .args(["--timing", "-o", "json"])
        .on_temp(&temp)
        .passes()
        .stdout_has(r#""files":"#)
        .stdout_has(r#""cache_hits":"#);
}
```

**Milestone:** Cache statistics specs written and ignored.

### Phase 6: Edge Cases and Integration

Write specs for edge cases and verify all specs compile.

```rust
/// Spec: --timing works with failing checks
#[test]
#[ignore = "TODO: Phase 1398 - implement --timing flag"]
fn timing_works_with_failures() {
    let temp = Project::empty();
    temp.config(r#"
        [check.cloc]
        max_lines = 5
    "#);
    // Create file that exceeds max_lines
    temp.file("src/main.rs", "fn main() {\n    let a = 1;\n    let b = 2;\n    let c = 3;\n    let d = 4;\n    let e = 5;\n    let f = 6;\n}");

    cli()
        .args(["--timing"])
        .on_temp(&temp)
        .fails()
        .stderr_has("total:");
}

/// Spec: --timing with --no-cache shows zero cache hits
#[test]
#[ignore = "TODO: Phase 1398 - implement --timing flag"]
fn timing_no_cache_shows_zero_hits() {
    let temp = Project::empty();
    temp.config(r#"
        [check.cloc]
        max_lines = 1000
    "#);
    temp.file("src/main.rs", "fn main() {}");

    // Warm the cache
    cli().on_temp(&temp).passes();

    // Run with --no-cache
    cli()
        .args(["--timing", "--no-cache"])
        .on_temp(&temp)
        .passes()
        .stderr_has("cache: 0/");
}

/// Spec: --timing without checks shows only discovery phase
#[test]
#[ignore = "TODO: Phase 1398 - implement --timing flag"]
fn timing_config_only_shows_discovery() {
    let temp = Project::empty();
    temp.config(r#"
        [check.cloc]
        max_lines = 1000
    "#);
    temp.file("src/main.rs", "fn main() {}");

    cli()
        .args(["--timing", "--config"])
        .on_temp(&temp)
        .passes()
        .stderr_lacks("discovery:")  // --config doesn't walk files
        .stderr_lacks("checking:");
}
```

Run verification:
```bash
cargo test --test specs timing -- --ignored
```

**Milestone:** All specs compile, run as ignored, and document expected behavior.

## Key Implementation Details

### Expected Output Format (Text)

When `--timing` is provided, output to stderr:

```
Timing:
  discovery:  45ms (234 files)
  checking:  123ms
    cloc:     45ms
    escapes:  78ms
  output:      2ms
  total:     170ms
  cache: 200/234 hits (85%)
```

### Expected Output Format (JSON)

When `--timing -o json` is provided, add `timing` field to output:

```json
{
  "timestamp": "2025-01-24T12:00:00Z",
  "passed": true,
  "checks": [...],
  "timing": {
    "discovery_ms": 45,
    "checking_ms": 123,
    "output_ms": 2,
    "total_ms": 170,
    "files": 234,
    "cache_hits": 200,
    "cache_total": 234,
    "checks": {
      "cloc": 45,
      "escapes": 78
    }
  }
}
```

### Implementation Notes for Future Phase

The implementation will need to:

1. **Add `--timing` flag** in `cli.rs`:
   ```rust
   /// Show timing breakdown
   #[arg(long)]
   pub timing: bool,
   ```

2. **Create `TimingCollector`** to aggregate timing:
   ```rust
   pub struct TimingCollector {
       pub discovery: Duration,
       pub checking: Duration,
       pub output: Duration,
       pub checks: HashMap<String, Duration>,
       pub files: usize,
       pub cache_hits: usize,
   }
   ```

3. **Instrument phases** in `main.rs`:
   ```rust
   let start = Instant::now();
   // ... file walking ...
   timing.discovery = start.elapsed();
   ```

4. **Output to stderr** in text mode (stdout reserved for check results)

5. **Include in JSON** when both flags provided

### Schema Update

Add to `docs/specs/output.schema.json`:
```json
"timing": {
  "type": "object",
  "properties": {
    "discovery_ms": { "type": "integer" },
    "checking_ms": { "type": "integer" },
    "output_ms": { "type": "integer" },
    "total_ms": { "type": "integer" },
    "files": { "type": "integer" },
    "cache_hits": { "type": "integer" },
    "cache_total": { "type": "integer" },
    "checks": {
      "type": "object",
      "additionalProperties": { "type": "integer" }
    }
  }
}
```

## Verification Plan

### Spec Verification

1. All specs compile:
   ```bash
   cargo test --test specs timing --no-run
   ```

2. All specs are properly ignored:
   ```bash
   cargo test --test specs timing 2>&1 | grep -c "ignored"
   # Should show count of ignored tests
   ```

3. Specs have correct TODO format:
   ```bash
   grep -r "TODO: Phase 1398" tests/specs/cli/timing.rs
   ```

### Future Implementation Verification

When implementing (future phase):
```bash
# Remove #[ignore] attributes
# Run specs
cargo test --test specs timing

# Full check suite
make check
```

## Checklist

- [ ] Create `tests/specs/cli/timing.rs`
- [ ] Update `tests/specs/cli/mod.rs` to include timing module
- [ ] Create `tests/fixtures/timing/small-project/` fixture
- [ ] Write phase breakdown specs (2 tests)
- [ ] Write per-check timing specs (2 tests)
- [ ] Write JSON output specs (3 tests)
- [ ] Write cache statistics specs (3 tests)
- [ ] Write edge case specs (3 tests)
- [ ] Verify all specs compile
- [ ] Run `make check` (specs should be ignored)
- [ ] Archive plan when complete
