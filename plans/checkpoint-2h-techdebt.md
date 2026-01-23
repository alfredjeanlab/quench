# Checkpoint 2H: Tech Debt - CLOC Test DRY-up

**Root Feature:** `quench-fda7`

## Overview

Refactor CLOC tests to eliminate repetitive patterns and improve maintainability. The CLOC check has comprehensive test coverage (~770 lines across unit tests and behavioral specs), but the tests contain significant duplication that can be consolidated using parameterized tests and shared utilities.

**Cleanup targets identified:**

| Category | Location | Current Lines | Impact |
|----------|----------|---------------|--------|
| File metrics tests | `cloc_tests.rs` L61-155 | 95 lines | ~60 lines reduction |
| Token counting tests | `cloc_tests.rs` L238-268 | 31 lines | ~20 lines reduction |
| Pattern matcher tests | `cloc_tests.rs` L161-232 | 72 lines | ~40 lines reduction |
| Shared utilities | `test_utils.rs` | 47 lines | +20 lines (new helpers) |

**Total estimated impact:** ~100 lines reduction, improved readability, better test maintainability.

## Project Structure

```
quench/
├── crates/cli/src/
│   ├── test_utils.rs         # Add file metrics test helpers
│   └── checks/
│       └── cloc_tests.rs     # Refactor to use yare and utilities
└── plans/
    └── checkpoint-2h-techdebt.md
```

## Dependencies

No new dependencies. Uses existing:
- `yare = "3"` (already in dev-dependencies)
- `tempfile = "3"` (already in dev-dependencies)

## Implementation Phases

### Phase 1: Add Test Utilities for File Metrics

Add helper functions to `test_utils.rs` for creating temp files with specific content and asserting file metrics.

**New helpers in `test_utils.rs`:**

```rust
use std::io::Write;
use tempfile::NamedTempFile;

/// Creates a temp file with the given content for testing.
///
/// Returns the NamedTempFile which keeps the file alive.
pub fn temp_file_with_content(content: &str) -> NamedTempFile {
    let mut file = NamedTempFile::new().unwrap();
    write!(file, "{}", content).unwrap();
    file.flush().unwrap();
    file
}

/// Creates a temp file with content using writeln! for each line.
///
/// Useful for tests that need explicit newlines.
pub fn temp_file_with_lines(lines: &[&str]) -> NamedTempFile {
    let mut file = NamedTempFile::new().unwrap();
    for line in lines {
        writeln!(file, "{}", line).unwrap();
    }
    file.flush().unwrap();
    file
}
```

**Milestone:** New utility functions available for file-based tests.

**Verification:**
```bash
cargo test -p quench -- test_utils
cargo clippy -p quench -- -D warnings
```

---

### Phase 2: Convert File Metrics Tests to Parameterized Tests

The 9 file metrics tests (lines 61-155) all follow the same pattern:
1. Create temp file
2. Write specific content
3. Call `count_file_metrics()`
4. Assert `nonblank_lines` equals expected value

**Current pattern (repeated 9 times):**
```rust
#[test]
fn file_metrics_empty_file() {
    let mut file = NamedTempFile::new().unwrap();
    file.flush().unwrap();
    let metrics = count_file_metrics(file.path()).unwrap();
    assert_eq!(metrics.nonblank_lines, 0);
    assert_eq!(metrics.tokens, 0);
}
```

**Refactored using yare:**
```rust
use yare::parameterized;

#[parameterized(
    empty_file = { "", 0 },
    whitespace_only = { "   \n\t\t\n\n    \t  \n", 0 },
    mixed_content = { "fn main() {\n\n    let x = 1;\n\n}\n", 3 },
    no_trailing_newline = { "line1\nline2\nline3", 3 },
    with_trailing_newline = { "line1\nline2\nline3\n", 3 },
    crlf_endings = { "line1\r\nline2\r\n\r\nline3", 3 },
    mixed_endings = { "line1\nline2\r\nline3\n", 3 },
    unicode_whitespace = { "content\n\u{00A0}\nmore\n", 2 },
)]
fn file_metrics_nonblank_lines(content: &str, expected: usize) {
    let file = temp_file_with_content(content);
    let metrics = count_file_metrics(file.path()).unwrap();
    assert_eq!(
        metrics.nonblank_lines, expected,
        "content: {:?} should have {} nonblank lines",
        content, expected
    );
}
```

