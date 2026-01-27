# Tech Debt: Test Runner Helper Functions

## Problem

All 11 test runners repeat identical patterns for setup, timeout handling, and coverage collection.

## Files Affected

| File | Setup | Timeout | Coverage |
|------|-------|---------|----------|
| `runners/cargo.rs` | ✓ | ✓ | ✓ |
| `runners/go.rs` | ✓ | ✓ | ✓ |
| `runners/pytest.rs` | ✓ | ✓ | - |
| `runners/vitest.rs` | ✓ | ✓ | ✓ |
| `runners/jest.rs` | ✓ | ✓ | ✓ |
| `runners/bun.rs` | ✓ | ✓ | - |
| `runners/bats.rs` | ✓ | ✓ | - |
| `runners/rspec.rs` | ✓ | ✓ | ✓ |
| `runners/minitest.rs` | ✓ | ✓ | ✓ |
| `runners/cucumber.rs` | ✓ | ✓ | - |
| `runners/custom.rs` | ✓ | ✓ | - |

## Duplicated Patterns

### 1. Setup Command (~6 lines x 11)
```rust
if let Some(setup) = &config.setup
    && let Err(e) = super::run_setup_command(setup, ctx.root)
{
    return TestRunResult::failed(Duration::ZERO, e);
}
```

### 2. Timeout Error Handling (~6 lines x 11)
```rust
Err(e) if e.kind() == ErrorKind::TimedOut => {
    let timeout_msg = config
        .timeout
        .map(|t| format_timeout_error("runner_name", t))
        .unwrap_or_else(|| "timed out".to_string());
    return TestRunResult::failed(start.elapsed(), timeout_msg);
}
```

### 3. Coverage Collection (~12 lines x 6)
```rust
if ctx.collect_coverage {
    let coverage = collect_xxx_coverage(ctx.root, config.path.as_deref());
    if let Some(line_coverage) = coverage.line_coverage {
        let mut cov_map = HashMap::new();
        cov_map.insert("language".to_string(), line_coverage);
        result = result.with_coverage(cov_map);
    }
    if !coverage.packages.is_empty() {
        result = result.with_package_coverage(coverage.packages);
    }
}
```

### 4. JSON Parse with Fallback (~10 lines x 4)
```rust
let json_str = find_json_object(stdout);
let output: Output = match json_str.and_then(|s| serde_json::from_str(s).ok()) {
    Some(o) => o,
    None => {
        if stdout.contains("FAIL") || stdout.contains("Error") {
            return TestRunResult::failed(total_time, "tool failed");
        }
        return TestRunResult::passed(total_time);
    }
};
```

## Proposed Solutions

### 1. Setup Macro

```rust
// In runners/mod.rs
macro_rules! run_setup_or_fail {
    ($config:expr, $ctx:expr) => {
        if let Some(setup) = &$config.setup {
            if let Err(e) = super::run_setup_command(setup, $ctx.root) {
                return TestRunResult::failed(Duration::ZERO, e);
            }
        }
    };
}
```

**Usage:**
```rust
pub fn run(ctx: &TestRunContext, config: &CargoConfig) -> TestRunResult {
    run_setup_or_fail!(config, ctx);
    // ... rest of runner
}
```

### 2. Timeout Helper Function

```rust
// In runners/mod.rs
pub fn handle_timeout_error(
    elapsed: Duration,
    timeout: Option<Duration>,
    runner_name: &str,
) -> TestRunResult {
    let msg = timeout
        .map(|t| format_timeout_error(runner_name, t))
        .unwrap_or_else(|| "timed out".to_string());
    TestRunResult::failed(elapsed, msg)
}

// Or as a macro for early return:
macro_rules! timeout_error {
    ($config:expr, $start:expr, $name:literal) => {
        return handle_timeout_error($start.elapsed(), $config.timeout, $name)
    };
}
```

**Usage:**
```rust
Err(e) if e.kind() == ErrorKind::TimedOut => {
    timeout_error!(config, start, "cargo");
}
```

### 3. Coverage Helper Method

```rust
// In runners/mod.rs or TestRunResult impl
impl TestRunResult {
    pub fn with_collected_coverage(
        self,
        coverage: CoverageResult,
        language: &str,
    ) -> Self {
        let mut result = self;
        if let Some(line_coverage) = coverage.line_coverage {
            result = result.with_coverage([(language.to_string(), line_coverage)].into());
        }
        if !coverage.packages.is_empty() {
            result = result.with_package_coverage(coverage.packages);
        }
        result
    }
}
```

**Usage:**
```rust
if ctx.collect_coverage {
    let coverage = collect_rust_coverage(ctx.root, config.path.as_deref());
    result = result.with_collected_coverage(coverage, "rust");
}
```

### 4. JSON Parse Helper

Already covered in `tech-debt-json-utils.md`, but could add:

```rust
pub fn parse_json_or_fallback<T: DeserializeOwned>(
    stdout: &str,
    total_time: Duration,
    failure_indicators: &[&str],
) -> Result<T, TestRunResult> {
    let json_str = find_json_object(stdout);
    match json_str.and_then(|s| serde_json::from_str(s).ok()) {
        Some(output) => Ok(output),
        None => {
            if failure_indicators.iter().any(|ind| stdout.contains(ind)) {
                Err(TestRunResult::failed(total_time, "tool failed (no JSON)"))
            } else {
                Err(TestRunResult::passed(total_time))
            }
        }
    }
}
```

## Implementation Steps

1. Add to `runners/mod.rs`:
   - `run_setup_or_fail!` macro
   - `handle_timeout_error()` function
   - `with_collected_coverage()` method on TestRunResult

2. Update each runner to use helpers:
   - cargo.rs, go.rs, pytest.rs, vitest.rs, jest.rs
   - bun.rs, bats.rs, rspec.rs, minitest.rs, cucumber.rs, custom.rs

3. Update runner tests

## Impact

- **Lines removed:** ~180 LOC (across 11 files)
- **Files modified:** 12 (11 runners + 1 mod.rs)
- **Risk:** Low (simple refactoring)
- **Benefit:** Consistent behavior, easier to modify error messages

## Verification

```bash
cargo test --all -- runners
cargo test --test specs -- tests
```

## Priority

**LOW** - Small gains per file, but many files benefit.
