# Plan: Review and Cleanup After Placeholders Refactor

**Status:** Draft
**Branch:** `feature/placeholders-review`
**Depends On:** `placeholders-spec`, `placeholders-impl`

## Overview

Review the completed placeholders refactor to ensure spec/implementation alignment, remove dead code, verify test coverage, and dogfood on the quench codebase itself. This is a verification and cleanup pass, not new feature work.

## Project Structure

```
docs/specs/
└── checks/tests.md              # Verify: placeholder metrics section accurate

tests/
├── specs/checks/
│   ├── placeholders.rs          # Verify: metric tests pass
│   └── tests/                   # Verify: correlation tests still work
└── fixtures/placeholders/       # Audit: remove orphaned fixtures

crates/cli/src/
├── checks/
│   ├── placeholders/            # Verify: no Check impl, metrics only
│   │   ├── mod.rs               # Should export collect_placeholder_metrics()
│   │   ├── rust.rs              # Detection logic (unchanged)
│   │   └── javascript.rs        # Detection logic (unchanged)
│   └── tests/
│       └── mod.rs               # Verify: calls placeholder metrics
└── config/checks.rs             # Verify: no PlaceholdersConfig

reports/                         # Output: dogfood findings
CHANGELOG.md                     # Verify: no incorrect placeholders references
```

## Dependencies

- Working `quench` binary (`cargo build`)
- A test project with placeholder tests (use `tests/fixtures/placeholders/`)
- Access to quench codebase for dogfooding

## Implementation Phases

### Phase 1: Verify Spec/Implementation Alignment

**Goal:** Confirm docs match actual behavior.

1. Read `docs/specs/checks/tests.md` and extract the placeholder metrics specification
2. Build and run `quench check --tests -o json` on `tests/fixtures/placeholders/rust-ignore/`
3. Compare JSON output structure against spec:
   ```json
   {
     "metrics": {
       "placeholders": {
         "rust": { "ignore": N, "todo": N },
         "javascript": { "todo": N, "fixme": N, "skip": N }
       }
     }
   }
   ```
4. Verify no `"placeholders"` check appears in the `checks` array
5. Document any mismatches

**Verification:** JSON output matches spec exactly.

### Phase 2: Dead Code Removal

**Goal:** Remove all traces of standalone placeholders check.

1. Search for `PlaceholdersCheck` references:
   ```bash
   rg 'PlaceholdersCheck' crates/
   ```
2. Search for `[check.placeholders]` config handling:
   ```bash
   rg '\[check\.placeholders\]' -g '*.rs' -g '*.toml'
   ```
3. Check `output.schema.json` for stale `"placeholders"` in check enum
4. Search for orphaned test fixtures:
   ```bash
   ls tests/fixtures/placeholders/
   # Cross-reference with tests/specs/checks/placeholders.rs imports
   ```
5. Remove any dead code or orphaned files found

**Verification:** All searches return empty (except archived plans).

### Phase 3: Test Coverage Verification

**Goal:** Ensure all tests pass and coverage is adequate.

1. Run placeholder-specific behavioral tests:
   ```bash
   cargo test -p quench -- placeholders --nocapture
   ```
2. Run correlation tests to verify placeholder/test integration:
   ```bash
   cargo test -p quench -- tests::correlation --nocapture
   ```
3. Run full test suite:
   ```bash
   make check
   ```
4. Review unit test coverage in:
   - `crates/cli/src/checks/placeholders/mod_tests.rs`
   - `crates/cli/src/checks/placeholders/rust_tests.rs`
   - `crates/cli/src/checks/placeholders/javascript_tests.rs`
   - `crates/cli/src/checks/tests/placeholder_tests.rs`

**Verification:** All tests pass, no panics or warnings.

### Phase 4: Documentation Consistency

**Goal:** Ensure all docs reference placeholders correctly.

1. Check `CHANGELOG.md`:
   ```bash
   rg 'placeholders' CHANGELOG.md
   ```
   - Should reference "placeholder metrics" not "placeholders check"
   - No mention of standalone `[check.placeholders]` config
