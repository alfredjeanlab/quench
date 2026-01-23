# Checkpoint 4H: Tech Debt - Rust Adapter

**Root Feature:** `quench-4h8d`

## Overview

Tech debt cleanup for the Rust adapter test suite. The adapter implementation is well-structured but tests contain repetitive patterns that can be consolidated using `yare` parameterized tests, consistent with the approach used in checkpoint-3h for escapes tests.

**Cleanup targets identified:**

| Category | Location | Impact |
|----------|----------|--------|
| Duplicate `default_policy()` helper | `policy_tests.rs`, `rust_tests.rs` | Remove duplication |
| Repeated `RustAdapter::new()` calls | `rust_tests.rs` (16+ calls) | Extract test fixture |
| Classification tests repetition | `rust_tests.rs` L7-66 | Parameterize ~5 tests |
| Default escapes tests repetition | `rust_tests.rs` L160-220 | Parameterize ~6 tests |
| Test module flattening | `rust_tests.rs` | Remove unnecessary nesting |

**Estimated impact:** ~40 lines reduction, improved test maintainability.

## Project Structure

```
crates/cli/src/adapter/
├── rust/
│   ├── policy_tests.rs        # MODIFY: Remove duplicate helper
│   └── ...                    # (other files unchanged)
├── rust_tests.rs              # MODIFY: Parameterize, flatten, DRY up
└── test_fixtures.rs           # ADD: Shared test fixtures (optional)
```

## Dependencies

No new dependencies. Uses existing:
- `yare = "3"` (already in dev-dependencies)

## Implementation Phases

### Phase 1: Extract Shared Test Fixtures

The `default_policy()` helper is duplicated between `policy_tests.rs` and `rust_tests.rs`. Extract to a shared location.

**Current duplication in `policy_tests.rs` (L10-20):**
```rust
fn default_policy() -> RustPolicyConfig {
    RustPolicyConfig {
        lint_changes: LintChangesPolicy::Standalone,
        lint_config: vec![
            "rustfmt.toml".to_string(),
            ".rustfmt.toml".to_string(),
            "clippy.toml".to_string(),
            ".clippy.toml".to_string(),
        ],
    }
}
```

**Same code in `rust_tests.rs` (L228-238).**

