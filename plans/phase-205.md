# Phase 205: Escapes Check - Behavioral Specs

**Root Feature:** `quench-920b`

## Overview

Write behavioral specifications (black-box tests) for the `escapes` check. These specs define the expected behavior for pattern-based escape hatch detection, including the three action types (count, comment, forbid), source/test separation, and JSON output format.

All specs will be marked with `#[ignore = "TODO: Phase 210/215/220"]` since the escapes check implementation comes in later phases.

## Project Structure

```
quench/
├── tests/
│   ├── specs/
│   │   └── checks/
│   │       └── escapes.rs     # NEW: Behavioral specs for escapes check
│   └── fixtures/
│       └── escapes/           # NEW: Dedicated fixtures for escapes specs
│           ├── basic/         # Pattern match detection
│           ├── comment-ok/    # Comment action passes
│           ├── comment-fail/  # Comment action fails
│           ├── forbid-source/ # Forbid in source code
│           ├── forbid-test/   # Forbid allowed in test code
│           ├── count-ok/      # Count under threshold
│           ├── count-fail/    # Count exceeds threshold
│           └── metrics/       # Source/test breakdown
└── plans/
    └── phase-205.md
```

## Dependencies

No new dependencies. Uses existing test infrastructure:
- `assert_cmd` (CLI testing)
- `serde_json` (JSON output validation)
- `tempfile` (temp directories)

## Implementation Phases

### Phase 1: Create Spec File Structure

Create the spec file with module structure and imports.

**New file `tests/specs/checks/escapes.rs`:**

```rust
//! Behavioral specs for the escapes (escape hatches) check.
//!
//! Tests that quench correctly:
//! - Detects pattern matches in source files
//! - Applies actions (count, comment, forbid)
//! - Separates source and test code
//! - Generates correct violation types
//! - Outputs metrics in JSON format
//!
//! Reference: docs/specs/checks/escape-hatches.md

#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::prelude::*;

// =============================================================================
// PATTERN DETECTION SPECS
// =============================================================================

// ... specs here

// =============================================================================
// COUNT ACTION SPECS
// =============================================================================

// ... specs here

// =============================================================================
// COMMENT ACTION SPECS
// =============================================================================

// ... specs here

// =============================================================================
// FORBID ACTION SPECS
// =============================================================================

// ... specs here

// =============================================================================
// SOURCE VS TEST SPECS
// =============================================================================

// ... specs here

// =============================================================================
// JSON OUTPUT SPECS
// =============================================================================

// ... specs here
```

**Update `tests/specs/main.rs`** (or equivalent) to include the new module:

```rust
mod checks {
    mod cloc;
    mod escapes;  // NEW
}
```

**Milestone:** Spec file compiles with no tests.

**Verification:**
```bash
cargo test --test specs -- --list 2>&1 | grep escapes || echo "No escapes tests yet"
```

---

### Phase 2: Pattern Detection Specs

Write specs for basic pattern matching behavior.

```rust
/// Spec: docs/specs/checks/escape-hatches.md#pattern-matching
///
/// > The escapes check detects patterns that bypass type safety or error handling.
#[test]
#[ignore = "TODO: Phase 210 - Escapes Check Core"]
fn escapes_detects_pattern_matches_in_source() {
    check("escapes")
        .on("escapes/basic")
        .fails()
        .stdout_has("escapes: FAIL");
}

/// Spec: docs/specs/checks/escape-hatches.md#output
///
/// > src/parser.rs:47: unsafe block without // SAFETY: comment
#[test]
#[ignore = "TODO: Phase 210 - Escapes Check Core"]
fn escapes_reports_line_number_of_match() {
    let escapes = check("escapes").on("escapes/basic").json().fails();
    let violations = escapes.require("violations").as_array().unwrap();

    assert!(violations.iter().any(|v| {
        v.get("line").and_then(|l| l.as_u64()).is_some()
    }), "violations should include line numbers");
}
```

**Fixture `tests/fixtures/escapes/basic/`:**

```toml
# quench.toml
version = 1

[[check.escapes.patterns]]
name = "unwrap"
pattern = "\\.unwrap\\(\\)"
action = "forbid"
```

```rust
// src/lib.rs
pub fn risky(opt: Option<i32>) -> i32 {
    opt.unwrap()  // Line 3: violation
}
```

