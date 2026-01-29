# Jest Coverage Collection (Phase 4981)

Enable end-to-end coverage reporting from Jest/Vitest/Bun test runs.

## Overview

The Jest coverage implementation is **code-complete** in `crates/cli/src/checks/tests/runners/js_coverage.rs`. The behavioral specs are disabled because:

1. Test fixtures lack `node_modules` (npm dependencies not installed)
2. End-to-end tests require real Jest execution with actual coverage output

This plan enables the behavioral specs by creating proper test fixtures with dependencies.

## Current State

### Implemented (js_coverage.rs)
- `collect_jest_coverage()` - runs Jest with `--coverage --coverageReporters=lcov`
- `collect_vitest_coverage()` - runs Vitest with `--coverage --coverage.reporter=lcov`
- `collect_bun_coverage()` - runs Bun with `--coverage`
- `parse_lcov_report()` - parses LCOV format with path normalization
- Coverage aggregation by file and package (monorepo support)
- Node modules exclusion

### Disabled Specs (tests/specs/checks/tests/coverage.rs)
```rust
#[ignore = "TODO: Phase 4981 - Requires npm install"]
fn jest_runner_collects_javascript_coverage()

#[ignore = "TODO: Phase 4981 - Requires npm install"]
fn vitest_runner_collects_javascript_coverage()

#[ignore = "TODO: Phase 4981 - Requires bun install"]
fn bun_runner_collects_javascript_coverage()

#[ignore = "TODO: Phase 4981 - Requires npm install"]
fn multiple_js_suite_coverages_merged()
```

## Project Structure

```
tests/fixtures/javascript/
├── jest-coverage/           # NEW: Jest coverage test fixture
│   ├── package.json
│   ├── jest.config.js
│   ├── src/
│   │   └── math.js
│   └── tests/
│       └── math.test.js
├── vitest-coverage/         # NEW: Vitest coverage test fixture
│   ├── package.json
│   ├── vitest.config.ts
│   ├── src/
│   │   └── math.ts
│   └── tests/
│       └── math.test.ts
└── (existing fixtures...)

scripts/
└── setup-js-fixtures.sh     # NEW: Install dependencies in JS fixtures
```

## Dependencies

**Runtime (for specs):**
- Node.js 18+ (for Jest/Vitest)
- npm (package manager)
- Bun (optional, for bun coverage spec)

**Crates (already present):**
- No new Rust dependencies needed

## Implementation Phases

### Phase 1: Create Jest Coverage Fixture

Create a minimal Jest project fixture that produces predictable coverage.

**Files:**

`tests/fixtures/javascript/jest-coverage/package.json`:
```json
{
  "name": "jest-coverage-fixture",
  "private": true,
  "scripts": {
    "test": "jest"
  },
  "devDependencies": {
    "jest": "^29.7.0"
  }
}
```

`tests/fixtures/javascript/jest-coverage/jest.config.js`:
```javascript
module.exports = {
  testEnvironment: 'node',
  collectCoverageFrom: ['src/**/*.js'],
};
```

`tests/fixtures/javascript/jest-coverage/src/math.js`:
```javascript
function covered() { return 42; }
function uncovered() { return 0; }
module.exports = { covered, uncovered };
```

`tests/fixtures/javascript/jest-coverage/tests/math.test.js`:
```javascript
const { covered } = require('../src/math');
test('covered function', () => { expect(covered()).toBe(42); });
```

**Verification:**
```bash
cd tests/fixtures/javascript/jest-coverage
npm install
npm test -- --coverage
# Should show ~50% coverage (covered function tested, uncovered not)
```

### Phase 2: Create Vitest Coverage Fixture

Create a minimal Vitest project with TypeScript support.

**Files:**

`tests/fixtures/javascript/vitest-coverage/package.json`:
```json
{
  "name": "vitest-coverage-fixture",
  "private": true,
  "type": "module",
  "scripts": {
    "test": "vitest run"
  },
  "devDependencies": {
    "vitest": "^2.0.0",
    "@vitest/coverage-v8": "^2.0.0"
  }
}
```

`tests/fixtures/javascript/vitest-coverage/vitest.config.ts`:
```typescript
import { defineConfig } from 'vitest/config';
export default defineConfig({
  test: {
    coverage: {
      provider: 'v8',
      include: ['src/**/*.ts'],
    },
  },
});
```

`tests/fixtures/javascript/vitest-coverage/src/math.ts`:
```typescript
export function covered(): number { return 42; }
export function uncovered(): number { return 0; }
```

`tests/fixtures/javascript/vitest-coverage/tests/math.test.ts`:
```typescript
import { covered } from '../src/math';
import { test, expect } from 'vitest';
test('covered function', () => { expect(covered()).toBe(42); });
```

**Verification:**
```bash
cd tests/fixtures/javascript/vitest-coverage
npm install
npm test -- --coverage
```

