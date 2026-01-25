# Phase 1502: Close Review Gaps

**Root Feature:** `quench-review-gaps`
**Depends On:** Phase 1501 (Init Command Specs)

## Overview

Close the minor gaps identified during the comprehensive code review of Checkpoint 17H and related features. All gaps are low priority but should be addressed for completeness.

**Identified Gaps:**
1. Missing `scope` field in Violation struct for git check JSON output
2. Missing json_tests.rs unit tests for report module
3. Missing markdown format behavioral spec in report.rs

## Project Structure

```
quench/
├── crates/cli/src/
│   ├── check.rs                    # UPDATE: Add scope field to Violation
│   ├── checks/git/mod.rs           # UPDATE: Populate scope from parsed commit
│   └── report/
│       └── json_tests.rs           # CREATE: Unit tests for JSON formatter
└── tests/specs/cli/
    └── report.rs                   # UPDATE: Add markdown format specs
```

## Implementation Phases

### Phase 1: Add Scope Field to Violation Struct

**Goal:** Enable git check to report the commit scope in JSON output.

**File:** `crates/cli/src/check.rs`

**Current:**
```rust
pub struct Violation {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<PathBuf>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<u32>,

    #[serde(rename = "type")]
    pub violation_type: String,

    pub advice: String,
}
```

**After:**
```rust
pub struct Violation {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<PathBuf>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<u32>,

    /// Optional scope context (e.g., commit scope for git check).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,

    #[serde(rename = "type")]
    pub violation_type: String,

    pub advice: String,
}
```

**Note:** Update `Violation::new()` and builder methods to handle the new field with `None` as default.

**Verification:**
```bash
cargo test -p quench -- violation
cargo test -p quench -- git
```

---

### Phase 2: Populate Scope in Git Check

**Goal:** When git check reports a violation, include the parsed scope if available.

**File:** `crates/cli/src/checks/git/mod.rs`

**Changes:**
1. When creating violations in `validate_commit()`, extract scope from `ParsedCommit`
2. Set `scope: Some(parsed.scope.clone())` when scope is present

**Example:**
```rust
// In validate_commit function
let violation = Violation {
    file: None,
    line: None,
    scope: parsed.scope.clone(), // Add this
    violation_type: "invalid_commit_format".to_string(),
    advice: format!("..."),
};
```

**Verification:**
```bash
cargo test -p quench -- git
# Manual test:
echo "feat(api): add endpoint" | cargo run -- check-git-message - --format json | jq .
```

---

### Phase 3: Add JSON Formatter Unit Tests

**Goal:** Add unit tests for the JSON report formatter to match other formatters.

**File:** `crates/cli/src/report/json_tests.rs`

```rust
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use crate::check::{CheckResult, Violation};

#[test]
fn formats_empty_results() {
    let mut output = Vec::new();
    let formatter = JsonFormatter::default();

    formatter.format(&[], &mut output).unwrap();

    let json: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert!(json["checks"].as_array().unwrap().is_empty());
}

#[test]
fn formats_passing_check() {
    let mut output = Vec::new();
    let formatter = JsonFormatter::default();
    let results = vec![CheckResult::passed("cloc")];

    formatter.format(&results, &mut output).unwrap();

    let json: serde_json::Value = serde_json::from_slice(&output).unwrap();
    let checks = json["checks"].as_array().unwrap();
    assert_eq!(checks.len(), 1);
    assert_eq!(checks[0]["name"], "cloc");
    assert_eq!(checks[0]["status"], "passed");
}

#[test]
fn formats_failed_check_with_violations() {
    let mut output = Vec::new();
    let formatter = JsonFormatter::default();
    let violations = vec![Violation::new(
        Some("src/lib.rs".into()),
        Some(10),
        "file_too_large",
        "Split into smaller modules",
    )];
    let results = vec![CheckResult::failed("cloc", violations)];

    formatter.format(&results, &mut output).unwrap();

    let json: serde_json::Value = serde_json::from_slice(&output).unwrap();
    let checks = json["checks"].as_array().unwrap();
    assert_eq!(checks[0]["violations"].as_array().unwrap().len(), 1);
}

#[test]
fn compact_mode_produces_single_line() {
    let mut output = Vec::new();
    let formatter = JsonFormatter::new(true); // compact = true
    let results = vec![CheckResult::passed("cloc")];

    formatter.format(&results, &mut output).unwrap();

    let output_str = String::from_utf8(output).unwrap();
    assert!(!output_str.contains('\n') || output_str.trim().lines().count() == 1);
}

#[test]
fn includes_timing_when_provided() {
    let mut output = Vec::new();
    let mut formatter = JsonFormatter::default();
    // Set timing info if available

    formatter.format(&[], &mut output).unwrap();

    // Verify timing structure when provided
}
```

