# Source scope check level ignored in early return

## Description

When `[rust.suppress].check = "allow"` is set at the base level, the `[rust.suppress.source].check` override is ignored due to an early return that only checks the base level.

## Reproduction

```toml
# quench.toml
version = 1

[rust.suppress]
check = "allow"

[rust.suppress.source]
check = "comment"  # Should require comments for source files
```

```rust
// src/lib.rs - NO comment above suppression
#[allow(dead_code)]
fn should_fail_without_comment() {}
```

**Expected**: Violation reported (source files require comments)
**Actual**: No violation (passes silently)

## Root Cause

In `check_suppress_violations()` at `crates/cli/src/checks/escapes/mod.rs:576-584`:

```rust
let effective_check = if is_test_file {
    config.test.check.unwrap_or(SuppressLevel::Allow)
} else {
    config.check  // BUG: Uses base level, ignores source.check!
};

if effective_check == SuppressLevel::Allow {
    return violations;  // Early return before checking source scope
}
```

Later at line 614-618, the code correctly uses `scope_config.check.unwrap_or(config.check)`, but the early return at line 583 prevents reaching that code.

## Fix

Change line 579 to respect `source.check`:

```rust
let effective_check = if is_test_file {
    config.test.check.unwrap_or(SuppressLevel::Allow)
} else {
    config.source.check.unwrap_or(config.check)  // FIX
};
```

## Shell Is Correct

Interestingly, `shell_suppress.rs:29-33` handles this correctly:

```rust
let effective_check = if is_test_file {
    config.test.check.unwrap_or(SuppressLevel::Allow)
} else {
    scope_config.check.unwrap_or(config.check)  // Correct!
};
```

The Rust check should be updated to match the Shell implementation.

## Impact

Users cannot set a permissive base policy (`allow`) while enforcing stricter rules for source files (`comment` or `forbid`). This prevents the intended use case of:
- Allow all suppressions in test code (default)
- Require comments for suppressions in source code