**Milestone:** 2 pattern detection specs compile and are ignored.

---

### Phase 3: Count Action Specs

Write specs for the count action behavior.

```rust
/// Spec: docs/specs/checks/escape-hatches.md#count
///
/// > Just count occurrences.
#[test]
#[ignore = "TODO: Phase 215 - Escapes Actions"]
fn escapes_count_action_counts_occurrences() {
    let escapes = check("escapes").on("escapes/count-ok").json().passes();
    let metrics = escapes.require("metrics");
    let source = metrics.get("source").unwrap();

    assert!(source.get("todo").and_then(|v| v.as_u64()).unwrap() > 0,
        "should count TODO occurrences");
}

/// Spec: docs/specs/checks/escape-hatches.md#count
///
/// > Fail if count exceeds per-pattern threshold (default: 0).
#[test]
#[ignore = "TODO: Phase 215 - Escapes Actions"]
fn escapes_count_action_fails_when_threshold_exceeded() {
    let escapes = check("escapes").on("escapes/count-fail").json().fails();
    let violations = escapes.require("violations").as_array().unwrap();

    assert!(violations.iter().any(|v| {
        v.get("type").and_then(|t| t.as_str()) == Some("threshold_exceeded")
    }), "should have threshold_exceeded violation");
}
```

**Fixtures:**

`escapes/count-ok/quench.toml`:
```toml
version = 1

[[check.escapes.patterns]]
name = "todo"
pattern = "TODO|FIXME"
action = "count"
threshold = 10  # High threshold, should pass
```

`escapes/count-fail/quench.toml`:
```toml
version = 1

[[check.escapes.patterns]]
name = "todo"
pattern = "TODO|FIXME"
action = "count"
threshold = 1  # Low threshold, will fail
```

**Milestone:** 2 count action specs compile and are ignored.

---

### Phase 4: Comment Action Specs

Write specs for the comment action behavior.

```rust
/// Spec: docs/specs/checks/escape-hatches.md#comment
///
/// > Pattern is allowed if accompanied by a justification comment.
#[test]
#[ignore = "TODO: Phase 215 - Escapes Actions"]
fn escapes_comment_action_passes_when_comment_on_same_line() {
    check("escapes")
        .on("escapes/comment-ok")
        .passes();
}

/// Spec: docs/specs/checks/escape-hatches.md#comment-detection
///
/// > On preceding lines, searching upward until a non-blank, non-comment line is found
#[test]
#[ignore = "TODO: Phase 215 - Escapes Actions"]
fn escapes_comment_action_passes_when_comment_on_preceding_line() {
    let dir = temp_project();
    std::fs::write(
        dir.path().join("quench.toml"),
        r#"
version = 1
[[check.escapes.patterns]]
name = "unsafe"
pattern = "unsafe\\s*\\{"
action = "comment"
comment = "// SAFETY:"
"#,
    ).unwrap();
    std::fs::create_dir_all(dir.path().join("src")).unwrap();
    std::fs::write(
        dir.path().join("src/lib.rs"),
        r#"
// SAFETY: Pointer guaranteed valid by caller
unsafe { *ptr }
"#,
    ).unwrap();

    check("escapes").pwd(dir.path()).passes();
}

/// Spec: docs/specs/checks/escape-hatches.md#comment
///
/// > Require a justification comment.
#[test]
#[ignore = "TODO: Phase 215 - Escapes Actions"]
fn escapes_comment_action_fails_when_no_comment_found() {
    let escapes = check("escapes").on("escapes/comment-fail").json().fails();
    let violations = escapes.require("violations").as_array().unwrap();

    assert!(violations.iter().any(|v| {
        v.get("type").and_then(|t| t.as_str()) == Some("missing_comment")
    }), "should have missing_comment violation");
}
```

**Fixtures:**

`escapes/comment-ok/src/lib.rs`:
```rust
pub fn safe_op(ptr: *const i32) -> i32 {
    // SAFETY: Caller guarantees ptr is valid
    unsafe { *ptr }
}
```

`escapes/comment-fail/src/lib.rs`:
```rust
pub fn risky_op(ptr: *const i32) -> i32 {
    unsafe { *ptr }  // Missing SAFETY comment
}
```

**Milestone:** 3 comment action specs compile and are ignored.

---

### Phase 5: Forbid Action Specs

