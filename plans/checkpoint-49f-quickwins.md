# Checkpoint 49f: JavaScript Adapter Quick Wins

## Overview

Clean up the JavaScript adapter code to improve consistency, remove dead code, and consolidate duplicate patterns with other language adapters. The JavaScript adapter has accumulated several inconsistencies that create maintenance burden and potential bugs.

## Project Structure

```
crates/cli/src/
├── adapter/
│   ├── javascript/
│   │   ├── mod.rs           # Main adapter implementation
│   │   ├── policy.rs        # Thin wrapper (consolidation target)
│   │   ├── suppress.rs      # Suppress directive parsing
│   │   └── workspace.rs     # Workspace detection
│   ├── javascript_tests.rs  # Adapter tests
│   └── common/
│       └── policy.rs        # Shared policy logic
├── checks/escapes/
│   ├── javascript_suppress.rs  # JS-specific suppress checking (cleanup target)
│   ├── go_suppress.rs          # Go version (reference pattern)
│   └── suppress_common.rs      # Shared suppress logic
└── config/
    └── javascript.rs        # JS config with pattern defaults
```

## Dependencies

No new dependencies required.

## Implementation Phases

### Phase 1: Consolidate Suppress Logic

**Goal**: Use the shared `build_suppress_missing_comment_advice()` from `suppress_common.rs` instead of the duplicate JavaScript-specific version.

**Files**:
- `crates/cli/src/checks/escapes/javascript_suppress.rs`
- `crates/cli/src/checks/escapes/suppress_common.rs`

**Changes**:

1. Add JavaScript to `build_suppress_missing_comment_advice()` in `suppress_common.rs`:

```rust
// In build_suppress_missing_comment_advice(), add to the match:
"javascript" => get_js_lint_guidance(code),
```

2. Add `get_js_lint_guidance()` to `suppress_common.rs`:

```rust
/// Get lint-specific guidance for JavaScript/TypeScript lints.
fn get_js_lint_guidance(lint_code: &str) -> &'static str {
    match lint_code {
        "no-console" => "Is this console output needed in production?",
        "no-explicit-any"
        | "@typescript-eslint/no-explicit-any"
        | "lint/suspicious/noExplicitAny" => "Can this be properly typed instead?",
        "no-unused-vars" | "@typescript-eslint/no-unused-vars" => "Is this variable still needed?",
        _ => "Is this suppression necessary?",
    }
}
```

3. Update `javascript_suppress.rs` to use the common function (like `go_suppress.rs` does):

```rust
// Replace:
let advice = build_js_missing_comment_advice(lint_code.as_deref(), required_patterns);

// With:
let advice = super::suppress_common::build_suppress_missing_comment_advice(
    "javascript",
    lint_code.as_deref(),
    required_patterns,
);
```

4. Remove the now-unused `get_js_lint_guidance()` and `build_js_missing_comment_advice()` functions from `javascript_suppress.rs`.

**Verification**: Run `cargo test` - existing suppress tests should pass unchanged.

### Phase 2: Fix Extension Coverage Gap

**Goal**: Align `is_js_extension()`, `extensions()`, and config defaults to cover the same set of extensions.

**Files**:
- `crates/cli/src/adapter/javascript/mod.rs`
- `crates/cli/src/config/javascript.rs`

**Current state**:
| Location | Extensions |
|----------|------------|
| `is_js_extension()` | js, jsx, ts, tsx, mjs, mts |
| `extensions()` | js, jsx, ts, tsx, mjs, mts |
| `default_source()` | js, jsx, ts, tsx, mjs, cjs |

**Issues**:
- `.cjs` is in config defaults but not in `is_js_extension()` (bypasses fast path)
- `.mts` is in adapter but not in config defaults

**Changes**:

1. Add `cjs` and `cts` to `is_js_extension()`:

```rust
fn is_js_extension(path: &Path) -> Option<bool> {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| matches!(ext, "js" | "jsx" | "ts" | "tsx" | "mjs" | "mts" | "cjs" | "cts"))
}
```

2. Update `extensions()` to match:

```rust
fn extensions(&self) -> &'static [&'static str] {
    &["js", "jsx", "ts", "tsx", "mjs", "mts", "cjs", "cts"]
}
```

3. Update `default_source()` in config to include all extensions:

```rust
pub(crate) fn default_source() -> Vec<String> {
    vec![
        "**/*.js".to_string(),
        "**/*.jsx".to_string(),
        "**/*.ts".to_string(),
        "**/*.tsx".to_string(),
        "**/*.mjs".to_string(),
        "**/*.mts".to_string(),
        "**/*.cjs".to_string(),
        "**/*.cts".to_string(),
    ]
}
```

**Verification**:
- Add test case for `.cjs` and `.cts` files in `javascript_tests.rs`
- Run `cargo test`

### Phase 3: Remove Dead Code

**Goal**: Remove the unused `package_name()` function.

