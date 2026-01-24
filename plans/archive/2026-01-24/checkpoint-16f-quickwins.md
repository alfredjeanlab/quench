# Checkpoint 16F: Quick Wins - Report Command

**Root Feature:** `quench-fc4c`

## Overview

This checkpoint delivers high-value, low-risk improvements to the `quench report` command. With streaming output and compact JSON mode already implemented (checkpoint-16e), this phase focuses on polish that improves output quality and usability.

Key goals:
1. **Human-readable binary sizes** - Use KB/MB format instead of raw bytes
2. **Deterministic output ordering** - Sort maps for reproducible output
3. **Markdown format** - Add proper Markdown formatter with tables
4. **Custom baseline path** - CLI flag to override baseline location
5. **Per-package coverage** - Display breakdown when available
6. **Test escapes display** - Show test escape counts separately

## Project Structure

```
quench/
├── crates/cli/src/
│   ├── cli.rs                  # UPDATE: Add --baseline flag
│   ├── main.rs                 # UPDATE: Pass baseline path to run_report
│   └── report/
│       ├── mod.rs              # EXISTS: ReportFormatter trait
│       ├── text.rs             # UPDATE: Use human_bytes, sorted output
│       ├── json.rs             # UPDATE: Sorted keys
│       ├── html.rs             # UPDATE: Sorted output
│       └── markdown.rs         # NEW: Markdown table formatter
├── tests/
│   ├── specs/cli/
│   │   └── report.rs           # UPDATE: Add specs for new features
│   └── fixtures/report/
│       └── with-packages/      # NEW: Baseline with per-package coverage
└── docs/specs/
    └── 01-cli.md               # UPDATE: Document --baseline flag
```

## Dependencies

No new external dependencies. Uses existing infrastructure:

- `chrono` - Date formatting (exists)
- `serde_json` - JSON output (exists)
- `clap` - CLI argument parsing (exists)

## Implementation Phases

### Phase 1: Human-Readable Binary Sizes

**Goal:** Use the existing `human_bytes()` helper for text output instead of raw bytes.

The function exists in `report/mod.rs` but text formatter displays "X bytes".

**File:** `crates/cli/src/report/text.rs`

Update binary size output:

```rust
use super::human_bytes;

// Before (line 91-93, 141-144):
writeln!(output, "binary_size.{}: {} bytes", name, size)?;

// After:
writeln!(output, "binary_size.{}: {}", name, human_bytes(*size))?;
```

**Verification:**
```bash
cargo test --test specs report
# Verify output shows "1.5 MB" instead of "1572864 bytes"
```

### Phase 2: Deterministic Output Ordering

**Goal:** Sort HashMap keys before output for reproducible reports.

Currently escapes and binary sizes come from `HashMap` which has non-deterministic iteration order. This makes output unstable across runs.

**File:** `crates/cli/src/report/text.rs`

Sort keys before iterating:

```rust
// For escapes (line 71-75, 123-127):
if let Some(escapes) = filtered.escapes() {
    let mut keys: Vec<_> = escapes.source.keys().collect();
    keys.sort();
    for name in keys {
        writeln!(output, "escapes.{}: {}", name, escapes.source[name])?;
    }
}

// For binary_size (line 89-93, 141-144):
if let Some(sizes) = filtered.binary_size() {
    let mut keys: Vec<_> = sizes.keys().collect();
    keys.sort();
    for name in keys {
        writeln!(output, "binary_size.{}: {}", name, human_bytes(sizes[name]))?;
    }
}
```

**Files to update:**
- `crates/cli/src/report/text.rs` - Both `write_to_fmt` and `write_to_io`
- `crates/cli/src/report/html.rs` - Table rows and cards
- `crates/cli/src/report/json.rs` - Consider `BTreeMap` in output

**Verification:**
```bash
# Run report multiple times, output should be identical
cargo run -- report > /tmp/r1.txt
cargo run -- report > /tmp/r2.txt
diff /tmp/r1.txt /tmp/r2.txt  # Should be empty
```

### Phase 3: Markdown Format

**Goal:** Add dedicated Markdown formatter with proper table formatting.

The spec mentions "Markdown to stdout" but currently `.md` files are treated as text. Markdown format should use tables for better readability.

**File:** `crates/cli/src/report/markdown.rs` (NEW)

