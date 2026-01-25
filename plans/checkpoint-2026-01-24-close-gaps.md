# Close Gaps: Test Coverage and API Completeness

**Root Feature:** `quench-test-coverage`
**Priority:** HIGH - Missing tests are not minor; they represent untested behavior

## Overview

Close all gaps identified in the comprehensive code review. Missing tests are treated as significant issues because:
1. **Untested code is unverified code** - Bugs can hide in untested paths
2. **Regression risk** - Future changes may break untested functionality
3. **Documentation gap** - Tests serve as executable documentation

## Identified Gaps

| Gap | Category | Impact | Priority |
|-----|----------|--------|----------|
| Missing `scope` field in Violation | API Completeness | Git check JSON lacks commit scope | HIGH |
| Missing json_tests.rs | Test Coverage | JSON formatter has no unit tests | HIGH |
| Missing markdown integration test | Test Coverage | Markdown format untested in specs | HIGH |

---

## Phase 1: Add Scope Field to Violation Struct

### 1.1 Update Violation Struct

**File:** `crates/cli/src/check.rs`

**Sync Risk:** HIGH - All violation creation sites must be updated
**Justification:** Enables git check to report commit scope in JSON output

**Current (lines 68-83):**
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

### 1.2 Update Violation Constructors

Update `Violation::new()` and any builder methods to accept optional scope:

```rust
impl Violation {
    pub fn new(
        file: Option<PathBuf>,
        line: Option<u32>,
        violation_type: impl Into<String>,
        advice: impl Into<String>,
    ) -> Self {
        Self {
            file,
            line,
            scope: None,  // Default to None
            violation_type: violation_type.into(),
            advice: advice.into(),
        }
    }

    /// Set scope for this violation.
    pub fn with_scope(mut self, scope: impl Into<String>) -> Self {
        self.scope = Some(scope.into());
        self
    }
}
```

### 1.3 Populate Scope in Git Check

**File:** `crates/cli/src/checks/git/mod.rs`

In `validate_commit()`, when creating violations for commits with scopes:

```rust
// When parsed commit has a scope
if let Some(ref scope) = parsed.scope {
    violation = violation.with_scope(scope.clone());
}
```

### 1.4 Add Unit Tests

**File:** `crates/cli/src/check_tests.rs` (or existing test file)

```rust
#[test]
fn violation_with_scope_serializes_correctly() {
    let violation = Violation::new(None, None, "test_type", "test advice")
        .with_scope("api");

    let json = serde_json::to_string(&violation).unwrap();
    assert!(json.contains("\"scope\":\"api\""));
}

#[test]
fn violation_without_scope_omits_field() {
    let violation = Violation::new(None, None, "test_type", "test advice");

    let json = serde_json::to_string(&violation).unwrap();
    assert!(!json.contains("scope"));
}
```

**Verification:**
```bash
cargo test -p quench -- violation
cargo test -p quench -- git
echo 'feat(api): add endpoint' | cargo run -- check-git-message - --format json | jq '.scope'
```

---

## Phase 2: Add JSON Formatter Unit Tests

### 2.1 Create json_tests.rs

**File:** `crates/cli/src/report/json_tests.rs`

**Justification:** Every other formatter has unit tests (text_tests.rs, html_tests.rs, markdown_tests.rs). JSON formatter is untested.

