# Phase 1310: Report Command - Formats

**Root Feature:** `quench-1310`

## Overview

Extend the `quench report` command with HTML format output and file output support. This phase builds on Phase 1305's text/JSON implementation to add:

- HTML dashboard output with metric cards and summary tables
- File output detection (`-o report.html` writes to file instead of stdout)
- Unified output format handling for text, JSON, and HTML

The HTML format produces a self-contained static dashboard suitable for CI integration and GitHub Pages deployment.

## Project Structure

```
crates/cli/src/
├── cli.rs              # Extend OutputFormat enum with Html variant
├── main.rs             # Add file output detection in run_report()
└── report.rs           # Add format_html_report() function

tests/specs/cli/
└── report.rs           # Remove #[ignore] from HTML and file specs
```

Key files to modify:
- `crates/cli/src/cli.rs:344-349` - Add `Html` to `OutputFormat` enum
- `crates/cli/src/main.rs` - Add file output routing
- `crates/cli/src/report.rs` - Add HTML formatter

No new test fixtures needed; existing `report/with-baseline` fixture contains all required metrics.

## Dependencies

No new external dependencies. HTML will be generated using string formatting (no template engine needed for a single-page static output).

## Implementation Phases

### Phase 1: Extend OutputFormat Enum

Add HTML variant and file output path to CLI arguments.

**File:** `crates/cli/src/cli.rs`

```rust
#[derive(Clone, Copy, Default, clap::ValueEnum)]
pub enum OutputFormat {
    #[default]
    Text,
    Json,
    Html,
}
```

**Milestone:** `cargo build` succeeds; `quench report --help` shows `html` option.

### Phase 2: Add File Output Detection

Modify the `-o` flag to detect file paths vs format names. When the value ends with `.html`, `.json`, or `.txt`, treat it as a file path and infer the format from the extension.

**File:** `crates/cli/src/cli.rs`

Update `ReportArgs` to support both format and file path:

```rust
#[derive(clap::Args, Default)]
pub struct ReportArgs {
    /// Output format or file path (e.g., text, json, html, report.html)
    #[arg(short, long, default_value = "text")]
    pub output: String,  // Changed from OutputFormat to String

    // ... existing check toggle flags ...
}

impl ReportArgs {
    /// Parse output argument into format and optional file path.
    pub fn output_target(&self) -> (OutputFormat, Option<PathBuf>) {
        let val = self.output.to_lowercase();

        // Check for file extension
        if val.ends_with(".html") {
            (OutputFormat::Html, Some(PathBuf::from(&self.output)))
        } else if val.ends_with(".json") {
            (OutputFormat::Json, Some(PathBuf::from(&self.output)))
        } else if val.ends_with(".txt") || val.ends_with(".md") {
            (OutputFormat::Text, Some(PathBuf::from(&self.output)))
        } else {
            // Parse as format name
            let format = match val.as_str() {
                "json" => OutputFormat::Json,
                "html" => OutputFormat::Html,
                _ => OutputFormat::Text,
            };
            (format, None)
        }
    }
}
```

**Milestone:** `quench report -o report.html` parses as HTML format with file output.

### Phase 3: Implement File Output Routing

Update `run_report()` to write to file when path is specified.

**File:** `crates/cli/src/main.rs`

```rust
fn run_report(cli: &Cli, args: &ReportArgs) -> anyhow::Result<()> {
    // ... existing config and baseline loading ...

    let (format, file_path) = args.output_target();

    match baseline {
        Some(baseline) => {
            let output = format_report_to_string(&args, &baseline, format)?;

            if let Some(path) = file_path {
                std::fs::write(&path, &output)?;
            } else {
                print!("{}", output);
            }
            Ok(())
        }
        None => {
            // Handle no baseline case...
        }
    }
}
```

Refactor `format_report()` to return a String instead of printing directly:

**File:** `crates/cli/src/report.rs`

