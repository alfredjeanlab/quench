# Checkpoint 8G: Bug Fixes - Tests Correlation

**Root Feature:** `quench-03b8`

## Overview

This checkpoint addresses bugs and edge cases identified in the tests correlation feature. While checkpoint 8B validated core functionality and 8F added multi-language support, code review revealed several edge cases that could cause incorrect behavior or failures in specific scenarios.

**Key bug fixes:**

1. **Initial commit handling** - `get_commit_changes()` fails on initial commits
2. **Placeholder detection edge cases** - Non-standard Rust attribute formatting not recognized
3. **Test-only filter logic asymmetry** - Single-source path differs from multi-source path
4. **JavaScript placeholder regex gaps** - Escaped quotes and template literals not handled
5. **Bidirectional matching edge cases** - Source files with test-like names misclassified

## Project Structure

```
quench/
├── crates/cli/src/checks/tests/
│   ├── diff.rs              # FIX: Initial commit handling
│   ├── diff_tests.rs        # ADD: Edge case tests
│   ├── correlation.rs       # FIX: Test-only filter, bidirectional matching
│   ├── correlation_tests.rs # ADD: Edge case tests
│   ├── placeholder.rs       # FIX: Attribute order, whitespace handling
│   └── placeholder_tests.rs # ADD: Edge case tests
├── tests/specs/checks/tests/
│   └── edge_cases.rs        # NEW: Behavioral tests for edge cases
└── reports/
    └── checkpoint-8g-bugfix.md  # Summary of fixes
```

## Dependencies

No new external dependencies. This is a bug-fix checkpoint using existing infrastructure.

## Implementation Phases

### Phase 1: Initial Commit Handling in Commit Scope

**Goal:** Gracefully handle the initial commit when using commit-scope checking.

**Bug:** In `diff.rs:220-223`, using `hash^..hash` fails for the initial commit because the parent commit doesn't exist. Git returns an error.

**File:** `crates/cli/src/checks/tests/diff.rs`

**Current code:**
```rust
fn get_commit_changes(root: &Path, commit_hash: &str) -> Result<Vec<FileChange>, String> {
    // Use hash^..hash to get changes in that specific commit
    // Note: This won't work for the initial commit, but for feature branches
    // comparing to main, we don't need to handle the initial commit.
    let range = format!("{}^..{}", commit_hash, commit_hash);
    // ...
}
```

**Fix:** Detect if the commit has a parent and use an empty tree for the initial commit.

```rust
fn get_commit_changes(root: &Path, commit_hash: &str) -> Result<Vec<FileChange>, String> {
    // Check if this is the initial commit (no parent)
    let has_parent = Command::new("git")
        .args(["rev-parse", "--verify", &format!("{}^", commit_hash)])
        .current_dir(root)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    let range = if has_parent {
        format!("{}^..{}", commit_hash, commit_hash)
    } else {
        // For initial commit, compare against empty tree
        // 4b825dc642cb6eb9a060e54bf8d69288fbee4904 is git's empty tree SHA
        format!("4b825dc642cb6eb9a060e54bf8d69288fbee4904..{}", commit_hash)
    };

    let numstat = run_git_diff(root, &["--numstat", &range])?;
    let name_status = run_git_diff(root, &["--name-status", &range])?;

    merge_diff_outputs(&numstat, &name_status, root)
}
```

**Tests to add** (`diff_tests.rs`):
```rust
#[test]
fn get_commit_changes_handles_initial_commit() {
    // Create temp repo with single initial commit
    // Verify changes are extracted correctly
}
```

**Verification:**
```bash
cargo test --lib -- diff::get_commit_changes
```

### Phase 2: Rust Placeholder Detection Edge Cases

**Goal:** Handle non-standard attribute formatting in Rust placeholder tests.

**Bug:** `placeholder.rs:54-93` uses strict line-by-line parsing that fails on:
- Attributes in reverse order (`#[ignore]` before `#[test]`)
- Non-standard whitespace (`#[ test ]` instead of `#[test]`)
- Multi-line ignore reasons (`#[ignore = "TODO:\n    details"]`)

**File:** `crates/cli/src/checks/tests/placeholder.rs`

**Current code:**
```rust
if trimmed == "#[test]" {  // Exact match only
    saw_test_attr = true;
    // ...
}
```

**Fix:** Use more flexible pattern matching:

