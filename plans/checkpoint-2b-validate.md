# Checkpoint 2B: CLOC Works - Validation

**Root Feature:** `quench-fda7`

## Overview

Validation checkpoint that verifies the CLOC check produces correct outputs on the standard test fixtures (`rust-simple`, `violations`). This creates documented evidence that line counting, violation detection, and output formatting work correctly.

## Project Structure

Key files involved:

```
quench/
├── tests/
│   ├── specs/checks/cloc.rs       # Existing behavioral specs (16 tests)
│   ├── fixtures/
│   │   ├── rust-simple/           # Clean Rust project for baseline testing
│   │   │   ├── src/lib.rs         # 10 source lines
│   │   │   └── src/lib_tests.rs   # 5 test lines
│   │   └── violations/            # Intentional violations project
│   │       ├── src/oversized.rs   # 799 lines (over default 750 limit)
│   │       └── quench.toml        # max_lines = 5 (very strict)
│   └── specs.rs                   # Test harness
├── reports/
│   └── checkpoint-2-cloc-works.md # Validation report (to be created)
└── crates/cli/src/checks/cloc.rs  # CLOC implementation
```

## Dependencies

No new dependencies required. Uses existing test infrastructure:
- `assert_cmd` - CLI execution
- `serde_json` - JSON parsing
- `predicates` - Assertion matchers

## Implementation Phases

### Phase 1: Verify CLOC on rust-simple Fixture

Run CLOC check on `tests/fixtures/rust-simple` and verify correct line counts.

```bash
cd tests/fixtures/rust-simple
quench check --cloc --output json | jq '.checks[0].metrics'
```

**Expected Results:**
- `source_lines`: 10 (from `src/lib.rs`)
- `source_files`: 1
- `test_lines`: 5 (from `src/lib_tests.rs`)
- `test_files`: 1
- `ratio`: 0.5 (5/10)
- `passed`: true (no violations)

**Milestone:** JSON output matches expected metrics exactly.

**Status:** [ ] Pending

### Phase 2: Verify CLOC on violations Fixture

Run CLOC check on `tests/fixtures/violations` and verify oversized file detection.

```bash
cd tests/fixtures/violations
quench check --cloc --output json | jq '.checks[0]'
```

**Expected Results:**
- `passed`: false
- `violations`: Array containing entries for files exceeding `max_lines = 5`
- `oversized.rs`: 799 lines (expected to fail)
- Multiple source files should fail the very strict 5-line limit

**Milestone:** CLOC correctly identifies all oversized files.

**Status:** [ ] Pending

### Phase 3: Add Snapshot Test for Text Output

Create a behavioral spec that verifies text output format on a controlled fixture.

```rust
// tests/specs/checks/cloc.rs

/// Spec: docs/specs/checks/cloc.md#text-output
///
/// > Text output shows violations with file path, line count, and advice
#[test]
fn cloc_text_output_format_on_violation() {
    check("cloc")
        .on("cloc/oversized-source")
        .fails()
        .stdout_has("cloc: FAIL")
        .stdout_has("big.rs")
        .stdout_has("file_too_large")
        .stdout_has("750");  // threshold in output
}
```

**Milestone:** Text output spec passes consistently.

**Status:** [ ] Pending

### Phase 4: Add Snapshot Test for JSON Output

Create a behavioral spec that validates JSON structure completeness.

```rust
// tests/specs/checks/cloc.rs

/// Spec: docs/specs/checks/cloc.md#json-output
///
/// > JSON output includes all required fields for violations
#[test]
fn cloc_json_violation_structure_complete() {
    let cloc = check("cloc").on("cloc/oversized-source").json().fails();
    let violations = cloc.require("violations").as_array().unwrap();

    // Each violation must have all required fields
    for violation in violations {
        assert!(violation.get("file").is_some(), "missing file");
        assert!(violation.get("type").is_some(), "missing type");
        assert!(violation.get("value").is_some(), "missing value");
        assert!(violation.get("threshold").is_some(), "missing threshold");
        assert!(violation.get("advice").is_some(), "missing advice");
    }
}
```

**Milestone:** JSON structure spec passes with all required fields present.

**Status:** [ ] Pending

### Phase 5: Create Validation Report

Generate `reports/checkpoint-2-cloc-works.md` documenting:
- Actual vs expected line counts for `rust-simple`
- Violation detection results for `violations`
- JSON output structure validation
- Any unexpected behaviors discovered

**Milestone:** Report created with all checkpoint criteria documented.

**Status:** [ ] Pending

### Phase 6: Run Full Test Suite

Execute `make check` to ensure all changes pass quality gates.

```bash
make check
```

**Milestone:** All tests pass, including new snapshot specs.

**Status:** [ ] Pending

## Key Implementation Details

### Line Counting Verification

The `rust-simple` fixture contains:

**src/lib.rs** (10 non-blank lines):
```rust
//! A simple library for testing quench.

/// Adds two numbers together.
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

#[cfg(test)]
#[path = "lib_tests.rs"]
mod tests;
```

**src/lib_tests.rs** (5 non-blank lines):
```rust
#![allow(clippy::unwrap_used)]
use super::*;

#[test]
fn test_add() {
    assert_eq!(add(2, 3), 5);
}
```

### Violation Detection

The `violations` fixture has `max_lines = 5` configured. Files exceeding this:
- `src/oversized.rs` - 799 lines
- `src/lib.rs` - likely exceeds 5 lines
- Other source files - need verification

### Output Format Snapshots

**Text Output Format:**
```
cloc: FAIL
  src/big.rs: file_too_large (800 vs 750)
    Can the code be made more concise? ...
```

**JSON Output Structure:**
```json
{
  "name": "cloc",
  "passed": false,
  "violations": [
    {
      "file": "src/big.rs",
      "type": "file_too_large",
      "value": 800,
      "threshold": 750,
      "advice": "..."
    }
  ],
  "metrics": {
    "source_lines": 800,
    "source_files": 1,
    "source_tokens": 3200,
    "test_lines": 0,
    "test_files": 0,
    "test_tokens": 0,
    "ratio": 0.0
  }
}
```

## Verification Plan

1. **Compile and build:**
   ```bash
   cargo build --release
   ```

2. **Run fixture tests manually:**
   ```bash
   ./target/release/quench check tests/fixtures/rust-simple --cloc -o json
   ./target/release/quench check tests/fixtures/violations --cloc -o json
   ```

3. **Run behavioral specs:**
   ```bash
   cargo test --test specs cloc
   ```

4. **Run full quality check:**
   ```bash
   make check
   ```

## Checkpoint Criteria Mapping

| Criterion | Phase | Verification Method |
|-----------|-------|---------------------|
| CLOC on rust-simple produces correct counts | Phase 1 | Manual JSON inspection + existing specs |
| CLOC on violations detects oversized file | Phase 2 | Manual inspection + existing specs |
| Snapshot test for CLOC text output | Phase 3 | New behavioral spec |
| Snapshot test for CLOC JSON output | Phase 4 | New behavioral spec |
| Report with line counts | Phase 5 | Document creation |
| Report with violation detection | Phase 5 | Document creation |
| Report with JSON validation | Phase 5 | Document creation |

## Summary

| Task | Status |
|------|--------|
| Verify rust-simple line counts | [ ] Pending |
| Verify violations detection | [ ] Pending |
| Text output snapshot spec | [ ] Pending |
| JSON output snapshot spec | [ ] Pending |
| Validation report | [ ] Pending |
| Full test suite | [ ] Pending |
