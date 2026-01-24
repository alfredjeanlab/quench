# Checkpoint 8F: Quick Wins - Tests Correlation

**Root Feature:** `quench-03b8`

## Overview

This checkpoint delivers high-value, low-risk improvements to the tests correlation check. With core functionality validated (8B), performance optimized (8E), this checkpoint focuses on polish that improves multi-language support and developer experience.

**Key improvements:**
1. **JavaScript/TypeScript placeholder detection** - Recognize `test.todo()`, `it.todo()`, `test.skip()` patterns
2. **Go test file patterns** - Add `*_test.go` to default test patterns
3. **Language-aware advice messages** - Tailor advice to the file's language
4. **Enhanced test pattern defaults** - Add `.spec.ts`, `.spec.js`, `__tests__/` patterns
5. **Documentation sync** - Ensure spec matches implementation

## Project Structure

```
quench/
├── crates/cli/src/checks/tests/
│   ├── correlation.rs           # MODIFY: Add JS placeholder detection
│   ├── correlation_tests.rs     # MODIFY: Add JS placeholder tests
│   └── mod.rs                   # MODIFY: Language-aware advice
├── docs/specs/
│   └── checks/tests.md          # UPDATE: Document JS/TS placeholders
├── tests/specs/checks/tests/
│   └── correlation.rs           # MODIFY: Add behavioral tests
└── reports/
    └── checkpoint-8f-quickwins.md  # NEW: Summary of changes
```

## Dependencies

No new external dependencies. This checkpoint uses existing infrastructure:

- `globset` - Pattern matching (exists)
- Existing diff/correlation modules

## Implementation Phases

### Phase 1: JavaScript/TypeScript Placeholder Detection

**Goal:** Recognize JS/TS placeholder tests as valid test correlation.

The spec (`docs/specs/langs/javascript.md`) documents placeholder patterns:
```typescript
test.todo('should handle edge case');
it.todo('validates input');
test.skip('temporarily disabled', () => { /* ... */ });
```

Currently, only Rust `#[ignore]` patterns are detected. Add detection for JS/TS patterns.

**File:** `crates/cli/src/checks/tests/correlation.rs`

Add JS/TS placeholder detection:

```rust
/// Parse JavaScript/TypeScript test file for placeholder tests.
///
/// Looks for patterns like:
/// - test.todo('description')
/// - it.todo('description')
/// - describe.todo('description')
/// - test.skip('description', ...)
/// - it.skip('description', ...)
fn find_js_placeholder_tests(content: &str) -> Vec<String> {
    let mut result = Vec::new();

    // Match: test.todo('name'), it.todo('name'), describe.todo('name')
    // Match: test.skip('name', ...), it.skip('name', ...)
    let todo_pattern = regex::Regex::new(
        r#"(?:test|it|describe)\.(todo|skip)\s*\(\s*['"`]([^'"`]+)['"`]"#
    ).expect("valid regex");

    for cap in todo_pattern.captures_iter(content) {
        if let Some(name) = cap.get(2) {
            result.push(name.as_str().to_string());
        }
    }

    result
}