```rust
//! Markdown format report output.

use std::fmt::Write;

use crate::baseline::Baseline;
use crate::cli::CheckFilter;

use super::{human_bytes, FilteredMetrics, ReportFormatter};

/// Markdown format report formatter.
pub struct MarkdownFormatter;

impl ReportFormatter for MarkdownFormatter {
    fn format(&self, baseline: &Baseline, filter: &dyn CheckFilter) -> anyhow::Result<String> {
        let filtered = FilteredMetrics::new(baseline, filter);
        let mut output = String::with_capacity(512);

        // Header
        writeln!(output, "# Quench Report\n")?;
        if let Some(ref commit) = baseline.commit {
            let date = baseline.updated.format("%Y-%m-%d");
            writeln!(output, "**Baseline:** {} ({})\n", commit, date)?;
        }

        // Summary table
        writeln!(output, "| Metric | Value |")?;
        writeln!(output, "|--------|-------|")?;

        if let Some(coverage) = filtered.coverage() {
            writeln!(output, "| Coverage | {:.1}% |", coverage.total)?;
        }

        if let Some(escapes) = filtered.escapes() {
            let mut keys: Vec<_> = escapes.source.keys().collect();
            keys.sort();
            for name in keys {
                writeln!(output, "| Escapes ({}) | {} |", name, escapes.source[name])?;
            }
        }

        if let Some(build) = filtered.build_time() {
            writeln!(output, "| Build (cold) | {:.1}s |", build.cold)?;
            writeln!(output, "| Build (hot) | {:.1}s |", build.hot)?;
        }

        if let Some(tests) = filtered.test_time() {
            writeln!(output, "| Test time | {:.1}s |", tests.total)?;
        }

        if let Some(sizes) = filtered.binary_size() {
            let mut keys: Vec<_> = sizes.keys().collect();
            keys.sort();
            for name in keys {
                writeln!(output, "| Binary ({}) | {} |", name, human_bytes(sizes[name]))?;
            }
        }

        Ok(output)
    }

    fn format_to(
        &self,
        writer: &mut dyn std::io::Write,
        baseline: &Baseline,
        filter: &dyn CheckFilter,
    ) -> anyhow::Result<()> {
        let output = self.format(baseline, filter)?;
        write!(writer, "{}", output)?;
        Ok(())
    }

    fn format_empty(&self) -> String {
        "# Quench Report\n\n*No baseline found.*\n".to_string()
    }
}
```

**File:** `crates/cli/src/report/mod.rs`

Add module and update format selection:

```rust
mod markdown;
pub use markdown::MarkdownFormatter;

// In OutputFormat enum or format_report_with_options:
OutputFormat::Markdown => Box::new(MarkdownFormatter),
```

**File:** `crates/cli/src/cli.rs`

Add Markdown to OutputFormat enum and parsing:

```rust
pub enum OutputFormat {
    Text,
    Json,
    Html,
    Markdown,
}

// In output_target():
} else if val.ends_with(".md") {
    (OutputFormat::Markdown, Some(PathBuf::from(&self.output)))
}
// And in format name parsing:
"md" | "markdown" => OutputFormat::Markdown,
```

**Verification:**
```bash
cargo run -- report -o markdown
# Should output formatted table

cargo run -- report -o report.md
# Should write Markdown to file
```

### Phase 4: Custom Baseline Path Flag

**Goal:** Add `--baseline <path>` flag to override default `.quench/baseline.json`.

Useful for CI pipelines and comparing different baselines.

**File:** `crates/cli/src/cli.rs`

Add flag to ReportArgs:

```rust
#[derive(clap::Args, Default)]
pub struct ReportArgs {
    /// Path to baseline file (default: .quench/baseline.json)
    #[arg(long, short = 'b')]
    pub baseline: Option<PathBuf>,

    // ... existing fields
}
```

**File:** `crates/cli/src/main.rs`

Update `run_report` to use custom path:

```rust
fn run_report(args: ReportArgs, root: &Path, config: &Config) -> Result<()> {
    let baseline_path = args.baseline.clone().unwrap_or_else(|| {
        root.join(&config.git.baseline)
    });

    let baseline = Baseline::load(&baseline_path)?;
    // ... rest of implementation
}
```

**Verification:**
```bash
cargo run -- report --baseline /path/to/other/baseline.json
# Should read from specified path

cargo run -- report -b .quench/old-baseline.json
# Short form should work
```

### Phase 5: Per-Package Coverage Display

**Goal:** Show per-package coverage breakdown when available in baseline.

The `CoverageMetrics` struct has `by_package: Option<HashMap<String, f64>>` but it's not displayed.

**File:** `crates/cli/src/report/text.rs`

Add per-package coverage:

```rust
if let Some(coverage) = filtered.coverage() {
    writeln!(output, "coverage: {:.1}%", coverage.total)?;

    if let Some(ref packages) = coverage.by_package {
        let mut keys: Vec<_> = packages.keys().collect();
        keys.sort();
        for name in keys {
            writeln!(output, "  {}: {:.1}%", name, packages[name])?;
        }
    }
}
```

**File:** `tests/fixtures/report/with-packages/.quench/baseline.json` (NEW)

```json
{
  "version": 1,
  "updated": "2026-01-20T12:00:00Z",
  "commit": "abc1234",
  "metrics": {
    "coverage": {
      "total": 85.5,
      "by_package": {
        "core": 92.0,
        "cli": 78.5,
        "adapters": 86.2
      }
    }
  }
}
```