**Solution:** Keep the helper in `policy_tests.rs` (closer to where it's most used) and have `rust_tests.rs` define a minimal wrapper that delegates to a shared constant or redefines inline. Given test isolation, keeping separate but identical helpers is acceptable, but the `rust_tests.rs` version can import from the adapter module if we expose it.

Alternative: Since this is test code, minimal duplication is acceptable. Focus on parameterization instead.

**Milestone:** Identify if extraction provides value or if duplication is acceptable.

**Verification:**
```bash
cargo test -p quench -- policy
cargo test -p quench -- adapter_check_lint_policy
```

**Status:** [ ] Pending

---

### Phase 2: Parameterize Classification Tests

The 5 tests in `rust_tests.rs::classification` module all follow the same pattern: call `RustAdapter::new()`, then `adapter.classify(Path::new(...))`, then assert.

**Current pattern (repeated 5 times):**
```rust
#[test]
fn source_file_in_src() {
    let adapter = RustAdapter::new();
    assert_eq!(adapter.classify(Path::new("src/lib.rs")), FileKind::Source);
    assert_eq!(adapter.classify(Path::new("src/main.rs")), FileKind::Source);
    assert_eq!(adapter.classify(Path::new("src/foo/bar.rs")), FileKind::Source);
}
```

**Refactored using yare:**
```rust
use yare::parameterized;

#[parameterized(
    src_lib = { "src/lib.rs", FileKind::Source },
    src_main = { "src/main.rs", FileKind::Source },
    src_nested = { "src/foo/bar.rs", FileKind::Source },
    tests_integration = { "tests/integration.rs", FileKind::Test },
    tests_nested = { "tests/foo/bar.rs", FileKind::Test },
    test_suffix_single = { "src/lib_test.rs", FileKind::Test },
    test_suffix_plural = { "src/lib_tests.rs", FileKind::Test },
    target_debug = { "target/debug/deps/foo.rs", FileKind::Other },
    target_release = { "target/release/build/bar.rs", FileKind::Other },
    cargo_toml = { "Cargo.toml", FileKind::Other },
    readme = { "README.md", FileKind::Other },
)]
fn classify_path(path: &str, expected: FileKind) {
    let adapter = RustAdapter::new();
    assert_eq!(
        adapter.classify(Path::new(path)),
        expected,
        "path {:?} should be {:?}",
        path,
        expected
    );
}
```

**Milestone:** 5 classification tests consolidated into 1 parameterized test (~50 lines → ~25 lines).

**Verification:**
```bash
cargo test -p quench -- classify_path
```

**Status:** [ ] Pending

---

### Phase 3: Parameterize Ignore Pattern Tests

The 2 tests in `rust_tests.rs::ignore_patterns` can be parameterized.

**Current pattern:**
```rust
#[test]
fn target_dir_ignored() {
    let adapter = RustAdapter::new();
    assert!(adapter.should_ignore(Path::new("target/debug/foo.rs")));
    assert!(adapter.should_ignore(Path::new("target/release/bar.rs")));
}
```

**Refactored using yare:**
```rust
#[parameterized(
    target_debug = { "target/debug/foo.rs", true },
    target_release = { "target/release/bar.rs", true },
    src_lib = { "src/lib.rs", false },
    tests_test = { "tests/test.rs", false },
)]
fn should_ignore_path(path: &str, expected: bool) {
    let adapter = RustAdapter::new();
    assert_eq!(
        adapter.should_ignore(Path::new(path)),
        expected,
        "path {:?} should_ignore = {}",
        path,
        expected
    );
}
```

**Milestone:** 2 ignore tests consolidated into 1 parameterized test (~15 lines → ~12 lines).

**Verification:**
```bash
cargo test -p quench -- should_ignore_path
```

**Status:** [ ] Pending

---

### Phase 4: Parameterize Default Escapes Tests

The 6 tests in `rust_tests.rs::default_escapes` follow repetitive patterns.

**Current pattern (repeated for each escape):**
```rust
#[test]
fn unsafe_pattern_requires_safety_comment() {
    let adapter = RustAdapter::new();
    let patterns = adapter.default_escapes();
    let unsafe_pattern = patterns.iter().find(|p| p.name == "unsafe").unwrap();

    assert_eq!(unsafe_pattern.action, EscapeAction::Comment);
    assert_eq!(unsafe_pattern.comment, Some("// SAFETY:"));
}
```

**Refactored using yare:**
```rust
#[parameterized(
    unsafe_requires_comment = { "unsafe", EscapeAction::Comment, Some("// SAFETY:") },
    transmute_requires_comment = { "transmute", EscapeAction::Comment, Some("// SAFETY:") },
    unwrap_forbidden = { "unwrap", EscapeAction::Forbid, None },
    expect_forbidden = { "expect", EscapeAction::Forbid, None },
)]
fn default_escape_pattern(name: &str, expected_action: EscapeAction, expected_comment: Option<&str>) {
    let adapter = RustAdapter::new();
    let patterns = adapter.default_escapes();
    let pattern = patterns.iter().find(|p| p.name == name)
        .unwrap_or_else(|| panic!("pattern {:?} not found", name));

    assert_eq!(pattern.action, expected_action, "pattern {:?} action", name);
    assert_eq!(pattern.comment, expected_comment, "pattern {:?} comment", name);
}

#[test]
fn returns_four_default_patterns() {
    let adapter = RustAdapter::new();
    assert_eq!(adapter.default_escapes().len(), 4);
}

#[test]
fn all_patterns_have_advice() {
    let adapter = RustAdapter::new();
    for pattern in adapter.default_escapes() {
        assert!(!pattern.advice.is_empty(), "Pattern {} should have advice", pattern.name);
    }
}
```

**Milestone:** 6 escape tests consolidated into 1 parameterized + 2 regular tests (~55 lines → ~30 lines).

**Verification:**
```bash
cargo test -p quench -- default_escape
```

**Status:** [ ] Pending

---

### Phase 5: Flatten Test Module Structure

The `rust_tests.rs` file has 5 nested modules:
- `mod classification` (L7-66)
- `mod ignore_patterns` (L68-84)
- `mod line_classification` (L86-158)
- `mod default_escapes` (L160-220)
- `mod adapter_check_lint_policy` (L224-266)

After parameterization, tests have descriptive names and module wrappers add unnecessary nesting.

**Actions:**
1. Remove `mod classification`, `mod ignore_patterns`, `mod default_escapes` wrappers
2. Keep `mod line_classification` (contains complex multi-line content tests)
3. Keep `mod adapter_check_lint_policy` (contains helper function `default_policy`)

**Milestone:** Simpler file structure, test names are self-documenting.

**Verification:**
```bash
cargo test -p quench -- rust
```

**Status:** [ ] Pending

---

### Phase 6: Final Verification

Run the full verification suite.

**Verification checklist:**
- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --all-targets --all-features -- -D warnings`
- [ ] `cargo test --all`
- [ ] `cargo build --all`
- [ ] `./scripts/bootstrap`
- [ ] `cargo audit`
- [ ] `cargo deny check`

**Run:**
```bash
make check
```

**Milestone:** All quality gates pass.

**Status:** [ ] Pending

## Key Implementation Details

### Why Parameterize with yare?

The project already uses `yare` (version 3) for parameterized tests in:
- `cloc_tests.rs`
- `walker_tests.rs`
- `error_tests.rs`
- `generic_tests.rs`

Benefits:
- Consistent with existing codebase patterns
- Clear test case names in output
- Reduced boilerplate
- Easier to add new test cases

### Preserving Test Coverage

Each refactoring must maintain:
1. **Same test cases** - all original paths/inputs preserved
2. **Same assertions** - verify the same properties
3. **Clear failure messages** - include context for debugging

### No Behavior Changes

This is purely a test code refactoring:
- No changes to implementation code
- Same test coverage
- Same assertions
- Improved maintainability

### Duplication Acceptance

Some duplication is acceptable in test code:
- The `default_policy()` helper is small (10 lines)
- It's only used in 2 files
- Extracting to a shared module adds complexity without proportional benefit
- Focus effort on high-value parameterization instead

## Verification Plan

1. **Before any changes** - capture test count:
   ```bash
   cargo test -p quench -- rust 2>&1 | grep "test result"
   # Note the number of tests
   ```

2. **After each phase** - verify same number of test cases:
   ```bash
   cargo test -p quench -- rust 2>&1 | grep "test result"
   # Parameterized tests expand to same number
   ```

3. **Full quality gates:**
   ```bash
   make check
   ```

## Summary

| Phase | Task | Files Changed | Status |
|-------|------|---------------|--------|
| 1 | Evaluate shared fixtures | - | [ ] Pending |
| 2 | Parameterize classification tests | `rust_tests.rs` | [ ] Pending |
| 3 | Parameterize ignore pattern tests | `rust_tests.rs` | [ ] Pending |
| 4 | Parameterize default escapes tests | `rust_tests.rs` | [ ] Pending |
| 5 | Flatten test module structure | `rust_tests.rs` | [ ] Pending |
| 6 | Final verification | - | [ ] Pending |

## Notes

- Total expected reduction: ~40 lines of test code
- No new dependencies
- No behavior changes
- Consistent with checkpoint-3h approach for escapes tests
- Line classification tests left unchanged (complex multi-line content tests don't benefit from parameterization)
- Policy tests left unchanged (already well-structured, `simple_classify` helper serves testing purpose)
