# Checkpoint 16A: Pre-Checkpoint Fix - Report Command Complete

**Plan:** `checkpoint-16a-precheck`
**Root Feature:** `quench-report`
**Depends On:** Phase 1310 (Report Command - Formats)

## Overview

Verify the `quench report` command is complete and all behavioral specs pass. Phase 1310 added HTML format output and file output support to the report command. This checkpoint validates the implementation against all specs before proceeding to subsequent features.

**Current State:**
- Core implementation: ✅ Complete (`crates/cli/src/report.rs`)
- CLI arguments: ✅ Complete (`crates/cli/src/cli.rs`)
- Entry point: ✅ Complete (`crates/cli/src/main.rs`)
- Behavioral specs: ✅ All 21 specs passing
- Fixtures: ✅ Exist (`tests/fixtures/report/`)

**Goal:** Confirm all Phase 1310 deliverables are complete and passing.

## Project Structure

```
crates/cli/src/
├── cli.rs              # ReportArgs, OutputFormat enum, output_target()
├── main.rs             # run_report() with file output routing
└── report.rs           # format_text_report, format_json_report, format_html_report

tests/specs/cli/
└── report.rs           # 21 behavioral specs (all passing)

tests/fixtures/report/
├── with-baseline/      # Fixture with .quench/baseline.json
│   ├── quench.toml
│   ├── .quench/baseline.json
│   └── CLAUDE.md
└── no-baseline/        # Fixture without baseline
    ├── quench.toml
    └── CLAUDE.md
```

## Dependencies

No external dependencies beyond existing crate dependencies:
- `serde_json` for JSON output
- `chrono` for date formatting
- `clap` for CLI argument parsing

## Implementation Phases

### Phase 1: Verify CLI Arguments

**Goal:** Confirm `ReportArgs` correctly handles all output modes.

**Implementation checklist:**
- [x] `output: String` field accepts format names or file paths
- [x] `output_target()` method returns `(OutputFormat, Option<PathBuf>)`
- [x] Format detection works for `.html`, `.json`, `.txt`, `.md` extensions
- [x] Check toggle flags: `--[no-]cloc`, `--[no-]escapes`, etc.

**Key code:** `crates/cli/src/cli.rs:303-320`

```rust
pub fn output_target(&self) -> (OutputFormat, Option<PathBuf>) {
    let val = self.output.to_lowercase();
    if val.ends_with(".html") {
        (OutputFormat::Html, Some(PathBuf::from(&self.output)))
    } else if val.ends_with(".json") {
        (OutputFormat::Json, Some(PathBuf::from(&self.output)))
    } else if val.ends_with(".txt") || val.ends_with(".md") {
        (OutputFormat::Text, Some(PathBuf::from(&self.output)))
    } else {
        let format = match val.as_str() {
            "json" => OutputFormat::Json,
            "html" => OutputFormat::Html,
            _ => OutputFormat::Text,
        };
        (format, None)
    }
}
```

**Verification:**
```bash
cargo build && quench report --help
# Should show: -o, --output <FMT> with text/json/html options
```

---

### Phase 2: Verify Output Formats

**Goal:** Confirm all three output formats produce correct content.

**Text format (default):**
- [x] Shows baseline info (commit hash, date)
- [x] Displays filtered metrics based on flags
- [x] Clean, readable terminal output

**JSON format:**
- [x] Machine-readable structure
- [x] Includes `updated` (RFC3339) and `commit` metadata
- [x] Filters metrics based on enable/disable flags

**HTML format:**
- [x] Self-contained static HTML with embedded CSS
- [x] Responsive card-based dashboard
- [x] Color-coded metric cards by category
- [x] Summary table with all metrics

**Key code:** `crates/cli/src/report.rs`

| Format | Function | Lines |
|--------|----------|-------|
| Text | `format_text_report()` | 39-100 |
| JSON | `format_json_report()` | 102-169 |
| HTML | `format_html_report()` | 171-363 |

**Verification:**
```bash
cd tests/fixtures/report/with-baseline
quench report              # Text output
quench report -o json      # JSON output
quench report -o html      # HTML output
```

---

### Phase 3: Verify File Output

**Goal:** Confirm file output routing works correctly.

**Implementation checklist:**
- [x] `run_report()` detects file path from `output_target()`
- [x] Writes output to file using `std::fs::write()`
- [x] Prints to stdout when no file path specified

**Key code:** `crates/cli/src/main.rs:537-594`

**Verification:**
```bash
cd tests/fixtures/report/with-baseline
quench report -o /tmp/report.html && cat /tmp/report.html | head -5
quench report -o /tmp/metrics.json && cat /tmp/metrics.json | head -5
```

---