/// Check if a JavaScript/TypeScript test file contains placeholder tests.
pub fn has_js_placeholder_test(
    test_path: &Path,
    source_base: &str,
    root: &Path,
) -> Result<bool, String> {
    let full_path = root.join(test_path);
    let content = std::fs::read_to_string(&full_path).map_err(|e| e.to_string())?;

    let placeholders = find_js_placeholder_tests(&content);

    // Check if any placeholder test name relates to the source file
    Ok(placeholders.iter().any(|test_name| {
        let normalized = test_name.to_lowercase();
        normalized.contains(&source_base.to_lowercase())
    }))
}
```

**Verification:**
```bash
cargo test --lib -- correlation::js_placeholder
# All JS placeholder tests pass
```

### Phase 2: Go Test File Pattern Defaults

**Goal:** Include Go test file patterns in defaults.

Go uses `*_test.go` for test files. Add this to the default test patterns so Go projects work out of the box.

**File:** `crates/cli/src/checks/tests/correlation.rs`

Update `CorrelationConfig::default()`:

```rust
impl Default for CorrelationConfig {
    fn default() -> Self {
        Self {
            test_patterns: vec![
                "tests/**/*".to_string(),
                "test/**/*".to_string(),
                "**/*_test.*".to_string(),
                "**/*_tests.*".to_string(),
                "**/*.spec.*".to_string(),
                // JavaScript/TypeScript specific
                "**/__tests__/**".to_string(),
                "**/*.test.*".to_string(),
                // Go specific (already covered by *_test.* but explicit for clarity)
            ],
            source_patterns: vec!["src/**/*".to_string()],
            exclude_patterns: vec![
                "**/mod.rs".to_string(),
                "**/lib.rs".to_string(),
                "**/main.rs".to_string(),
                "**/generated/**".to_string(),
            ],
        }
    }
}
```

**Note:** Go test files (`*_test.go`) are already covered by the `**/*_test.*` pattern, so no change needed for Go. However, the `**/__tests__/**` and `**/*.test.*` patterns should be added for better JS/TS support.

**Verification:**
```bash
cargo test --lib -- correlation::default_patterns
# Verify patterns match expected files
```

### Phase 3: Language-Aware Advice Messages

**Goal:** Tailor violation advice to the file's language.

Current advice always suggests Rust patterns:
```
Add tests in tests/{base}_tests.rs or update inline #[cfg(test)] block
```

For JavaScript files, this should suggest:
```
Add tests in {base}.test.ts or __tests__/{base}.test.ts
```

**File:** `crates/cli/src/checks/tests/mod.rs`

Add language detection and advice generation:

```rust
/// Detect the language of a source file.
fn detect_language(path: &Path) -> Language {
    match path.extension().and_then(|e| e.to_str()) {
        Some("rs") => Language::Rust,
        Some("go") => Language::Go,
        Some("ts" | "tsx" | "js" | "jsx" | "mjs" | "mts") => Language::JavaScript,
        Some("py") => Language::Python,
        _ => Language::Unknown,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Language {
    Rust,
    Go,
    JavaScript,
    Python,
    Unknown,
}

/// Generate language-specific advice for missing tests.
fn missing_tests_advice(file_stem: &str, lang: Language) -> String {
    match lang {
        Language::Rust => format!(
            "Add tests in tests/{}_tests.rs or update inline #[cfg(test)] block",
            file_stem
        ),
        Language::Go => format!(
            "Add tests in {}_test.go",
            file_stem
        ),
        Language::JavaScript => format!(
            "Add tests in {}.test.ts or __tests__/{}.test.ts",
            file_stem, file_stem
        ),
        Language::Python => format!(
            "Add tests in test_{}.py or tests/test_{}.py",
            file_stem, file_stem
        ),
        Language::Unknown => format!(
            "Add tests for {}",
            file_stem
        ),
    }
}
```

Update the violation generation to use language-aware advice:

```rust
// In run_branch_scope():
let lang = detect_language(&path);
let advice = missing_tests_advice(file_stem, lang);
```

**Verification:**
```bash
cargo test --lib -- tests_check::advice
# Verify advice matches expected language patterns
```

### Phase 4: Enhanced Default Patterns

**Goal:** Add missing common test patterns to defaults.

Add patterns that users commonly expect:

1. `**/*.test.*` - Jest/Vitest convention (`foo.test.ts`)
2. `**/__tests__/**` - Jest convention for test directories
3. `**/spec/**` - RSpec/Ruby convention (also used by some JS projects)

**File:** `crates/cli/src/checks/tests/correlation.rs`

```rust
impl Default for CorrelationConfig {
    fn default() -> Self {
        Self {
            test_patterns: vec![
                // Directory-based patterns
                "tests/**/*".to_string(),
                "test/**/*".to_string(),
                "spec/**/*".to_string(),
                "**/__tests__/**".to_string(),
                // Suffix patterns
                "**/*_test.*".to_string(),
                "**/*_tests.*".to_string(),
                "**/*.test.*".to_string(),
                "**/*.spec.*".to_string(),
                // Prefix patterns
                "**/test_*.*".to_string(),
            ],
            source_patterns: vec!["src/**/*".to_string()],
            exclude_patterns: vec![
                "**/mod.rs".to_string(),
                "**/lib.rs".to_string(),
                "**/main.rs".to_string(),
                "**/generated/**".to_string(),
            ],
        }
    }
}
```

**Verification:**
```bash
cargo test --lib -- correlation::pattern_matching
# Verify all expected patterns match
```

### Phase 5: Documentation Sync

**Goal:** Ensure spec documentation matches implementation.

**File:** `docs/specs/checks/tests.md`

Add JavaScript/TypeScript placeholder documentation:

```markdown
## Placeholder Tests

Placeholder tests indicate planned test implementation in the target project. Quench recognizes these patterns and treats them as valid test correlation:

**Rust:**
```rust
#[test]
#[ignore = "TODO: implement parser"]
fn test_parser() { todo!() }
```

**JavaScript/TypeScript:**
```javascript
test.todo('parser should handle edge case');
it.todo('validates input correctly');
test.skip('temporarily disabled', () => { /* ... */ });
describe.todo('edge cases');
```

When placeholder tests exist for a source file, correlation is satisfied even without implementation—the test intent is recorded.
```

**File:** `docs/specs/checks/tests.md`

Update test file patterns section:

```markdown
## Test File Matching