```rust
//! Unit tests for JSON report formatter.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use crate::check::{CheckResult, Violation};
use crate::report::test_support::create_test_baseline;

// =============================================================================
// BASIC FORMATTING
// =============================================================================

#[test]
fn formats_empty_results() {
    let formatter = JsonFormatter::default();
    let output = formatter.format(&[], &AllChecks).unwrap();

    let json: serde_json::Value = serde_json::from_str(&output).unwrap();
    assert!(json["checks"].as_array().unwrap().is_empty());
}

#[test]
fn formats_passing_check() {
    let formatter = JsonFormatter::default();
    let results = vec![CheckResult::passed("cloc")];
    let output = formatter.format(&results, &AllChecks).unwrap();

    let json: serde_json::Value = serde_json::from_str(&output).unwrap();
    let checks = json["checks"].as_array().unwrap();
    assert_eq!(checks.len(), 1);
    assert_eq!(checks[0]["name"], "cloc");
    assert_eq!(checks[0]["status"], "passed");
}

#[test]
fn formats_failed_check_with_violations() {
    let formatter = JsonFormatter::default();
    let violations = vec![Violation::new(
        Some("src/lib.rs".into()),
        Some(10),
        "file_too_large",
        "Split into smaller modules",
    )];
    let results = vec![CheckResult::failed("cloc", violations)];
    let output = formatter.format(&results, &AllChecks).unwrap();

    let json: serde_json::Value = serde_json::from_str(&output).unwrap();
    let checks = json["checks"].as_array().unwrap();
    assert_eq!(checks[0]["status"], "failed");

    let violations = checks[0]["violations"].as_array().unwrap();
    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0]["file"], "src/lib.rs");
    assert_eq!(violations[0]["line"], 10);
    assert_eq!(violations[0]["type"], "file_too_large");
}

#[test]
fn formats_skipped_check() {
    let formatter = JsonFormatter::default();
    let results = vec![CheckResult::skipped("git", "Not a git repository")];
    let output = formatter.format(&results, &AllChecks).unwrap();

    let json: serde_json::Value = serde_json::from_str(&output).unwrap();
    let checks = json["checks"].as_array().unwrap();
    assert_eq!(checks[0]["status"], "skipped");
    assert_eq!(checks[0]["reason"], "Not a git repository");
}

// =============================================================================
// COMPACT MODE
// =============================================================================

#[test]
fn compact_mode_produces_single_line() {
    let formatter = JsonFormatter::new(true); // compact = true
    let results = vec![CheckResult::passed("cloc")];
    let output = formatter.format(&results, &AllChecks).unwrap();

    // Compact JSON should not have embedded newlines (except possibly trailing)
    let trimmed = output.trim();
    assert!(!trimmed.contains('\n'), "Compact output should be single line");
}

#[test]
fn non_compact_mode_is_pretty_printed() {
    let formatter = JsonFormatter::new(false); // compact = false
    let results = vec![CheckResult::passed("cloc")];
    let output = formatter.format(&results, &AllChecks).unwrap();

    // Pretty-printed JSON should have newlines
    assert!(output.contains('\n'), "Non-compact output should be multi-line");
}

// =============================================================================
// STREAMING CONSISTENCY
// =============================================================================

#[test]
fn buffered_matches_streamed_output() {
    let formatter = JsonFormatter::default();
    let results = vec![
        CheckResult::passed("cloc"),
        CheckResult::failed("escapes", vec![
            Violation::new(Some("src/lib.rs".into()), Some(5), "unsafe", "Document safety"),
        ]),
    ];

    let buffered = formatter.format(&results, &AllChecks).unwrap();

    let mut streamed = Vec::new();
    formatter.format_to(&mut streamed, &results, &AllChecks).unwrap();
    let streamed_str = String::from_utf8(streamed).unwrap();

    assert_eq!(buffered, streamed_str, "Buffered and streamed output should match");
}

#[test]
fn empty_buffered_matches_streamed() {
    let formatter = JsonFormatter::default();
    let results: Vec<CheckResult> = vec![];

    let buffered = formatter.format(&results, &AllChecks).unwrap();

    let mut streamed = Vec::new();
    formatter.format_to(&mut streamed, &results, &AllChecks).unwrap();
    let streamed_str = String::from_utf8(streamed).unwrap();

    assert_eq!(buffered, streamed_str);
}

// =============================================================================
// VIOLATION DETAILS
// =============================================================================

#[test]
fn violation_with_scope_included_in_json() {
    let formatter = JsonFormatter::default();
    let violation = Violation::new(None, None, "invalid_format", "Use conventional commits")
        .with_scope("api");
    let results = vec![CheckResult::failed("git", vec![violation])];
    let output = formatter.format(&results, &AllChecks).unwrap();

    let json: serde_json::Value = serde_json::from_str(&output).unwrap();
    let violations = json["checks"][0]["violations"].as_array().unwrap();
    assert_eq!(violations[0]["scope"], "api");
}

#[test]
fn violation_without_optional_fields_omits_them() {
    let formatter = JsonFormatter::default();
    let violation = Violation::new(None, None, "missing_docs", "Add documentation");
    let results = vec![CheckResult::failed("docs", vec![violation])];
    let output = formatter.format(&results, &AllChecks).unwrap();

    let json: serde_json::Value = serde_json::from_str(&output).unwrap();
    let violation = &json["checks"][0]["violations"][0];

    // Optional fields should be absent, not null
    assert!(violation.get("file").is_none() || violation["file"].is_null());
    assert!(violation.get("line").is_none() || violation["line"].is_null());
    assert!(violation.get("scope").is_none() || violation["scope"].is_null());
}

// =============================================================================
// MULTIPLE CHECKS
// =============================================================================

#[test]
fn formats_multiple_checks() {
    let formatter = JsonFormatter::default();
    let results = vec![
        CheckResult::passed("cloc"),
        CheckResult::passed("escapes"),
        CheckResult::skipped("git", "Disabled"),
        CheckResult::failed("placeholders", vec![
            Violation::new(Some("README.md".into()), Some(1), "todo", "Remove TODO"),
        ]),
    ];
    let output = formatter.format(&results, &AllChecks).unwrap();

    let json: serde_json::Value = serde_json::from_str(&output).unwrap();
    let checks = json["checks"].as_array().unwrap();
    assert_eq!(checks.len(), 4);
}
```