### Phase 4: Verify Behavioral Specs

**Goal:** Confirm all 21 report specs pass.

**Spec categories:**

| Category | Count | Status |
|----------|-------|--------|
| Baseline reading | 2 | ✅ Pass |
| Text format | 3 | ✅ Pass |
| JSON format | 3 | ✅ Pass |
| HTML format | 2 | ✅ Pass |
| File output | 1 | ✅ Pass |
| Check filtering | 10 | ✅ Pass |

**Specs in `tests/specs/cli/report.rs`:**
1. `report_reads_baseline_file`
2. `report_without_baseline_shows_message`
3. `report_default_format_is_text`
4. `report_text_shows_summary`
5. `report_text_shows_baseline_info`
6. `report_json_outputs_metrics`
7. `report_json_includes_metadata`
8. `report_json_no_baseline_empty_metrics`
9. `report_html_produces_valid_html`
10. `report_html_includes_metrics`
11. `report_writes_to_file`

**Verification:**
```bash
cargo test --test specs report -- --nocapture
```

---

### Phase 5: Full Integration Testing

**Goal:** Run complete test suite and verify no regressions.

**Actions:**
1. Run all report specs
2. Run full make check
3. Verify no ignored specs remain for Phase 1310

**Verification:**
```bash
# Report specs
cargo test --test specs report

# Full suite
make check

# Verify no remaining Phase 1310 ignores
grep -r "Phase 1310" tests/specs/
# Should return empty or only comments
```

---

### Phase 6: Final Verification and Documentation

**Goal:** Confirm completion and archive plan.

**Actions:**
1. Verify all checklist items are complete
2. Run final `make check`
3. Archive implementation plan

**Commit message template:**
```
feat(report): complete report command implementation

Verify all Phase 1310 deliverables for the report command:
- Text format: baseline info and filtered metrics
- JSON format: machine-readable with metadata
- HTML format: dashboard with cards and summary table
- File output: automatic format detection from extension

Passing specs (21 total):
- report_reads_baseline_file
- report_without_baseline_shows_message
- report_default_format_is_text
- report_text_shows_summary
- report_text_shows_baseline_info
- report_json_outputs_metrics
- report_json_includes_metadata
- report_json_no_baseline_empty_metrics
- report_html_produces_valid_html
- report_html_includes_metrics
- report_writes_to_file
```

## Key Implementation Details

### Output Format Detection Logic

The `-o` flag accepts either format names or file paths:

| Input | Format | Output |
|-------|--------|--------|
| `text` | Text | stdout |
| `json` | JSON | stdout |
| `html` | HTML | stdout |
| `report.html` | HTML | file |
| `metrics.json` | JSON | file |
| `output.txt` | Text | file |

### HTML Dashboard Structure

Cards are color-coded by category:
- **Tests** (green): Coverage, test time
- **Escapes** (amber): Escape hatch counts
- **Build** (purple): Build time, binary sizes

```
+------------------+------------------+------------------+
|    Coverage      |  Escapes: unsafe |  Build (cold)    |
|      85.5%       |        3         |      12.3s       |
+------------------+------------------+------------------+
```

### Metric Filtering

Check toggle flags control which metrics appear:
- `--no-tests` hides coverage and test time
- `--no-escapes` hides escape counts
- `--no-build` hides build time and binary sizes

## Verification Plan

### Behavioral Specs
```bash
cargo test --test specs report
# Expected: 21 specs pass, 0 ignored
```

### Manual Testing
```bash
# Text format
cd tests/fixtures/report/with-baseline
quench report
# Expected: "Quench Report" header with metrics

# JSON format
quench report -o json | jq .metrics
# Expected: {"coverage": {"total": 85.5}, ...}

# HTML format
quench report -o /tmp/report.html
open /tmp/report.html
# Expected: Styled dashboard with metric cards

# File output verification
quench report -o /tmp/test.json
test -f /tmp/test.json && echo "File created"
```

### Full Suite
```bash
make check
# Expected: All checks pass (fmt, clippy, test, build, audit, deny)
```

## Checklist

- [x] `OutputFormat::Html` variant in `cli.rs`
- [x] `ReportArgs.output` is `String` type
- [x] `output_target()` method for format/file detection
- [x] `format_text_report()` returns `String`
- [x] `format_json_report()` returns `String`
- [x] `format_html_report()` with cards and table
- [x] `html_template()` function for HTML structure
- [x] `metric_card()` and `table_row()` helpers
- [x] `human_bytes()` helper for binary sizes
- [x] `run_report()` handles file output
- [x] All 21 report specs passing
- [x] No remaining Phase 1310 ignores
- [x] `make check` passes
- [x] Plan archived after verification
