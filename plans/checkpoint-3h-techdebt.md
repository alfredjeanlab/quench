# Checkpoint 3H: Tech Debt - Escapes Tests DRY-up

**Root Feature:** `quench-fda7`

## Overview

Refactor escapes tests to eliminate repetitive patterns using `yare` parameterized tests and extract common pattern matching test utilities. The escapes tests (361 lines in `escapes_tests.rs`) contain four modules with highly repetitive test patterns that follow identical structures.

**Cleanup targets identified:**

| Category | Location | Tests | Impact |
|----------|----------|-------|--------|
| Comment detection tests | `escapes_tests.rs` L8-46 | 6 tests | ~25 lines reduction |
| Is comment line tests | `escapes_tests.rs` L48-74 | 4 tests | ~15 lines reduction |
| Comment boundary tests | `escapes_tests.rs` L76-124 | 6 tests | ~30 lines reduction |
| Strip comment markers tests | `escapes_tests.rs` L126-151 | 4 tests | ~15 lines reduction |
| Pattern matching utilities | `test_utils.rs` | - | +15 lines (new helpers) |

**Total estimated impact:** ~70 lines reduction, improved readability, better test maintainability.

## Project Structure

```
quench/
├── crates/cli/src/
│   ├── test_utils.rs              # Add pattern matching test helpers
│   └── checks/
│       └── escapes_tests.rs       # Refactor to use yare
└── plans/
    └── checkpoint-3h-techdebt.md
```

## Dependencies

No new dependencies. Uses existing:
- `yare = "3"` (already in dev-dependencies)
- `tempfile = "3"` (already in dev-dependencies)

## Implementation Phases

### Phase 1: Convert Comment Detection Tests to Parameterized Tests

The 6 tests in `comment_detection` module (lines 8-46) all test `has_justification_comment()` with different inputs.

**Current pattern (repeated 6 times):**
```rust
#[test]
fn finds_comment_on_same_line() {
    let content = "unsafe { code } // SAFETY: reason";
    assert!(has_justification_comment(content, 1, "// SAFETY:"));
}
```

**Refactored using yare:**
```rust
use yare::parameterized;

#[parameterized(
    same_line = { "unsafe { code } // SAFETY: reason", 1, true },
    preceding_line = { "// SAFETY: reason\nunsafe { code }", 2, true },
    through_blank_lines = { "// SAFETY: reason\n\nunsafe { code }", 3, true },
    through_other_comments = { "// SAFETY: reason\n// more context\nunsafe { code }", 3, true },
    stops_at_code_line = { "// SAFETY: old\nfn other() {}\nunsafe { code }", 3, false },
    no_comment_returns_false = { "unsafe { code }", 1, false },
)]
fn has_justification_comment_cases(content: &str, line: usize, expected: bool) {
    assert_eq!(
        has_justification_comment(content, line, "// SAFETY:"),
        expected,
        "content {:?} at line {} should {} have justification",
        content, line, if expected { "" } else { "not" }
    );
}
```

**Milestone:** 6 tests consolidated into 1 parameterized test (~38 lines → ~15 lines).

**Verification:**
```bash
cargo test -p quench -- has_justification_comment_cases
```

---

### Phase 2: Convert Is Comment Line Tests to Parameterized Tests

The 4 tests in `is_comment_line_tests` module (lines 48-74) all test `is_comment_line()`.

**Current pattern:**
```rust
#[test]
fn c_style_single() {
    assert!(is_comment_line("// comment"));
    assert!(is_comment_line("  // indented"));
}
```

**Refactored using yare:**
```rust
#[parameterized(
    c_style_single = { "// comment", true },
    c_style_indented = { "  // indented", true },
    c_style_block = { "/* block */", true },
    block_continuation = { " * continuation", true },
    shell_style = { "# comment", true },
    shell_indented = { "  # indented", true },
    code_fn = { "fn main() {}", false },
    code_let = { "let x = 1;", false },
)]
fn is_comment_line_cases(input: &str, expected: bool) {
    assert_eq!(
        is_comment_line(input),
        expected,
        "input {:?} should {} be a comment line",
        input, if expected { "" } else { "not" }
    );
}
```