2. Check `README.md` for any placeholders mentions (should be minimal/none)
3. Validate `output.schema.json`:
   ```bash
   # Ensure valid JSON
   jq . docs/specs/output.schema.json > /dev/null
   # Check check enum
   jq '.definitions.CheckName.enum' docs/specs/output.schema.json
   # Should NOT include "placeholders"
   ```

**Verification:** No incorrect documentation references.

### Phase 5: Dogfood on Quench Codebase

**Goal:** Run quench on itself and verify placeholder metrics.

1. Run quench on quench:
   ```bash
   cargo run -- check --tests -o json > reports/dogfood-placeholders.json
   ```
2. Check for `#[ignore]` tests in quench codebase:
   ```bash
   rg '#\[ignore' crates/ tests/
   ```
3. Verify metrics match actual placeholder count
4. Document findings in `reports/placeholders-review.md`:
   - Total placeholder counts by type
   - Any unexpected results
   - Issues found (if any)

**Verification:** Metrics accurately reflect quench's own placeholder tests.

### Phase 6: Test Pattern Consolidation

**Goal:** Clean up test code organization.

1. Audit `tests/specs/checks/placeholders.rs` for repetition:
   - Look for similar test patterns that could use `yare` parameterization
   - Example candidates: multiple fixture tests with same assertion pattern
2. Check fixture organization:
   ```bash
   ls -la tests/fixtures/placeholders/
   ```
   - Each fixture should be used by at least one test
   - Remove unused fixtures
3. Verify sibling `_tests.rs` convention is followed:
   - `mod_tests.rs`, `rust_tests.rs`, `javascript_tests.rs` should use `#[path = "..."]`
4. If consolidation opportunities exist, create small focused changes

**Verification:** No duplicate test patterns, all fixtures used.

## Key Implementation Details

### Metrics Collection Call Site

The tests check should call placeholder metrics collection:

```rust
// In crates/cli/src/checks/tests/mod.rs
use crate::checks::placeholders::collect_placeholder_metrics;

fn build_output(&self, ...) -> CheckOutput {
    let placeholder_metrics = collect_placeholder_metrics(&files);
    CheckOutput {
        metrics: json!({
            "placeholders": placeholder_metrics,
            // ... other metrics
        }),
        // ...
    }
}
```

### Expected JSON Output Structure

```json
{
  "version": 1,
  "checks": [
    {
      "name": "tests",
      "status": "pass",
      "metrics": {
        "placeholders": {
          "rust": { "ignore": 0, "todo": 0 },
          "javascript": { "todo": 0, "fixme": 0, "skip": 0 }
        }
      }
    }
  ]
}
```

### Correlation vs Metrics

Two distinct concepts (spec lines 184-197 in tests.md):
- **Correlation** (`placeholders = "allow"`): Affects pass/fail
- **Metrics**: Always collected, independent of correlation setting

## Verification Plan

### Phase 1 Verification
- [ ] `quench check --tests -o json` produces expected structure
- [ ] No "placeholders" check in output

### Phase 2 Verification
- [ ] `rg PlaceholdersCheck crates/` returns nothing
- [ ] `rg '\[check\.placeholders\]'` returns nothing (except archives)
- [ ] All test fixtures are referenced by tests

### Phase 3 Verification
- [ ] `make check` passes with no warnings
- [ ] All placeholders tests pass

### Phase 4 Verification
- [ ] CHANGELOG references "placeholder metrics" correctly
- [ ] `output.schema.json` is valid JSON
- [ ] "placeholders" not in CheckName enum

### Phase 5 Verification
- [ ] `reports/placeholders-review.md` exists with findings
- [ ] Metrics match actual `#[ignore]` count in quench

### Phase 6 Verification
- [ ] No duplicate test patterns remain
- [ ] All fixtures under `tests/fixtures/placeholders/` are used
- [ ] Unit tests follow `_tests.rs` convention
