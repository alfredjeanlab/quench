# Plan: Refactor Placeholders from Standalone Check to Tests Check Metrics

**Status:** Draft
**Branch:** `feature/placeholders-spec`

## Overview

Refactor the placeholders system from a standalone check that reports violations to a metrics collector integrated into the tests check. Placeholder tests (`#[ignore]`, `todo!()`, `test.todo()`, `test.fixme()`) represent test debt—a measurable quality signal—not violations that need fixing.

## Current State

- **Standalone check**: `crates/cli/src/checks/placeholders/` with its own `Check` trait impl
- **Generates violations**: Reports each placeholder as a violation with line number and advice
- **Separate config**: `[check.placeholders]` with `check = "off"` default
- **Dual purpose**: Also used for test correlation via `placeholders = "allow"` in tests config

## Target State

- **Metrics only**: Placeholder counts collected as part of tests check metrics
- **No violations**: Placeholders don't generate violations
- **Unified config**: Only `[check.tests.commit]` with `placeholders = "allow"` for correlation
- **Metrics structure**: `metrics.placeholders.{rust,javascript}.{ignore,todo,fixme,skip}`

## Project Structure

```
docs/specs/
├── checks/tests.md          # Update: add Placeholder Metrics section
├── checks/placeholders.md   # DELETE if exists (doesn't currently)
├── 02-config.md             # Update: remove [check.placeholders] if present
└── output.schema.json       # Update: remove 'placeholders' from check enum

tests/specs/checks/
└── placeholders.rs          # Rewrite: metric tests instead of violation tests

crates/cli/src/
├── checks/
│   ├── placeholders/        # Keep detection logic, remove Check impl
│   │   ├── mod.rs           # Convert to metrics collector
│   │   ├── rust.rs          # Keep as-is (detection logic)
│   │   └── javascript.rs    # Keep as-is (detection logic)
│   └── tests/
│       ├── mod.rs           # Update: integrate placeholder metrics
│       └── placeholder.rs   # Keep: correlation logic
└── config/checks.rs         # Update: remove PlaceholdersConfig
```

## Dependencies

No new external dependencies required. This is a refactoring of existing code.

## Implementation Phases

### Phase 1: Update Specifications

**Goal:** Align documentation with target behavior.

1. **Update `docs/specs/checks/tests.md`**
   - Add "Placeholder Metrics" section after "Placeholder Tests" section
   - Document that placeholders are collected as metrics, not violations
   - Define metrics structure:
     ```
     metrics.placeholders.rust.ignore    # Count of #[ignore] tests
     metrics.placeholders.rust.todo      # Count of todo!() in test bodies
     metrics.placeholders.javascript.todo   # Count of test.todo() etc.
     metrics.placeholders.javascript.fixme  # Count of test.fixme() etc.
     metrics.placeholders.javascript.skip   # Count of test.skip() etc.
     ```
   - Clarify that `placeholders = "allow"` controls correlation, not metrics

2. **Update `docs/specs/02-config.md`**
   - Remove `[check.placeholders]` section if present
   - Confirm `placeholders = "allow"` stays under `[check.tests.commit]`

3. **Update `docs/specs/output.schema.json`**
   - Remove `"placeholders"` from check names enum
   - Update tests check metrics schema to include placeholder counts

**Verification:** Specs review—changes are documentation only.

### Phase 2: Rewrite Behavioral Tests

**Goal:** Convert violation-expecting tests to metric-expecting tests.

1. **Rewrite `tests/specs/checks/placeholders.rs`**
   - Remove tests expecting `placeholders: FAIL` output
   - Remove tests expecting violation structures
   - Add tests verifying metrics appear in tests check JSON:
     ```rust
     #[test]
     fn tests_check_includes_placeholder_metrics() {
         // Run tests check, verify metrics.placeholders.rust.ignore > 0
     }
     ```
   - Keep tests for `placeholders = "allow"` correlation behavior
   - Tests should run `tests` check, not `placeholders` check

2. **Update test fixtures if needed**
   - Fixtures in `tests/fixtures/placeholders/` may need config updates
   - Remove `[check.placeholders]` from fixture quench.toml files
   - Ensure `[check.tests]` is enabled in fixtures

**Verification:** `cargo test --test specs` should show new tests as `#[ignore = "TODO: Phase 3"]`.

### Phase 3: Refactor Implementation

**Goal:** Integrate placeholder detection into tests check.

1. **Convert `placeholders/mod.rs` to metrics collector**
   - Remove `Check` trait implementation
   - Export `collect_placeholder_metrics(test_files: &[PathBuf]) -> PlaceholderMetrics`
   - Keep detection logic in `rust.rs` and `javascript.rs` unchanged

