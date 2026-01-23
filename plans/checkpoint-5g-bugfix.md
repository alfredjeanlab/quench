# Checkpoint 5G: Bug Fix - Shell Adapter Config

**Root Feature:** `quench-68fa`

## Overview

Bug fix checkpoint to synchronize the shell configuration defaults with the shell adapter implementation.

In checkpoint 5E, the redundant `*_test.sh` pattern was removed from `ShellAdapter::new()` for performance (since `**/*_test.sh` already matches root-level files). In checkpoint 5F, the specification documents were updated. However, the `ShellConfig::default_tests()` function was never updated, causing a mismatch between:

- **Config defaults** (4 patterns): `*_test.sh` + `**/*_test.sh`
- **Adapter implementation** (3 patterns): only `**/*_test.sh`

This causes configuration inconsistency and test assertion failures when the pattern is corrected.

**Goals:**
1. Remove redundant `*_test.sh` from `ShellConfig::default_tests()`
2. Update test assertions to match corrected config
3. Ensure config defaults align with adapter and documentation

**Non-Goals:**
- New features or functionality
- Architectural changes
- Changes to the adapter (already correct)

## Project Structure

Files to modify:

```
quench/
├── crates/cli/src/config/
│   ├── shell.rs              # Remove redundant pattern (line 47)
│   └── mod_tests.rs          # Update test assertion (line 342)
└── plans/
    └── checkpoint-5g-bugfix.md  # This plan
```

Reference files (no changes needed):

```
quench/
├── crates/cli/src/adapter/shell/
│   └── mod.rs                # Correct implementation (reference)
└── docs/specs/
    ├── 10-language-adapters.md  # Already updated in 5F
    └── langs/shell.md           # Already updated in 5F
```

## Dependencies

None. Bug fix only with no new dependencies.

## Implementation Phases

### Phase 1: Fix ShellConfig Default Tests Pattern

**Goal:** Remove the redundant `*_test.sh` pattern from `ShellConfig::default_tests()`.

**File:** `crates/cli/src/config/shell.rs`

**Change:**

```diff
 impl ShellConfig {
     pub(crate) fn default_source() -> Vec<String> {
         vec!["**/*.sh".to_string(), "**/*.bash".to_string()]
     }

     pub(crate) fn default_tests() -> Vec<String> {
         vec![
             "tests/**/*.bats".to_string(),
             "test/**/*.bats".to_string(),
-            "*_test.sh".to_string(),
             "**/*_test.sh".to_string(),
         ]
     }
 }
```

**Rationale:**
- `**/*_test.sh` matches root-level files (e.g., `foo_test.sh`) because `**/` matches zero or more path components
- This aligns with the adapter implementation in `ShellAdapter::new()`
- This aligns with the spec documentation updated in checkpoint 5F

**Verification:**
```bash
# Check that default_tests() returns 3 patterns
cargo test -p quench --lib -- shell_default_test_patterns --nocapture
```

**Milestone:** Config defaults match adapter implementation.

**Status:** [ ] Pending

---

### Phase 2: Update Config Test Assertion

**Goal:** Update the test assertion that validates shell default test patterns.

**File:** `crates/cli/src/config/mod_tests.rs`

**Change:**

```diff
 #[test]
 fn shell_default_test_patterns() {
     let path = PathBuf::from("quench.toml");
     let content = "version = 1\n";
     let config = parse_with_warnings(content, &path).unwrap();
     assert!(config.shell.tests.contains(&"tests/**/*.bats".to_string()));
-    assert!(config.shell.tests.contains(&"*_test.sh".to_string()));
+    assert!(config.shell.tests.contains(&"**/*_test.sh".to_string()));
 }
```

**Verification:**
```bash
cargo test -p quench --lib -- shell_default_test_patterns
```

**Milestone:** Test correctly validates the fixed config.

**Status:** [ ] Pending

---

### Phase 3: Verify Consistency

**Goal:** Ensure all components are aligned: config, adapter, and documentation.

**Cross-check validation:**

1. **Config defaults** (after fix):
   ```rust
   // ShellConfig::default_tests()
   ["tests/**/*.bats", "test/**/*.bats", "**/*_test.sh"]
   ```

2. **Adapter patterns** (reference):
   ```rust
   // ShellAdapter::new()
   test_patterns: ["tests/**/*.bats", "test/**/*.bats", "**/*_test.sh"]
   ```

3. **Documentation patterns** (reference):
   ```toml
   # docs/specs/langs/shell.md
   tests = ["tests/**/*.bats", "test/**/*.bats", "**/*_test.sh"]
   ```

**Verification:**
```bash
# All shell tests pass
cargo test -p quench --lib shell

# Search for any remaining *_test.sh without **/ prefix
grep -rn '"[^*]*\*_test\.sh"' crates/cli/src/config/
# Should return no results
```

**Milestone:** Config, adapter, and docs all match.

**Status:** [ ] Pending

---

### Phase 4: Quality Gates

**Goal:** Verify all quality checks pass.

```bash
make check
```

This runs:
- `cargo fmt --all -- --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --all`
- `cargo build --all`
- `./scripts/bootstrap`
- `cargo audit`
- `cargo deny check`

**Milestone:** All quality gates pass.

**Status:** [ ] Pending

## Key Implementation Details

### The Bug

The bug was introduced when checkpoint 5E optimized the shell adapter by removing the redundant `*_test.sh` pattern. The change was correctly applied to:
- ✅ `ShellAdapter::new()` - adapter implementation
- ✅ `docs/specs/langs/shell.md` - specification (via checkpoint 5F)
- ✅ `docs/specs/10-language-adapters.md` - specification (via checkpoint 5F)
- ❌ `ShellConfig::default_tests()` - **MISSED**
- ❌ `mod_tests.rs::shell_default_test_patterns` - **MISSED**

### Why `**/*_test.sh` Matches Root-Level Files

The glob pattern `**/*_test.sh` matches root-level files like `foo_test.sh` because:
- The `**/` prefix matches **zero or more** path components
- When matching zero components, `**/*_test.sh` effectively becomes `*_test.sh`
- This is standard glob behavior documented in the `globset` crate

### Pattern Summary After This Checkpoint

| Component | Source Patterns | Test Patterns |
|-----------|-----------------|---------------|
| ShellAdapter | `**/*.sh`, `**/*.bash` | `tests/**/*.bats`, `test/**/*.bats`, `**/*_test.sh` |
| ShellConfig | `**/*.sh`, `**/*.bash` | `tests/**/*.bats`, `test/**/*.bats`, `**/*_test.sh` |
| Documentation | `**/*.sh`, `**/*.bash` | `tests/**/*.bats`, `test/**/*.bats`, `**/*_test.sh` |

## Verification Plan

1. **Unit tests:**
   ```bash
   cargo test -p quench --lib shell
   cargo test -p quench --lib -- shell_default_test_patterns
   ```

2. **Pattern consistency check:**
   ```bash
   # Should find NO occurrences of bare *_test.sh (without **/ prefix)
   grep -rn '"[^*]*\*_test\.sh"' crates/cli/src/
   ```

3. **Full quality gates:**
   ```bash
   make check
   ```

## Summary

| Phase | Task | Status |
|-------|------|--------|
| 1 | Remove redundant `*_test.sh` from config | [ ] Pending |
| 2 | Update test assertion | [ ] Pending |
| 3 | Verify consistency | [ ] Pending |
| 4 | Quality gates | [ ] Pending |

## Notes

- This is a 2-line bug fix with corresponding test update
- The fix aligns config defaults with adapter implementation and documentation
- No behavioral change for users who override the default patterns
