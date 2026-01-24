# Checkpoint 16G: Bug Fixes - Report Command

**Root Feature:** `quench-fc4c`

## Overview

This checkpoint addresses code quality issues and minor bugs in the `quench report` command implementation. Following the quick wins in checkpoint-16f, this phase focuses on:

1. **Eliminate code duplication** - TextFormatter has two nearly identical 70-line methods
2. **Fix streaming in MarkdownFormatter** - `format_to()` creates intermediate String instead of streaming
3. **Add error context** - Baseline load errors lack file path context
4. **Add unit tests** - Coverage for report formatters in sibling `_tests.rs` files

## Project Structure

```
quench/
├── crates/cli/src/
│   ├── main.rs                 # UPDATE: Add context to baseline load error
│   └── report/
│       ├── mod.rs              # EXISTS: ReportFormatter trait
│       ├── mod_tests.rs        # NEW: Unit tests for human_bytes, FilteredMetrics
│       ├── text.rs             # UPDATE: Consolidate duplicated methods
│       ├── text_tests.rs       # NEW: Unit tests for TextFormatter
│       ├── markdown.rs         # UPDATE: Implement streaming format_to
│       ├── markdown_tests.rs   # NEW: Unit tests for MarkdownFormatter
│       ├── html.rs             # EXISTS: Reference implementation
│       ├── html_tests.rs       # NEW: Unit tests for HtmlFormatter
│       └── json.rs             # EXISTS: Already clean
└── tests/specs/cli/
    └── report.rs               # EXISTS: Behavioral tests (no changes)
```

## Dependencies

No new external dependencies. Uses existing infrastructure:

- `anyhow` - Error handling with context (exists)
- `std::fmt::Write` - Format trait for unified writing

## Implementation Phases

### Phase 1: Add Error Context to Baseline Load

**Goal:** When `Baseline::load` fails, include the file path in the error message.

Currently (main.rs:566):
```rust
let baseline = Baseline::load(&baseline_path)?;
```

If loading fails (parse error, version mismatch), users see the error without knowing which file was attempted.

**File:** `crates/cli/src/main.rs`

Add context using anyhow:

```rust
use anyhow::Context;

// In run_report:
let baseline = Baseline::load(&baseline_path)
    .with_context(|| format!("failed to load baseline from {}", baseline_path.display()))?;
```

**Verification:**
```bash
# Create invalid baseline
echo "not json" > /tmp/bad-baseline.json
cargo run -- report --baseline /tmp/bad-baseline.json
# Should show: "failed to load baseline from /tmp/bad-baseline.json"
```

### Phase 2: Consolidate TextFormatter Duplication

**Goal:** Eliminate 70+ lines of duplicated code between `write_to_fmt` and `write_to_io`.

The problem: Rust's `fmt::Write` and `io::Write` are different traits with the same `write!` macro syntax but incompatible error types. Currently both methods duplicate all formatting logic.

**Solution:** Use a macro to generate both implementations from a single template.

**File:** `crates/cli/src/report/text.rs`