### 2.2 Register Module in json.rs

**File:** `crates/cli/src/report/json.rs`

Add at the end of the file:

```rust
#[cfg(test)]
#[path = "json_tests.rs"]
mod tests;
```

**Verification:**
```bash
cargo test -p quench -- json_tests
cargo test -p quench -- report::json
```

---

## Phase 3: Add Markdown Integration Tests

### 3.1 Add Behavioral Specs

**File:** `tests/specs/cli/report.rs`

Add after the HTML FORMAT section:

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
        .stdout_has("|---");
}

/// Spec: docs/specs/01-cli.md#quench-report
///
/// > Markdown format includes all metric values
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
/// > Markdown format with no baseline shows appropriate message
#[test]
fn report_markdown_no_baseline_shows_message() {
    report()
        .on("report/no-baseline")
        .markdown()
        .runs()
        .stdout_has("No baseline");
}

/// Spec: docs/specs/01-cli.md#quench-report
///
/// > Markdown format includes baseline metadata
#[test]
fn report_markdown_includes_metadata() {
    report()
        .on("report/with-baseline")
        .markdown()
        .runs()
        .stdout_has("abc1234")  // commit hash
        .stdout_has("2026-01-20");  // date
}
```

### 3.2 Add Helper Method (if needed)

**File:** `tests/specs/prelude.rs`

If `.markdown()` helper doesn't exist, add:

```rust
impl ReportRunner {
    /// Use markdown output format.
    pub fn markdown(self) -> Self {
        self.args(["--format", "markdown"])
    }
}
```

**Verification:**
```bash
cargo test --test specs report_markdown
```

---

## Phase 4: Final Verification

### 4.1 Run All Tests

```bash
# Unit tests including new json_tests
cargo test -p quench

# Behavioral specs including new markdown tests
cargo test --test specs

# Full CI check
make check
```

### 4.2 Manual Verification

```bash
# Verify scope field in git check JSON
echo 'feat(api): add endpoint' | cargo run -- check-git-message - --format json | jq '.'

# Verify markdown format
cargo run -- report --format markdown

# Verify JSON formatter
cargo run -- check --format json | jq '.checks'
```

---

## Summary

| Gap | Solution | New Tests | LOC Added |
|-----|----------|-----------|-----------|
| Missing scope field | Add to Violation struct with builder | 2 unit tests | ~20 |
| Missing json_tests.rs | Create comprehensive test file | 12 unit tests | ~150 |
| Missing markdown spec | Add behavioral specs | 4 integration tests | ~40 |

**Total new tests:** 18
**Total LOC:** ~210

## Exit Criteria

- [ ] Violation struct has `scope: Option<String>` field
- [ ] `Violation::with_scope()` builder method exists
- [ ] Git check populates scope from parsed commit
- [ ] `crates/cli/src/report/json_tests.rs` exists with 12+ tests
- [ ] `tests/specs/cli/report.rs` has 4 markdown tests
- [ ] All new tests pass
- [ ] `make check` passes
- [ ] Manual verification of JSON scope output works