```rust
pub fn format_report_to_string(
    args: &ReportArgs,
    baseline: &Baseline,
    format: OutputFormat,
) -> anyhow::Result<String> {
    match format {
        OutputFormat::Text => format_text_report(args, baseline),
        OutputFormat::Json => format_json_report(args, baseline),
        OutputFormat::Html => format_html_report(args, baseline),
    }
}
```

**Milestone:** `quench report -o /tmp/test.json` creates file; `cat /tmp/test.json` shows JSON output.

### Phase 4: Implement HTML Format

Create HTML dashboard with metric cards and summary table.

**File:** `crates/cli/src/report.rs`

```rust
fn format_html_report(args: &ReportArgs, baseline: &Baseline) -> anyhow::Result<String> {
    let enabled = args.enabled_checks();
    let disabled = args.disabled_checks();
    let metrics = &baseline.metrics;

    let mut cards = Vec::new();
    let mut rows = Vec::new();

    // Coverage card
    if should_show("tests", &enabled, &disabled) {
        if let Some(ref coverage) = metrics.coverage {
            cards.push(metric_card("Coverage", &format!("{:.1}%", coverage.total), "tests"));
            rows.push(table_row("coverage", &format!("{:.1}%", coverage.total)));
        }
    }

    // Escapes cards
    if should_show("escapes", &enabled, &disabled) {
        if let Some(ref escapes) = metrics.escapes {
            for (name, count) in &escapes.source {
                cards.push(metric_card(
                    &format!("Escapes: {}", name),
                    &count.to_string(),
                    "escapes"
                ));
                rows.push(table_row(&format!("escapes.{}", name), &count.to_string()));
            }
        }
    }

    // Build metrics
    if should_show("build", &enabled, &disabled) {
        if let Some(ref build) = metrics.build_time {
            cards.push(metric_card("Build (cold)", &format!("{:.1}s", build.cold), "build"));
            cards.push(metric_card("Build (hot)", &format!("{:.1}s", build.hot), "build"));
            rows.push(table_row("build_time.cold", &format!("{:.1}s", build.cold)));
            rows.push(table_row("build_time.hot", &format!("{:.1}s", build.hot)));
        }
        if let Some(ref sizes) = metrics.binary_size {
            for (name, size) in sizes {
                let human = human_bytes(*size);
                cards.push(metric_card(&format!("Binary: {}", name), &human, "build"));
                rows.push(table_row(&format!("binary_size.{}", name), &human));
            }
        }
    }

    // Test time
    if should_show("tests", &enabled, &disabled) {
        if let Some(ref tests) = metrics.test_time {
            cards.push(metric_card("Test Time", &format!("{:.1}s", tests.total), "tests"));
            rows.push(table_row("test_time.total", &format!("{:.1}s", tests.total)));
        }
    }

    // Generate HTML
    Ok(html_template(baseline, &cards.join("\n"), &rows.join("\n")))
}

fn metric_card(title: &str, value: &str, category: &str) -> String {
    format!(
        r#"      <div class="card {category}">
        <div class="card-title">{title}</div>
        <div class="card-value">{value}</div>
      </div>"#
    )
}

fn table_row(metric: &str, value: &str) -> String {
    format!(
        r#"        <tr><td>{metric}</td><td>{value}</td></tr>"#
    )
}

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

fn html_template(baseline: &Baseline, cards: &str, rows: &str) -> String {
    let commit = baseline.commit.as_deref().unwrap_or("unknown");
    let date = baseline.updated.format("%Y-%m-%d %H:%M UTC");

    format!(r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Quench Report</title>
  <style>
    :root {{
      --bg: #1a1a2e;
      --card-bg: #16213e;
      --text: #eef;
      --muted: #8892b0;
      --accent: #64ffda;
    }}
    * {{ box-sizing: border-box; margin: 0; padding: 0; }}
    body {{
      font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
      background: var(--bg);
      color: var(--text);
      padding: 2rem;
      line-height: 1.6;
    }}
    .container {{ max-width: 1200px; margin: 0 auto; }}
    header {{
      margin-bottom: 2rem;
      padding-bottom: 1rem;
      border-bottom: 1px solid var(--card-bg);
    }}
    h1 {{ color: var(--accent); font-size: 1.5rem; }}
    .meta {{ color: var(--muted); font-size: 0.875rem; margin-top: 0.5rem; }}
    .cards {{
      display: grid;
      grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
      gap: 1rem;
      margin-bottom: 2rem;
    }}
    .card {{
      background: var(--card-bg);
      padding: 1.5rem;
      border-radius: 8px;
      border-left: 4px solid var(--accent);
    }}
    .card.escapes {{ border-color: #f59e0b; }}
    .card.build {{ border-color: #8b5cf6; }}
    .card.tests {{ border-color: #10b981; }}
    .card-title {{ color: var(--muted); font-size: 0.75rem; text-transform: uppercase; }}
    .card-value {{ font-size: 2rem; font-weight: 600; margin-top: 0.5rem; }}
    table {{
      width: 100%;
      border-collapse: collapse;
      background: var(--card-bg);
      border-radius: 8px;
      overflow: hidden;
    }}
    th, td {{ padding: 0.75rem 1rem; text-align: left; }}
    th {{ background: rgba(0,0,0,0.2); color: var(--muted); font-size: 0.75rem; text-transform: uppercase; }}
    tr:not(:last-child) td {{ border-bottom: 1px solid var(--bg); }}
    td:last-child {{ text-align: right; font-family: monospace; }}
  </style>
</head>
<body>
  <div class="container">
    <header>
      <h1>Quench Report</h1>
      <div class="meta">Baseline: {commit} &middot; {date}</div>
    </header>
    <section class="cards">
{cards}
    </section>
    <section>
      <table>
        <thead><tr><th>Metric</th><th>Value</th></tr></thead>
        <tbody>
{rows}
        </tbody>
      </table>
    </section>
  </div>
</body>
</html>"#)
}
```