```rust
/// Macro to generate write implementations for both fmt::Write and io::Write.
/// The $write_fn and $writeln_fn parameters handle the different error types.
macro_rules! impl_text_format {
    ($writer:expr, $baseline:expr, $filtered:expr, $write_fn:ident, $writeln_fn:ident) => {{
        // Header with baseline info
        $writeln_fn!($writer, "Quench Report")?;
        $writeln_fn!($writer, "=============")?;
        if let Some(ref commit) = $baseline.commit {
            let date = $baseline.updated.format("%Y-%m-%d");
            $writeln_fn!($writer, "Baseline: {} ({})", commit, date)?;
        } else {
            let date = $baseline.updated.format("%Y-%m-%d");
            $writeln_fn!($writer, "Baseline: {}", date)?;
        }
        $writeln_fn!($writer)?;

        // Coverage
        if let Some(coverage) = $filtered.coverage() {
            $writeln_fn!($writer, "coverage: {:.1}%", coverage.total)?;
            if let Some(ref packages) = coverage.by_package {
                let mut keys: Vec<_> = packages.keys().collect();
                keys.sort();
                for name in keys {
                    $writeln_fn!($writer, "  {}: {:.1}%", name, packages[name])?;
                }
            }
        }

        // Escapes
        if let Some(escapes) = $filtered.escapes() {
            let mut keys: Vec<_> = escapes.source.keys().collect();
            keys.sort();
            for name in keys {
                $writeln_fn!($writer, "escapes.{}: {}", name, escapes.source[name])?;
            }
            if let Some(ref test) = escapes.test {
                let mut keys: Vec<_> = test.keys().collect();
                keys.sort();
                for name in keys {
                    $writeln_fn!($writer, "escapes.test.{}: {}", name, test[name])?;
                }
            }
        }

        // Build time
        if let Some(build) = $filtered.build_time() {
            $writeln_fn!($writer, "build_time.cold: {:.1}s", build.cold)?;
            $writeln_fn!($writer, "build_time.hot: {:.1}s", build.hot)?;
        }

        // Test time
        if let Some(tests) = $filtered.test_time() {
            $writeln_fn!($writer, "test_time.total: {:.1}s", tests.total)?;
        }

        // Binary size
        if let Some(sizes) = $filtered.binary_size() {
            let mut keys: Vec<_> = sizes.keys().collect();
            keys.sort();
            for name in keys {
                $writeln_fn!($writer, "binary_size.{}: {}", name, human_bytes(sizes[name]))?;
            }
        }

        Ok(())
    }};
}

impl TextFormatter {
    fn write_to_fmt(
        &self,
        output: &mut String,
        baseline: &Baseline,
        filtered: &FilteredMetrics<'_>,
    ) -> anyhow::Result<()> {
        impl_text_format!(output, baseline, filtered, write, writeln)
    }

    fn write_to_io(
        &self,
        writer: &mut dyn std::io::Write,
        baseline: &Baseline,
        filtered: &FilteredMetrics<'_>,
    ) -> anyhow::Result<()> {
        impl_text_format!(writer, baseline, filtered, write, writeln)
    }
}
```

**Verification:**
```bash
cargo test --test specs report
# All report tests should pass
```

### Phase 3: Implement Streaming for MarkdownFormatter

**Goal:** Make `format_to()` write directly to the writer instead of creating intermediate String.

Currently (markdown.rs:86-95):
```rust
fn format_to(&self, writer: &mut dyn std::io::Write, ...) -> anyhow::Result<()> {
    let output = self.format(baseline, filter)?;  // Creates String
    write!(writer, "{}", output)?;  // Then writes it
    Ok(())
}
```

This defeats the purpose of streaming output.

**Solution:** Apply the same macro pattern from TextFormatter, or write directly to the writer.

**File:** `crates/cli/src/report/markdown.rs`

```rust
impl ReportFormatter for MarkdownFormatter {
    fn format(&self, baseline: &Baseline, filter: &dyn CheckFilter) -> anyhow::Result<String> {
        let filtered = FilteredMetrics::new(baseline, filter);
        let mut output = String::with_capacity(512);
        self.write_to(&mut output, baseline, &filtered)?;
        Ok(output)
    }

    fn format_to(
        &self,
        writer: &mut dyn std::io::Write,
        baseline: &Baseline,
        filter: &dyn CheckFilter,
    ) -> anyhow::Result<()> {
        let filtered = FilteredMetrics::new(baseline, filter);
        self.write_to(writer, baseline, &filtered)
    }

    fn format_empty(&self) -> String {
        "# Quench Report\n\n*No baseline found.*\n".to_string()
    }
}

impl MarkdownFormatter {
    /// Write formatted output to any writer (String via fmt::Write, or io::Write).
    fn write_to<W: std::io::Write>(
        &self,
        writer: &mut W,
        baseline: &Baseline,
        filtered: &FilteredMetrics<'_>,
    ) -> anyhow::Result<()> {
        // Header
        writeln!(writer, "# Quench Report\n")?;
        if let Some(ref commit) = baseline.commit {
            let date = baseline.updated.format("%Y-%m-%d");
            writeln!(writer, "**Baseline:** {} ({})\n", commit, date)?;
        }

        // Summary table
        writeln!(writer, "| Metric | Value |")?;
        writeln!(writer, "|--------|------:|")?;

        // ... rest of formatting logic (single implementation)
    }
}
```

