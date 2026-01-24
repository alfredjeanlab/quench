# Phase 1547: Per-Language Policy Check Level Implementation

**Root Feature:** `quench-eb8d`

## Overview

Implement per-language policy check levels for all language adapters (`rust`, `golang`, `javascript`, `shell`). Each language's `[lang.policy]` section will support a `check` field that controls whether policy violations (lint config standalone requirement) are errors, warnings, or disabled.

This completes the config schema work started in Phase 1545 (which created behavioral specs and test fixtures).

## Project Structure

Files to modify:
```
crates/cli/src/
├── config/
│   ├── mod.rs              # Add policy_check_level_for_language() method
│   ├── go.rs               # Add check field to GoPolicyConfig
│   ├── javascript.rs       # Add check field to JavaScriptPolicyConfig
│   └── shell.rs            # Add check field to ShellPolicyConfig
│   # Note: RustPolicyConfig in mod.rs also needs check field
├── checks/escapes/
│   └── lint_policy.rs      # Respect per-language check level
└── config/
    └── mod_tests.rs        # Unit tests for policy config parsing

tests/specs/checks/
└── policy_lang.rs          # Remove #[ignore] attributes
```

## Dependencies

No new external dependencies. Uses existing:
- `serde` for deserialization
- `CheckLevel` enum from `crates/cli/src/config/checks.rs`

## Implementation Phases

### Phase 1: Add Check Field to PolicyConfig Structs

Add `check: Option<CheckLevel>` to all per-language policy config structs.

**`crates/cli/src/config/mod.rs` - RustPolicyConfig:**
```rust
/// Rust lint policy configuration.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RustPolicyConfig {
    /// Check level: "error" | "warn" | "off" (default: inherits from global).
    #[serde(default)]
    pub check: Option<CheckLevel>,

    /// Lint config changes policy: "standalone" requires separate PRs.
    #[serde(default)]
    pub lint_changes: LintChangesPolicy,

    /// Files that trigger the standalone requirement.
    #[serde(default = "RustPolicyConfig::default_lint_config")]
    pub lint_config: Vec<String>,
}
```

**`crates/cli/src/config/go.rs` - GoPolicyConfig:**
```rust
/// Go lint policy configuration.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GoPolicyConfig {
    /// Check level: "error" | "warn" | "off" (default: inherits from global).
    #[serde(default)]
    pub check: Option<CheckLevel>,

    /// Lint config changes policy: "standalone" requires separate PRs.
    #[serde(default)]
    pub lint_changes: LintChangesPolicy,

    /// Files that trigger the standalone requirement.
    #[serde(default = "GoPolicyConfig::default_lint_config")]
    pub lint_config: Vec<String>,
}
```

Same pattern for `JavaScriptPolicyConfig` and `ShellPolicyConfig`.

**Verification:** `cargo build` compiles, existing tests pass.

### Phase 2: Add Config Resolution Method

Add `policy_check_level_for_language()` method to `Config` in `crates/cli/src/config/mod.rs`:

```rust
impl Config {
    /// Get effective policy check level for a language.
    ///
    /// Resolution order:
    /// 1. {lang}.policy.check if set
    /// 2. CheckLevel::Error (default - policy violations fail)
    ///
    /// The `language` parameter should be an adapter name (e.g., "rust", "go").
    pub fn policy_check_level_for_language(&self, language: &str) -> CheckLevel {
        let lang_level = match language {
            "rust" => self.rust.policy.check,
            "go" | "golang" => self.golang.policy.check,
            "javascript" | "js" => self.javascript.policy.check,
            "shell" | "sh" => self.shell.policy.check,
            _ => None,
        };
        lang_level.unwrap_or(CheckLevel::Error)
    }
}
```

**Verification:** Method compiles and returns expected defaults.

### Phase 3: Update Lint Policy Check to Respect Check Level

Modify `crates/cli/src/checks/escapes/lint_policy.rs` to use the per-language check level.

Current flow:
1. `check_lint_policy()` dispatches to language-specific functions
2. Each function checks `policy.lint_changes != LintChangesPolicy::Standalone`
3. Returns `Vec<Violation>` if violated