**Milestone:** `quench report -o html` outputs valid HTML with styled cards.

### Phase 5: Update Specs and Verify

Remove `#[ignore]` from HTML and file output specs.

**File:** `tests/specs/cli/report.rs`

Specs to enable (update ignore attribute to reference Phase 1310):
- `report_html_produces_valid_html` - Remove `#[ignore]`
- `report_html_includes_metrics` - Remove `#[ignore]`
- `report_writes_to_file` - Remove `#[ignore]`

**Milestone:** `cargo test --test specs report` passes all specs.

### Phase 6: Final Verification

Run full test suite and manual verification.

```bash
# Specs pass
cargo test --test specs report

# Full suite
make check

# Manual verification
quench report -o html > /tmp/report.html
open /tmp/report.html  # Verify visual appearance

quench report -o report.json
cat report.json | jq .  # Verify JSON structure
rm report.json
```

**Milestone:** All checks pass; HTML renders correctly in browser.

## Key Implementation Details

### Output Format Detection Logic

The `-o` flag accepts either format names or file paths:

| Input | Format | File Output |
|-------|--------|-------------|
| `text` | Text | stdout |
| `json` | JSON | stdout |
| `html` | HTML | stdout |
| `report.html` | HTML | `report.html` |
| `metrics.json` | JSON | `metrics.json` |
| `output.txt` | Text | `output.txt` |

### HTML Structure

The HTML output follows a responsive card-based layout:

```
+------------------+------------------+------------------+
|    Coverage      |  Escapes: unsafe |  Build (cold)    |
|      85.5%       |        3         |      12.3s       |
+------------------+------------------+------------------+
|  Build (hot)     |   Test Time      |                  |
|      2.1s        |      5.4s        |                  |
+------------------+------------------+------------------+

+------------------------------------------------+
| Metric              | Value                     |
|---------------------|---------------------------|
| coverage            | 85.5%                     |
| escapes.unsafe      | 3                         |
| build_time.cold     | 12.3s                     |
| ...                 | ...                       |
+------------------------------------------------+
```

