# Phase 1305: Report Command - Basic

**Root Feature:** `quench-1305`

## Overview

Implement the basic `quench report` command that reads baseline files and outputs metrics in text format. This phase establishes the core report infrastructure with check toggle flags for filtering which metrics to display.

The report command provides a quick view of stored quality metrics from `.quench/baseline.json`, complementing the `check` command which runs live checks.

## Project Structure

```
crates/cli/src/
├── cli.rs              # Extend ReportArgs with check toggle flags
├── main.rs             # Implement run_report()
└── report/             # NEW: Report module (optional, may inline in main.rs)
    └── mod.rs          # Report formatting logic

tests/specs/cli/
└── report.rs           # Update #[ignore] tags for passing specs
```

Key files to modify:
- `crates/cli/src/cli.rs` - Add check toggle flags to `ReportArgs`
- `crates/cli/src/main.rs` - Implement `run_report()` function
- `tests/specs/cli/report.rs` - Remove `#[ignore]` from passing specs

Existing infrastructure to reuse:
- `baseline.rs` - `Baseline::load()` for reading baseline files
- `cli.rs` - `collect_checks!` macro pattern for toggle flags
- `tests/specs/prelude.rs` - `ReportBuilder` already implemented

## Dependencies

No new external dependencies required. Uses existing:
- `chrono` for timestamp formatting
- `serde_json` for baseline parsing
- `anyhow` for error handling

## Implementation Phases

### Phase 1: Extend ReportArgs with Check Toggles

Add check enable/disable flags to `ReportArgs` following the `CheckArgs` pattern.

**File:** `crates/cli/src/cli.rs`

```rust
#[derive(clap::Args)]
pub struct ReportArgs {
    /// Output format
    #[arg(short, long, default_value = "text")]
    pub output: OutputFormat,

    // Check enable flags (show only these metrics)
    /// Show only cloc metrics
    #[arg(long)]
    pub cloc: bool,

    /// Show only escapes metrics
    #[arg(long)]
    pub escapes: bool,

    /// Show only agents metrics
    #[arg(long)]
    pub agents: bool,

    /// Show only docs metrics
    #[arg(long)]
    pub docs: bool,

    /// Show only tests metrics
    #[arg(long = "tests")]
    pub tests_check: bool,

    /// Show only git metrics
    #[arg(long)]
    pub git: bool,

    /// Show only build metrics
    #[arg(long)]
    pub build: bool,

    /// Show only license metrics
    #[arg(long)]
    pub license: bool,

    /// Show only placeholders metrics
    #[arg(long)]
    pub placeholders: bool,

    // Check disable flags (skip these metrics)
    /// Skip cloc metrics
    #[arg(long)]
    pub no_cloc: bool,

    /// Skip escapes metrics
    #[arg(long)]
    pub no_escapes: bool,

    /// Skip agents metrics
    #[arg(long)]
    pub no_agents: bool,

    /// Skip docs metrics
    #[arg(long)]
    pub no_docs: bool,

    /// Skip tests metrics
    #[arg(long)]
    pub no_tests: bool,

    /// Skip git metrics
    #[arg(long)]
    pub no_git: bool,

    /// Skip build metrics
    #[arg(long)]
    pub no_build: bool,

    /// Skip license metrics
    #[arg(long)]
    pub no_license: bool,

    /// Skip placeholders metrics
    #[arg(long)]
    pub no_placeholders: bool,
}

impl ReportArgs {
    pub fn enabled_checks(&self) -> Vec<String> {
        collect_checks!(self,
            cloc => "cloc",
            escapes => "escapes",
            agents => "agents",
            docs => "docs",
            tests_check => "tests",
            git => "git",
            build => "build",
            license => "license",
            placeholders => "placeholders",
        )
    }

    pub fn disabled_checks(&self) -> Vec<String> {
        collect_checks!(self,
            no_cloc => "cloc",
            no_escapes => "escapes",
            no_agents => "agents",
            no_docs => "docs",
            no_tests => "tests",
            no_git => "git",
            no_build => "build",
            no_license => "license",
            no_placeholders => "placeholders",
        )
    }
}
```

**Milestone:** `cargo build` succeeds with new ReportArgs flags.

### Phase 2: Implement Baseline Loading in run_report

Load baseline from the configured path (default `.quench/baseline.json`).

**File:** `crates/cli/src/main.rs`