**Milestone:** 4 tests (8 assertions) consolidated into 1 parameterized test (~26 lines → ~15 lines).

**Verification:**
```bash
cargo test -p quench -- is_comment_line_cases
```

---

### Phase 3: Convert Comment Boundary Tests to Parameterized Tests

The 6 tests in `comment_boundary_tests` module (lines 76-124) test edge cases for `has_justification_comment()`.

**Refactored using yare:**
```rust
#[parameterized(
    ignores_embedded_patterns = {
        "code  // VIOLATION: missing // SAFETY: comment\nmore code",
        1,
        false
    },
    finds_standalone_pattern = {
        "// SAFETY: this is safe\nunsafe { *ptr }",
        2,
        true
    },
    finds_pattern_on_same_line = {
        "unsafe { *ptr }  // SAFETY: this is safe",
        1,
        true
    },
    extra_text_after_pattern = {
        "// SAFETY: reason here // more notes",
        1,
        true
    },
    embedded_at_end_does_not_match = {
        "code // error message about // SAFETY:",
        1,
        false
    },
)]
fn comment_boundary_cases(content: &str, line: usize, expected: bool) {
    assert_eq!(
        has_justification_comment(content, line, "// SAFETY:"),
        expected,
        "content {:?} at line {} should {} have justification",
        content, line, if expected { "" } else { "not" }
    );
}
```

**Doc comment variants test (separate due to multiple assertions):**
```rust
#[test]
fn doc_comment_variants() {
    // Triple-slash doc comments should match
    let content = "/// SAFETY: reason\nunsafe { code }";
    assert!(has_justification_comment(content, 2, "// SAFETY:"));

    // Inner doc comments should match
    let content = "//! SAFETY: reason\nunsafe { code }";
    assert!(has_justification_comment(content, 2, "// SAFETY:"));
}
```

**Milestone:** 6 tests consolidated into 1 parameterized test + 1 regular test (~48 lines → ~25 lines).

**Verification:**
```bash
cargo test -p quench -- comment_boundary
```

---

### Phase 4: Convert Strip Comment Markers Tests to Parameterized Tests

The 4 tests in `strip_comment_markers_tests` module (lines 126-151) all test `strip_comment_markers()`.

**Current pattern:**
```rust
#[test]
fn strips_single_line_comment() {
    assert_eq!(strip_comment_markers("// SAFETY:"), "SAFETY:");
    assert_eq!(strip_comment_markers("  // SAFETY:"), "SAFETY:");
}
```

**Refactored using yare:**
```rust
#[parameterized(
    single_line = { "// SAFETY:", "SAFETY:" },
    single_line_indented = { "  // SAFETY:", "SAFETY:" },
    doc_triple_slash = { "/// SAFETY:", "SAFETY:" },
    doc_inner = { "//! SAFETY:", "SAFETY:" },
    shell_comment = { "# SAFETY:", "SAFETY:" },
)]
fn strip_comment_markers_cases(input: &str, expected: &str) {
    assert_eq!(
        strip_comment_markers(input),
        expected,
        "input {:?} should strip to {:?}",
        input, expected
    );
}
```

**Milestone:** 4 tests consolidated into 1 parameterized test (~25 lines → ~12 lines).

**Verification:**
```bash
cargo test -p quench -- strip_comment_markers_cases
```

---

### Phase 5: Extract Common Pattern Matching Test Utilities

Add helper functions to `test_utils.rs` for common pattern matching test operations used across `escapes_tests.rs`, `pattern/matcher_tests.rs`, and `cloc_tests.rs`.

**New helpers in `test_utils.rs`:**