**Verification:**
```bash
cargo test --test specs report_shows_per_package_coverage
```

### Phase 6: Test Escapes Display

**Goal:** Show test escape counts separately from source escapes.

The `EscapesMetrics` struct has `test: Option<HashMap<String, usize>>` but it's not displayed.

**File:** `crates/cli/src/report/text.rs`

Add test escapes section:

```rust
if let Some(escapes) = filtered.escapes() {
    // Source escapes
    let mut keys: Vec<_> = escapes.source.keys().collect();
    keys.sort();
    for name in keys {
        writeln!(output, "escapes.{}: {}", name, escapes.source[name])?;
    }

    // Test escapes (if present)
    if let Some(ref test) = escapes.test {
        let mut keys: Vec<_> = test.keys().collect();
        keys.sort();
        for name in keys {
            writeln!(output, "escapes.test.{}: {}", name, test[name])?;
        }
    }
}
```

Apply same pattern to markdown.rs and html.rs formatters.

**Verification:**
```bash
# Create fixture with test escapes
# Run report and verify test escapes appear
```

### Phase 7: Final Verification

**Goal:** Ensure all changes work together and pass CI.

**Steps:**
1. Run full test suite
2. Dogfood: run quench on quench
3. Verify all formatters produce consistent output
4. Update specs to verify new features

**Verification:**
```bash
# Full CI check
make check

# Dogfooding
cargo run -- check

# Test all output formats
cargo run -- report -o text
cargo run -- report -o json
cargo run -- report -o html
cargo run -- report -o markdown

# Verify deterministic output
for i in 1 2 3; do cargo run -- report > /tmp/r$i.txt; done
diff /tmp/r1.txt /tmp/r2.txt && diff /tmp/r2.txt /tmp/r3.txt
```

## Key Implementation Details

### Output Ordering Strategy

For deterministic output, always sort map keys before iteration:

```rust
// Pattern for sorted iteration
let mut keys: Vec<_> = map.keys().collect();
keys.sort();
for key in keys {
    // use map[key]
}
```

This adds minimal overhead (small maps) while ensuring reproducible output.

### Human Bytes Formatting

The existing `human_bytes()` function (mod.rs:184-194) handles:
- `< 1024` → "X B"
- `< 1MB` → "X.Y KB"
- `>= 1MB` → "X.Y MB"

No changes needed to the helper, just use it consistently.

### Markdown Table Alignment

For Markdown tables, right-align numeric values:

```markdown
| Metric | Value |
|--------|------:|
| Coverage | 85.5% |
```

Use `:` in header separator row for alignment hints.

### Backward Compatibility

These changes are backward compatible:
- Human bytes is more readable, same data
- Sorted output is deterministic, same content
- Markdown is a new format, doesn't affect existing
- Custom baseline path is optional, default unchanged
- Per-package/test escapes only shown when present

## Verification Plan

### Phase 1 Verification
```bash
# Binary sizes show human-readable format
cargo test --test specs report_text_shows_summary
# Verify output contains "KB" or "MB" for binary sizes
```

### Phase 2 Verification
```bash
# Repeated runs produce identical output
for i in 1 2 3; do
    cargo run -- report > /tmp/report_$i.txt
done
diff /tmp/report_1.txt /tmp/report_2.txt  # empty = success
```

### Phase 3 Verification
```bash
# Markdown format outputs table
cargo run -- report -o markdown | grep -E "^\|.*\|$"
# Should match table rows

# File extension detection
cargo run -- report -o test.md
head -1 test.md  # Should be "# Quench Report"
```

### Phase 4 Verification
```bash
# Custom baseline path works
echo '{"version":1,"updated":"2026-01-01T00:00:00Z","metrics":{}}' > /tmp/test-baseline.json
cargo run -- report --baseline /tmp/test-baseline.json
# Should read from custom path
```

### Phase 5 Verification
```bash
# Per-package coverage displayed
cargo test --test specs report_shows_packages
```

### Phase 6 Verification
```bash
# Test escapes shown separately
cargo test --test specs report_shows_test_escapes
```

### Phase 7 (Final) Verification
```bash
# Full CI
make check

# Dogfooding passes
cargo run -- check

# All formats work
cargo run -- report -o text   | head -5
cargo run -- report -o json   | jq .metrics
cargo run -- report -o html   | grep DOCTYPE
cargo run -- report -o markdown | grep "^|"
```

## Exit Criteria

- [ ] Binary sizes display as "1.5 MB" instead of "1572864 bytes"
- [ ] Report output is deterministic (sorted keys)
- [ ] `quench report -o markdown` outputs formatted tables
- [ ] `quench report --baseline <path>` overrides default
- [ ] Per-package coverage displayed when present
- [ ] Test escapes displayed when present
- [ ] All tests pass: `make check`
- [ ] Dogfooding passes: `quench check` on quench