```rust
use quench::baseline::Baseline;
use quench::config::Config;
use quench::discovery;

fn run_report(cli: &Cli, args: &ReportArgs) -> anyhow::Result<()> {
    // 1. Find and load config
    let cwd = std::env::current_dir()?;
    let (config, _warnings) = if let Some(ref path) = cli.config {
        quench::config::load_with_warnings(path)?
    } else if let Some(path) = discovery::find_config(&cwd)? {
        quench::config::load_with_warnings(&path)?
    } else {
        (Config::default(), vec![])
    };

    // 2. Determine baseline path
    let baseline_path = cwd.join(&config.git.baseline);

    // 3. Load baseline
    let baseline = Baseline::load(&baseline_path)?;

    match baseline {
        Some(baseline) => format_report(args, &baseline),
        None => {
            match args.output {
                OutputFormat::Text => println!("No baseline found."),
                OutputFormat::Json => println!(r#"{{"metrics": {{}}}}"#),
            }
            Ok(())
        }
    }
}
```

**Milestone:** `quench report` loads baseline and shows "No baseline found" when missing.

### Phase 3: Implement Text Format Output

Format baseline metrics as human-readable text output.

**File:** `crates/cli/src/main.rs`

```rust
fn format_report(args: &ReportArgs, baseline: &Baseline) -> anyhow::Result<()> {
    match args.output {
        OutputFormat::Text => format_text_report(args, baseline),
        OutputFormat::Json => format_json_report(args, baseline),
    }
}

fn format_text_report(args: &ReportArgs, baseline: &Baseline) -> anyhow::Result<()> {
    let enabled = args.enabled_checks();
    let disabled = args.disabled_checks();

    // Header with baseline info
    println!("Quench Report");
    println!("=============");
    if let Some(ref commit) = baseline.commit {
        let date = baseline.updated.format("%Y-%m-%d");
        println!("Baseline: {} ({})", commit, date);
    } else {
        let date = baseline.updated.format("%Y-%m-%d");
        println!("Baseline: {}", date);
    }
    println!();

    // Filter and display metrics
    let metrics = &baseline.metrics;

    // Coverage
    if should_show("coverage", &enabled, &disabled) {
        if let Some(ref coverage) = metrics.coverage {
            println!("coverage: {:.1}%", coverage.total);
        }
    }

    // Escapes
    if should_show("escapes", &enabled, &disabled) {
        if let Some(ref escapes) = metrics.escapes {
            for (name, count) in &escapes.source {
                println!("escapes.{}: {}", name, count);
            }
        }
    }

    // Build time
    if should_show("build", &enabled, &disabled) {
        if let Some(ref build) = metrics.build_time {
            println!("build_time.cold: {:.1}s", build.cold);
            println!("build_time.hot: {:.1}s", build.hot);
        }
    }

    // Test time
    if should_show("tests", &enabled, &disabled) {
        if let Some(ref tests) = metrics.test_time {
            println!("test_time.total: {:.1}s", tests.total);
        }
    }

    // Binary size
    if should_show("build", &enabled, &disabled) {
        if let Some(ref sizes) = metrics.binary_size {
            for (name, size) in sizes {
                println!("binary_size.{}: {} bytes", name, size);
            }
        }
    }

    Ok(())
}

fn should_show(metric: &str, enabled: &[String], disabled: &[String]) -> bool {
    if !enabled.is_empty() {
        // Explicit enable mode: only show specified metrics
        enabled.iter().any(|e| e == metric)
    } else {
        // Default mode: show all except disabled
        !disabled.iter().any(|d| d == metric)
    }
}
```

**Milestone:** `quench report` displays formatted text output with metrics.

### Phase 4: Implement JSON Format Output

Format baseline as JSON output (mirrors baseline structure).

**File:** `crates/cli/src/main.rs`

```rust
fn format_json_report(args: &ReportArgs, baseline: &Baseline) -> anyhow::Result<()> {
    use serde_json::json;

    let enabled = args.enabled_checks();
    let disabled = args.disabled_checks();
    let metrics = &baseline.metrics;

    let mut output = serde_json::Map::new();

    // Metadata
    output.insert("updated".to_string(), json!(baseline.updated.to_rfc3339()));
    if let Some(ref commit) = baseline.commit {
        output.insert("commit".to_string(), json!(commit));
    }

    // Filtered metrics
    let mut filtered_metrics = serde_json::Map::new();

    if should_show("coverage", &enabled, &disabled) {
        if let Some(ref coverage) = metrics.coverage {
            filtered_metrics.insert("coverage".to_string(), json!({ "total": coverage.total }));
        }
    }

    if should_show("escapes", &enabled, &disabled) {
        if let Some(ref escapes) = metrics.escapes {
            filtered_metrics.insert("escapes".to_string(), json!({ "source": escapes.source }));
        }
    }

    if should_show("build", &enabled, &disabled) {
        if let Some(ref build) = metrics.build_time {
            filtered_metrics.insert("build_time".to_string(), json!({
                "cold": build.cold,
                "hot": build.hot,
            }));
        }
        if let Some(ref sizes) = metrics.binary_size {
            filtered_metrics.insert("binary_size".to_string(), json!(sizes));
        }
    }

    if should_show("tests", &enabled, &disabled) {
        if let Some(ref tests) = metrics.test_time {
            filtered_metrics.insert("test_time".to_string(), json!({
                "total": tests.total,
                "avg": tests.avg,
                "max": tests.max,
            }));
        }
    }

    output.insert("metrics".to_string(), serde_json::Value::Object(filtered_metrics));

    println!("{}", serde_json::to_string_pretty(&serde_json::Value::Object(output))?);
    Ok(())
}
```

