# Tech Debt: Add yare Parameterized Tests to Runners

## Problem

New test runner tests (rspec, minitest, cucumber) use separate `#[test]` functions for each case, while older adapter tests use `yare::parameterized` for cleaner, more maintainable test code.

Example of current verbose pattern:
```rust
#[test]
fn parses_passing_tests() { ... }  // 35 lines with inline JSON

#[test]
fn parses_failing_tests() { ... }  // 30 lines with inline JSON

#[test]
fn parses_pending_tests() { ... }  // 25 lines with inline JSON
```

## Files to Touch

### High-value refactors (test runners)
| File | Current Tests | Parameterizable |
|------|---------------|-----------------|
| `runners/rspec_tests.rs` | 8 tests | 3-4 groups |
| `runners/minitest_tests.rs` | 10+ tests | 3-4 groups |
| `runners/cucumber_tests.rs` | 6+ tests | 2-3 groups |
| `runners/vitest_tests.rs` | 8 tests | 2-3 groups |
| `runners/jest_tests.rs` | 8 tests | 2-3 groups |

### Medium-value refactors (coverage)
| File | Current Tests | Parameterizable |
|------|---------------|-----------------|
| `runners/js_coverage_tests.rs` | Multiple similar | File format cases |
| `runners/go_coverage_tests.rs` | Multiple similar | Profile format cases |
| `runners/ruby_coverage_tests.rs` | Multiple similar | SimpleCov format cases |

## Implementation Pattern

### Before (rspec_tests.rs)
```rust
#[test]
fn parses_passing_tests() {
    let output = r#"{ "examples": [...], "summary": {...} }"#;
    let result = parse_rspec_json(output, Duration::from_secs(1));
    assert!(result.passed);
    assert_eq!(result.tests.len(), 2);
}

#[test]
fn parses_failing_tests() {
    let output = r#"{ "examples": [...], "summary": {...} }"#;
    let result = parse_rspec_json(output, Duration::from_secs(1));
    assert!(!result.passed);
}

#[test]
fn parses_pending_tests() {
    let output = r#"{ "examples": [...], "summary": {...} }"#;
    let result = parse_rspec_json(output, Duration::from_secs(1));
    assert!(result.passed);  // pending doesn't fail
}
```

### After (rspec_tests.rs)
```rust
use yare::parameterized;

const PASSING_JSON: &str = r#"{ "examples": [...], "summary": {...} }"#;
const FAILING_JSON: &str = r#"{ "examples": [...], "summary": {...} }"#;
const PENDING_JSON: &str = r#"{ "examples": [...], "summary": {...} }"#;

#[parameterized(
    passing = { PASSING_JSON, true, 2, 0 },
    failing = { FAILING_JSON, false, 2, 1 },
    pending = { PENDING_JSON, true, 2, 0 },  // pending doesn't fail overall
)]
fn parses_test_results(json: &str, expect_passed: bool, test_count: usize, fail_count: usize) {
    let result = parse_rspec_json(json, Duration::from_secs(1));
    assert_eq!(result.passed, expect_passed, "passed mismatch");
    assert_eq!(result.tests.len(), test_count, "test count mismatch");
    let actual_fails = result.tests.iter().filter(|t| !t.passed).count();
    assert_eq!(actual_fails, fail_count, "failure count mismatch");
}
```

### Minitest text parsing cases
```rust
#[parameterized(
    standard = { "10 runs, 20 assertions, 1 failures, 0 errors, 1 skips", 10, 1, 0, 1 },
    no_skips = { "5 runs, 10 assertions, 0 failures, 0 errors, 0 skips", 5, 0, 0, 0 },
    with_errors = { "8 runs, 15 assertions, 2 failures, 1 errors, 0 skips", 8, 2, 1, 0 },
)]
fn parses_summary_line(line: &str, runs: u32, failures: u32, errors: u32, skips: u32) {
    let summary = parse_summary_line(line).unwrap();
    assert_eq!(summary.runs, runs);
    assert_eq!(summary.failures, failures);
    assert_eq!(summary.errors, errors);
    assert_eq!(summary.skips, skips);
}
```

## Phased Approach

### Phase 1: Runner parsing tests
1. `rspec_tests.rs` - status parsing, timing extraction
2. `minitest_tests.rs` - JSON parsing, text fallback parsing
3. `cucumber_tests.rs` - scenario aggregation

### Phase 2: Coverage tests
1. `js_coverage_tests.rs` - LCOV format variations
2. `go_coverage_tests.rs` - profile format variations
3. `ruby_coverage_tests.rs` - SimpleCov format variations

## Verification

```bash
# Verify yare is already a dependency
grep yare Cargo.toml

# Run refactored tests
cargo test --all -- rspec
cargo test --all -- minitest
cargo test --all -- cucumber

# Check test count didn't decrease
cargo test --all -- --list 2>&1 | grep -c "test$"
```

## Impact

- **Lines reduced:** ~20-30% per test file
- **Readability:** Test cases clearly enumerated in parameterized block
- **Maintenance:** Add new cases without new functions
- **Consistency:** Matches existing adapter test patterns