**Alternative approach:** Since `String` implements `std::io::Write` (via a wrapper), we could potentially use a single method. However, the cleanest approach is a generic method.

**Note:** For `format()` to work with `String`, we need an adapter since `String` doesn't implement `io::Write` directly. The solution is to keep format() using `fmt::Write` via `writeln!` on String, and have `format_to` write directly. Use a macro or shared helper for the logic.

**Verification:**
```bash
cargo test --test specs report
# Verify markdown output is correct
cargo run -- report -o markdown
```

### Phase 4: Add Unit Tests

**Goal:** Add unit tests in sibling `_tests.rs` files per project convention.

**File:** `crates/cli/src/report/mod_tests.rs`

```rust
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;

#[test]
fn human_bytes_formats_bytes() {
    assert_eq!(human_bytes(500), "500 B");
    assert_eq!(human_bytes(1023), "1023 B");
}

#[test]
fn human_bytes_formats_kilobytes() {
    assert_eq!(human_bytes(1024), "1.0 KB");
    assert_eq!(human_bytes(1536), "1.5 KB");
    assert_eq!(human_bytes(10240), "10.0 KB");
}

#[test]
fn human_bytes_formats_megabytes() {
    assert_eq!(human_bytes(1048576), "1.0 MB");
    assert_eq!(human_bytes(1572864), "1.5 MB");
    assert_eq!(human_bytes(10485760), "10.0 MB");
}

#[test]
fn filtered_metrics_respects_filter() {
    // Test that FilteredMetrics correctly applies check filter
    // Create baseline with metrics, filter that excludes some checks
}
```

**File:** `crates/cli/src/report/text_tests.rs`

```rust
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;
use crate::baseline::{Baseline, BaselineMetrics};
use crate::cli::AllChecks;

#[test]
fn text_format_empty_baseline() {
    let formatter = TextFormatter;
    assert_eq!(formatter.format_empty(), "No baseline found.\n");
}

#[test]
fn text_format_includes_header() {
    let baseline = Baseline::default();
    let formatter = TextFormatter;
    let output = formatter.format(&baseline, &AllChecks).unwrap();
    assert!(output.contains("Quench Report"));
    assert!(output.contains("============="));
}

#[test]
fn text_format_to_matches_format() {
    // Verify streaming output matches buffered output
    let baseline = create_test_baseline();
    let formatter = TextFormatter;

    let buffered = formatter.format(&baseline, &AllChecks).unwrap();

    let mut streamed = Vec::new();
    formatter.format_to(&mut streamed, &baseline, &AllChecks).unwrap();
    let streamed_str = String::from_utf8(streamed).unwrap();

    assert_eq!(buffered, streamed_str);
}
```

**File:** `crates/cli/src/report/markdown_tests.rs`

```rust
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;

#[test]
fn markdown_format_empty_baseline() {
    let formatter = MarkdownFormatter;
    let empty = formatter.format_empty();
    assert!(empty.contains("# Quench Report"));
    assert!(empty.contains("No baseline found"));
}

#[test]
fn markdown_format_produces_table() {
    let baseline = create_test_baseline();
    let formatter = MarkdownFormatter;
    let output = formatter.format(&baseline, &AllChecks).unwrap();
    assert!(output.contains("| Metric | Value |"));
    assert!(output.contains("|--------|"));
}

#[test]
fn markdown_format_to_streams_directly() {
    // Verify format_to doesn't create intermediate String
    // (behavior test - output should match format)
    let baseline = create_test_baseline();
    let formatter = MarkdownFormatter;

    let buffered = formatter.format(&baseline, &AllChecks).unwrap();

    let mut streamed = Vec::new();
    formatter.format_to(&mut streamed, &baseline, &AllChecks).unwrap();

    assert_eq!(buffered, String::from_utf8(streamed).unwrap());
}
```