New flow:
1. Check `config.policy_check_level_for_language(lang)` first
2. If `CheckLevel::Off`, return empty (skip check)
3. If `CheckLevel::Warn`, mark violations as warnings (non-failing)
4. If `CheckLevel::Error`, mark violations as errors (failing)

**Key change in `check_rust_lint_policy`:**
```rust
fn check_rust_lint_policy(ctx: &CheckContext, rust_config: &RustConfig) -> Vec<Violation> {
    // Check policy check level first
    let check_level = ctx.config.policy_check_level_for_language("rust");
    if check_level == CheckLevel::Off {
        return Vec::new();
    }

    if rust_config.policy.lint_changes != LintChangesPolicy::Standalone {
        return Vec::new();
    }

    // ... existing logic ...

    // Mark violations with check level for proper handling
    make_policy_violation(
        result.standalone_violated,
        &result.changed_lint_config,
        &result.changed_source,
        check_level,  // Pass check level
    )
}
```

**Update `make_policy_violation` to include check level:**
```rust
fn make_policy_violation(
    violated: bool,
    lint_config: &[String],
    source: &[String],
    check_level: CheckLevel,
) -> Vec<Violation> {
    if !violated {
        return Vec::new();
    }
    vec![Violation {
        file: None,
        line: None,
        violation_type: "lint_policy".to_string(),
        // ... existing fields ...
        is_warning: check_level == CheckLevel::Warn,  // Add warning flag
    }]
}
```

**Note:** The `Violation` struct may need an `is_warning` field or the check runner needs to handle CheckLevel. Check existing patterns in escapes check.

**Verification:** Unit tests in lint_policy module pass.

### Phase 4: Add Unit Tests for Policy Config Parsing

Add unit tests to `crates/cli/src/config/mod_tests.rs`:

```rust
#[test]
fn parses_rust_policy_check_off() {
    let config = parse(
        r#"
version = 1

[rust.policy]
check = "off"
lint_changes = "standalone"
"#,
        Path::new("test.toml"),
    )
    .unwrap();

    assert_eq!(config.rust.policy.check, Some(CheckLevel::Off));
    assert_eq!(config.policy_check_level_for_language("rust"), CheckLevel::Off);
}

#[test]
fn parses_rust_policy_check_warn() {
    let config = parse(
        r#"
version = 1

[rust.policy]
check = "warn"
lint_changes = "standalone"
"#,
        Path::new("test.toml"),
    )
    .unwrap();

    assert_eq!(config.rust.policy.check, Some(CheckLevel::Warn));
    assert_eq!(config.policy_check_level_for_language("rust"), CheckLevel::Warn);
}

#[test]
fn rust_policy_check_defaults_to_error() {
    let config = parse(
        r#"
version = 1

[rust.policy]
lint_changes = "standalone"
"#,
        Path::new("test.toml"),
    )
    .unwrap();

    assert_eq!(config.rust.policy.check, None);
    assert_eq!(config.policy_check_level_for_language("rust"), CheckLevel::Error);
}

// Similar tests for golang, javascript, shell
#[test]
fn parses_golang_policy_check_off() { /* ... */ }

#[test]
fn parses_javascript_policy_check_off() { /* ... */ }

#[test]
fn parses_shell_policy_check_off() { /* ... */ }

#[test]
fn mixed_policy_check_levels() {
    let config = parse(
        r#"
version = 1

[rust.policy]
check = "error"
lint_changes = "standalone"

[golang.policy]
check = "warn"
lint_changes = "standalone"

[javascript.policy]
check = "off"
lint_changes = "standalone"
"#,
        Path::new("test.toml"),
    )
    .unwrap();

    assert_eq!(config.policy_check_level_for_language("rust"), CheckLevel::Error);
    assert_eq!(config.policy_check_level_for_language("golang"), CheckLevel::Warn);
    assert_eq!(config.policy_check_level_for_language("javascript"), CheckLevel::Off);
}
```

**Verification:** `cargo test config` passes all new tests.

### Phase 5: Enable Behavioral Specs

