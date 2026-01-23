# Checkpoint 2C: Post-Checkpoint Refactor - CLOC Works

**Root Feature:** `quench-fda7`

## Overview

Post-checkpoint refactor to review the validation report, address any edge cases in line counting or pattern matching, and ensure the JSON structure matches the spec exactly. Based on the checkpoint-2b validation report, the CLOC check is working correctly with all 20 behavioral specs passing.

## Project Structure

Key files involved:

```
quench/
├── reports/
│   └── checkpoint-2-cloc-works.md   # Validation report (reviewed)
├── docs/specs/
│   ├── checks/cloc.md               # CLOC specification
│   └── output.schema.json           # JSON output schema
├── crates/cli/src/
│   ├── checks/cloc.rs               # CLOC implementation
│   ├── checks/cloc_tests.rs         # Unit tests
│   └── check.rs                     # Violation/CheckResult types
└── tests/
    ├── specs/checks/cloc.rs         # Behavioral specs (20 tests)
    └── fixtures/cloc/               # Test fixtures
```

## Dependencies

No new dependencies required.

## Implementation Phases

### Phase 1: Review Checkpoint Report for Gaps

Review `reports/checkpoint-2-cloc-works.md` and compare against spec requirements.

**Findings from Review:**

| Area | Status | Notes |
|------|--------|-------|
| Line counting | PASS | Non-blank lines counted correctly |
| Test/source separation | PASS | Pattern matching works |
| Ratio calculation | PASS | test_lines / source_lines correct |
| Violation detection | PASS | Files exceeding thresholds detected |
| Text output format | PASS | Shows violations with file, count, advice |
| JSON output format | PASS | All required fields present |
| All specs pass | PASS | 20 behavioral tests passing |

**Gap Analysis:**
- No functional gaps identified
- Minor discrepancy: plan estimated line counts vs actual (expected)

**Milestone:** Report reviewed, no critical gaps found.

**Status:** [ ] Pending

### Phase 2: Verify Line Counting Edge Cases

Add unit tests for edge cases not currently covered:

```rust
// crates/cli/src/checks/cloc_tests.rs

#[test]
fn count_nonblank_lines_crlf_endings() {
    // Windows-style line endings
    let mut file = NamedTempFile::new().unwrap();
    write!(file, "line1\r\nline2\r\n\r\nline3").unwrap();
    file.flush().unwrap();

    let count = count_nonblank_lines(file.path()).unwrap();
    assert_eq!(count, 3); // Should handle CRLF correctly
}

#[test]
fn count_nonblank_lines_mixed_endings() {
    // Mixed LF and CRLF
    let mut file = NamedTempFile::new().unwrap();
    write!(file, "line1\nline2\r\nline3\n").unwrap();
    file.flush().unwrap();

    let count = count_nonblank_lines(file.path()).unwrap();
    assert_eq!(count, 3);
}

#[test]
fn count_nonblank_lines_unicode_whitespace() {
    // Non-breaking space (U+00A0) should still be whitespace
    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, "content").unwrap();
    writeln!(file, "\u{00A0}").unwrap(); // non-breaking space only
    writeln!(file, "more").unwrap();
    file.flush().unwrap();

    let count = count_nonblank_lines(file.path()).unwrap();
    // Note: Rust's trim() handles unicode whitespace
    assert_eq!(count, 2);
}
```

**Milestone:** Edge case tests added and passing.

**Status:** [ ] Pending

### Phase 3: Verify Pattern Matching Completeness

Verify all spec patterns are covered:

| Pattern | Spec | Config Default | Unit Test |
|---------|------|----------------|-----------|
| `**/tests/**` | Yes | Yes | Yes |
| `**/test/**` | Yes | Yes | Yes |
| `**/*_test.*` | Yes | Yes | Yes |
| `**/*_tests.*` | Yes | Yes | Yes |
| `**/*.test.*` | Yes | Yes | Yes |
| `**/*.spec.*` | Yes | Yes | Yes |
| `**/test_*.*` | Yes | Yes | No |

Add missing test coverage:

```rust
// crates/cli/src/checks/cloc_tests.rs

#[test]
fn pattern_matcher_identifies_test_prefix() {
    let matcher = PatternMatcher::new(
        &["**/test_*.*".to_string()],
        &[],
    );

    let root = Path::new("/project");

    // Files with test_ prefix should match
    assert!(matcher.is_test_file(Path::new("/project/src/test_utils.rs"), root));
    assert!(matcher.is_test_file(Path::new("/project/test_helpers.py"), root));

    // Regular source files should not match
    assert!(!matcher.is_test_file(Path::new("/project/src/testing.rs"), root));
    assert!(!matcher.is_test_file(Path::new("/project/src/contest.rs"), root));
}
```

