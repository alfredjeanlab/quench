# Checkpoint 16B: Report Command Complete - Validation

**Plan:** `checkpoint-16b-validate`
**Root Feature:** `quench-report`
**Depends On:** `checkpoint-16a-precheck` (Report Command Implementation)

## Overview

Validation checkpoint to confirm the `quench report` command meets all criteria. This plan runs manual and automated tests to verify text, JSON, and HTML output formats work correctly, then documents results in `reports/checkpoint-16-report-command.md`.

**Checkpoint Criteria:**
- [ ] `quench report` produces readable summary
- [ ] `quench report -o json` produces valid JSON
- [ ] `quench report -o html` produces valid HTML

## Project Structure

```
tests/
├── fixtures/report/
│   ├── with-baseline/           # Fixture with .quench/baseline.json
│   │   ├── quench.toml
│   │   ├── .quench/baseline.json
│   │   └── CLAUDE.md
│   └── no-baseline/             # Fixture without baseline
│       ├── quench.toml
│       └── CLAUDE.md
└── specs/cli/
    └── report.rs                # 11 behavioral specs

crates/cli/src/
├── cli.rs                       # ReportArgs, OutputFormat enum
├── main.rs                      # run_report() entry point
└── report.rs                    # format_text_report, format_json_report, format_html_report

reports/
└── checkpoint-16-report-command.md  # Validation report (to create)
```

## Dependencies

- Built quench binary (`cargo build`)
- Test fixtures (already present in `tests/fixtures/report/`)
- `jq` for JSON validation (optional)

## Implementation Phases

### Phase 1: Run Automated Test Suite

Run all behavioral specs for the report command.

```bash
cargo test --test specs report -- --nocapture
```

**Expected Results:**
- 11 behavioral specs pass:
  - `report_reads_baseline_file`
  - `report_without_baseline_shows_message`
  - `report_default_format_is_text`
  - `report_text_shows_summary`
  - `report_text_shows_baseline_info`
  - `report_json_outputs_metrics`
  - `report_json_includes_metadata`
  - `report_json_no_baseline_empty_metrics`
  - `report_html_produces_valid_html`
  - `report_html_includes_metrics`
  - `report_writes_to_file`

**Verification:**
- [ ] All 11 specs pass
- [ ] No ignored specs remaining for report command
- [ ] No test failures or panics

---

### Phase 2: Validate Text Format

Manually verify `quench report` produces readable text output.

```bash
cd tests/fixtures/report/with-baseline
cargo run -p quench -- report
```

**Expected Output:**
```
Quench Report
=============
Baseline: abc1234 (2026-01-20)

coverage: 85.5%
escapes.unsafe: 3
escapes.unwrap: 0
build_time.cold: 5.2s
build_time.hot: 1.1s
test_time.total: 3.5s
test_time.avg: 0.1s
test_time.max: 2.0s
binary_size.quench: 12.3 MB
```

**Verification Checklist:**
- [ ] Header shows "Quench Report"
- [ ] Baseline commit hash and date displayed
- [ ] Metrics show with readable names and values
- [ ] Percentages formatted with `%` suffix
- [ ] Times formatted with `s` suffix
- [ ] Sizes formatted as human-readable (KB/MB)

---

### Phase 3: Validate JSON Format

Manually verify `quench report -o json` produces valid, parseable JSON.

```bash
cd tests/fixtures/report/with-baseline
cargo run -p quench -- report -o json | jq .
```

**Expected Structure:**
```json
{
  "updated": "2026-01-20T12:00:00Z",
  "commit": "abc1234",
  "metrics": {
    "coverage": { "total": 85.5 },
    "escapes": { "source": { "unsafe": 3, "unwrap": 0 } },
    "build_time": { "cold": 5.2, "hot": 1.1 },
    "test_time": { "total": 3.5, "avg": 0.1, "max": 2.0 },
    "binary_size": { "quench": 12345678 }
  }
}
```

**Verification Checklist:**
- [ ] Output parses as valid JSON (jq exits 0)
- [ ] `updated` field present with ISO 8601 timestamp
- [ ] `commit` field present with commit hash
- [ ] `metrics` object contains all expected sections
- [ ] Numeric values are proper JSON numbers (not strings)

---

### Phase 4: Validate HTML Format

Manually verify `quench report -o html` produces valid HTML document.

```bash
cd tests/fixtures/report/with-baseline
cargo run -p quench -- report -o html > /tmp/report.html
```