2. **Update `checks/tests/mod.rs`**
   - Call `collect_placeholder_metrics()` during tests check run
   - Include placeholder counts in metrics output
   - No violations generated for placeholders

3. **Update `checks/mod.rs`**
   - Remove `placeholders` from check registration
   - Remove `Placeholders` variant from check enum

4. **Update `config/checks.rs`**
   - Remove `PlaceholdersConfig` struct
   - Remove `placeholders` field from checks config
   - Keep `placeholders` field in `TestsConfig` for correlation

5. **Update cache if needed**
   - Bump `CACHE_VERSION` in `crates/cli/src/cache.rs`

**Verification:** `cargo test` passes, `make check` passes.

### Phase 4: Cleanup and Verification

**Goal:** Remove dead code, verify end-to-end behavior.

1. **Remove standalone check artifacts**
   - Delete `docs/specs/checks/placeholders.md` if it exists
   - Remove placeholder-specific violation types from output schema
   - Update any CLI help text that mentions placeholders check

2. **Verify behavior**
   - Run `quench` on fixture projects
   - Confirm `tests` check includes placeholder metrics
   - Confirm no `placeholders` check available
   - Confirm `quench --json` output matches updated schema

3. **Update README if needed**
   - Remove placeholders check from feature list
   - Mention placeholder metrics under tests check

**Verification:** Full `make check`, manual testing on fixtures.

## Key Implementation Details

### Metrics Structure

The tests check should output placeholder metrics in this structure:

```json
{
  "check": "tests",
  "status": "pass",
  "metrics": {
    "coverage": { ... },
    "placeholders": {
      "rust": {
        "ignore": 3,
        "todo": 1
      },
      "javascript": {
        "todo": 2,
        "fixme": 1,
        "skip": 0
      }
    }
  }
}
```

### Detection Logic Reuse

The existing detection modules are well-structured and can be reused:

```rust
// crates/cli/src/checks/placeholders/mod.rs
pub struct PlaceholderMetrics {
    pub rust: RustMetrics,
    pub javascript: JsMetrics,
}

pub fn collect_placeholder_metrics(test_files: &[PathBuf]) -> PlaceholderMetrics {
    let mut metrics = PlaceholderMetrics::default();
    for file in test_files {
        match file.extension() {
            Some("rs") => {
                let content = std::fs::read_to_string(file).unwrap_or_default();
                for p in rust::detect_placeholders(&content) {
                    match p.kind {
                        RustPlaceholderKind::Ignore => metrics.rust.ignore += 1,
                        RustPlaceholderKind::Todo => metrics.rust.todo += 1,
                    }
                }
            }
            // similar for JS/TS
        }
    }
    metrics
}
```

### Correlation vs. Metrics

Two separate concepts must remain distinct:

1. **Correlation** (`placeholders = "allow"`): Whether placeholder tests satisfy the "test exists" requirement for a source file. This stays in `TestsConfig`.

2. **Metrics**: Counting total placeholder tests for quality measurement. This is the new behavior.

## Verification Plan

### Unit Tests
- Detection functions in `rust.rs` and `javascript.rs` already have tests
- Add unit tests for `collect_placeholder_metrics()`

### Integration Tests (Behavioral Specs)
- `tests/specs/checks/placeholders.rs` → rewritten to verify metrics
- `tests/specs/checks/tests.rs` → may need additional tests for metric output

### Manual Testing
```bash
# Verify no placeholders check
quench check placeholders  # Should error: unknown check

# Verify metrics in tests check
quench check tests --json | jq '.metrics.placeholders'

# Verify correlation still works
cd tests/fixtures/placeholders/rust-ignore
quench check tests  # Should pass if placeholders = "allow"
```

### CI
- `make check` must pass
- All behavioral specs must pass (no ignored tests remaining)

## Risks and Mitigations

| Risk | Mitigation |
|------|------------|
| Breaking existing configs with `[check.placeholders]` | Add deprecation warning, ignore unknown sections gracefully |
| Missing metrics in JSON output | Behavioral spec tests verify JSON structure |
| Correlation behavior regression | Keep existing correlation tests, run on fixtures |

## Commit Strategy

1. `docs: update specs for placeholders as tests metrics` (Phase 1)
2. `test: rewrite placeholder behavioral specs for metrics` (Phase 2, tests ignored)
3. `refactor: integrate placeholder metrics into tests check` (Phase 3)
4. `chore: remove standalone placeholders check artifacts` (Phase 4)
