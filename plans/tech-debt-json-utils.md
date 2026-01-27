# Tech Debt: Extract Shared JSON Utilities

## Problem

`find_json_object()` is duplicated 4 times across test runners with identical implementations (~25 lines each). `find_json_array()` exists as a variant in cucumber.rs.

## Files to Touch

### Extract to new shared module
- `crates/cli/src/checks/tests/runners/json_utils.rs` (NEW)
- `crates/cli/src/checks/tests/runners/json_utils_tests.rs` (NEW)

### Remove duplicates from
| File | Function | Lines |
|------|----------|-------|
| `runners/rspec.rs` | `find_json_object()` | 177-201 |
| `runners/minitest.rs` | `find_json_object()` | 323-347 |
| `runners/vitest.rs` | `find_json_object()` | 188-212 |
| `runners/jest.rs` | `find_json_object()` | 182-206 |
| `runners/cucumber.rs` | `find_json_array()` | 235-259 |

### Update module exports
- `crates/cli/src/checks/tests/runners/mod.rs`

### Move tests from
| File | Tests to consolidate |
|------|---------------------|
| `runners/rspec_tests.rs` | `find_json_object_*` (181-199) |
| `runners/minitest_tests.rs` | `find_json_object_*` (241+) |
| `runners/vitest_tests.rs` | `find_json_object_*` (123-141) |
| `runners/jest_tests.rs` | `find_json_object_*` (158-176) |

## Implementation

### New json_utils.rs

```rust
//! Shared JSON extraction utilities for test runners.
//!
//! Test runner output often contains JSON embedded in other text (logs,
//! warnings, etc.). These utilities extract valid JSON from mixed output.

/// Find the first complete JSON object in a string.
///
/// Handles nested braces correctly. Returns None if no valid object found.
pub fn find_json_object(s: &str) -> Option<&str> {
    find_json_delimited(s, '{', '}')
}

/// Find the first complete JSON array in a string.
///
/// Handles nested brackets correctly. Returns None if no valid array found.
pub fn find_json_array(s: &str) -> Option<&str> {
    find_json_delimited(s, '[', ']')
}

fn find_json_delimited(s: &str, open: char, close: char) -> Option<&str> {
    let start = s.find(open)?;
    let mut depth = 0;
    let mut end = start;

    for (i, c) in s[start..].char_indices() {
        match c {
            c if c == open => depth += 1,
            c if c == close => {
                depth -= 1;
                if depth == 0 {
                    end = start + i + 1;
                    break;
                }
            }
            _ => {}
        }
    }

    if depth == 0 && end > start {
        Some(&s[start..end])
    } else {
        None
    }
}
```

### Update runners to use shared module

```rust
// In rspec.rs, minitest.rs, vitest.rs, jest.rs:
use super::json_utils::find_json_object;

// In cucumber.rs:
use super::json_utils::find_json_array;
```

## Verification

```bash
# Ensure no duplicate definitions remain
rg "fn find_json_object" crates/cli/src/checks/tests/runners/
# Should only show json_utils.rs

# Run tests
cargo test --all -- json_utils
cargo test --all -- rspec
cargo test --all -- minitest
cargo test --all -- vitest
cargo test --all -- jest
cargo test --all -- cucumber
```

## Impact

- **Lines removed:** ~100 (4 × 25 duplicates)
- **Lines added:** ~35 (shared module + tests)
- **Net reduction:** ~65 lines
- **Maintenance:** Single location for JSON parsing fixes
