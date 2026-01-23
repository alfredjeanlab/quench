# Checkpoint 3B: Escapes Works - Validation

**Root Feature:** `quench-0dc9`

## Overview

Validation checkpoint that verifies the escapes check produces correct outputs on the standard test fixtures (`violations` and dedicated escape fixtures). This creates documented evidence that escape type detection, comment search, source/test separation, and output formatting all work correctly.

## Project Structure

Key files involved:

```
quench/
├── tests/
│   ├── specs/checks/escapes.rs     # Existing behavioral specs (13 tests)
│   ├── fixtures/
│   │   ├── violations/             # Multi-violation project
│   │   │   ├── src/escapes.rs      # .unwrap(), unsafe violations
│   │   │   └── quench.toml         # escape patterns config
│   │   └── escapes/                # Dedicated escape fixtures
│   │       ├── basic/              # Pattern detection
│   │       ├── count-ok/           # Threshold passing
│   │       ├── count-fail/         # Threshold exceeded
│   │       ├── comment-ok/         # Comment detection passing
│   │       ├── comment-fail/       # Missing comment
│   │       ├── forbid-source/      # Forbid in source
│   │       ├── forbid-test/        # Forbid in test (allowed)
│   │       └── metrics/            # Metrics breakdown
│   └── specs.rs                    # Test harness
├── reports/
│   └── checkpoint-3-escapes-works.md  # Validation report (to be created)
└── crates/cli/src/checks/escapes.rs   # Escapes implementation
```

## Dependencies

No new dependencies required. Uses existing test infrastructure:
- `assert_cmd` - CLI execution
- `serde_json` - JSON parsing
- `predicates` - Assertion matchers

## Implementation Phases

### Phase 1: Verify Escapes on violations Fixture

Run escapes check on `tests/fixtures/violations` and verify all expected violations are detected.

**Expected Violations in `src/escapes.rs`:**

| Line | Pattern | Action | Expected Result |
|------|---------|--------|-----------------|
| 5 | `.unwrap()` | forbid | `forbidden` violation |
| 15 | `unsafe { *ptr }` | comment | `missing_comment` violation |
| 21 | `unsafe { *ptr }` (with SAFETY) | comment | Pass (comment found) |

**Commands:**
```bash
./target/release/quench check tests/fixtures/violations --escapes -o json
./target/release/quench check tests/fixtures/violations --escapes
```

**Verification:**
- JSON output contains exactly 2 violations from escapes.rs
- Text output shows file paths and line numbers
- Proper violations escaped (unsafe with SAFETY comment passes)

**Milestone:** All escape types correctly detected in violations fixture.

**Status:** [ ] Pending

### Phase 2: Verify Each Escape Type in Dedicated Fixtures

Verify each escape action type works correctly using the dedicated fixtures.

**Test Matrix:**

| Fixture | Action | Expected | Verification |
|---------|--------|----------|--------------|
| `escapes/count-ok` | count | Pass | Count within threshold |
| `escapes/count-fail` | count | Fail | `threshold_exceeded` violation |
| `escapes/comment-ok` | comment | Pass | Comment found |
| `escapes/comment-fail` | comment | Fail | `missing_comment` violation |
| `escapes/forbid-source` | forbid | Fail | `forbidden` violation |
| `escapes/forbid-test` | forbid | Pass | Test code exempt |

**Commands:**
```bash
# Test each fixture
for f in count-ok count-fail comment-ok comment-fail forbid-source forbid-test; do
  echo "=== escapes/$f ==="
  ./target/release/quench check tests/fixtures/escapes/$f --escapes -o json
done
```

**Milestone:** All fixture results match expected outcomes.

**Status:** [ ] Pending

### Phase 3: Add Snapshot Test for Text Output

Create behavioral specs that verify text output format for each violation type.