**File:** `crates/cli/src/report/mod.rs` - Add test module:

```rust
#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
```

Similar additions for text.rs, markdown.rs, html.rs.

**Verification:**
```bash
cargo test report
# All unit tests should pass
```

### Phase 5: Final Verification

**Goal:** Ensure all changes work together and pass CI.

**Steps:**
1. Run full test suite
2. Verify streaming works correctly for all formatters
3. Verify error messages include file paths
4. Run clippy and ensure no warnings

**Verification:**
```bash
# Full CI check
make check

# Test error context
echo "bad json" > /tmp/bad.json
cargo run -- report -b /tmp/bad.json 2>&1 | grep "failed to load"

# Verify all formatters produce consistent output
cargo run -- report -o text
cargo run -- report -o json
cargo run -- report -o html
cargo run -- report -o markdown

# Verify streaming matches buffered for all formats
cargo test report
```

## Key Implementation Details

### Macro for DRY Formatting

The core issue is that `fmt::Write::write_fmt` returns `fmt::Result` while `io::Write::write_fmt` returns `io::Result`. The `writeln!` macro adapts to both, but error handling differs.

Using a macro that takes the macro names as parameters allows the same formatting logic to work with both trait types:

```rust
macro_rules! impl_format {
    ($writer:expr, $writeln:ident) => {{
        $writeln!($writer, "Header")?;
        // ... more formatting
        Ok(())
    }};
}

// Usage:
fn write_to_fmt(&self, w: &mut String) -> fmt::Result {
    impl_format!(w, writeln)
}

fn write_to_io(&self, w: &mut dyn io::Write) -> io::Result<()> {
    impl_format!(w, writeln)
}
```

### Streaming vs Buffered Trade-offs

| Approach | Memory | Complexity | Error Handling |
|----------|--------|------------|----------------|
| Buffer then write | O(output size) | Simple | Late errors |
| Stream directly | O(1) | Moderate | Immediate errors |

For report output (typically <10KB), the difference is minimal. However, streaming is the correct pattern and matches what HtmlFormatter already does.

### Error Context Pattern

Always add context when errors could originate from multiple sources:

```rust
// Bad: raw error propagation
Baseline::load(&path)?;

// Good: error includes context
Baseline::load(&path)
    .with_context(|| format!("failed to load baseline from {}", path.display()))?;
```

## Verification Plan

### Phase 1 Verification
```bash
# Test error context is included
echo "invalid" > /tmp/test.json
cargo run -- report -b /tmp/test.json 2>&1 | grep -q "failed to load baseline from"
echo "Exit code: $?"  # Should be 0
```

### Phase 2 Verification
```bash
# Verify no regressions in text output
cargo test --test specs report_text

# Check code compiles and duplication is eliminated
wc -l crates/cli/src/report/text.rs  # Should be ~100 lines vs ~190
```

### Phase 3 Verification
```bash
# Verify markdown streaming works
cargo test --test specs report_markdown

# Verify output is identical between format and format_to
cargo test report::markdown_tests::markdown_format_to_streams_directly
```

### Phase 4 Verification
```bash
# Run unit tests
cargo test report

# Verify test files exist
ls crates/cli/src/report/*_tests.rs
```

### Phase 5 (Final) Verification
```bash
# Full CI
make check

# Dogfooding
cargo run -- check

# All formats still work
for fmt in text json html markdown; do
    cargo run -- report -o $fmt > /dev/null && echo "$fmt: OK"
done
```

## Exit Criteria

- [ ] Baseline load errors include file path context
- [ ] TextFormatter has single implementation (macro-based)
- [ ] MarkdownFormatter streams directly without intermediate String
- [ ] Unit tests added for `human_bytes()` and formatters
- [ ] All tests pass: `make check`
- [ ] Code reduction: text.rs ~100 lines (from ~190)