**Milestone:** All pattern matching cases have unit test coverage.

**Status:** [ ] Pending

### Phase 4: Verify JSON Structure Against Schema

Compare implementation output against `docs/specs/output.schema.json`:

**Schema Requirements for Violations:**
```json
{
  "required": ["type", "advice"],
  "properties": {
    "file": { "type": ["string", "null"] },
    "line": { "type": ["integer", "null"] },
    "type": { "type": "string" },
    "advice": { "type": "string" },
    "value": { "type": "number" },
    "threshold": { "type": "number" }
  }
}
```

**Current Implementation:**
- `file`: `Option<PathBuf>` with `skip_serializing_if = "Option::is_none"` - **Valid** (schema allows omission)
- `line`: `Option<u32>` with `skip_serializing_if = "Option::is_none"` - **Valid** (schema allows omission)
- `type`: Required string - **Present**
- `advice`: Required string - **Present**
- `value`: Optional number - **Present when threshold exceeded**
- `threshold`: Optional number - **Present when threshold exceeded**

**Verification:** No changes needed. The implementation correctly follows the schema where optional fields can be omitted.

**Milestone:** JSON structure verified against schema.

**Status:** [ ] Pending

### Phase 5: Run Full Quality Check

Execute `make check` to ensure all changes pass quality gates:

```bash
make check
```

**Expected Results:**
- `cargo fmt --all -- --check` - Pass
- `cargo clippy --all-targets --all-features -- -D warnings` - Pass
- `cargo test --all` - Pass (including new edge case tests)
- `cargo build --all` - Pass
- `./scripts/bootstrap` - Pass
- `cargo audit` - Pass
- `cargo deny check` - Pass

**Milestone:** All quality gates pass.

**Status:** [ ] Pending

## Key Implementation Details

### Line Counting Implementation

Current implementation in `cloc.rs:313-320`:

```rust
fn count_nonblank_lines(path: &Path) -> std::io::Result<usize> {
    let content = std::fs::read(path)?;
    let text = String::from_utf8(content)
        .unwrap_or_else(|e| String::from_utf8_lossy(e.as_bytes()).into_owned());

    Ok(text.lines().filter(|l| !l.trim().is_empty()).count())
}
```

This correctly:
- Handles UTF-8 with lossy fallback for encoding issues
- Uses `lines()` which handles both `\n` and `\r\n`
- Uses `trim()` which handles unicode whitespace (U+0020, U+00A0, etc.)
- Filters empty/whitespace-only lines

### Pattern Matching Implementation

The `PatternMatcher` struct correctly uses `globset` for efficient pattern matching:

```rust
impl PatternMatcher {
    fn is_test_file(&self, path: &Path, root: &Path) -> bool {
        let relative = path.strip_prefix(root).unwrap_or(path);
        self.test_patterns.is_match(relative)
    }
}
```

### JSON Output Fields

Violation fields emitted by CLOC:
- `file`: File path relative to project root
- `type`: Always `"file_too_large"`
- `advice`: Source or test advice from config
- `value`: Actual line/token count
- `threshold`: Configured limit

## Verification Plan

1. **Review report:**
   ```bash
   cat reports/checkpoint-2-cloc-works.md | head -50
   ```

2. **Add edge case tests:**
   ```bash
   cargo test --lib -- cloc_tests --nocapture
   ```

3. **Verify pattern coverage:**
   ```bash
   cargo test --lib -- pattern_matcher --nocapture
   ```

4. **Validate JSON output:**
   ```bash
   ./target/release/quench check tests/fixtures/cloc/oversized-source --cloc -o json | \
     python3 -c "import sys, json; json.load(sys.stdin)"
   ```

5. **Run full quality check:**
   ```bash
   make check
   ```

## Summary

| Task | Status |
|------|--------|
| Review checkpoint report | [ ] Pending |
| Add line counting edge case tests | [ ] Pending |
| Add pattern matching test coverage | [ ] Pending |
| Verify JSON structure against schema | [ ] Pending |
| Run full quality check | [ ] Pending |

## Notes

The checkpoint-2b validation report shows CLOC is working correctly with all 20 behavioral specs passing. This refactor checkpoint focuses on:

1. **Defensive testing** - Adding edge case coverage for line counting
2. **Pattern completeness** - Ensuring all spec patterns have unit tests
3. **Schema compliance** - Verifying JSON matches `output.schema.json`

No functional changes are required based on the report review.