Write specs for the forbid action behavior.

```rust
/// Spec: docs/specs/checks/escape-hatches.md#forbid
///
/// > Pattern is never allowed in source code.
#[test]
#[ignore = "TODO: Phase 215 - Escapes Actions"]
fn escapes_forbid_action_always_fails_in_source_code() {
    let escapes = check("escapes").on("escapes/forbid-source").json().fails();
    let violations = escapes.require("violations").as_array().unwrap();

    assert!(violations.iter().any(|v| {
        v.get("type").and_then(|t| t.as_str()) == Some("forbidden")
    }), "should have forbidden violation");
}

/// Spec: docs/specs/checks/escape-hatches.md#forbid
///
/// > Always allowed in test code.
#[test]
#[ignore = "TODO: Phase 215 - Escapes Actions"]
fn escapes_forbid_action_allowed_in_test_code() {
    check("escapes")
        .on("escapes/forbid-test")
        .passes();
}
```

**Fixtures:**

`escapes/forbid-source/src/lib.rs`:
```rust
pub fn risky(opt: Option<i32>) -> i32 {
    opt.unwrap()  // Forbidden in source
}
```

`escapes/forbid-test/tests/lib_test.rs`:
```rust
#[test]
fn test_something() {
    let opt: Option<i32> = Some(42);
    assert_eq!(opt.unwrap(), 42);  // Allowed in tests
}
```

`escapes/forbid-test/src/lib.rs`:
```rust
pub fn safe_fn() -> i32 {
    42  // No violations
}
```

**Milestone:** 2 forbid action specs compile and are ignored.

---

### Phase 6: Source vs Test and Metrics Specs

Write specs for source/test separation and JSON metrics.

```rust
/// Spec: docs/specs/checks/escape-hatches.md#source-vs-test
///
/// > Escape hatches are counted separately for source and test code.
#[test]
#[ignore = "TODO: Phase 220 - Escapes Metrics"]
fn escapes_test_code_counted_separately_in_metrics() {
    let escapes = check("escapes").on("escapes/metrics").json().passes();
    let metrics = escapes.require("metrics");

    let source = metrics.get("source").expect("should have source metrics");
    let test = metrics.get("test").expect("should have test metrics");

    // Both should have counts (actual values depend on fixture)
    assert!(source.is_object(), "source should be object");
    assert!(test.is_object(), "test should be object");
}

/// Spec: docs/specs/checks/escape-hatches.md#configurable-advice
///
/// > Each pattern can have custom advice
#[test]
#[ignore = "TODO: Phase 215 - Escapes Actions"]
fn escapes_per_pattern_advice_shown_in_violation() {
    let dir = temp_project();
    std::fs::write(
        dir.path().join("quench.toml"),
        r#"
version = 1
[[check.escapes.patterns]]
name = "unwrap"
pattern = "\\.unwrap\\(\\)"
action = "forbid"
advice = "Use .context() from anyhow instead."
"#,
    ).unwrap();
    std::fs::create_dir_all(dir.path().join("src")).unwrap();
    std::fs::write(
        dir.path().join("src/lib.rs"),
        "pub fn f() { None::<i32>.unwrap(); }",
    ).unwrap();

    let escapes = check("escapes").pwd(dir.path()).json().fails();
    let violations = escapes.require("violations").as_array().unwrap();

    let advice = violations[0].get("advice").and_then(|a| a.as_str()).unwrap();
    assert_eq!(advice, "Use .context() from anyhow instead.");
}

/// Spec: docs/specs/checks/escape-hatches.md#json-output
///
/// > metrics: { source: {...}, test: {...} }
#[test]
#[ignore = "TODO: Phase 220 - Escapes Metrics"]
fn escapes_json_includes_source_test_breakdown_per_pattern() {
    let escapes = check("escapes").on("escapes/metrics").json().passes();
    let metrics = escapes.require("metrics");

    // Source metrics by pattern name
    let source = metrics.get("source").unwrap();
    assert!(source.get("unwrap").is_some() || source.get("todo").is_some(),
        "source should have pattern counts");

    // Test metrics by pattern name
    let test = metrics.get("test").unwrap();
    assert!(test.is_object(), "test should have pattern counts");
}

/// Spec: docs/specs/checks/escape-hatches.md#json-output
///
/// > Violation types: missing_comment, forbidden, threshold_exceeded
#[test]
#[ignore = "TODO: Phase 215 - Escapes Actions"]
fn escapes_violation_type_is_one_of_expected_values() {
    let escapes = check("escapes").on("violations").json().fails();
    let violations = escapes.require("violations").as_array().unwrap();

    let valid_types = ["missing_comment", "forbidden", "threshold_exceeded"];
    for violation in violations {
        let vtype = violation.get("type").and_then(|t| t.as_str()).unwrap();
        assert!(valid_types.contains(&vtype),
            "unexpected violation type: {}", vtype);
    }
}
```

