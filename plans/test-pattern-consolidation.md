# Test Pattern Consolidation

**Root Feature:** `quench-7781`

Consolidate test file classification to a single source of truth per language.

## Problem

Multiple overlapping ways to define test patterns cause confusion:

| Location | Current Use |
|----------|-------------|
| `[project].tests` | Generic project patterns |
| `[check.cloc].test_patterns` | Cloc-specific (unused in code) |
| `[rust].tests` / `[shell].tests` | Language-specific |
| Adapter code (hardcoded) | Built-in defaults |

User configured `[check.cloc].test_patterns` expecting it to work, but adapters use hardcoded patterns.

## Solution

**Resolution hierarchy:**

```
1. [<language>].tests   ← Language-specific override (most specific)
2. [project].tests      ← Project-wide patterns (applies to all languages)
3. Adapter defaults     ← Built-in convention (zero-config)
```

**Key changes:**
- Remove `test_patterns` and `source_patterns` from `[check.cloc]`
- Adapters read patterns from config, falling back to built-in defaults
- `[project].tests` applies to all languages unless overridden

---

## Phase 1: Spec Updates

### 1.1: Update docs/specs/02-config.md

- [ ] Remove `test_patterns` from `[check.cloc]` example
- [ ] Add note that `[project].tests` applies to all languages unless overridden by `[<lang>].tests`
- [ ] Document resolution hierarchy in a new "Pattern Resolution" section

### 1.2: Update docs/specs/checks/cloc.md

- [ ] Remove `source_patterns` and `test_patterns` from Configuration section
- [ ] Add cross-reference to language sections for pattern configuration
- [ ] Update "Source vs Test Separation" section to reference hierarchy

### 1.3: Update docs/specs/langs/*.md

- [ ] Update shell.md: change `tests/**/*.bats` to `**/tests/**/*.bats` in defaults
- [ ] Update rust.md: change `tests/**` to `**/tests/**` in defaults
- [ ] Add note in each lang spec about inheriting from `[project].tests`

---

## Phase 2: Behavioral Tests (tests/specs/)

### 2.1: Add pattern resolution tests

- [ ] Test: `[project].tests` applies when no `[shell].tests` configured
- [ ] Test: `[shell].tests` overrides `[project].tests`
- [ ] Test: adapter defaults used when neither configured
- [ ] Test: nested test dirs (`packages/*/tests/**`) classified correctly

### 2.2: Remove cloc test_patterns tests (if any exist)

- [ ] Search for tests using `test_patterns` in cloc config
- [ ] Remove or update to use language/project patterns instead

---

## Phase 3: Implementation

### 3.1: Config schema changes

- [ ] Remove `test_patterns` and `source_patterns` from `ClocCheckConfig`
- [ ] Add `tests` and `source` fields to each language config if not present
- [ ] Update config parsing to handle new fields

### 3.2: Adapter pattern resolution

- [ ] Add `TestPatternResolver` that implements hierarchy:
  1. Check `config.<lang>.tests`
  2. Fall back to `config.project.tests`
  3. Fall back to adapter built-in defaults
- [ ] Update `ShellAdapter::new()` to accept resolved patterns
- [ ] Update `RustAdapter::new()` to accept resolved patterns
- [ ] Update `GoAdapter::new()` to accept resolved patterns
- [ ] Update `JavaScriptAdapter::new()` to accept resolved patterns
- [ ] Update `GenericAdapter` to use `[project].tests`

### 3.3: Update AdapterRegistry

- [ ] `AdapterRegistry::for_project()` takes `&Config` parameter
- [ ] Pass resolved patterns to each adapter constructor
- [ ] Update all call sites of `AdapterRegistry::for_project()`

### 3.4: Update built-in defaults

- [ ] ShellAdapter: `**/tests/**/*.bats` instead of `tests/**/*.bats`
- [ ] RustAdapter: `**/tests/**` instead of `tests/**`
- [ ] Ensure defaults match updated specs

---

## Phase 4: Unit Tests

### 4.1: Pattern resolver tests

- [ ] Test hierarchy with all three levels specified
- [ ] Test fallback from lang to project
- [ ] Test fallback from project to defaults
- [ ] Test empty patterns don't break resolution

### 4.2: Adapter classification tests

- [ ] Test ShellAdapter with custom patterns from config
- [ ] Test nested paths like `packages/cli/tests/foo.bats` classified as test
- [ ] Test RustAdapter with custom patterns from config

---

## Checkpoint

- [ ] User's original config (`[project].tests = ["packages/*/tests/**/*.bats"]`) works correctly
- [ ] `quench check` on shell project with nested test dirs passes
- [ ] Existing tests still pass (no regressions)