```rust
/// Parse Rust test file for placeholder tests (#[test] #[ignore]).
fn find_rust_placeholders(content: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut saw_test_attr = false;
    let mut saw_ignore_attr = false;

    for line in content.lines() {
        let trimmed = line.trim();

        // More flexible #[test] detection (handles whitespace variations)
        if is_test_attribute(trimmed) {
            saw_test_attr = true;
            saw_ignore_attr = false;
            continue;
        }

        // More flexible #[ignore] detection
        if saw_test_attr && is_ignore_attribute(trimmed) {
            saw_ignore_attr = true;
            continue;
        }

        // Also accept #[ignore] before #[test] (reverse order)
        if is_ignore_attribute(trimmed) && !saw_test_attr {
            saw_ignore_attr = true;
            continue;
        }

        if is_test_attribute(trimmed) && saw_ignore_attr {
            saw_test_attr = true;
            continue;
        }

        if saw_test_attr
            && saw_ignore_attr
            && trimmed.starts_with("fn ")
            && let Some(name_part) = trimmed.strip_prefix("fn ")
            && let Some(name) = name_part.split('(').next()
        {
            result.push(name.to_string());
            saw_test_attr = false;
            saw_ignore_attr = false;
            continue;
        }

        // Reset if we see something else
        if !trimmed.starts_with('#') && !trimmed.is_empty() {
            saw_test_attr = false;
            saw_ignore_attr = false;
        }
    }

    result
}

/// Check if a line is a #[test] attribute (with whitespace tolerance).
fn is_test_attribute(trimmed: &str) -> bool {
    // Remove all whitespace for comparison
    let normalized: String = trimmed.chars().filter(|c| !c.is_whitespace()).collect();
    normalized == "#[test]"
}

/// Check if a line starts an #[ignore...] attribute.
fn is_ignore_attribute(trimmed: &str) -> bool {
    let normalized: String = trimmed.chars().filter(|c| !c.is_whitespace()).collect();
    normalized.starts_with("#[ignore")
}
```

**Tests to add** (`placeholder_tests.rs`):
```rust
#[test]
fn finds_placeholder_with_whitespace_in_attribute() {
    let content = r#"
#[ test ]
#[ignore = "TODO"]
fn test_parser() {}
"#;
    let result = find_rust_placeholders(content);
    assert!(result.contains(&"test_parser".to_string()));
}

#[test]
fn finds_placeholder_with_reversed_attribute_order() {
    let content = r#"
#[ignore = "TODO"]
#[test]
fn test_parser() {}
"#;
    let result = find_rust_placeholders(content);
    assert!(result.contains(&"test_parser".to_string()));
}
```

**Verification:**
```bash
cargo test --lib -- placeholder::find_rust
```

### Phase 3: Test-Only Filter Logic Simplification

**Goal:** Ensure consistent test-only filtering between single-source and multi-source paths.

**Bug:** `analyze_single_source()` at `correlation.rs:421-434` and `analyze_correlation()` at `correlation.rs:282-296` use slightly different logic for determining test-only changes.

**File:** `crates/cli/src/checks/tests/correlation.rs`

**Fix:** Extract common logic into a shared helper:

```rust
/// Check if a test file is considered "test-only" (no corresponding source change).
///
/// A test is test-only if its base name doesn't match any source file's base name,
/// even when accounting for common test suffixes/prefixes.
fn is_test_only(test_base: &str, source_base_names: &HashSet<String>) -> bool {
    // Direct match
    if source_base_names.contains(test_base) {
        return false;
    }

    // Test has suffix that matches source
    // e.g., "parser_test" matches source "parser"
    for suffix in ["_test", "_tests"] {
        if let Some(stripped) = test_base.strip_suffix(suffix) {
            if source_base_names.contains(stripped) {
                return false;
            }
        }
    }

    // Test has prefix that matches source
    // e.g., "test_parser" matches source "parser"
    if let Some(stripped) = test_base.strip_prefix("test_") {
        if source_base_names.contains(stripped) {
            return false;
        }
    }

    // Source has suffix that matches test
    // e.g., source "parser" matches test "parser_test"
    for source in source_base_names {
        if *source == format!("{}_test", test_base)
            || *source == format!("{}_tests", test_base)
            || *source == format!("test_{}", test_base)
        {
            return false;
        }
    }

    true
}
```

Update both call sites to use this shared helper, ensuring consistency.