**Fixture `escapes/metrics/`:**

```toml
# quench.toml
version = 1

[[check.escapes.patterns]]
name = "unwrap"
pattern = "\\.unwrap\\(\\)"
action = "count"
threshold = 100  # High threshold to pass

[[check.escapes.patterns]]
name = "todo"
pattern = "TODO"
action = "count"
threshold = 100
```

```rust
// src/lib.rs
pub fn safe() -> i32 {
    // TODO: Refactor this
    42
}
```

```rust
// tests/lib_test.rs
#[test]
fn test_safe() {
    // TODO: More tests
    let x: Option<i32> = Some(1);
    assert_eq!(x.unwrap(), 1);  // unwrap in test
}
```

**Milestone:** 4 metrics/output specs compile and are ignored.

---

## Key Implementation Details

### Spec Organization by Feature Area

Specs are grouped by the feature they test, not by pass/fail:
- Pattern detection (basic matching, line numbers)
- Count action (counting, thresholds)
- Comment action (same line, preceding line, missing)
- Forbid action (source vs test)
- Metrics (source/test breakdown, JSON structure)

### Fixture Design Principles

Each fixture tests ONE specific behavior:
- `basic/` - Pattern matches are detected
- `comment-ok/` - Comment action passes with valid comment
- `comment-fail/` - Comment action fails without comment
- `forbid-source/` - Forbid fails in source
- `forbid-test/` - Forbid passes in test
- `count-ok/` - Count under threshold
- `count-fail/` - Count over threshold
- `metrics/` - JSON metrics structure

### Phase Assignment for Ignores

Specs are marked with the phase where the feature will be implemented:
- **Phase 210**: Core escapes check (pattern detection, basic output)
- **Phase 215**: Actions (count/comment/forbid logic)
- **Phase 220**: Metrics (source/test breakdown, JSON metrics)

### Violation Type Enum

The spec validates that `violation.type` is one of:
- `missing_comment` - Comment action, no comment found
- `forbidden` - Forbid action, pattern in source
- `threshold_exceeded` - Count action, over threshold

## Verification Plan

### After Each Phase

```bash
# Compile specs (should succeed)
cargo test --test specs -- --list 2>&1 | grep escapes

# Run non-ignored specs (should be none initially)
cargo test --test specs escapes

# Count ignored specs
cargo test --test specs escapes -- --ignored --list 2>&1 | grep -c "test escapes"
```

### Full Quality Gates

```bash
make check
```

### Expected Counts

| Phase | New Specs | Total Ignored |
|-------|-----------|---------------|
| 1     | 0         | 0             |
| 2     | 2         | 2             |
| 3     | 2         | 4             |
| 4     | 3         | 7             |
| 5     | 2         | 9             |
| 6     | 4         | 13            |

## Summary

| Phase | Task | Key Files | Status |
|-------|------|-----------|--------|
| 1 | Create spec file structure | `tests/specs/checks/escapes.rs` | [ ] Pending |
| 2 | Pattern detection specs | `tests/fixtures/escapes/basic/` | [ ] Pending |
| 3 | Count action specs | `tests/fixtures/escapes/count-*/` | [ ] Pending |
| 4 | Comment action specs | `tests/fixtures/escapes/comment-*/` | [ ] Pending |
| 5 | Forbid action specs | `tests/fixtures/escapes/forbid-*/` | [ ] Pending |
| 6 | Source/test and metrics specs | `tests/fixtures/escapes/metrics/` | [ ] Pending |

## Future Phases (Not This Phase)

- **Phase 210**: Implement escapes check core (pattern matching, basic violations)
- **Phase 215**: Implement actions (count/comment/forbid logic)
- **Phase 220**: Implement metrics (source/test breakdown, JSON output)