```rust
use crate::pattern::CompiledPattern;

/// Compiles a pattern and asserts it finds expected number of matches.
///
/// Useful for parameterized tests that verify pattern matching behavior.
pub fn assert_pattern_matches(pattern: &str, content: &str, expected_count: usize) {
    let compiled = CompiledPattern::compile(pattern)
        .unwrap_or_else(|e| panic!("failed to compile pattern {:?}: {}", pattern, e));
    let matches = compiled.find_all(content);
    assert_eq!(
        matches.len(),
        expected_count,
        "pattern {:?} in {:?} should have {} matches, found {}",
        pattern,
        content,
        expected_count,
        matches.len()
    );
}

/// Compiles a pattern and asserts it matches at specific line numbers.
///
/// Line numbers are 1-indexed.
pub fn assert_pattern_at_lines(pattern: &str, content: &str, expected_lines: &[u32]) {
    let compiled = CompiledPattern::compile(pattern)
        .unwrap_or_else(|e| panic!("failed to compile pattern {:?}: {}", pattern, e));
    let matches = compiled.find_all_with_lines(content);
    let actual_lines: Vec<u32> = matches.iter().map(|m| m.line).collect();
    assert_eq!(
        actual_lines, expected_lines,
        "pattern {:?} should match at lines {:?}, found {:?}",
        pattern, expected_lines, actual_lines
    );
}
```

**Milestone:** New utility functions available for pattern matching tests.

**Verification:**
```bash
cargo test -p quench -- test_utils
cargo clippy -p quench -- -D warnings
```

---

### Phase 6: Final Cleanup and Verification

1. **Remove module wrappers** - Since parameterized tests already have descriptive names, the module wrappers (`mod comment_detection`, etc.) can be flattened
2. **Remove any now-unused imports** in `escapes_tests.rs`
3. **Ensure consistent formatting** with `cargo fmt`
4. **Run full quality gates**

**Verification checklist:**
- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --all-targets --all-features -- -D warnings`
- [ ] `cargo test --all`
- [ ] `cargo build --all`
- [ ] `./scripts/bootstrap`
- [ ] `cargo audit`
- [ ] `cargo deny check`

**Run `make check` to verify all gates pass.**

**Milestone:** All quality gates pass, escapes tests are DRY.

## Key Implementation Details

### Why Use yare Over test_case?

The project already uses `yare` (version 3) as seen in:
- `cloc_tests.rs` line 5
- `walker_tests.rs` line 7
- `error_tests.rs` line 3
- `adapter/generic_tests.rs` line 3

Yare advantages:
- Already a dependency (no version conflicts)
- Clean syntax for named cases
- Good error messages on failure

### Parameterized Test Naming

Yare generates test names from the case labels:
```
test has_justification_comment_cases::same_line
test has_justification_comment_cases::preceding_line
test is_comment_line_cases::c_style_single
```

Use descriptive snake_case names for clarity in test output.

### Preserving Test Intent

Each consolidated test should maintain:
1. **Clear case names** - Describe what's being tested
2. **Assertion messages** - Include context for failures
3. **Coverage** - All original test cases preserved

### Benchmarks Unchanged

The `benchmarks` module (lines 153-360) should remain unchanged:
- These are ignored by default (`#[ignore = "benchmark only"]`)
- They have different structure (timing loops, performance assertions)
- No benefit from parameterization

### No Behavior Changes

This refactoring should produce **no observable behavior changes**:
- Same test coverage
- Same assertions
- Same functionality tested
- Improved readability and maintainability

## Verification Plan

1. **Before any changes** - verify test count:
   ```bash
   cargo test -p quench -- escapes 2>&1 | grep "test result"
   # Note the number of tests (expect ~20 non-ignored tests)
   ```

2. **After all changes** - verify same number of test cases:
   ```bash
   cargo test -p quench -- escapes 2>&1 | grep "test result"
   # Parameterized tests expand to same number of test cases
   ```

3. **Full quality gates:**
   ```bash
   make check
   ```

## Summary

| Phase | Task | Impact | Status |
|-------|------|--------|--------|
| 1 | Convert comment detection tests | -23 lines | [ ] Pending |
| 2 | Convert is_comment_line tests | -11 lines | [ ] Pending |
| 3 | Convert comment boundary tests | -23 lines | [ ] Pending |
| 4 | Convert strip_comment_markers tests | -13 lines | [ ] Pending |
| 5 | Extract pattern matching utilities | +15 lines | [ ] Pending |
| 6 | Final cleanup and verification | 0 lines | [ ] Pending |

## Notes

- Total expected reduction: ~55 lines of test code
- No new dependencies
- No behavior changes
- Improved test maintainability and readability
- Consistent with existing test patterns in `cloc_tests.rs` and `walker_tests.rs`
- Benchmarks module left unchanged (different structure, different purpose)
