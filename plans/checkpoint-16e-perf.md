# Checkpoint 16E: Performance - Report Command

**Plan:** `checkpoint-16e-perf`
**Root Feature:** `quench-report`
**Depends On:** `checkpoint-16d-benchmark` (Report Command Benchmarks)

## Overview

Optimize the `quench report` command based on benchmarks from checkpoint 16D. While current performance is excellent (all targets exceeded by 100-1000x), this checkpoint adds streaming output support, memory efficiency improvements, and compact output modes for CI/scripting use cases.

**Key Optimizations:**
1. Streaming output via Write trait (zero-copy to stdout)
2. Pre-allocated string buffers for reduced allocations
3. Compact JSON mode for smaller CI output
4. End-to-end benchmarks including file I/O

**Context from 16D Benchmarks:**
- Formatters: <50µs (all cases)
- File I/O: ~1ms (dominates total time)
- Total CLI: ~3ms (including process startup)
- Formatting represents <2% of execution time

## Project Structure

```
quench/
├── crates/cli/
│   ├── src/report/
│   │   ├── mod.rs              # MODIFY: Add format_to() with Write trait
│   │   ├── text.rs             # MODIFY: Add streaming support
│   │   ├── json.rs             # MODIFY: Add compact mode, streaming
│   │   ├── html.rs             # MODIFY: Add streaming support
│   │   ├── text_tests.rs       # NEW: Unit tests for text formatter
│   │   ├── json_tests.rs       # NEW: Unit tests for json formatter
│   │   └── html_tests.rs       # NEW: Unit tests for html formatter
│   ├── src/cli.rs              # MODIFY: Add --compact flag to ReportArgs
│   ├── src/main.rs             # MODIFY: Use streaming output
│   └── benches/
│       └── report.rs           # MODIFY: Add streaming + allocation benchmarks
├── tests/specs/
│   └── report_compact_spec.rs  # NEW: Spec tests for compact mode
└── reports/
    └── checkpoint-16e-benchmarks.md  # Output: comparison report
```

## Dependencies

No new dependencies required. Uses existing:
- `std::io::Write` - Standard library Write trait
- `serde_json` - Already supports `to_writer()` for streaming JSON
- `criterion` - Existing benchmark framework

## Implementation Phases

### Phase 1: Add Write Trait Support to Formatters

**Goal:** Enable streaming output directly to stdout without intermediate String allocation.

**Tasks:**

1. Extend `ReportFormatter` trait with streaming method:

```rust
// crates/cli/src/report/mod.rs
pub trait ReportFormatter {
    /// Format baseline metrics to a String (existing).
    fn format(&self, baseline: &Baseline, filter: &dyn CheckFilter) -> anyhow::Result<String>;

    /// Format baseline metrics directly to a writer (streaming).
    fn format_to<W: std::io::Write>(
        &self,
        writer: &mut W,
        baseline: &Baseline,
        filter: &dyn CheckFilter,
    ) -> anyhow::Result<()>;

    /// Return output for when no baseline exists.
    fn format_empty(&self) -> String;

    /// Write empty output to a writer.
    fn format_empty_to<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        write!(writer, "{}", self.format_empty())
    }
}
```

2. Implement `format_to` for `TextFormatter`:

```rust
// crates/cli/src/report/text.rs
impl ReportFormatter for TextFormatter {
    fn format_to<W: std::io::Write>(
        &self,
        writer: &mut W,
        baseline: &Baseline,
        filter: &dyn CheckFilter,
    ) -> anyhow::Result<()> {
        writeln!(writer, "Quench Report")?;
        writeln!(writer, "=============")?;
        // ... write directly to writer instead of String
        Ok(())
    }
}
```

3. Implement `format_to` for `JsonFormatter`:

```rust
// crates/cli/src/report/json.rs
// Use serde_json::to_writer for streaming JSON
fn format_to<W: std::io::Write>(
    &self,
    writer: &mut W,
    baseline: &Baseline,
    filter: &dyn CheckFilter,
) -> anyhow::Result<()> {
    let value = self.build_json(baseline, filter);
    serde_json::to_writer_pretty(writer, &value)?;
    Ok(())
}
```