**Register module in json.rs:**
```rust
#[cfg(test)]
#[path = "json_tests.rs"]
mod tests;
```

**Verification:**
```bash
cargo test -p quench -- json
```

---

### Phase 4: Add Markdown Format Behavioral Specs

**Goal:** Add behavioral specs for markdown report format to match other formats.

**File:** `tests/specs/cli/report.rs`

Add to the end of the file after HTML section:

```rust
// =============================================================================
// MARKDOWN FORMAT
// =============================================================================

/// Spec: docs/specs/01-cli.md#quench-report
///
/// > Markdown format produces valid markdown table
#[test]
fn report_markdown_produces_table() {
    report()
        .on("report/with-baseline")
        .markdown()
        .runs()
        .stdout_has("| Metric |")
        .stdout_has("|---|");
}

/// Spec: docs/specs/01-cli.md#quench-report
///
/// > Markdown format includes metrics data
#[test]
fn report_markdown_includes_metrics() {
    report()
        .on("report/with-baseline")
        .markdown()
        .runs()
        .stdout_has("coverage")
        .stdout_has("85.5");
}

/// Spec: docs/specs/01-cli.md#quench-report
///
/// > Markdown format with no baseline shows message
#[test]
fn report_markdown_no_baseline_message() {
    report()
        .on("report/no-baseline")
        .markdown()
        .runs()
        .stdout_has("No baseline");
}
```

**Note:** May need to add `.markdown()` helper to prelude if not present:
```rust
pub fn markdown(self) -> Self {
    self.args(["--format", "markdown"])
}
```

**Verification:**
```bash
cargo test --test specs report_markdown
```

---

### Phase 5: Final Verification

**Goal:** Ensure all changes pass tests and don't introduce regressions.

**Steps:**
```bash
# Unit tests
cargo test -p quench

# Behavioral specs
cargo test --test specs

# Full CI check
make check

# Manual verification
cargo run -- check --format json | jq '.checks[].violations[].scope'
```

## Key Implementation Details

### Scope Field Design

The `scope` field is optional and only populated when relevant:
- Git check: Populated from parsed commit scope (e.g., "api" from "feat(api): ...")
- Other checks: Left as `None`

JSON output example:
```json
{
  "checks": [{
    "name": "git",
    "violations": [{
      "type": "invalid_commit_format",
      "scope": "api",
      "advice": "..."
    }]
  }]
}
```

### Backwards Compatibility

- `scope` field uses `skip_serializing_if = "Option::is_none"`
- Existing JSON consumers won't see the field unless it has a value
- No breaking changes to existing output format

## Verification Plan

| Phase | Command | Expected Result |
|-------|---------|-----------------|
| 1 | `cargo test -p quench -- violation` | Tests pass |
| 2 | `cargo test -p quench -- git` | Tests pass |
| 3 | `cargo test -p quench -- json` | New tests pass |
| 4 | `cargo test --test specs report_markdown` | Specs pass |
| 5 | `make check` | All quality gates pass |

## Exit Criteria

- [ ] Violation struct has optional `scope` field
- [ ] Git check populates scope from parsed commit
- [ ] json_tests.rs exists with formatter unit tests
- [ ] Markdown format behavioral specs exist and pass
- [ ] `make check` passes
- [ ] No regressions in existing tests

## Estimated Scope

- ~30 lines changed in check.rs
- ~20 lines changed in git/mod.rs
- ~80 lines new in json_tests.rs
- ~40 lines new in report.rs specs
- Low risk, additive changes only
