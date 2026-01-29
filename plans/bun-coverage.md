# Bun Test Coverage Collection (Phase 4981)

## Overview

Enable coverage reporting from Bun test runs. The Bun runner and coverage infrastructure are implemented, but coverage collection fails because:

1. **Missing LCOV reporter flag**: `collect_bun_coverage` runs `bun test --coverage` which outputs console text, not LCOV format. Needs `--coverage-reporter=lcov` flag.
2. **Spec tests ignored**: Behavioral specs marked `#[ignore]` need to be enabled and verified.

## Project Structure

```
crates/cli/src/checks/tests/runners/
├── bun.rs              # Bun runner (complete)
├── js_coverage.rs      # Coverage collection (needs fix)
└── js_coverage_tests.rs # Unit tests (needs Bun-specific test)

tests/specs/checks/tests/
└── coverage.rs         # Behavioral specs (needs #[ignore] removed)
```

## Dependencies

No new dependencies required. Bun must be installed on the system for tests to pass.

## Implementation Phases

### Phase 1: Fix LCOV Reporter Flag

**File:** `crates/cli/src/checks/tests/runners/js_coverage.rs`

The `collect_bun_coverage` function needs the `--coverage-reporter=lcov` flag to produce LCOV output:

**Current (line 129):**
```rust
cmd.args(["test", "--coverage"]);
```

**Fixed:**
```rust
cmd.args(["test", "--coverage", "--coverage-reporter=lcov"]);
```

This matches Bun's documented CLI usage for LCOV output.

**Verification:**
```bash
# Manual test
cd /tmp && mkdir bun-test && cd bun-test
echo '{"name":"test"}' > package.json
echo 'export function foo() { return 1; }' > lib.ts
echo 'import { test, expect } from "bun:test"; import { foo } from "./lib"; test("foo", () => expect(foo()).toBe(1));' > lib.test.ts
bun test --coverage --coverage-reporter=lcov
cat coverage/lcov.info
```

### Phase 2: Add Unit Test for Bun Coverage

**File:** `crates/cli/src/checks/tests/runners/js_coverage_tests.rs`

Add a unit test that verifies LCOV parsing works with Bun-generated output. Since we can't run Bun in unit tests, test the parser with representative LCOV content:

```rust
#[test]
fn parses_bun_style_lcov() {
    // Bun's LCOV output format (similar to other JS tools)
    let lcov = r#"TN:
SF:/project/src/lib.ts
FN:1,foo
FN:2,bar
FNDA:5,foo
FNDA:0,bar
FNF:2
FNH:1
DA:1,5
DA:2,0
LH:1
LF:2
end_of_record
"#;
    let result = parse_lcov_report(lcov, Duration::from_millis(100));
    assert!(result.success);
    assert!(result.line_coverage.is_some());
    let pct = result.line_coverage.unwrap();
    assert!((pct - 50.0).abs() < 0.1, "Expected 50%, got {}", pct);
}
```

### Phase 3: Enable Behavioral Spec

**File:** `tests/specs/checks/tests/coverage.rs`

Remove `#[ignore]` from `bun_runner_collects_javascript_coverage`:

```rust
/// Spec: docs/specs/11-test-runners.md#implicit-coverage
///
/// > Bun runner provides implicit JavaScript/TypeScript coverage.
#[test]
fn bun_runner_collects_javascript_coverage() {
    // ... existing test body unchanged
}
```

**Note:** The Bun spec test does NOT require npm install (unlike Jest/Vitest specs). Bun's test runner is built-in (`bun:test`), so no devDependencies are needed. The ignore message was misleading.

### Phase 4: Verify Coverage Merging

**File:** `tests/specs/checks/tests/coverage.rs`

If Phase 3 passes, consider enabling `multiple_js_suite_coverages_merged` test. However, this test uses Jest which requires npm install. Keep this ignored for now unless there's a CI environment with npm available.

## Key Implementation Details

### Bun Coverage Output

Bun supports multiple coverage reporters via `--coverage-reporter`:
- `text` (default): Console output with percentages
- `lcov`: LCOV format to `coverage/lcov.info`
- `json`: JSON format (alternative)

Reference: [Bun Code Coverage Documentation](https://bun.com/docs/test/code-coverage)

### LCOV Path Normalization

The existing `normalize_js_path()` and `extract_js_package()` functions handle Bun's LCOV paths correctly. Bun outputs absolute paths in LCOV format, which get normalized to project-relative paths.

### No Setup Command Needed

Unlike Jest/Vitest which require `npm install` to install the test framework, Bun's test runner is built-in. The spec test correctly uses:
- `import { test, expect } from 'bun:test'`

No `devDependencies` or setup commands are needed.

## Verification Plan

### Unit Tests
```bash
cargo test --package quench -- js_coverage_tests
```

### Integration Test (Manual)
```bash
# Create temp project
cd /tmp && rm -rf bun-cov-test && mkdir bun-cov-test && cd bun-cov-test

# Setup files
cat > package.json << 'EOF'
{"name": "test-project"}
EOF

cat > src/lib.ts << 'EOF'
export function covered(): number { return 42; }
export function uncovered(): number { return 0; }
EOF

mkdir -p tests
cat > tests/lib.test.ts << 'EOF'
import { covered } from '../src/lib';
import { test, expect } from 'bun:test';
test('covered function', () => { expect(covered()).toBe(42); });
EOF

cat > quench.toml << 'EOF'
[[check.tests.suite]]
runner = "bun"
EOF

# Run quench
quench check tests --ci -o json | jq '.metrics.coverage'
# Expected: {"javascript": ~50}
```

### Behavioral Specs
```bash
# Run the specific test
cargo test --test specs -- bun_runner_collects_javascript_coverage

# Run all coverage specs
cargo test --test specs -- coverage
```

### Full Check
```bash
make check
```

## Summary of Changes

| File | Change |
|------|--------|
| `crates/cli/src/checks/tests/runners/js_coverage.rs` | Add `--coverage-reporter=lcov` flag |
| `crates/cli/src/checks/tests/runners/js_coverage_tests.rs` | Add Bun LCOV parsing test |
| `tests/specs/checks/tests/coverage.rs` | Remove `#[ignore]` from Bun spec |

Total: ~5 lines changed, ~15 lines added.

## References

- [Bun Code Coverage](https://bun.com/docs/test/code-coverage)
- [Bun LCOV Reporter Issue](https://github.com/oven-sh/bun/issues/4015) (resolved)
- `docs/specs/11-test-runners.md` - Runner specification
