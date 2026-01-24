# Checkpoint Go-1F: Quick Wins Cleanup

**Root Feature:** `quench-0d07`

## Overview

Post-checkpoint cleanup focusing on quick wins (<30 min each) to improve code quality. Based on codebase analysis, the following cleanup opportunities were identified:

| Item | Type | Effort | Decision |
|------|------|--------|----------|
| `golang` config alias | Backwards compat shim | ~10 min | REMOVE |
| Duplicate scope-check logic in `go_suppress.rs` | Code simplification | ~15 min | SIMPLIFY |
| `parse_go_mod` / `enumerate_packages` | Unused in production | N/A | KEEP (used in benchmarks) |
| `bench-deep` fixture for ignored specs | Missing fixture | ~60 min | DEFER |
| Multi-line attribute parsing | Feature gap | ~120 min | DEFER |

**Scope:** 2 quick wins, ~25 min total effort.

## Project Structure

Files affected:

```
quench/
└── crates/cli/src/
    ├── config/
    │   └── mod.rs             # Remove golang alias (lines 57, 470, 690)
    └── checks/escapes/
        └── go_suppress.rs     # Simplify duplicate logic (lines 27-58)
```

## Dependencies

None - cleanup only.

## Implementation Phases

### Phase 1: Remove `golang` Config Alias

**Goal:** Remove backwards compatibility shim for `golang` key.

The config parser accepts both `go` and `golang` as top-level keys, with `golang` being a legacy alias. This shim is no longer needed.

**Changes to `crates/cli/src/config/mod.rs`:**

1. Remove `golang` field from `FlexibleConfig` struct (line 57):
```rust
// DELETE:
#[serde(default)]
golang: Option<toml::Value>,
```

2. Remove `"golang"` from `KNOWN_KEYS` array (line 470):
```rust
// BEFORE:
const KNOWN_KEYS: &[&str] = &[
    "version", "project", "workspace", "check", "rust", "go", "golang", "shell",
];

// AFTER:
const KNOWN_KEYS: &[&str] = &[
    "version", "project", "workspace", "check", "rust", "go", "shell",
];
```

3. Remove `.or(flexible.golang.as_ref())` fallback (line 690):
```rust
// BEFORE:
let go = parse_go_config(flexible.go.as_ref().or(flexible.golang.as_ref()));

// AFTER:
let go = parse_go_config(flexible.go.as_ref());
```

**Verification:**
```bash
cargo test config
cargo clippy
```

**Milestone:** `golang` alias removed, only `go` key supported.

---

### Phase 2: Simplify Go Suppress Logic

**Goal:** Extract duplicate scope-check-level computation into a helper function.

In `crates/cli/src/checks/escapes/go_suppress.rs`, the same logic appears twice:
- Lines 27-31: Initial computation
- Lines 54-58: In-loop computation

**Changes to `crates/cli/src/checks/escapes/go_suppress.rs`:**

Extract helper function:

```rust
/// Get effective check level for a scope.
fn effective_check_level(config: &GoSuppressConfig, is_test: bool) -> SuppressLevel {
    if is_test {
        config.test.check.unwrap_or(SuppressLevel::Allow)
    } else {
        config.source.check.unwrap_or(config.check)
    }
}
```

Update call sites:

```rust
// Line 27 (replace lines 27-31):
let effective_check = effective_check_level(config, is_test_file);

// Line 54 (replace lines 54-58):
let scope_check = effective_check_level(config, is_test_file);
```

**Note:** After extraction, `effective_check` and `scope_check` become identical. This reveals that the second computation is redundant - we can use `effective_check` directly.

**Simplified approach:**
```rust
pub fn check_go_suppress_violations(
    ctx: &CheckContext,
    path: &Path,
    content: &str,
    config: &GoSuppressConfig,
    is_test_file: bool,
    limit_reached: &mut bool,
) -> Vec<Violation> {
    let mut violations = Vec::new();

    // Determine effective check level based on source vs test
    let effective_check = if is_test_file {
        config.test.check.unwrap_or(SuppressLevel::Allow)
    } else {
        config.source.check.unwrap_or(config.check)
    };

    // If allow, no checking needed
    if effective_check == SuppressLevel::Allow {
        return violations;
    }

    // ... rest of function uses effective_check instead of scope_check ...
```

**Verification:**
```bash
cargo test go_suppress
cargo test golang
```

**Milestone:** Duplicate logic removed, single source of truth for check level.

---

### Phase 3: Quality Gates

**Goal:** Ensure all quality checks pass.

**Validation:**
```bash
make check
```

This runs:
- `cargo fmt --all -- --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --all`
- `cargo build --all`
- `./scripts/bootstrap`
- `cargo audit`
- `cargo deny check`

**Milestone:** All quality gates pass.

---

## Key Implementation Details

### Why Keep `parse_go_mod` and `enumerate_packages`

These functions appear unused in production code but are:
1. Exported from `crates/cli/src/adapter/mod.rs` (line 26)
2. Used in benchmarks (`crates/cli/benches/adapter.rs`)
3. Have comprehensive unit tests

They represent future capability for package-level analysis and are actively benchmarked. Removing them would break benchmarks and reduce future functionality.

### Why Defer Multi-line Attribute Parsing

Two specs are ignored with FIXME:
- `tests/specs/adapters/rust.rs:199` - multi-line `#[cfg(test)]`
- `tests/specs/adapters/rust.rs:468` - multi-line `#[allow(...)]`

These require significant parser changes and are out of scope for quick wins.

### Why Defer `bench-deep` Fixture

Four specs in `tests/specs/modes/file_walking.rs` are ignored pending a `bench-deep` fixture. Creating this fixture requires:
1. Generating a deep directory structure
2. Many files to exercise walking performance
3. Documentation of expected behavior

This is estimated at 60+ minutes and deferred.

## Verification Plan

| Phase | Verification Command | Expected Result |
|-------|---------------------|-----------------|
| 1 | `cargo test config` | All config tests pass |
| 2 | `cargo test go_suppress && cargo test golang` | All Go tests pass |
| 3 | `make check` | All quality gates pass |

## Summary

| Phase | Task | Effort | Status |
|-------|------|--------|--------|
| 1 | Remove `golang` config alias | ~10 min | [ ] Pending |
| 2 | Simplify Go suppress logic | ~15 min | [ ] Pending |
| 3 | Quality gates | ~5 min | [ ] Pending |

**Total estimated effort:** ~30 minutes

## Deferred Items

For future cleanup (larger refactors):

1. **Multi-line attribute parsing** - Enable ignored specs in `tests/specs/adapters/rust.rs`
2. **`bench-deep` fixture** - Enable ignored specs in `tests/specs/modes/file_walking.rs`
3. **Go suppress patterns** - Add Go-specific lint patterns (errcheck, gosec, staticcheck) to `config/go.rs:98`
