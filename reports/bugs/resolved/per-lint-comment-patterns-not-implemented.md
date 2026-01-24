# Per-lint comment patterns not implemented

## Description

The `[rust.suppress.source]` and `[rust.suppress.source.<lint-code>]` configuration sections accept per-lint-specific comment patterns, but they are not enforced. All suppressions use only the global `[rust.suppress].comment` pattern.

## Current Behavior

Configuration like this is silently accepted but has no effect:

```toml
[rust.suppress]
check = "comment"
comment = "// KEEP UNTIL:"

[rust.suppress.source.dead_code]
comment = "// NOTE(compat):"

[rust.suppress.source.unused_variables]
comment = "// TODO:"
```

When checking suppressions:
- `#[allow(dead_code)]` requires `// KEEP UNTIL:` comment (global pattern)
- `#[allow(unused_variables)]` requires `// KEEP UNTIL:` comment (global pattern)
- Per-lint patterns like `// NOTE(compat):` and `// TODO:` are **not** respected

### Expected Behavior

Each lint code should be able to have its own required comment pattern:
- `#[allow(dead_code)]` with `// NOTE(compat):` → ✅ pass
- `#[allow(unused_variables)]` with `// TODO:` → ✅ pass
- `#[allow(dead_code)]` with `// KEEP UNTIL:` → ✅ pass (global pattern as fallback)
- `#[allow(dead_code)]` with any other comment → ❌ fail

## Root Cause

The `SuppressScopeConfig` struct (line 207, `crates/cli/src/config/mod.rs`) only supports:
- `allow`: Vec<String> - codes that don't require comments
- `forbid`: Vec<String> - codes never allowed

It does **not** support per-lint-code comment patterns. The configuration sections like `[rust.suppress.source.dead_code]` are parsed as `toml::Value::Table` but never checked for a `comment` field.

### Related Code

- Config structure: `crates/cli/src/config/mod.rs:207`
- Suppress checking: `crates/cli/src/checks/escapes/mod.rs:564`
- Config parsing: `crates/cli/src/config/parse.rs:190`

## Implementation

To fix this, `SuppressScopeConfig` would need a new field to store per-code comment patterns:

```rust
#[derive(Debug, Clone, Default, Deserialize)]
pub struct SuppressScopeConfig {
    pub check: Option<SuppressLevel>,
    pub allow: Vec<String>,
    pub forbid: Vec<String>,
    pub patterns: HashMap<String, String>,  // NEW: lint code → comment pattern
}
```

Then update `check_suppress_violations()` to check per-lint patterns before falling back to global pattern.

## Testing

- Requires "black box" tests in `tests/specs`
- Requires crate-level tests

## Impact

Users expecting fine-grained comment requirements per lint code cannot achieve this configuration, and quench silently accepts the config without enforcing it.

## Architectural Concern

The suppress checking logic is **duplicated** between Rust and Shell with no shared abstraction:

| Component | Rust | Shell |
|-----------|------|-------|
| Config | `SuppressConfig` | `ShellSuppressConfig` |
| Parser | `parse_suppress_config()` | `parse_shell_suppress_config()` |
| Checker | `check_suppress_violations()` | `check_shell_suppress_violations()` |

Both checkers have ~130 lines of nearly identical logic. Adding per-lint patterns would require updating both in parallel, and new adapters (e.g., Go) would need to copy-paste this logic.

**Recommendation:** Refactor into a shared trait/generic function before implementing per-lint patterns, so the feature works automatically for all language adapters.