Remove `#[ignore]` attributes from tests in `tests/specs/checks/policy_lang.rs`:

```rust
// Before:
#[test]
#[ignore = "TODO: Phase 1547 - Per-language policy config"]
fn rust_policy_check_off_disables_policy() { ... }

// After:
#[test]
fn rust_policy_check_off_disables_policy() { ... }
```

All tests to enable:
- `rust_policy_check_off_disables_policy`
- `rust_policy_check_warn_reports_without_failing`
- `golang_policy_check_off_disables_policy`
- `golang_policy_check_warn_reports_without_failing`
- `javascript_policy_check_off_disables_policy`
- `shell_policy_check_off_disables_policy`
- `each_language_can_have_independent_policy_check_level`
- `mixed_levels_go_warn_rust_error`

**Verification:** `cargo test --test specs policy_lang` passes all tests.

### Phase 6: Final Verification

Run full verification:
```bash
make check
```

This runs:
- `cargo fmt --all -- --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --all`
- `cargo build --all`
- `cargo audit`
- `cargo deny check`

**Verification:** All checks pass.

## Key Implementation Details

### CheckLevel Reuse

The existing `CheckLevel` enum from `crates/cli/src/config/checks.rs` is reused:
```rust
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CheckLevel {
    #[default]
    Error,
    Warn,
    Off,
}
```

### Warning vs Error Handling

The escapes check already handles warnings vs errors. Review how `is_warning` is propagated through the `Violation` struct and check result handling. The policy violations should follow the same pattern.

If `Violation` doesn't have an `is_warning` field, check if there's a different mechanism (e.g., separate check result types or severity field).

### PolicyConfig Trait

The `PolicyConfig` trait in `crates/cli/src/adapter/common/policy.rs` defines:
```rust
pub trait PolicyConfig {
    fn lint_changes(&self) -> LintChangesPolicy;
    fn lint_config(&self) -> &[String];
}
```

Consider whether to add `fn check(&self) -> Option<CheckLevel>` to this trait, or keep check level resolution in `Config::policy_check_level_for_language()`. The latter is simpler and matches the cloc pattern.

### Fixture Behavior

The test fixtures in `tests/fixtures/policy-lang/` are designed to trigger policy violations when lint config and source files are both "changed". In behavioral tests, this is simulated by running the check in a specific context.

The fixtures have `lint_changes = "standalone"` enabled, so when the implementation is correct:
- `check = "off"` → No violation reported, check passes
- `check = "warn"` → Violation reported but doesn't fail
- `check = "error"` (or unset) → Violation reported and fails

## Verification Plan

### Unit Tests

```bash
cargo test config::tests::parses_rust_policy_check
cargo test config::tests::parses_golang_policy_check
cargo test config::tests::parses_javascript_policy_check
cargo test config::tests::parses_shell_policy_check
cargo test config::tests::mixed_policy_check_levels
```

### Behavioral Specs

```bash
cargo test --test specs policy_lang
```

All 8 specs should pass (no longer ignored).

### Manual Verification

Test on actual fixtures:
```bash
# Should pass (check = "off")
cargo run -- check --escapes tests/fixtures/policy-lang/rust-off

# Should pass with warning output (check = "warn")
cargo run -- check --escapes tests/fixtures/policy-lang/rust-warn

# Should fail (check = "error" in mixed-levels for rust)
cargo run -- check --escapes tests/fixtures/policy-lang/mixed-levels
```

### Full Suite

```bash
make check
```

All existing tests continue to pass, new tests pass.

## Spec Summary

| Test | Status After Implementation |
|------|----------------------------|
| `rust_policy_check_off_disables_policy` | Passes |
| `rust_policy_check_warn_reports_without_failing` | Passes |
| `golang_policy_check_off_disables_policy` | Passes |
| `golang_policy_check_warn_reports_without_failing` | Passes |
| `javascript_policy_check_off_disables_policy` | Passes |
| `shell_policy_check_off_disables_policy` | Passes |
| `each_language_can_have_independent_policy_check_level` | Passes |
| `mixed_levels_go_warn_rust_error` | Passes |
