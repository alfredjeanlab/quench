# Checkpoint 16C: Refactor - Report Command

**Plan:** `checkpoint-16c-refactor`
**Root Feature:** `quench-report`
**Depends On:** `checkpoint-16b-validate` (Report Command Validation)

## Overview

Refactor the `quench report` command implementation to eliminate code duplication, improve maintainability, and establish patterns for future output formatters. The report command is fully functional (validated in 16B); this checkpoint focuses on code quality and architecture.

**Key Refactoring Goals:**
1. Extract shared check filtering logic between `CheckArgs` and `ReportArgs`
2. Introduce a `ReportFormatter` trait for consistent output formatting
3. Move "no baseline" handling into the report module
4. Reduce HTML template verbosity with better structure

## Project Structure

```
crates/cli/src/
├── cli.rs                    # CheckArgs, ReportArgs (modify)
├── main.rs                   # run_report() (simplify)
├── report.rs                 # Report formatters (refactor)
└── report/                   # NEW: report submodule
    ├── mod.rs                # ReportFormatter trait, format dispatch
    ├── text.rs               # TextReportFormatter
    ├── json.rs               # JsonReportFormatter
    └── html.rs               # HtmlReportFormatter + template

quench/src/
└── cli.rs                    # CheckFilter trait (new)
```

## Dependencies

No new external dependencies required. Uses existing:
- `serde_json` for JSON output
- `chrono` for date formatting

## Implementation Phases

### Phase 1: Extract Check Filter Trait

**Goal:** Eliminate duplication of enable/disable flag handling between `CheckArgs` and `ReportArgs`.

**Current Problem:**
```rust
// In cli.rs - both CheckArgs and ReportArgs have identical fields:
pub struct CheckArgs {
    #[arg(long)] pub cloc: bool,
    #[arg(long)] pub no_cloc: bool,
    // ... 16 more identical fields
}

pub struct ReportArgs {
    #[arg(long)] pub cloc: bool,
    #[arg(long)] pub no_cloc: bool,
    // ... same 16 fields duplicated
}
```

**Solution:** Create a `CheckFilter` trait in `quench/src/cli.rs`:

```rust
// quench/src/cli.rs
pub trait CheckFilter {
    fn enabled_checks(&self) -> Vec<String>;
    fn disabled_checks(&self) -> Vec<String>;

    fn should_include(&self, check_name: &str) -> bool {
        let enabled = self.enabled_checks();
        let disabled = self.disabled_checks();

        if !enabled.is_empty() {
            enabled.iter().any(|e| e == check_name)
        } else {
            !disabled.iter().any(|d| d == check_name)
        }
    }
}
```

**Files Modified:**
- `quench/src/cli.rs` - Add `CheckFilter` trait
- `quench/src/lib.rs` - Re-export trait
- `crates/cli/src/cli.rs` - Implement `CheckFilter` for both args structs

**Verification:**
```bash
cargo test --all
# Ensure CheckArgs and ReportArgs both work with trait
```

---

### Phase 2: Create Report Formatter Trait

**Goal:** Replace three separate format functions with a trait-based approach.

**Current Problem:**
```rust
// report.rs - three similar functions with repeated patterns
fn format_text_report(args: &ReportArgs, baseline: &Baseline) -> Result<String>
fn format_json_report(args: &ReportArgs, baseline: &Baseline) -> Result<String>
fn format_html_report(args: &ReportArgs, baseline: &Baseline) -> Result<String>
```

**Solution:** Create `ReportFormatter` trait:

```rust
// crates/cli/src/report/mod.rs
use quench::baseline::Baseline;
use quench::cli::CheckFilter;

pub trait ReportFormatter {
    fn format(&self, baseline: &Baseline, filter: &dyn CheckFilter) -> anyhow::Result<String>;
    fn format_empty(&self) -> String;
}

pub fn format_report<F: CheckFilter>(
    format: OutputFormat,
    baseline: Option<&Baseline>,
    filter: &F,
) -> anyhow::Result<String> {
    let formatter: Box<dyn ReportFormatter> = match format {
        OutputFormat::Text => Box::new(TextFormatter),
        OutputFormat::Json => Box::new(JsonFormatter),
        OutputFormat::Html => Box::new(HtmlFormatter),
    };

    match baseline {
        Some(b) => formatter.format(b, filter),
        None => Ok(formatter.format_empty()),
    }
}
```