```rust
// tests/specs/checks/escapes.rs

/// Spec: docs/specs/checks/escape-hatches.md#text-output
///
/// > Text output shows violations with file path, line, and advice
#[test]
fn escapes_text_output_format_on_missing_comment() {
    check("escapes")
        .on("escapes/comment-fail")
        .fails()
        .stdout_has("escapes: FAIL")
        .stdout_has("lib.rs")
        .stdout_has("missing_comment");
}

/// Spec: docs/specs/checks/escape-hatches.md#text-output
///
/// > Forbidden violations show pattern name and advice
#[test]
fn escapes_text_output_format_on_forbidden() {
    check("escapes")
        .on("escapes/forbid-source")
        .fails()
        .stdout_has("escapes: FAIL")
        .stdout_has("forbidden");
}

/// Spec: docs/specs/checks/escape-hatches.md#text-output
///
/// > Threshold exceeded shows count vs limit
#[test]
fn escapes_text_output_format_on_threshold_exceeded() {
    check("escapes")
        .on("escapes/count-fail")
        .fails()
        .stdout_has("escapes: FAIL")
        .stdout_has("threshold_exceeded");
}
```

**Milestone:** Text output snapshot specs pass for all three violation types.

**Status:** [ ] Pending

### Phase 4: Add Snapshot Test for JSON Output

Create behavioral specs that validate JSON structure completeness for each violation type.

```rust
// tests/specs/checks/escapes.rs

/// Spec: docs/specs/checks/escape-hatches.md#json-output
///
/// > JSON output includes all required fields for violations
#[test]
fn escapes_json_violation_structure_complete() {
    let escapes = check("escapes").on("violations").json().fails();
    let violations = escapes.require("violations").as_array().unwrap();

    assert!(!violations.is_empty(), "should have violations");

    // Each violation must have all required fields
    for violation in violations {
        assert!(violation.get("file").is_some(), "missing file");
        assert!(violation.get("line").is_some(), "missing line");
        assert!(violation.get("type").is_some(), "missing type");
        assert!(violation.get("pattern").is_some(), "missing pattern");
        assert!(violation.get("advice").is_some(), "missing advice");
    }
}

/// Spec: docs/specs/checks/escape-hatches.md#json-output
///
/// > JSON metrics include source and test breakdowns per pattern
#[test]
fn escapes_json_metrics_structure_complete() {
    let escapes = check("escapes").on("escapes/metrics").json().passes();
    let metrics = escapes.require("metrics");

    // Verify structure
    assert!(metrics.get("source").is_some(), "missing source metrics");
    assert!(metrics.get("test").is_some(), "missing test metrics");

    // Source and test should be objects with pattern counts
    let source = metrics.get("source").unwrap();
    let test = metrics.get("test").unwrap();
    assert!(source.is_object(), "source should be object");
    assert!(test.is_object(), "test should be object");
}
```

**Milestone:** JSON structure specs pass with all required fields present.

**Status:** [ ] Pending

### Phase 5: Create Validation Report

Generate `reports/checkpoint-3-escapes-works.md` documenting:

1. **Each escape type detection verified**
   - Forbidden action detection results
   - Comment action (missing/present) detection results
   - Count action (threshold exceeded) detection results

2. **Comment search behavior verified**
   - Same-line comment detection
   - Preceding-line comment detection
   - Stop at non-comment/non-blank line

3. **Source/test separation verified**
   - Test code exempt from forbid/comment enforcement
   - Metrics tracked separately for source vs test

4. **Unexpected behaviors** (if any)

**Template:**
```markdown
# Checkpoint 3B: Escapes Works - Validation Report

Generated: YYYY-MM-DD

## Summary

| Criterion | Status | Notes |
|-----------|--------|-------|
| Escapes on violations detects all types | ✓/✗ | ... |
| Snapshot test for text output | ✓/✗ | ... |
| Snapshot test for JSON output | ✓/✗ | ... |
| All behavioral specs pass | ✓/✗ | N specs total |

**Overall Status: PASS/FAIL**

## Detailed Results

### 1. Escape Type Detection

[Command outputs and verification]

### 2. Comment Search Behavior

[Evidence of comment detection working]

### 3. Source/Test Separation

[Metrics showing separate tracking]

### 4. Snapshot Tests

[New specs added and results]

## Unexpected Behaviors

[Any deviations or discoveries]

## Conclusion

[Summary of findings]
```

