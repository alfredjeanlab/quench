# Plan: Remove `[workspace]` Namespace

**Root Feature:** `quench-ecc4`

## Overview

Remove the `[workspace]` TOML config section and `WorkspaceConfig` struct, consolidating all fields into `[project]`/`ProjectConfig`. The two config sections have overlapping `packages` fields, creating confusion. After this change, `ProjectConfig` will hold both user-configured fields and auto-detected workspace metadata.

## Current State

**WorkspaceConfig** (`crates/cli/src/config/mod.rs:200-212`):
```rust
pub struct WorkspaceConfig {
    pub packages: Vec<String>,           // Overlaps with ProjectConfig.packages
    pub package_names: HashMap<String, String>, // Auto-populated for Rust workspaces
}
```

**ProjectConfig** (`crates/cli/src/config/mod.rs:277-299`):
```rust
pub struct ProjectConfig {
    pub name: Option<String>,
    pub source: Vec<String>,
    pub tests: Vec<String>,
    pub packages: Vec<String>,  // Overlaps with WorkspaceConfig.packages
    pub ignore: IgnoreConfig,
}
```

Fallback pattern in agents check:
```rust
let packages = if ctx.config.workspace.packages.is_empty() {
    &ctx.config.project.packages
} else {
    &ctx.config.workspace.packages
};
```

## Dependencies

None - this is an internal refactor with no new external dependencies.

## Implementation Phases

### Phase 1: Add `package_names` to ProjectConfig

Add the auto-detected metadata field to `ProjectConfig`, keeping `WorkspaceConfig` temporarily.

**Files:**
- `crates/cli/src/config/mod.rs`

**Changes:**
```rust
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectConfig {
    pub name: Option<String>,
    #[serde(default)]
    pub source: Vec<String>,
    #[serde(default = "ProjectConfig::default_test_patterns")]
    pub tests: Vec<String>,
    #[serde(default)]
    pub packages: Vec<String>,
    #[serde(default)]
    pub ignore: IgnoreConfig,
    // NEW: Auto-populated package name lookup (path -> name)
    #[serde(default, skip_serializing)]
    pub package_names: std::collections::HashMap<String, String>,
}
```

**Verification:** `make check` passes.

### Phase 2: Update Auto-Detection to Populate ProjectConfig

Change `main.rs` to populate `config.project.packages` and `config.project.package_names` instead of the workspace fields.

**Files:**
- `crates/cli/src/main.rs` (auto-detection logic around lines 130-226)

**Changes:**
- Rust workspace detection: write to `config.project.packages`/`package_names`
- JavaScript workspace detection: write to `config.project.packages`
- Only auto-detect if `config.project.packages.is_empty()` (preserve user config)

**Verification:** Run quench on `tests/fixtures/rust-workspace/` and verify packages are detected correctly.

### Phase 3: Update Check Consumers

Remove fallback logic and read directly from `ProjectConfig`.

**Files:**
- `crates/cli/src/checks/agents/mod.rs` (lines 52-57)
- `crates/cli/src/checks/cloc.rs` (lines 44, 145)
- `crates/cli/src/checks/escapes/mod.rs` (line 175)
- `crates/cli/src/cache.rs` (line 304)

**Changes:**
```rust
// Before:
let packages = if ctx.config.workspace.packages.is_empty() {
    &ctx.config.project.packages
} else {
    &ctx.config.workspace.packages
};

// After:
let packages = &ctx.config.project.packages;
```

Update cache hashing:
```rust
// Before:
config.workspace.packages.hash(&mut hasher);

// After:
config.project.packages.hash(&mut hasher);
```

**Verification:** All behavioral specs pass (`cargo test --all`).

### Phase 4: Remove WorkspaceConfig

Delete the `WorkspaceConfig` struct and `workspace` field from `Config`.

**Files:**
- `crates/cli/src/config/mod.rs`

**Changes:**
1. Remove `WorkspaceConfig` struct definition (lines 200-212)
2. Remove `workspace` field from `Config` struct
3. Remove any remaining workspace references

**Verification:** `make check` passes, no compiler errors.

### Phase 5: Bump Cache Version and Update Tests

Ensure cache invalidation for users upgrading, and update any test fixtures.

**Files:**
- `crates/cli/src/cache.rs` - bump `CACHE_VERSION`
- `tests/fixtures/*/quench.toml` - remove any `[workspace]` sections (if any exist)
- `tests/specs/` - update any specs testing workspace config

**Verification:** Full `make check` passes.

## Key Implementation Details

### Preserving User-Configured Packages

Auto-detection must respect user configuration:
```rust
// In main.rs auto-detection:
if config.project.packages.is_empty() {
    // Only auto-detect if user hasn't specified packages
    if let Some(workspace) = CargoWorkspace::from_root(&root_path) {
        config.project.packages = workspace.members;
        config.project.package_names = workspace.names;
    }
}
```

### Skip Serialization for Auto-Detected Fields

The `package_names` field should not appear in user-facing output or be expected in config files:
```rust
#[serde(default, skip_serializing)]
pub package_names: std::collections::HashMap<String, String>,
```

### Config Validation

Consider adding validation that warns if user specifies both `packages` in config and the project has a detectable workspace (Cargo.toml workspace members). This helps catch misconfigurations.

## Verification Plan

1. **Unit tests:** Existing config parsing tests should continue to pass
2. **Behavioral specs:** Run `cargo test -p quench -- --test-threads=1` for spec tests
3. **Fixtures:** Test against `tests/fixtures/rust-workspace/` and other multi-package fixtures
4. **Manual testing:** Run `quench` on a real Rust workspace to verify auto-detection
5. **Full validation:** `make check` must pass before merge

## Migration Notes

Users with existing `[workspace]` sections in `quench.toml` will see a parse error after this change (due to `deny_unknown_fields`). The error message will guide them to move their config to `[project]`.