### Default Patterns

| Pattern | Description |
|---------|-------------|
| `tests/**/*` | Tests directory |
| `test/**/*` | Test directory (singular) |
| `**/__tests__/**` | Jest convention |
| `**/*_test.*` | Underscore suffix |
| `**/*_tests.*` | Underscore suffix (plural) |
| `**/*.test.*` | Dot suffix |
| `**/*.spec.*` | Spec suffix |
| `**/test_*.*` | Test prefix |
```

**Verification:**
```bash
# Verify documentation examples are accurate
grep -A5 "test.todo" docs/specs/checks/tests.md
```

### Phase 6: Behavioral Tests

**Goal:** Add behavioral tests for new functionality.

**File:** `tests/specs/checks/tests/correlation.rs`

Add tests for JS placeholder detection and language-aware advice:

```rust
#[test]
fn js_placeholder_test_todo_satisfies_correlation() {
    // Create fixture with:
    // - src/parser.ts (source)
    // - parser.test.ts with test.todo('parser handles empty input')
    // Should pass correlation check
}

#[test]
fn js_placeholder_test_skip_satisfies_correlation() {
    // test.skip should also satisfy correlation
}

#[test]
fn advice_message_matches_file_language() {
    // Rust file -> Rust-specific advice
    // TypeScript file -> TypeScript-specific advice
    // Go file -> Go-specific advice
}

#[test]
fn jest_test_directory_matches() {
    // __tests__/parser.test.ts should match src/parser.ts
}
```

**Verification:**
```bash
cargo test --test specs correlation
# All new behavioral tests pass
```

### Phase 7: Final Verification

**Goal:** Ensure all changes work together and pass CI.

**Steps:**
1. Run full test suite
2. Dogfood: run quench on quench
3. Verify JS project correlation works end-to-end

**Verification:**
```bash
# Full CI check
make check

# Dogfooding
cargo run -- check

# Verify test patterns
cargo test --lib -- correlation
```

## Key Implementation Details

### Placeholder Detection Strategy

Different languages have different placeholder conventions:

| Language | Placeholder Pattern |
|----------|---------------------|
| Rust | `#[test]` + `#[ignore]` |
| JavaScript | `test.todo()`, `test.skip()` |
| Go | `t.Skip()` (in test body) |
| Python | `@pytest.mark.skip` |

For this checkpoint, focus on Rust (existing) and JavaScript (new). Python and Go can be added in future checkpoints.

### Pattern Priority

Test patterns are checked in order. More specific patterns (like `__tests__`) should come before general patterns to avoid false negatives with glob matching.

### Language Detection Accuracy

Use file extension for simple, fast detection:
- `.rs` → Rust
- `.ts`, `.tsx`, `.js`, `.jsx`, `.mjs`, `.mts` → JavaScript
- `.go` → Go
- `.py` → Python

This is sufficient for advice messages and doesn't need sophisticated detection.

### Backward Compatibility

All changes are additive:
- New patterns extend existing defaults
- New placeholder detection is opt-in via existing `placeholders = "allow"` config
- Advice messages improve UX without changing behavior

## Verification Plan

### Phase 1 Verification
```bash
cargo test --lib -- js_placeholder
# JS placeholder tests pass

# Manual test with fixture
echo "test.todo('parser')" > /tmp/parser.test.ts
# Should detect as placeholder
```

### Phase 2 Verification
```bash
cargo test --lib -- default_patterns
# Pattern tests pass
```

### Phase 3 Verification
```bash
cargo test --lib -- advice
# Advice generation tests pass
```

### Phase 4 Verification
```bash
cargo test --lib -- pattern_matching
# All patterns match expected files
```

### Phase 5 Verification
```bash
# Documentation is consistent with implementation
grep "test.todo" docs/specs/checks/tests.md
```

### Phase 6 Verification
```bash
cargo test --test specs correlation
# Behavioral tests pass
```

### Phase 7 (Final) Verification
```bash
# Full CI
make check

# Dogfooding
cargo run -- check

# End-to-end JS project
mkdir /tmp/js-test && cd /tmp/js-test
echo '{}' > package.json
mkdir src && echo "export const foo = 1;" > src/parser.ts
cargo run -- check --staged 2>&1
# Should suggest TypeScript-specific test location
```

## Exit Criteria

- [ ] JavaScript `test.todo()` and `it.todo()` patterns recognized as placeholders
- [ ] Default patterns include `**/__tests__/**` and `**/*.test.*`
- [ ] Advice messages are language-specific (Rust, JS, Go, Python)
- [ ] Documentation updated with JS/TS placeholder examples
- [ ] All tests pass: `make check`
- [ ] Dogfooding passes: `quench check` on quench