### Phase 3: Fixture Setup Script

Create a script to install dependencies in JS fixtures.

`scripts/setup-js-fixtures.sh`:
```bash
#!/bin/bash
set -e

FIXTURES_DIR="tests/fixtures/javascript"

for fixture in jest-coverage vitest-coverage; do
    if [ -d "$FIXTURES_DIR/$fixture" ]; then
        echo "Installing dependencies in $fixture..."
        (cd "$FIXTURES_DIR/$fixture" && npm install --silent)
    fi
done

echo "JavaScript fixtures ready."
```

**CI Integration:**
- Add setup step to CI workflow before running specs
- Only install if fixtures exist and node_modules missing

### Phase 4: Enable Behavioral Specs

Update specs to use new fixtures and remove `#[ignore]` attributes.

`tests/specs/checks/tests/coverage.rs` changes:

```rust
/// Spec: docs/specs/11-test-runners.md#implicit-coverage
///
/// > Jest runner provides implicit JavaScript coverage via --coverage.
#[test]
fn jest_runner_collects_javascript_coverage() {
    let result = check("tests")
        .on("javascript/jest-coverage")
        .args(&["--ci"])
        .json()
        .passes();
    let metrics = result.require("metrics");

    let coverage = metrics.get("coverage").and_then(|v| v.as_object());
    assert!(coverage.is_some(), "Expected coverage metrics");

    let js_coverage = coverage.unwrap().get("javascript").and_then(|v| v.as_f64());
    assert!(js_coverage.is_some(), "Expected javascript coverage");

    let pct = js_coverage.unwrap();
    assert!(
        pct > 40.0 && pct < 60.0,
        "Expected ~50% coverage, got {}",
        pct
    );
}
```

**Key Changes:**
1. Remove `#[ignore = "TODO: Phase 4981 - ..."]`
2. Use fixture path `"javascript/jest-coverage"` instead of temp project
3. Keep coverage assertion range (40-60% for ~50% expected)

### Phase 5: Add Merged Coverage Spec

Enable the multi-suite coverage merging spec.

Create fixture `tests/fixtures/javascript/jest-merged-coverage/`:
- Two test directories (`tests/unit/`, `tests/integration/`)
- Source file with two functions
- Each test suite covers one function
- Merged coverage should be ~100%

Update spec to use fixture and remove `#[ignore]`.

### Phase 6: CI Integration

Update CI workflow to:
1. Install Node.js (if not present)
2. Run `scripts/setup-js-fixtures.sh` before specs
3. Cache `node_modules` for faster subsequent runs

**Makefile addition:**
```makefile
.PHONY: setup-js-fixtures
setup-js-fixtures:
	@./scripts/setup-js-fixtures.sh

.PHONY: check
check: setup-js-fixtures
	cargo fmt --all -- --check
	cargo clippy --all-targets --all-features -- -D warnings
	cargo test --all
	# ... rest of checks
```

## Key Implementation Details

### LCOV Format

Jest/Vitest/Bun all output LCOV format (`coverage/lcov.info`):
```
SF:/path/to/file.js
LH:2          # Lines hit
LF:4          # Lines found
end_of_record
```

Coverage calculation: `(LH / LF) * 100`

### Path Normalization

`normalize_js_path()` handles:
- Absolute paths → relative paths
- Monorepo patterns (`packages/`, `apps/`, `libs/`)
- Node modules exclusion
- Fallback to filename only

### Package Aggregation

`extract_js_package()` extracts package from path:
- `packages/core/src/index.ts` → `packages/core`
- `src/utils.js` → `root`

### Coverage Cleanup

All collectors remove `coverage/` directory after parsing to keep project clean.

## Verification Plan

### Unit Tests (existing, in js_coverage_tests.rs)
- LCOV parsing with various formats
- Path normalization edge cases
- Package extraction patterns
- Multi-file coverage aggregation

### Behavioral Specs (Phase 4)
```bash
# Run specific coverage specs
cargo test --test specs jest_runner_collects
cargo test --test specs vitest_runner_collects
cargo test --test specs multiple_js_suite_coverages
```

### Manual Verification
```bash
# Direct fixture test
cd tests/fixtures/javascript/jest-coverage
npm test -- --coverage
cat coverage/lcov.info

# Full integration
cd /path/to/quench
./scripts/setup-js-fixtures.sh
cargo build
./target/debug/quench check tests --ci -C tests/fixtures/javascript/jest-coverage
```

### CI Verification
- All `cargo test --test specs` pass
- Coverage specs no longer marked `#[ignore]`
- CI workflow includes JS fixture setup step

## Notes

- Bun coverage spec may remain ignored if Bun is not installed in CI
- Consider conditional compilation or skip logic for Bun tests
- Node.js 18+ required for modern Jest/Vitest versions
- Fixtures should be minimal to keep test suite fast