**Milestone:** Report created with all checkpoint criteria documented.

**Status:** [ ] Pending

### Phase 6: Run Full Test Suite

Execute `make check` to ensure all changes pass quality gates.

```bash
make check
```

**Checklist:**
- [ ] `cargo fmt --all -- --check` - no formatting issues
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` - no warnings
- [ ] `cargo test --all` - all tests pass
- [ ] `cargo test escapes` - all 17+ escapes specs pass (13 existing + 4 new)
- [ ] `cargo build --all` - builds successfully
- [ ] `./scripts/bootstrap` - conventions pass
- [ ] `cargo audit` - no critical vulnerabilities
- [ ] `cargo deny check` - licenses/bans OK

**Milestone:** All quality gates pass.

**Status:** [ ] Pending

## Key Implementation Details

### Escape Type Detection

The violations fixture `src/escapes.rs` contains:

**Line 5 - Forbidden `.unwrap()`:**
```rust
input.parse().unwrap()  // VIOLATION: .unwrap() forbidden
```

**Line 15 - Missing comment `unsafe`:**
```rust
unsafe { *ptr }  // VIOLATION: unsafe without // SAFETY: comment
```

**Line 20-21 - Properly commented `unsafe` (should pass):**
```rust
// SAFETY: Caller guarantees ptr is valid and aligned.
unsafe { *ptr }
```

### Comment Search Algorithm

The escapes check searches for required comments:
1. Same line as the pattern match
2. Preceding lines, searching upward
3. Stops at first non-blank, non-comment line

Language-agnostic comment detection supports: `//`, `#`, `/*`, `--`, `;;`

### Violation Types

| Type | Trigger | Message |
|------|---------|---------|
| `missing_comment` | Comment action without required comment | "requires {comment} justification" |
| `forbidden` | Forbid action in source code | "forbidden in production code" |
| `threshold_exceeded` | Count exceeds threshold | "{count} occurrences (threshold: {n})" |

### Output Format

**Text Format:**
```
escapes: FAIL
  src/escapes.rs:5: forbidden (.unwrap() in production code)
    Remove this escape hatch from production code.
  src/escapes.rs:15: missing_comment (unsafe without // SAFETY:)
    Add a // SAFETY: comment explaining why this unsafe block is sound.
```

**JSON Format:**
```json
{
  "name": "escapes",
  "passed": false,
  "violations": [
    {
      "file": "src/escapes.rs",
      "line": 5,
      "type": "forbidden",
      "pattern": "unwrap",
      "advice": "Remove this escape hatch..."
    }
  ],
  "metrics": {
    "source": { "unwrap": 1, "unsafe": 2 },
    "test": { "unwrap": 0, "unsafe": 0 }
  }
}
```

## Verification Plan

1. **Build release binary:**
   ```bash
   cargo build --release
   ```

2. **Run fixture tests manually:**
   ```bash
   ./target/release/quench check tests/fixtures/violations --escapes -o json
   ./target/release/quench check tests/fixtures/escapes/count-fail --escapes
   ```

3. **Run behavioral specs:**
   ```bash
   cargo test --test specs escapes
   ```

4. **Run full quality check:**
   ```bash
   make check
   ```

## Checkpoint Criteria Mapping

| Criterion | Phase | Verification Method |
|-----------|-------|---------------------|
| Escapes on violations detects all types | Phase 1-2 | Manual inspection + existing specs |
| Snapshot test for escapes text output | Phase 3 | New behavioral specs |
| Snapshot test for escapes JSON output | Phase 4 | New behavioral specs |
| Report: each escape type detection | Phase 5 | Document |
| Report: comment search behavior | Phase 5 | Document |
| Report: source/test separation | Phase 5 | Document |

## Summary

| Task | Status |
|------|--------|
| Verify violations fixture detection | [ ] Pending |
| Verify each escape type in dedicated fixtures | [ ] Pending |
| Text output snapshot specs | [ ] Pending |
| JSON output snapshot specs | [ ] Pending |
| Validation report | [ ] Pending |
| Full test suite | [ ] Pending |
