# Plan: Remove Duplicate test_patterns/source_patterns from [check.tests.commit]

## Overview

Remove the `test_patterns` and `source_patterns` fields from `[check.tests.commit]` configuration. The commit correlation check should always use the project-level and language-level patterns that are already resolved via `resolve_project_patterns()`, eliminating the redundant override layer. No backwards compatibility—unknown TOML keys will be rejected by `deny_unknown_fields`.

## Project Structure

Key files affected:

```
crates/cli/src/
├── config/
│   ├── test_config.rs       # Remove fields from TestsCommitConfig
│   └── tests_check.rs       # Orphaned file; also has the fields (cleanup)
├── checks/testing/
│   ├── mod.rs               # Simplify pattern resolution in run()
│   └── mod_tests.rs         # Update tests referencing config fields
├── cmd_check.rs             # Remove verbose logging for commit patterns
└── adapter/patterns.rs      # No changes (already correct)
docs/specs/checks/tests.md   # Update spec to remove pattern fields
```

## Dependencies

None. All changes are internal removals; no new crates needed.

## Implementation Phases

### Phase 1: Remove fields from `TestsCommitConfig`

**File:** `crates/cli/src/config/test_config.rs`

Remove from `TestsCommitConfig`:
- `test_patterns` field + serde attribute
- `source_patterns` field + serde attribute
- `default_test_patterns()` method
- `default_source_patterns()` method
- Corresponding lines in `Default` impl

The struct should keep: `check`, `scope`, `placeholders`, `exclude`.

```rust
/// Tests commit check configuration.
#[derive(Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TestsCommitConfig {
    pub check: String,
    pub scope: String,
    pub placeholders: String,
    pub exclude: Vec<String>,
}
```

**Also:** Delete or update `crates/cli/src/config/tests_check.rs` (orphaned file with duplicate struct). If it's not imported anywhere, delete it.

### Phase 2: Simplify correlation pattern resolution in the check

**File:** `crates/cli/src/checks/testing/mod.rs` (lines 100–125)

Replace the three-tier `if !config.source_patterns.is_empty()` / `if config.test_patterns.is_empty()` branches with direct use of `resolved` patterns:

```rust
let resolved = resolve_project_patterns(ctx.root, ctx.config);
let lang = detect_language(ctx.root);

let correlation_config = CorrelationConfig {
    source_patterns: if !resolved.source.is_empty() {
        resolved.source
    } else {
        vec!["src/**/*".to_string()]
    },
    test_patterns: resolved.test,
    exclude_patterns: if config.exclude.is_empty() {
        correlation_exclude_defaults(lang)
    } else {
        config.exclude.clone()
    },
};
```

The `exclude` field stays on `TestsCommitConfig` since correlation excludes (entry points like `mod.rs`, `main.rs`) are distinct from walker-level excludes.

### Phase 3: Remove verbose logging for removed fields

**File:** `crates/cli/src/cmd_check.rs` (lines 118–132)

Remove the two blocks that log `check.tests.commit.source_patterns` and `check.tests.commit.test_patterns`:

```rust
// DELETE these blocks:
if !config.check.tests.commit.source_patterns.is_empty() { ... }
if !config.check.tests.commit.test_patterns.is_empty() { ... }
```

The project-level patterns are already logged via `verbose_patterns(&verbose, "project.source", ...)` and `verbose_patterns(&verbose, "project.tests", ...)`, so no information is lost.

### Phase 4: Update unit tests

**File:** `crates/cli/src/checks/testing/mod_tests.rs`

Update `tests_commit_config_defaults` test: remove assertions for `test_patterns` and `source_patterns`.

```rust
#[test]
fn tests_commit_config_defaults() {
    let config = TestsCommitConfig::default();
    assert_eq!(config.check, "off");
    assert_eq!(config.scope, "branch");
    assert_eq!(config.placeholders, "allow");
    assert!(config.exclude.is_empty());
}
```

**File:** `crates/cli/src/checks/testing/correlation/mod_tests.rs`

No changes needed—`rust_correlation_config()` constructs `CorrelationConfig` directly with inline patterns, not from `TestsCommitConfig`. The `CorrelationConfig` struct itself is unchanged.

### Phase 5: Update specification

**File:** `docs/specs/checks/tests.md`

Remove the `test_patterns` and `source_patterns` entries from the Configuration section TOML example (lines ~289–300). The patterns are now always inherited from `[project]` / `[rust]` / etc.

Before:
```toml
[check.tests.commit]
check = "error"
scope = "branch"
placeholders = "allow"
test_patterns = [...]
source_patterns = [...]
exclude = [...]
```

After:
```toml
[check.tests.commit]
check = "error"
scope = "branch"
placeholders = "allow"

# Exclude patterns (never require tests)
# Defaults are language-dependent. For Rust:
exclude = [
  "**/mod.rs",
  "**/lib.rs",
  "**/main.rs",
  "**/generated/**",
]
```

### Phase 6: Verify

1. `cargo fmt --all -- --check`
2. `cargo clippy --all-targets --all-features -- -D warnings`
3. `cargo test --all`
4. `cargo build --all`

Confirm that existing `quench.toml` files (project root and test fixtures) don't use `test_patterns` or `source_patterns` under `[check.tests.commit]`—if any do, they will fail to parse due to `deny_unknown_fields`, which is the intended behavior (no backwards compat).

## Key Implementation Details

- **`deny_unknown_fields`** is already on `TestsCommitConfig`, so any TOML file that still specifies `test_patterns` or `source_patterns` will produce a clear parse error. This is the desired breaking change.
- **`CorrelationConfig`** (used by `classify.rs`) is unchanged—it still has `test_patterns`, `source_patterns`, and `exclude_patterns` fields. The difference is that these are now always populated from `resolve_project_patterns()` rather than from commit config fields.
- **The `exclude` field remains** on `TestsCommitConfig` because correlation excludes (entry points, declarations) are semantically different from project/language walker excludes. Users may want to customize which files are exempt from requiring tests.
- **Orphaned file `tests_check.rs`**: This file defines a duplicate `TestsCommitConfig` but is not imported by `config/mod.rs`. It should be deleted.

## Verification Plan

1. **Unit tests** – `cargo test --all` passes (Phase 4 updates ensure this)
2. **Clippy** – No warnings from dead code removal
3. **Config rejection** – A TOML file with `test_patterns` under `[check.tests.commit]` produces a parse error
4. **Behavioral** – Run `quench check` on the quench project itself; correlation behavior unchanged since the project doesn't configure custom commit patterns
5. **Verbose output** – Run with `--verbose` and confirm project-level patterns appear but commit-level pattern overrides do not