**Files Created/Modified:**
- `crates/cli/src/report/mod.rs` - Trait definition and dispatch
- `crates/cli/src/report/text.rs` - `TextFormatter` impl
- `crates/cli/src/report/json.rs` - `JsonFormatter` impl
- `crates/cli/src/report/html.rs` - `HtmlFormatter` impl
- `crates/cli/src/report.rs` - Remove (replaced by module)

**Verification:**
```bash
cargo test --test specs report
# All 12 report specs must pass
```

---

### Phase 3: Simplify run_report()

**Goal:** Move "no baseline" handling to formatters, simplify main.rs.

**Current Problem:**
```rust
// main.rs:558-592 - duplicated no-baseline handling inline
match baseline {
    Some(baseline) => { /* format */ }
    None => {
        let output = match format {
            OutputFormat::Text => "No baseline found.\n".to_string(),
            OutputFormat::Json => r#"{"metrics": {}}"#.to_string(),
            OutputFormat::Html => r#"<!DOCTYPE html>..."#.to_string(),  // 10 lines
        };
        // duplicate write logic
    }
}
```

**Solution:** Each formatter implements `format_empty()`, dispatch handles both cases:

```rust
// main.rs - simplified run_report()
fn run_report(cli: &Cli, args: &ReportArgs) -> anyhow::Result<()> {
    let cwd = std::env::current_dir()?;
    let config = load_config(cli, &cwd)?;
    let baseline_path = cwd.join(&config.git.baseline);
    let (format, file_path) = args.output_target();

    let baseline = Baseline::load(&baseline_path)?;
    let output = report::format_report(format, baseline.as_ref(), args)?;

    match file_path {
        Some(path) => std::fs::write(&path, &output)?,
        None => print!("{}", output),
    }
    Ok(())
}
```

**Files Modified:**
- `crates/cli/src/main.rs` - Simplify `run_report()`
- `crates/cli/src/report/mod.rs` - Handle None baseline

**Verification:**
```bash
cargo test --test specs report
# Specifically: report_without_baseline_shows_message, report_json_no_baseline_empty_metrics
```

---

### Phase 4: Refactor HTML Template

**Goal:** Make HTML generation more modular and maintainable.

**Current Problem:**
- `html_template()` is an 80-line format string
- CSS and structure mixed together
- Hard to modify or extend

**Solution:** Split into logical components:

```rust
// crates/cli/src/report/html.rs
struct HtmlFormatter;

impl HtmlFormatter {
    fn css() -> &'static str {
        include_str!("html_styles.css")  // Or inline const
    }

    fn render_card(title: &str, value: &str, category: &str) -> String { ... }
    fn render_table_row(metric: &str, value: &str) -> String { ... }
    fn render_header(baseline: &Baseline) -> String { ... }
    fn render_document(header: &str, cards: &str, table: &str) -> String { ... }
}

impl ReportFormatter for HtmlFormatter {
    fn format(&self, baseline: &Baseline, filter: &dyn CheckFilter) -> Result<String> {
        let header = Self::render_header(baseline);
        let (cards, rows) = self.collect_metrics(baseline, filter);
        Ok(Self::render_document(&header, &cards, &rows))
    }
}
```

**Files Modified:**
- `crates/cli/src/report/html.rs` - Modular HTML generation

**Verification:**
```bash
cargo test --test specs report
# Specifically: report_html_produces_valid_html, report_html_includes_metrics
```

---

### Phase 5: Extract Metric Iteration

**Goal:** Reduce repetition in metric collection across all formatters.

**Current Problem:**
Each formatter repeats the same metric iteration pattern:
```rust
// Repeated 3 times (text, json, html)
if should_show("tests", &enabled, &disabled) && let Some(ref coverage) = metrics.coverage {
    // format coverage
}
if should_show("escapes", &enabled, &disabled) && let Some(ref escapes) = metrics.escapes {
    // format escapes
}
// ... etc for each metric type
```

**Solution:** Create a metric visitor abstraction:

```rust
// crates/cli/src/report/mod.rs
pub struct FilteredMetrics<'a> {
    baseline: &'a Baseline,
    filter: &'a dyn CheckFilter,
}

impl<'a> FilteredMetrics<'a> {
    pub fn coverage(&self) -> Option<&Coverage> {
        if self.filter.should_include("tests") {
            self.baseline.metrics.coverage.as_ref()
        } else {
            None
        }
    }

    pub fn escapes(&self) -> Option<&Escapes> {
        if self.filter.should_include("escapes") {
            self.baseline.metrics.escapes.as_ref()
        } else {
            None
        }
    }
    // ... etc
}
```