**Tests to add** (`correlation_tests.rs`):
```rust
#[test]
fn test_only_filter_single_source_matches_multi_source() {
    // Verify same test files are identified as test-only
    // in both single-source and multi-source paths
    let root = Path::new("/project");

    // Single source case
    let single_changes = vec![
        make_change("/project/src/parser.rs", ChangeType::Modified),
        make_change("/project/tests/other_tests.rs", ChangeType::Modified),
    ];

    // Multi source case (add another source)
    let multi_changes = vec![
        make_change("/project/src/parser.rs", ChangeType::Modified),
        make_change("/project/src/lexer.rs", ChangeType::Modified),
        make_change("/project/tests/other_tests.rs", ChangeType::Modified),
    ];

    let config = CorrelationConfig::default();
    let single_result = analyze_correlation(&single_changes, &config, root);
    let multi_result = analyze_correlation(&multi_changes, &config, root);

    // Both should identify other_tests.rs as test-only
    assert_eq!(single_result.test_only.len(), 1);
    assert_eq!(multi_result.test_only.len(), 1);
}
```

**Verification:**
```bash
cargo test --lib -- correlation::test_only
```

### Phase 4: JavaScript Placeholder Regex Improvements

**Goal:** Handle more JavaScript/TypeScript placeholder syntax variations.

**Bug:** The regex at `placeholder.rs:44` doesn't handle:
- Escaped quotes: `test.todo('doesn\'t work')`
- Template literals with expressions: `` test.todo(`name ${var}`) ``

**File:** `crates/cli/src/checks/tests/placeholder.rs`

