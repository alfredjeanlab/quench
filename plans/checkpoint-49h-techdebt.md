# JavaScript Adapter Tech Debt: DRY and Test Consolidation

## Overview

Reduce code duplication in JavaScript adapter tests by:
1. Using `yare::parameterized` more effectively to shorten repetitive tests
2. Creating shared test utilities for common patterns across language adapters
3. Consolidating policy test boilerplate that is nearly identical across JS, Go, Rust, and Shell adapters

## Project Structure

```
crates/cli/src/
├── test_utils.rs                      # Extended with new shared helpers
├── adapter/
│   ├── common/
│   │   ├── mod.rs
│   │   ├── policy.rs
│   │   ├── suppress.rs
│   │   └── test_utils.rs              # NEW: shared adapter test utilities
│   ├── javascript/
│   │   ├── policy_tests.rs            # Refactored to use shared helpers
│   │   ├── suppress_tests.rs          # Shortened with yare where possible
│   │   └── workspace_tests.rs
│   ├── javascript_tests.rs            # Already uses yare well
│   ├── go/policy_tests.rs             # Refactored to use shared helpers
│   ├── rust/policy_tests.rs           # Refactored to use shared helpers
│   └── shell/policy_tests.rs          # Refactored to use shared helpers
```

## Dependencies

No new dependencies. Uses existing:
- `yare = "3"` - parameterized test macro
- `tempfile` - test fixtures

## Implementation Phases

### Phase 1: Create Shared Adapter Test Utilities

Create `crates/cli/src/adapter/common/test_utils.rs` with shared helpers for policy tests.

**Key pattern to extract:** Policy tests across all 4 adapters (JS, Go, Rust, Shell) have identical structure:

```rust
// Current: Each adapter defines this separately (4x duplication)
fn default_policy() -> XxxPolicyConfig { ... }
fn xxx_classifier(path: &Path) -> FileKind { ... }

#[test]
fn no_violation_when_only_source_changed() {
    let policy = default_policy();
    let files = [...];
    let result = check_lint_policy(&files, &policy, xxx_classifier);
    assert!(!result.standalone_violated);
}
// 7-9 more nearly-identical tests...
```

**New shared test helper:**

```rust
// crates/cli/src/adapter/common/test_utils.rs

/// Test scenario for policy tests.
pub struct PolicyTestScenario<'a> {
    pub policy: &'a dyn PolicyConfig,
    pub files: &'a [&'a str],
    pub classifier: &'a dyn Fn(&Path) -> FileKind,
}

/// Asserts no standalone violation.
pub fn assert_no_violation(scenario: PolicyTestScenario) {
    let paths: Vec<&Path> = scenario.files.iter().map(|f| Path::new(f)).collect();
    let result = check_lint_policy(&paths, scenario.policy, scenario.classifier);
    assert!(!result.standalone_violated);
}

/// Asserts standalone violation.
pub fn assert_violation(scenario: PolicyTestScenario) {
    let paths: Vec<&Path> = scenario.files.iter().map(|f| Path::new(f)).collect();
    let result = check_lint_policy(&paths, scenario.policy, scenario.classifier);
    assert!(result.standalone_violated);
}

/// Macro for generating standard policy tests.
macro_rules! policy_tests {
    (
        adapter: $adapter:ident,
        policy_type: $policy_type:ty,
        default_policy: $default_policy:expr,
        classifier: $classifier:expr,
        lint_config: $lint_config:expr,
        source_files: [$($src:expr),+],
        test_files: [$($test:expr),+],
    ) => {
        // Generates all 9 standard policy tests
    };
}
```

**Verification:** `cargo test adapter::common`

---

### Phase 2: Refactor JavaScript Policy Tests

Refactor `javascript/policy_tests.rs` to use the shared utilities, reducing ~150 lines to ~50.

**Before (152 lines, 8 tests):**
```rust
fn default_policy() -> JavaScriptPolicyConfig { ... }  // 11 lines
fn js_classifier(path: &Path) -> FileKind { ... }      // 24 lines

#[test]
fn no_policy_allows_mixed_changes() { ... }            // 10 lines
// ... 7 more tests at ~12-20 lines each
```

**After (~50 lines):**
```rust
use crate::adapter::common::test_utils::policy_tests;

policy_tests! {
    adapter: javascript,
    policy_type: JavaScriptPolicyConfig,
    default_policy: JavaScriptPolicyConfig {
        lint_changes: LintChangesPolicy::Standalone,
        lint_config: vec![".eslintrc", "eslint.config.js", "biome.json"]
            .into_iter().map(String::from).collect(),
    },
    classifier: JavaScriptAdapter::new().classify,
    lint_config: [".eslintrc", "eslint.config.js"],
    source_files: ["src/app.ts", "src/utils.js"],
    test_files: ["src/app.test.ts"],
}

// Keep JavaScript-specific tests that don't fit the pattern:
#[test]
fn recognizes_eslint_config_variants() { ... }

#[test]
fn recognizes_biome_config_variants() { ... }

#[test]
fn recognizes_commonjs_extensions() { ... }
```

**Verification:** `cargo test adapter::javascript::policy`

---

### Phase 3: Shorten Suppress Tests with yare

Refactor `javascript/suppress_tests.rs` to use `yare::parameterized` for similar test cases, reducing ~310 lines by ~30%.