Cards are color-coded by category:
- `tests` - Green accent
- `escapes` - Amber accent
- `build` - Purple accent

### Refactoring format_report

The existing `format_report()` function prints directly. Refactor to return strings:

```rust
// Before (Phase 1305)
pub fn format_report(args: &ReportArgs, baseline: &Baseline) -> anyhow::Result<()> {
    match args.output {
        OutputFormat::Text => format_text_report(args, baseline),
        OutputFormat::Json => format_json_report(args, baseline),
    }
}

// After (Phase 1310)
pub fn format_report_to_string(
    args: &ReportArgs,
    baseline: &Baseline,
    format: OutputFormat,
) -> anyhow::Result<String> {
    match format {
        OutputFormat::Text => format_text_report(args, baseline),
        OutputFormat::Json => format_json_report(args, baseline),
        OutputFormat::Html => format_html_report(args, baseline),
    }
}

// Update internal functions to return String instead of printing
fn format_text_report(args: &ReportArgs, baseline: &Baseline) -> anyhow::Result<String> {
    let mut output = String::new();
    writeln!(output, "Quench Report")?;
    // ... rest of formatting ...
    Ok(output)
}
```

### CI Integration Example

After this phase, CI pipelines can generate reports:

```yaml
- name: Generate reports
  run: |
    quench check --ci --save .quench/baseline.json
    quench report -o docs/reports/latest.json
    quench report -o docs/reports/index.html
```

## Verification Plan

### Unit Tests

No additional unit tests needed; behavior is covered by spec tests.

### Spec Tests

```bash
# Run all report specs
cargo test --test specs report

# Should pass:
# - report_reads_baseline_file
# - report_without_baseline_shows_message
# - report_default_format_is_text
# - report_text_shows_summary
# - report_text_shows_baseline_info
# - report_json_outputs_metrics
# - report_json_includes_metadata
# - report_json_no_baseline_empty_metrics
# - report_html_produces_valid_html       # NEW
# - report_html_includes_metrics          # NEW
# - report_writes_to_file                 # NEW
```

### Manual Testing

```bash
# HTML to stdout
quench report -o html | head -20
# Expected: <!DOCTYPE html> ... metric cards ...

# HTML to file
cd tests/fixtures/report/with-baseline
quench report -o /tmp/report.html
open /tmp/report.html
# Expected: Styled dashboard with coverage 85.5%, escapes.unsafe 3

# JSON to file
quench report -o /tmp/metrics.json
cat /tmp/metrics.json | jq .metrics
# Expected: {"coverage": {"total": 85.5}, "escapes": {...}}

# Verify file contains correct data
grep "85.5" /tmp/report.html && echo "Coverage found"
grep "coverage" /tmp/metrics.json && echo "JSON has coverage"
```

### Full Suite

```bash
make check
```

## Checklist

- [ ] Add `Html` variant to `OutputFormat` enum in `cli.rs`
- [ ] Change `ReportArgs.output` from `OutputFormat` to `String`
- [ ] Add `output_target()` method for format/file detection
- [ ] Refactor `format_text_report` to return `String`
- [ ] Refactor `format_json_report` to return `String`
- [ ] Implement `format_html_report` with cards and table
- [ ] Add `html_template` function for HTML structure
- [ ] Add `metric_card` and `table_row` helpers
- [ ] Add `human_bytes` helper for binary sizes
- [ ] Update `run_report` to handle file output
- [ ] Remove `#[ignore]` from `report_html_produces_valid_html`
- [ ] Remove `#[ignore]` from `report_html_includes_metrics`
- [ ] Remove `#[ignore]` from `report_writes_to_file`
- [ ] Run `cargo test --test specs report` to verify
- [ ] Run `make check` to verify no regressions
- [ ] Manual test HTML rendering in browser