**Current regex:**
```rust
r#"(?:test|it|describe)\.(todo|skip)\s*\(\s*['"`]([^'"`]+)['"`]"#
```

**Fix:** Use a more robust regex with negative lookbehind for escaped quotes:

```rust
/// Parse JS/TS test file for test.todo(), it.todo(), test.skip(), etc.
pub fn find_js_placeholders(content: &str) -> Vec<String> {
    use regex::Regex;
    use std::sync::OnceLock;

    // Pattern explanation:
    // - (?:test|it|describe)\.(todo|skip) - Match test method
    // - \s*\(\s* - Opening paren with optional whitespace
    // - (['"`]) - Capture the quote type
    // - ((?:[^'"`\\]|\\.)*)  - Capture content, handling escaped chars
    // - \3 - Match closing quote of same type
    static PAT: OnceLock<Option<Regex>> = OnceLock::new();
    let pat = PAT.get_or_init(|| {
        Regex::new(
            r#"(?:test|it|describe)\.(todo|skip)\s*\(\s*(['"`])((?:[^'"`\\]|\\.)*)\2"#
        ).ok()
    });

    pat.as_ref().map_or_else(Vec::new, |re| {
        re.captures_iter(content)
            .filter_map(|c| c.get(3).map(|m| {
                // Unescape the captured string
                m.as_str()
                    .replace("\\'", "'")
                    .replace("\\\"", "\"")
                    .replace("\\`", "`")
            }))
            .collect()
    })
}
```

**Tests to add** (`placeholder_tests.rs`):
```rust
#[test]
fn finds_js_placeholder_with_escaped_quotes() {
    let content = r#"test.todo('doesn\'t break on escaped quotes');"#;
    let result = find_js_placeholders(content);
    assert_eq!(result.len(), 1);
    assert!(result[0].contains("doesn't"));
}

#[test]
fn finds_js_placeholder_with_double_quotes() {
    let content = r#"test.todo("parser \"quoted\" test");"#;
    let result = find_js_placeholders(content);
    assert_eq!(result.len(), 1);
}

#[test]
fn finds_multiple_js_placeholders() {
    let content = r#"
test.todo('first test');
it.skip('second test', () => {});
describe.todo('third group');
"#;
    let result = find_js_placeholders(content);
    assert_eq!(result.len(), 3);
}
```

**Verification:**
```bash
cargo test --lib -- placeholder::js
```

### Phase 5: Bidirectional Test Matching Edge Cases

**Goal:** Handle source files with test-like names correctly.

**Bug:** `TestIndex::has_test_for()` at `correlation.rs:111-126` assumes tests have suffixes, not sources. A source file named `parser_test_utils.rs` would have base name `parser_test_utils` which wouldn't match a test named `parser_test_utils_tests.rs`.

**File:** `crates/cli/src/checks/tests/correlation.rs`

**Fix:** The current logic is actually correct for common cases, but we should add documentation and tests to clarify edge cases:

```rust
impl TestIndex {
    /// O(1) check for correlated test by base name.
    ///
    /// Matching strategy:
    /// 1. Direct match: source "parser" matches test "parser"
    /// 2. Test suffix: source "parser" matches test "parser_test" or "parser_tests"
    /// 3. Test prefix: source "parser" matches test "test_parser"
    ///
    /// Note: Source files with test-like names (e.g., "test_utils.rs") are handled
    /// correctly because the source base name "test_utils" would need a test with
    /// base name "test_utils", "test_utils_test", etc.
    pub fn has_test_for(&self, source_path: &Path) -> bool {
        // ... existing implementation ...
    }
}
```

**Tests to add** (`correlation_tests.rs`):
```rust
#[test]
fn source_with_test_like_name_correlates_correctly() {
    // Source file named test_utils.rs should correlate with test_utils_tests.rs
    let root = Path::new("/project");
    let changes = vec![
        make_change("/project/src/test_utils.rs", ChangeType::Modified),
        make_change("/project/tests/test_utils_tests.rs", ChangeType::Modified),
    ];

    let config = CorrelationConfig::default();
    let result = analyze_correlation(&changes, &config, root);

    assert_eq!(result.with_tests.len(), 1);
    assert_eq!(result.without_tests.len(), 0);
}

#[test]
fn source_with_test_suffix_requires_matching_test() {
    // Source file named parser_test.rs (not a test file, but source)
    // should require a test named parser_test_tests.rs
    let root = Path::new("/project");
    let changes = vec![
        make_change("/project/src/parser_test.rs", ChangeType::Modified),
    ];

    let config = CorrelationConfig::default();
    let result = analyze_correlation(&changes, &config, root);

    // Should be in without_tests since no matching test exists
    assert_eq!(result.without_tests.len(), 1);
}
```

**Verification:**
```bash
cargo test --lib -- correlation::source_with_test
```

### Phase 6: Final Verification and Documentation

**Goal:** Ensure all bug fixes work together and update documentation.

**Steps:**

1. Run full test suite to verify no regressions
2. Add behavioral tests for edge cases in `tests/specs/checks/tests/edge_cases.rs`
3. Update validation report

**File:** `tests/specs/checks/tests/edge_cases.rs`

```rust
//! Edge case behavioral tests for tests correlation check.

#[test]
fn commit_scope_handles_initial_commit() {
    // Create fixture repo where base..HEAD includes initial commit
    // Verify check runs without error
}

#[test]
fn placeholder_with_nonstandard_formatting_recognized() {
    // Create fixture with whitespace in attributes
    // Verify placeholder is recognized
}

#[test]
fn js_placeholder_with_escaped_quotes_recognized() {
    // Create fixture with escaped quotes
    // Verify placeholder is recognized
}
```

**Verification:**
```bash
# Full CI check
make check

# Specific edge case tests
cargo test --test specs edge_cases

# Dogfooding
cargo run -- check
```

## Key Implementation Details

### Git Empty Tree SHA

The SHA `4b825dc642cb6eb9a060e54bf8d69288fbee4904` is a well-known constant representing git's empty tree. It's used to diff against when there's no parent commit.

### Attribute Order in Rust

Rust allows attributes in any order, so both of these are valid:
```rust
#[test]
#[ignore = "TODO"]
fn test_foo() {}

#[ignore = "TODO"]
#[test]
fn test_foo() {}
```

The parser should recognize both patterns.

### Regex Capture Group Backreferences

In the JavaScript placeholder regex, `\2` backreferences the quote type captured by group 2, ensuring the closing quote matches the opening quote style.

### Test-Only Consistency

The test-only filter logic must be identical in both `analyze_single_source()` and `analyze_correlation()`. Extracting to a shared helper ensures this and makes testing easier.

## Verification Plan

### Phase 1 Verification
```bash
# Unit tests for initial commit handling
cargo test --lib -- diff::initial_commit
# Should pass without git errors
```

### Phase 2 Verification
```bash
# Placeholder detection with variations
cargo test --lib -- placeholder::whitespace
cargo test --lib -- placeholder::reversed_order
```

### Phase 3 Verification
```bash
# Test-only filter consistency
cargo test --lib -- correlation::test_only_filter
```

### Phase 4 Verification
```bash
# JavaScript placeholder edge cases
cargo test --lib -- placeholder::js_escaped
cargo test --lib -- placeholder::js_multiple
```

### Phase 5 Verification
```bash
# Bidirectional matching
cargo test --lib -- correlation::source_with_test
```

### Phase 6 (Final) Verification
```bash
# Full CI
make check

# All edge case tests
cargo test --test specs edge_cases

# Dogfooding
cargo run -- check
```

## Exit Criteria

- [ ] Initial commit handling works without error
- [ ] Rust placeholder detection handles whitespace and attribute order
- [ ] Test-only filter logic is consistent between code paths
- [ ] JavaScript placeholders handle escaped quotes
- [ ] Source files with test-like names correlate correctly
- [ ] All tests pass: `make check`
- [ ] Dogfooding passes: `quench check` on quench
