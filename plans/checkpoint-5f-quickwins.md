# Checkpoint 5F: Quick Wins - Shell Adapter

**Root Feature:** `quench-68fa`

## Overview

Documentation cleanup checkpoint following the Shell adapter performance optimization in checkpoint 5E. The 5E work removed the redundant `*_test.sh` pattern (since `**/*_test.sh` already matches root-level files), but the specification documents were not updated to reflect this change.

This checkpoint synchronizes documentation with the implemented behavior:
- Update test pattern specifications from `*_test.sh` to `**/*_test.sh`
- Add implementation note explaining the pattern behavior

**Goals:**
1. Fix documentation inconsistencies between specs and implementation
2. Document the pattern optimization rationale for future reference
3. Ensure all pattern examples are accurate

**Non-Goals:**
- Code changes (implementation is correct)
- New features or functionality
- Architectural changes

## Project Structure

Files to update:

```
quench/
├── crates/cli/src/adapter/shell/
│   └── mod.rs                          # Already correct (reference)
├── docs/specs/
│   ├── 10-language-adapters.md         # Update: line 61
│   └── langs/
│       └── shell.md                    # Update: lines 17, 57, 125
└── plans/
    └── checkpoint-5f-quickwins.md      # This plan
```

## Dependencies

None. Documentation-only changes.

## Implementation Phases

### Phase 1: Update Shell Spec Test Patterns

**Goal:** Fix test pattern documentation in `docs/specs/langs/shell.md`.

**Changes:**

1. Line 17 - Profile defaults example:
```diff
-tests = ["tests/**/*.bats", "test/**/*.bats", "*_test.sh"]
+tests = ["tests/**/*.bats", "test/**/*.bats", "**/*_test.sh"]
```

2. Line 57-58 - Default patterns example:
```diff
 [shell]
 source = ["**/*.sh", "**/*.bash"]
-tests = ["tests/**/*.bats", "test/**/*.bats", "*_test.sh"]
+tests = ["tests/**/*.bats", "test/**/*.bats", "**/*_test.sh"]
```

3. Line 125 - Configuration example:
```diff
 # source = ["**/*.sh", "**/*.bash"]
-# tests = ["tests/**/*.bats", "test/**/*.bats", "*_test.sh"]
+# tests = ["tests/**/*.bats", "test/**/*.bats", "**/*_test.sh"]
```

**Verification:**
```bash
grep -n "_test.sh" docs/specs/langs/shell.md
# Should only show **/*_test.sh patterns
```

**Milestone:** Shell spec patterns match implementation.

**Status:** [ ] Pending

---

### Phase 2: Update Language Adapters Spec

**Goal:** Fix test pattern documentation in `docs/specs/10-language-adapters.md`.

**Changes:**

Line 61 - Shell adapter summary:
```diff
 [shell]
 # source = ["**/*.sh", "**/*.bash"]
-# tests = ["tests/**/*.bats", "*_test.sh"]
+# tests = ["tests/**/*.bats", "test/**/*.bats", "**/*_test.sh"]
```

Note: This also adds the missing `test/**/*.bats` pattern that was present in the shell.md spec but missing from the summary.

**Verification:**
```bash
grep -n "_test.sh" docs/specs/10-language-adapters.md
# Should only show **/*_test.sh patterns
```

**Milestone:** Language adapters spec patterns match implementation.

**Status:** [ ] Pending

---

### Phase 3: Verify Documentation Consistency

**Goal:** Ensure all documentation accurately reflects the implementation.

**Cross-check:**

1. Compare implementation patterns (from `crates/cli/src/adapter/shell/mod.rs`):
   ```rust
   source_patterns: ["**/*.sh", "**/*.bash"]
   test_patterns: ["tests/**/*.bats", "test/**/*.bats", "**/*_test.sh"]
   ```

2. Verify all spec files match:
   ```bash
   # Find all shell pattern references
   grep -rn "tests.*bats\|_test\.sh" docs/specs/
   ```

3. Run full test suite to ensure no behavioral changes:
   ```bash
   cargo test -p quench --lib shell
   ```

**Acceptance criteria:**
- All documentation shows `**/*_test.sh` (not `*_test.sh`)
- All documentation includes full pattern list: `tests/**/*.bats`, `test/**/*.bats`, `**/*_test.sh`
- All Shell adapter tests pass

**Milestone:** Documentation fully synchronized with implementation.

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

### Why `**/*_test.sh` Covers Root-Level Files

The glob pattern `**/*_test.sh` matches root-level files like `foo_test.sh` because:
- The `**/` prefix matches **zero or more** path components
- When matching zero components, it effectively becomes `*_test.sh`
- This is standard glob behavior documented in the `globset` crate

Therefore, having both `*_test.sh` and `**/*_test.sh` is redundant. The checkpoint 5E optimization removed `*_test.sh`, and this checkpoint updates the documentation to match.

### Pattern Summary After This Checkpoint

| Adapter | Source Patterns | Test Patterns |
|---------|-----------------|---------------|
| Shell | `**/*.sh`, `**/*.bash` | `tests/**/*.bats`, `test/**/*.bats`, `**/*_test.sh` |

## Verification Plan

1. **Documentation grep check:**
   ```bash
   # Should find NO occurrences of bare *_test.sh (without **/ prefix)
   grep -rn '"[^*]*\*_test\.sh"' docs/specs/

   # Should find **/*_test.sh in shell specs
   grep -rn '\*\*/\*_test\.sh' docs/specs/
   ```

2. **Unit test verification:**
   ```bash
   cargo test -p quench --lib shell
   ```

3. **Full quality gates:**
   ```bash
   make check
   ```

## Summary

| Phase | Task | Status |
|-------|------|--------|
| 1 | Update shell.md test patterns | [ ] Pending |
| 2 | Update 10-language-adapters.md test patterns | [ ] Pending |
| 3 | Verify documentation consistency | [ ] Pending |
| 4 | Quality gates | [ ] Pending |

## Notes

- This is a documentation-only checkpoint with no code changes
- Changes align specs with the 5E pattern optimization
- Future adapters should use `**/*_suffix.ext` pattern (not `*_suffix.ext`)