**Files**:
- `crates/cli/src/adapter/javascript/mod.rs`
- `crates/cli/src/adapter/javascript_tests.rs`

**Analysis**:
- `JavaScriptAdapter::package_name()` reads a single package.json
- `JsWorkspace` handles package names for workspaces (different mechanism)
- Only usage is a single test that verifies `None` for missing files
- This is dead code - workspace functionality superseded it

**Changes**:

1. Remove `package_name()` from `mod.rs` (lines 118-124):

```rust
// DELETE:
/// Read package name from package.json at the given root.
pub fn package_name(root: &Path) -> Option<String> {
    let pkg_json = root.join("package.json");
    let content = fs::read_to_string(&pkg_json).ok()?;
    let value: Value = serde_json::from_str(&content).ok()?;
    value.get("name")?.as_str().map(String::from)
}
```

2. Remove `use std::fs;` if no longer needed.

3. Remove test `package_name_returns_none_for_missing_file` from `javascript_tests.rs`.

**Verification**: `cargo test` and `cargo clippy` should pass.

### Phase 4: Align Test Patterns

**Goal**: Ensure adapter's hardcoded test patterns align with config defaults for consistency.

**Files**:
- `crates/cli/src/adapter/javascript/mod.rs`
- `crates/cli/src/config/javascript.rs`

**Current state**:

Adapter patterns (hardcoded):
```rust
"**/*.test.js", "**/*.test.ts", "**/*.test.jsx", "**/*.test.tsx",
"**/*.spec.js", "**/*.spec.ts", "**/*.spec.jsx", "**/*.spec.tsx",
"**/__tests__/**", "test/**", "tests/**"
```

Config defaults:
```rust
"**/tests/**", "**/test/**",
"**/*_test.*", "**/*_tests.*",
"**/*.test.*", "**/*.spec.*",
"**/test_*.*"
```

**Issues**:
- Adapter uses extension-specific (`*.test.js`) while config uses wildcards (`*.test.*`)
- `__tests__/` only in adapter
- `*_test.*` and `test_*.*` only in config
- Different prefix patterns (`test/**` vs `**/test/**`)

**Decision**: Use wildcards in adapter to match more patterns (like config does), add `__tests__` to config.

**Changes**:

1. Update adapter's `test_patterns` in `new()`:

```rust
test_patterns: build_glob_set(&[
    "**/*.test.*".to_string(),
    "**/*.spec.*".to_string(),
    "**/*_test.*".to_string(),
    "**/*_tests.*".to_string(),
    "**/test_*.*".to_string(),
    "**/__tests__/**".to_string(),
    "**/test/**".to_string(),
    "**/tests/**".to_string(),
]),
```

2. Add `__tests__` to config's `default_tests()`:

```rust
pub(crate) fn default_tests() -> Vec<String> {
    vec![
        "**/tests/**".to_string(),
        "**/test/**".to_string(),
        "**/__tests__/**".to_string(),
        "**/*_test.*".to_string(),
        "**/*_tests.*".to_string(),
        "**/*.test.*".to_string(),
        "**/*.spec.*".to_string(),
        "**/test_*.*".to_string(),
    ]
}
```

**Verification**:
- Update tests in `javascript_tests.rs` to cover new patterns
- Run `cargo test`

## Key Implementation Details

### Why Consolidate Suppress Logic?

Go adapter already uses `build_suppress_missing_comment_advice()` from the common module:

```rust
// go_suppress.rs:99
let advice = super::suppress_common::build_suppress_missing_comment_advice(
    "go",
    lint_code.as_deref(),
    required_patterns,
);
```

JavaScript should follow the same pattern instead of having its own duplicate implementation.

### Extension Philosophy

The JavaScript ecosystem uses several module extensions:
- `.js` / `.jsx` - Standard ES/React
- `.ts` / `.tsx` - TypeScript
- `.mjs` / `.mts` - ES modules (explicit)
- `.cjs` / `.cts` - CommonJS modules (explicit)

All should be treated as source files. The fast-path optimization (`is_js_extension`) must cover all of them to avoid falling back to GlobSet unnecessarily.

### Test Pattern Wildcards

Using `**/*.test.*` instead of `**/*.test.js` means:
- Catches `.test.ts`, `.test.jsx`, `.test.tsx` with one pattern
- More resilient to new extensions (e.g., `.mts`)
- Matches config defaults style
- Smaller GlobSet = faster matching

## Verification Plan

1. **Phase 1**: `cargo test -p quench checks::escapes` - Suppress tests pass
2. **Phase 2**: Add extension tests, `cargo test -p quench adapter::javascript`
3. **Phase 3**: `cargo clippy` reports no unused code warnings
4. **Phase 4**: `cargo test -p quench adapter::javascript` - All classification tests pass

**Final verification**:
```bash
make check
```

This runs fmt, clippy, all tests, build, bootstrap, audit, and deny checks.