**Files Modified:**
- `crates/cli/src/report/mod.rs` - Add `FilteredMetrics` helper
- `crates/cli/src/report/text.rs` - Use helper
- `crates/cli/src/report/json.rs` - Use helper
- `crates/cli/src/report/html.rs` - Use helper

**Verification:**
```bash
cargo test --all
make check
```

---

### Phase 6: Final Cleanup

**Goal:** Remove old code, update documentation, verify all tests pass.

**Tasks:**
1. Delete `crates/cli/src/report.rs` (replaced by module)
2. Update imports in `main.rs`
3. Run full test suite
4. Verify clippy passes with no warnings

**Commands:**
```bash
make check
cargo test --test specs -- --nocapture
```

**Verification:**
- All 466+ tests pass
- No clippy warnings
- `cargo fmt --check` passes

## Key Implementation Details

### CheckFilter Trait Design

The trait enables flexible filtering without coupling to clap:

```rust
pub trait CheckFilter {
    fn enabled_checks(&self) -> Vec<String>;
    fn disabled_checks(&self) -> Vec<String>;

    // Default implementation provides consistent logic
    fn should_include(&self, check: &str) -> bool { ... }
}
```

Both CLI args and future programmatic callers can implement this trait.

### ReportFormatter Trait

The formatter trait separates format concerns from dispatch:

```rust
pub trait ReportFormatter {
    /// Format a baseline into the target format
    fn format(&self, baseline: &Baseline, filter: &dyn CheckFilter) -> Result<String>;

    /// Return output for when no baseline exists
    fn format_empty(&self) -> String;
}
```

This allows:
- Easy addition of new formats (e.g., CSV, Markdown)
- Testing formatters in isolation
- Consistent error handling

### Human-Readable Byte Formatting

Keep the existing `human_bytes()` helper, but move to a shared utilities location:

```rust
fn human_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}
```

### File Structure After Refactor

```
crates/cli/src/
├── cli.rs               # Args structs implement CheckFilter
├── main.rs              # Simplified run_report()
├── report/
│   ├── mod.rs           # ReportFormatter trait, format_report(), FilteredMetrics
│   ├── text.rs          # TextFormatter
│   ├── json.rs          # JsonFormatter
│   └── html.rs          # HtmlFormatter with modular template
└── git.rs               # (unchanged)

quench/src/
├── cli.rs               # CheckFilter trait definition
└── lib.rs               # pub mod cli (re-export)
```

## Verification Plan

### Per-Phase Verification

Each phase includes specific test commands. All must pass before proceeding.

### Full Verification Checklist

After all phases complete:

```bash
# 1. All tests pass
cargo test --all

# 2. Report specs specifically
cargo test --test specs report -- --nocapture

# 3. Clippy clean
cargo clippy --all-targets --all-features -- -D warnings

# 4. Format check
cargo fmt --all -- --check

# 5. Full make check
make check
```

### Behavioral Spec Coverage

The existing 12 report specs verify:

| Spec | What It Tests |
|------|---------------|
| `report_reads_baseline_file` | Basic baseline loading |
| `report_without_baseline_shows_message` | No-baseline text output |
| `report_default_format_is_text` | Default format selection |
| `report_text_shows_summary` | Text format content |
| `report_text_shows_baseline_info` | Baseline metadata in text |
| `report_json_outputs_metrics` | JSON format content |
| `report_json_includes_metadata` | JSON metadata fields |
| `report_json_no_baseline_empty_metrics` | No-baseline JSON |
| `report_html_produces_valid_html` | HTML document structure |
| `report_html_includes_metrics` | HTML metric cards |
| `report_writes_to_file` | File output via `-o path` |
| `report_command_exists` | Command registration |

All specs must continue passing through the refactor.

### Manual Verification

```bash
cd tests/fixtures/report/with-baseline

# Text format
cargo run -p quench -- report

# JSON format (validate with jq)
cargo run -p quench -- report -o json | jq .

# HTML format (visual check)
cargo run -p quench -- report -o /tmp/report.html
open /tmp/report.html
```

## Deliverables

1. **Refactored Report Module:** `crates/cli/src/report/` with trait-based formatters
2. **CheckFilter Trait:** `quench/src/cli.rs` for shared filtering logic
3. **Simplified run_report():** Reduced from ~60 lines to ~15 lines
4. **All Tests Passing:** 466+ tests, no regressions