4. Implement `format_to` for `HtmlFormatter` (write template pieces directly).

5. Add `format_report_to()` function in `mod.rs`:

```rust
pub fn format_report_to<W: std::io::Write, F: CheckFilter>(
    writer: &mut W,
    format: OutputFormat,
    baseline: Option<&Baseline>,
    filter: &F,
) -> anyhow::Result<()> {
    let formatter: Box<dyn ReportFormatter> = match format {
        OutputFormat::Text => Box::new(TextFormatter),
        OutputFormat::Json => Box::new(JsonFormatter::new(false)),
        OutputFormat::Html => Box::new(HtmlFormatter),
    };

    match baseline {
        Some(b) => formatter.format_to(writer, b, filter),
        None => Ok(formatter.format_empty_to(writer)?),
    }
}
```

**Verification:**
```bash
cargo test -p quench --lib report
cargo run -p quench -- report tests/fixtures/bench-report/typical
```

---

### Phase 2: Add Pre-allocated Buffers

**Goal:** Reduce allocations by pre-sizing String buffers based on estimated output size.

**Tasks:**

1. Add size estimation to `FilteredMetrics`:

```rust
// crates/cli/src/report/mod.rs
impl<'a> FilteredMetrics<'a> {
    /// Estimate number of metrics that will be included.
    pub fn count(&self) -> usize {
        let mut n = 0;
        if self.coverage().is_some() { n += 1; }
        if let Some(esc) = self.escapes() { n += esc.source.len(); }
        if self.build_time().is_some() { n += 2; } // cold + hot
        if let Some(sizes) = self.binary_size() { n += sizes.len(); }
        if self.test_time().is_some() { n += 1; }
        n
    }
}
```

2. Update `TextFormatter::format()` to pre-allocate:

```rust
fn format(&self, baseline: &Baseline, filter: &dyn CheckFilter) -> anyhow::Result<String> {
    let filtered = FilteredMetrics::new(baseline, filter);
    // Estimate: ~50 bytes per metric + 100 bytes header
    let capacity = 100 + filtered.count() * 50;
    let mut output = String::with_capacity(capacity);
    // ... existing formatting logic
    Ok(output)
}
```

3. Update `HtmlFormatter::format()` with capacity estimation:
   - Base template: ~1500 bytes
   - Per metric card: ~200 bytes
   - Per table row: ~80 bytes

4. Add unit tests verifying allocation behavior.

**Verification:**
```bash
cargo test -p quench --lib report
cargo bench --bench report -- 'text/format/typical'
# Compare allocations in benchmark output
```

---

### Phase 3: Add Compact JSON Mode

**Goal:** Provide smaller JSON output for CI pipelines and scripting.

**Tasks:**

1. Add `--compact` flag to `ReportArgs`:

```rust
// crates/cli/src/cli.rs
#[derive(Parser, Debug)]
pub struct ReportArgs {
    // ... existing fields ...

    /// Output compact JSON (no whitespace, single line)
    #[arg(long, requires = "output")]
    pub compact: bool,
}
```

2. Update `JsonFormatter` to support compact mode:

```rust
// crates/cli/src/report/json.rs
pub struct JsonFormatter {
    compact: bool,
}

impl JsonFormatter {
    pub fn new(compact: bool) -> Self {
        Self { compact }
    }

    fn serialize<W: std::io::Write>(&self, writer: &mut W, value: &Value) -> serde_json::Result<()> {
        if self.compact {
            serde_json::to_writer(writer, value)
        } else {
            serde_json::to_writer_pretty(writer, value)
        }
    }
}
```

3. Update `format_report()` and `format_report_to()` to accept compact option:

```rust
pub fn format_report<F: CheckFilter>(
    format: OutputFormat,
    baseline: Option<&Baseline>,
    filter: &F,
    compact: bool,  // NEW parameter
) -> anyhow::Result<String>
```

4. Add spec test for compact mode:

```rust
// tests/specs/report_compact_spec.rs
#[test]
fn compact_json_is_single_line() {
    let output = run_quench(&["report", "--output", "json", "--compact"]);
    assert!(!output.contains('\n') || output.ends_with('\n'));
}

#[test]
fn compact_only_applies_to_json() {
    // --compact with text/html should be ignored or error
}
```

**Verification:**
```bash
cargo test -p quench -- compact
cargo run -p quench -- report tests/fixtures/bench-report/typical -o json --compact
# Output should be single line
```

---

### Phase 4: Update main.rs for Streaming

**Goal:** Use streaming output in the report command for reduced memory usage.

**Tasks:**

1. Update `run_report()` to use streaming when writing to stdout:

```rust
// crates/cli/src/main.rs
fn run_report(cli: &Cli, args: &ReportArgs) -> anyhow::Result<()> {
    // ... existing setup ...

    let (format, file_path) = args.output_target();
    let baseline = Baseline::load(&baseline_path)?;

    match file_path {
        Some(path) => {
            // File output: use buffered writer
            let file = std::fs::File::create(&path)?;
            let mut writer = std::io::BufWriter::new(file);
            report::format_report_to(&mut writer, format, baseline.as_ref(), args, args.compact)?;
        }
        None => {
            // Stdout: use stdout lock for efficiency
            let stdout = std::io::stdout();
            let mut handle = stdout.lock();
            report::format_report_to(&mut handle, format, baseline.as_ref(), args, args.compact)?;
        }
    }
    Ok(())
}
```

2. Keep original `format_report()` for backwards compatibility (benchmarks, tests).

**Verification:**
```bash
cargo test --all
cargo run -p quench -- report tests/fixtures/bench-report/typical
cargo run -p quench -- report tests/fixtures/bench-report/typical -o report.html
```

---

### Phase 5: Add Streaming and Allocation Benchmarks

**Goal:** Measure performance impact of streaming vs. buffered output.

**Tasks:**

1. Add streaming benchmarks to `benches/report.rs`:

```rust
/// Benchmark streaming output vs. String allocation.
fn bench_streaming_vs_buffered(c: &mut Criterion) {
    let mut group = c.benchmark_group("report/streaming");
    let baseline = load_fixture_baseline("large-escapes");
    let args = ReportArgs::default();

    // Buffered (current): format to String, then write
    group.bench_function("buffered/html", |b| {
        b.iter(|| {
            let output = format_report(OutputFormat::Html, Some(&baseline), &args).unwrap();
            std::io::sink().write_all(output.as_bytes()).unwrap();
        })
    });

    // Streaming: write directly to sink
    group.bench_function("streaming/html", |b| {
        b.iter(|| {
            format_report_to(
                &mut std::io::sink(),
                OutputFormat::Html,
                Some(&baseline),
                &args,
                false,
            ).unwrap();
        })
    });

    group.finish();
}
```

2. Add compact vs. pretty JSON benchmark:

```rust
fn bench_compact_json(c: &mut Criterion) {
    let mut group = c.benchmark_group("report/json-compact");
    let baseline = load_fixture_baseline("large-escapes");
    let args = ReportArgs::default();

    group.bench_function("pretty", |b| {
        b.iter(|| format_report(OutputFormat::Json, Some(&baseline), &args, false))
    });

    group.bench_function("compact", |b| {
        b.iter(|| format_report(OutputFormat::Json, Some(&baseline), &args, true))
    });

    group.finish();
}
```

3. Run full benchmark suite and compare against 16D baseline:
```bash
cargo bench --bench report -- --baseline checkpoint-16d
```

**Verification:**
```bash
cargo bench --bench report -- --test
cargo bench --bench report
```

---

### Phase 6: Document Results and Establish Baseline

**Goal:** Write performance comparison report and save new baseline.

**Tasks:**

1. Run benchmarks and collect results:
```bash
cargo bench --bench report -- --save-baseline checkpoint-16e
```

2. Compare against 16D baseline:
```bash
cargo bench --bench report -- --baseline checkpoint-16d
```

3. Write benchmark comparison report to `reports/checkpoint-16e-benchmarks.md`:

```markdown
# Checkpoint 16E: Performance Optimization Results

## Summary

| Optimization | Before (16D) | After (16E) | Improvement |
|--------------|--------------|-------------|-------------|
| Text typical | X µs | Y µs | Z% |
| HTML streaming vs. buffered | N/A | A vs. B µs | C% |
| JSON compact vs. pretty | N/A | D vs. E µs | F% |

## Memory Improvements

- Pre-allocated buffers reduce allocations by ~N%
- Streaming output eliminates intermediate String for stdout

## Compact Mode

- JSON output size reduced by ~30% with --compact
- Useful for CI pipelines where output is parsed programmatically
```

4. Update CHANGELOG if applicable.

**Verification:**
```bash
cat reports/checkpoint-16e-benchmarks.md
make check
```

## Key Implementation Details

### Streaming Write Pattern

The `format_to` method writes directly to any `std::io::Write` implementor:

```rust
// Efficient stdout writing
let stdout = std::io::stdout();
let mut handle = stdout.lock();
formatter.format_to(&mut handle, baseline, filter)?;

// Efficient file writing
let file = std::fs::File::create(path)?;
let mut writer = std::io::BufWriter::new(file);
formatter.format_to(&mut writer, baseline, filter)?;
```

### Backwards Compatibility

The existing `format()` method remains for:
- Benchmarks that measure String generation
- Tests that assert on output content
- Cases where String is needed (e.g., further processing)

```rust
// Default format() implementation using format_to()
fn format(&self, baseline: &Baseline, filter: &dyn CheckFilter) -> anyhow::Result<String> {
    let mut buffer = Vec::new();
    self.format_to(&mut buffer, baseline, filter)?;
    Ok(String::from_utf8(buffer)?)
}
```

### Compact JSON Validation

The `--compact` flag requires JSON output format:

```rust
// In cli.rs or validation
if args.compact {
    match args.output_target().0 {
        OutputFormat::Json => Ok(()),
        _ => Err(anyhow!("--compact only applies to JSON output")),
    }
}
```

### Size Estimation Constants

Based on actual output analysis:

```rust
const TEXT_HEADER_SIZE: usize = 100;
const TEXT_METRIC_SIZE: usize = 50;

const HTML_BASE_SIZE: usize = 1500;  // Template + CSS
const HTML_CARD_SIZE: usize = 200;
const HTML_ROW_SIZE: usize = 80;

const JSON_OVERHEAD: usize = 100;    // Metadata, brackets
const JSON_METRIC_SIZE: usize = 80;  // Per metric object
```

## Verification Plan

### Per-Phase Verification

Each phase includes specific commands that must pass before proceeding.

### Full Verification Checklist

```bash
# 1. All tests pass
cargo test --all

# 2. Clippy clean
cargo clippy --all-targets --all-features -- -D warnings

# 3. Benchmarks run without errors
cargo bench --bench report -- --test

# 4. Streaming output works
cargo run -p quench -- report tests/fixtures/bench-report/typical

# 5. Compact mode works
cargo run -p quench -- report tests/fixtures/bench-report/typical -o json --compact

# 6. File output works
cargo run -p quench -- report tests/fixtures/bench-report/typical -o /tmp/report.html

# 7. Full make check
make check
```

### Success Criteria

- [ ] `ReportFormatter` trait extended with `format_to()` method
- [ ] All three formatters implement streaming output
- [ ] Pre-allocated buffers added with size estimation
- [ ] `--compact` flag added for JSON output
- [ ] `run_report()` uses streaming for stdout
- [ ] Benchmarks added for streaming vs. buffered
- [ ] Benchmark comparison report written
- [ ] New criterion baseline saved as `checkpoint-16e`
- [ ] `make check` passes

## Deliverables

1. **Streaming Formatters:** `format_to()` method on all formatters
2. **Compact JSON:** `--compact` flag for single-line JSON output
3. **Pre-allocated Buffers:** Size estimation for reduced allocations
4. **Streaming Benchmarks:** Comparison of streaming vs. buffered output
5. **Benchmark Report:** `reports/checkpoint-16e-benchmarks.md`
6. **Criterion Baseline:** Saved as `checkpoint-16e` for regression detection