**Verification Checklist:**
- [ ] Starts with `<!DOCTYPE html>`
- [ ] Contains `<html>` and `</html>` tags
- [ ] Contains `<head>` with `<style>` for CSS
- [ ] Contains `<body>` with metric cards
- [ ] Metric values (85.5%, etc.) appear in content
- [ ] Opens correctly in browser (manual visual check)

**HTML Structure Validation:**
```bash
# Check document structure
grep -q '<!DOCTYPE html>' /tmp/report.html && echo "DOCTYPE: OK"
grep -q '<html' /tmp/report.html && echo "<html>: OK"
grep -q '</html>' /tmp/report.html && echo "</html>: OK"
grep -q '<style>' /tmp/report.html && echo "CSS: OK"
grep -q '85.5' /tmp/report.html && echo "Metrics: OK"
```

---

### Phase 5: Validate File Output

Verify `-o filename.ext` writes to file with correct format detection.

```bash
cd tests/fixtures/report/with-baseline

# HTML file output
cargo run -p quench -- report -o /tmp/metrics.html
file /tmp/metrics.html  # Should identify as HTML

# JSON file output
cargo run -p quench -- report -o /tmp/metrics.json
file /tmp/metrics.json  # Should identify as JSON
jq . /tmp/metrics.json  # Should parse

# Text file output
cargo run -p quench -- report -o /tmp/metrics.txt
head /tmp/metrics.txt   # Should show text format
```

**Verification Checklist:**
- [ ] `.html` extension produces HTML content
- [ ] `.json` extension produces JSON content
- [ ] `.txt` extension produces text content
- [ ] Files are created at specified paths
- [ ] No stdout output when writing to file

---

### Phase 6: Run Full Test Suite

Run the complete `make check` to verify no regressions.

```bash
make check
```

**Expected:**
- `cargo fmt --all -- --check` passes
- `cargo clippy --all-targets --all-features -- -D warnings` passes
- `cargo test --all` passes
- `cargo build --all` succeeds
- `cargo audit` passes
- `cargo deny check` passes

**Verification:**
- [ ] All make check steps pass
- [ ] No warnings or errors

---

### Phase 7: Create Validation Report

Create `reports/checkpoint-16-report-command.md` with validation results.

**Report Template:**
```markdown
# Checkpoint 16: Report Command Complete - Validation Report

Generated: [DATE]

## Summary

| Criterion | Status | Notes |
|-----------|--------|-------|
| `quench report` readable summary | ? | |
| `quench report -o json` valid JSON | ? | |
| `quench report -o html` valid HTML | ? | |

**Overall Status: ?**

## Detailed Results

### 1. Automated Tests
[cargo test output summary]

### 2. Text Format Validation
[Output sample and checklist results]

### 3. JSON Format Validation
[Output sample and jq validation]

### 4. HTML Format Validation
[Structure checks and browser test]

### 5. File Output Validation
[File creation and format detection tests]

## Conclusion
[Summary of report command status]
```

## Key Implementation Details

### Output Format Detection

The `-o` flag accepts format names or file paths:

| Input | Format | Destination |
|-------|--------|-------------|
| `text` | Text | stdout |
| `json` | JSON | stdout |
| `html` | HTML | stdout |
| `report.html` | HTML | file |
| `metrics.json` | JSON | file |
| `output.txt` | Text | file |

### Baseline Location

Reports read from `.quench/baseline.json` which is created by `quench baseline` command.

### Metric Categories

Metrics are organized by category for display:
- **Coverage**: test coverage percentage
- **Escapes**: escape hatch counts by pattern
- **Build Time**: cold/hot build times
- **Test Time**: total/avg/max test times
- **Binary Size**: per-binary sizes

## Verification Plan

### Automated Verification
```bash
# All report specs
cargo test --test specs report

# Full test suite
make check
```

### Manual Verification
```bash
cd tests/fixtures/report/with-baseline

# Text format
cargo run -p quench -- report

# JSON format (pipe through jq to validate)
cargo run -p quench -- report -o json | jq .

# HTML format (open in browser)
cargo run -p quench -- report -o /tmp/report.html
open /tmp/report.html  # macOS
```

### Checkpoint Criteria Mapping

| Criterion | Automated | Manual |
|-----------|-----------|--------|
| Readable text summary | `report_text_shows_summary` | Phase 2 visual check |
| Valid JSON | `report_json_outputs_metrics` | Phase 3 jq validation |
| Valid HTML | `report_html_produces_valid_html` | Phase 4 browser test |

## Deliverables

1. **Validation Report:** `reports/checkpoint-16-report-command.md`
   - Document test results for all criteria
   - Include sample outputs
   - Note any gaps or issues found

2. **Archive Plan:** Move to `plans/archive/` after validation passes