**Current pattern (repetitive):**
```rust
#[test]
fn eslint_next_line_no_rules() {
    let content = "// eslint-disable-next-line\nconsole.log('test');";
    let result = parse_eslint_suppresses(content, None);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].line, 0);
    assert!(result[0].codes.is_empty());
}

#[test]
fn eslint_next_line_single_rule() {
    let content = "// eslint-disable-next-line no-console\nconsole.log('test');";
    let result = parse_eslint_suppresses(content, None);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].codes, vec!["no-console"]);
}
// ... many similar tests
```

**Refactored with yare:**
```rust
#[parameterized(
    no_rules = {
        "// eslint-disable-next-line\ncode",
        vec![],
        false,
        None
    },
    single_rule = {
        "// eslint-disable-next-line no-console\ncode",
        vec!["no-console"],
        false,
        None
    },
    multiple_rules = {
        "// eslint-disable-next-line no-console, no-debugger\ncode",
        vec!["no-console", "no-debugger"],
        false,
        None
    },
    with_inline_reason = {
        "// eslint-disable-next-line no-console -- reason\ncode",
        vec!["no-console"],
        true,
        Some("reason")
    },
)]
fn eslint_next_line(
    content: &str,
    expected_codes: Vec<&str>,
    has_comment: bool,
    comment_text: Option<&str>,
) {
    let result = parse_eslint_suppresses(content, None);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].codes, expected_codes);
    assert_eq!(result[0].has_comment, has_comment);
    assert_eq!(result[0].comment_text.as_deref(), comment_text);
}
```

**Tests to consolidate:**
- ESLint next-line tests (5 tests → 1 parameterized)
- Biome ignore tests (4 tests → 1 parameterized)
- Edge case tests (keep as individual for clarity)

**Verification:** `cargo test adapter::javascript::suppress`

---

### Phase 4: Refactor Other Adapter Policy Tests

Apply the same pattern from Phase 2 to Go, Rust, and Shell policy tests.

**Files to update:**
- `go/policy_tests.rs` (88 lines → ~30 lines)
- `rust/policy_tests.rs` (166 lines → ~50 lines)
- `shell/policy_tests.rs` (165 lines → ~50 lines)

Each adapter keeps only its unique tests that don't fit the standard pattern.

**Verification:** `cargo test adapter::go::policy adapter::rust::policy adapter::shell::policy`

---

### Phase 5: Update test_utils.rs

Add suppress test helpers to `crates/cli/src/test_utils.rs`:

```rust
/// Assert a single suppress directive was found with expected properties.
pub fn assert_suppress(
    result: &[impl SuppressDirective],
    index: usize,
    codes: &[&str],
    has_comment: bool,
    comment_text: Option<&str>,
) {
    assert!(result.len() > index, "expected at least {} results", index + 1);
    assert_eq!(result[index].codes(), codes);
    assert_eq!(result[index].has_comment(), has_comment);
    assert_eq!(result[index].comment_text(), comment_text);
}
```

**Verification:** `cargo test test_utils`

---

## Key Implementation Details

### Macro Design for Policy Tests

The `policy_tests!` macro generates standard test cases that all adapters share:

| Test Name | What it Tests |
|-----------|---------------|
| `no_violation_when_only_source_changed` | Source files only → no violation |
| `no_violation_when_only_lint_config_changed` | Config files only → no violation |
| `violation_when_both_changed` | Source + config → violation |
| `no_violation_when_policy_disabled` | Policy::None → no violation |
| `detects_hidden_lint_config_files` | Dotfiles detected |
| `detects_nested_lint_config_files` | Subdirectory configs detected |
| `test_files_count_as_source_for_policy` | Tests trigger violation |
| `custom_lint_config_list` | Custom config list works |
| `non_source_non_lint_files_ignored` | README.md etc. don't trigger |

### Yare Parameterization Guidelines

Use yare when:
- 3+ tests have identical structure with different inputs
- Test names follow pattern like `xxx_case_a`, `xxx_case_b`
- Assertions are the same, only data changes

Keep individual tests when:
- Test logic differs (different assertions)
- Test name clarity is important for failure diagnosis
- Edge case that doesn't fit the pattern

### Code Reduction Estimates

| File | Before | After | Reduction |
|------|--------|-------|-----------|
| `javascript/policy_tests.rs` | 152 | ~50 | ~67% |
| `javascript/suppress_tests.rs` | 310 | ~220 | ~29% |
| `go/policy_tests.rs` | 88 | ~30 | ~66% |
| `rust/policy_tests.rs` | 166 | ~50 | ~70% |
| `shell/policy_tests.rs` | 165 | ~50 | ~70% |
| **Total** | **881** | **~400** | **~55%** |

## Verification Plan

1. **Unit tests pass:** `cargo test --all`
2. **Clippy clean:** `cargo clippy --all-targets -- -D warnings`
3. **Format check:** `cargo fmt --all -- --check`
4. **Full CI:** `make check`

### Per-phase verification:
- Phase 1: `cargo test adapter::common`
- Phase 2: `cargo test adapter::javascript`
- Phase 3: `cargo test adapter::javascript::suppress`
- Phase 4: `cargo test adapter::go adapter::rust adapter::shell`
- Phase 5: `cargo test test_utils`