**Milestone:** `quench report -o json` outputs valid JSON.

### Phase 5: Update Specs and Verify

Remove `#[ignore]` tags from specs that pass with this implementation.

**File:** `tests/specs/cli/report.rs`

Specs to enable (remove `#[ignore]`):
- `report_reads_baseline_file` - Baseline reading works
- `report_without_baseline_shows_message` - No baseline message
- `report_default_format_is_text` - Text is default
- `report_text_shows_summary` - Metrics in text output
- `report_text_shows_baseline_info` - Commit/timestamp display
- `report_json_outputs_metrics` - JSON metrics output
- `report_json_includes_metadata` - JSON metadata
- `report_json_no_baseline_empty_metrics` - Empty JSON for missing baseline

Specs to keep ignored (future phases):
- `report_html_produces_valid_html` - HTML format (Phase 1306)
- `report_html_includes_metrics` - HTML format (Phase 1306)
- `report_writes_to_file` - File output (Phase 1306)

**Milestone:** `cargo test --test specs report` passes for enabled specs.

## Key Implementation Details

### Metric-to-Check Mapping

Metrics in baseline map to checks as follows:

| Metric | Check | Notes |
|--------|-------|-------|
| `coverage` | tests | Test coverage percentage |
| `escapes` | escapes | Escape hatch counts |
| `build_time` | build | Build timing metrics |
| `test_time` | tests | Test execution time |
| `binary_size` | build | Binary/bundle sizes |

### Text Output Format

Per `docs/specs/01-cli.md`, text output format:

```
Quench Report
=============
Baseline: abc1234 (2026-01-20)

coverage: 85.5%
escapes.unsafe: 3
escapes.unwrap: 0
```

Key decisions:
- Flat key format (`escapes.unsafe` not nested)
- Percentages with one decimal
- Commit hash abbreviated (7 chars)
- Date in YYYY-MM-DD format

### JSON Output Format

JSON output mirrors the baseline structure:

```json
{
  "updated": "2026-01-20T12:00:00Z",
  "commit": "abc1234",
  "metrics": {
    "coverage": { "total": 85.5 },
    "escapes": { "source": { "unsafe": 3, "unwrap": 0 } }
  }
}
```

### Check Toggle Logic

The `should_show` function follows the same pattern as `check` command:

1. If any enable flag is set (`--escapes`), only show those metrics
2. Otherwise, show all metrics except those with disable flags (`--no-escapes`)
3. Default: show all metrics

## Verification Plan

### Unit Tests

Test toggle logic in `cli_tests.rs`:

```rust
#[test]
fn report_args_enabled_checks() {
    let args = ReportArgs {
        escapes: true,
        ..Default::default()
    };
    assert_eq!(args.enabled_checks(), vec!["escapes"]);
}

#[test]
fn report_args_disabled_checks() {
    let args = ReportArgs {
        no_escapes: true,
        no_docs: true,
        ..Default::default()
    };
    assert_eq!(args.disabled_checks(), vec!["escapes", "docs"]);
}
```

### Spec Tests

Verify behavioral specs pass:

```bash
# Run report specs
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
```

### Manual Testing

```bash
# No baseline
cd /tmp && mkdir test && cd test
quench init
quench report
# Expected: "No baseline found."

# With baseline
cd tests/fixtures/report/with-baseline
quench report
# Expected: Formatted text output with metrics

# JSON format
quench report -o json
# Expected: Valid JSON with metrics

# Filtered output
quench report --escapes
# Expected: Only escapes metrics

quench report --no-coverage
# Expected: All metrics except coverage
```

### Full Suite

```bash
make check
```

## Checklist

- [ ] Add check toggle flags to `ReportArgs` in `cli.rs`
- [ ] Add `enabled_checks()` and `disabled_checks()` methods to `ReportArgs`
- [ ] Implement `run_report()` with baseline loading
- [ ] Implement `format_text_report()` for text output
- [ ] Implement `format_json_report()` for JSON output
- [ ] Add `should_show()` helper for metric filtering
- [ ] Add unit tests for `ReportArgs` toggle methods
- [ ] Remove `#[ignore]` from passing specs in `report.rs`
- [ ] Keep `#[ignore]` on HTML and file output specs
- [ ] Run `cargo test --test specs report` to verify
- [ ] Run `make check` to verify no regressions