**Milestone:** 9 tests consolidated into 1 parameterized test (~60 lines → ~20 lines).

**Verification:**
```bash
cargo test -p quench -- file_metrics_nonblank
```

---

### Phase 3: Convert Token Counting Tests to Parameterized Tests

The 3 token counting tests (lines 238-268) follow the same pattern:
1. Create temp file with content
2. Assert `metrics.tokens` equals expected value

**Refactored using yare:**
```rust
#[parameterized(
    short_content = { "abc", 0 },           // 3 chars < 4
    exact_hundred = { &"a".repeat(100), 25 }, // 100 / 4 = 25
    unicode_chars = { "日本語の", 1 },        // 4 Unicode chars / 4 = 1
)]
fn file_metrics_tokens(content: &str, expected: usize) {
    let file = temp_file_with_content(content);
    let metrics = count_file_metrics(file.path()).unwrap();
    assert_eq!(
        metrics.tokens, expected,
        "content {:?} should have {} tokens",
        content, expected
    );
}
```

**Note:** The `exact_hundred` case requires using `String` for the repeat. Alternative approach:

```rust
#[test]
fn file_metrics_tokens_exact_math() {
    let file = temp_file_with_content(&"a".repeat(100));
    let metrics = count_file_metrics(file.path()).unwrap();
    assert_eq!(metrics.tokens, 25);
}

#[parameterized(
    short = { "abc", 0 },
    unicode = { "日本語の", 1 },
)]
fn file_metrics_tokens_simple(content: &str, expected: usize) {
    let file = temp_file_with_content(content);
    let metrics = count_file_metrics(file.path()).unwrap();
    assert_eq!(metrics.tokens, expected);
}
```

**Milestone:** 3 tests consolidated into 1-2 parameterized tests (~31 lines → ~15 lines).

**Verification:**
```bash
cargo test -p quench -- file_metrics_tokens
```

---

### Phase 4: Consolidate Pattern Matcher Tests

The 4 pattern matcher tests (lines 161-232) each create a `PatternMatcher` and test various paths against it. These can be combined using a table-driven approach.

**Current structure:**
```rust
#[test]
fn pattern_matcher_identifies_test_directories() {
    let matcher = PatternMatcher::new(&["**/tests/**", "**/test/**"], &[]);
    let root = Path::new("/project");
    assert!(matcher.is_test_file(Path::new("/project/tests/foo.rs"), root));
    assert!(!matcher.is_test_file(Path::new("/project/src/lib.rs"), root));
}
```

**Refactored approach:**

Create test data arrays and use `yare` with tuples:

```rust
#[parameterized(
    test_dirs_matches_tests = {
        &["**/tests/**", "**/test/**"],
        "/project/tests/foo.rs",
        true
    },
    test_dirs_matches_nested = {
        &["**/tests/**", "**/test/**"],
        "/project/crate/tests/test.rs",
        true
    },
    test_dirs_excludes_src = {
        &["**/tests/**", "**/test/**"],
        "/project/src/lib.rs",
        false
    },
    suffix_matches_test_rs = {
        &["**/*_test.*", "**/*_tests.*"],
        "/project/src/foo_test.rs",
        true
    },
    suffix_excludes_testing = {
        &["**/*_test.*", "**/*_tests.*"],
        "/project/src/testing.rs",
        false
    },
    prefix_matches_test_utils = {
        &["**/test_*.*"],
        "/project/src/test_utils.rs",
        true
    },
    prefix_excludes_contest = {
        &["**/test_*.*"],
        "/project/src/contest.rs",
        false
    },
)]
fn pattern_matcher_test_file(patterns: &[&str], path: &str, expected: bool) {
    let owned: Vec<String> = patterns.iter().map(|s| s.to_string()).collect();
    let matcher = PatternMatcher::new(&owned, &[]);
    let root = Path::new("/project");
    assert_eq!(
        matcher.is_test_file(Path::new(path), root),
        expected,
        "path {} with patterns {:?} should be {}",
        path,
        patterns,
        if expected { "test" } else { "non-test" }
    );
}

#[parameterized(
    excludes_generated = { "**/generated/**", "/project/generated/foo.rs", true },
    excludes_nested = { "**/generated/**", "/project/src/generated/bar.rs", true },
    allows_regular = { "**/generated/**", "/project/src/lib.rs", false },
)]
fn pattern_matcher_exclusion(pattern: &str, path: &str, expected: bool) {
    let matcher = PatternMatcher::new(&[], &[pattern.to_string()]);
    let root = Path::new("/project");
    assert_eq!(
        matcher.is_excluded(Path::new(path), root),
        expected,
        "path {} with exclude pattern {} should be {}",
        path,
        pattern,
        if expected { "excluded" } else { "included" }
    );
}
```

**Milestone:** 4 tests consolidated into 2 parameterized tests (~72 lines → ~40 lines).

**Verification:**
```bash
cargo test -p quench -- pattern_matcher
```

---

### Phase 5: Final Cleanup and Verification

1. **Remove any now-unused imports** in `cloc_tests.rs`
2. **Ensure consistent formatting** with `cargo fmt`
3. **Run full quality gates**

**Verification checklist:**
- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --all-targets --all-features -- -D warnings`
- [ ] `cargo test --all`
- [ ] `cargo build --all`
- [ ] `./scripts/bootstrap`
- [ ] `cargo audit`
- [ ] `cargo deny check`

**Run `make check` to verify all gates pass.**

**Milestone:** All quality gates pass, CLOC tests are DRY.

## Key Implementation Details

### Why Use yare Over test_case?

The project already uses `yare` (version 3) as seen in:
- `walker_tests.rs` line 7
- `error_tests.rs` line 3

Yare advantages:
- Already a dependency (no version conflicts)
- Clean syntax for named cases
- Good error messages on failure

### Parameterized Test Naming

Yare generates test names from the case labels:
```
test file_metrics_nonblank_lines::empty_file
test file_metrics_nonblank_lines::whitespace_only
test file_metrics_nonblank_lines::mixed_content
```

Use descriptive snake_case names for clarity in test output.

### Preserving Test Intent

Each consolidated test should maintain:
1. **Clear case names** - Describe what's being tested
2. **Assertion messages** - Include context for failures
3. **Coverage** - All original test cases preserved

### No Behavior Changes

This refactoring should produce **no observable behavior changes**:
- Same test coverage
- Same assertions
- Same functionality tested
- Improved readability and maintainability

## Verification Plan

1. **Before any changes** - verify test count:
   ```bash
   cargo test -p quench -- cloc 2>&1 | grep "test result"
   # Note the number of tests
   ```

2. **After all changes** - verify same number of test cases:
   ```bash
   cargo test -p quench -- cloc 2>&1 | grep "test result"
   # Parameterized tests expand to same number of test cases
   ```

3. **Full quality gates:**
   ```bash
   make check
   ```

## Summary

| Phase | Task | Impact | Status |
|-------|------|--------|--------|
| 1 | Add test utilities for file metrics | +20 lines | [ ] Pending |
| 2 | Convert file metrics tests to parameterized | -40 lines | [ ] Pending |
| 3 | Convert token counting tests to parameterized | -16 lines | [ ] Pending |
| 4 | Consolidate pattern matcher tests | -32 lines | [ ] Pending |
| 5 | Final cleanup and verification | 0 lines | [ ] Pending |

## Notes

- Total expected reduction: ~88 lines of test code
- No new dependencies
- No behavior changes
- Improved test maintainability and readability
- Consistent with existing test patterns in `walker_tests.rs` and `error_tests.rs`
