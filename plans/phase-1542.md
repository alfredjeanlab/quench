# Phase 1542: Per-Language Cloc Configuration

## Overview

Implement per-language cloc configuration allowing each language (`rust`, `golang`, `javascript`, `shell`) to have its own `check` level and `advice` message for cloc violations. This enables projects to disable or warn-only for specific languages while keeping others strict.

## Project Structure

Files to modify:
```
crates/cli/src/
├── config/
│   ├── mod.rs           # RustConfig struct, Config::cloc_advice_for_language()
│   ├── go.rs            # GoConfig struct
│   ├── javascript.rs    # JavaScriptConfig struct
│   ├── shell.rs         # ShellConfig struct
│   └── checks.rs        # Add LangClocConfig struct
└── checks/
    └── cloc.rs          # Per-language check level filtering
```

Test fixtures (already created in Phase 1540):
```
tests/fixtures/cloc-lang/
├── rust-off/           # [rust.cloc] check = "off"
├── rust-warn/          # [rust.cloc] check = "warn"
├── rust-advice/        # [rust.cloc] advice = "..."
├── golang-off/         # [golang.cloc] check = "off"
├── golang-warn/        # [golang.cloc] check = "warn"
├── javascript-off/     # [javascript.cloc] check = "off"
├── shell-off/          # [shell.cloc] check = "off"
├── mixed-levels/       # Multiple languages with different levels
└── inherits/           # Unset inherits from global
```

## Dependencies

No new external dependencies required.

## Implementation Phases

### Phase 1: Add LangClocConfig Struct

Create a minimal struct for per-language cloc settings in `crates/cli/src/config/checks.rs`:

```rust
/// Per-language cloc configuration.
///
/// Allows overriding the global cloc.check level and advice per language.
/// Unset fields inherit from [check.cloc].
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LangClocConfig {
    /// Check level: error, warn, or off.
    /// If None, inherits from check.cloc.check.
    #[serde(default)]
    pub check: Option<CheckLevel>,

    /// Custom advice for violations.
    /// If None, uses language-specific default or check.cloc.advice.
    #[serde(default)]
    pub advice: Option<String>,
}
```

**Verification**: Unit test that `LangClocConfig` deserializes correctly from TOML.

### Phase 2: Add cloc Field to Language Configs

Add `cloc: Option<LangClocConfig>` to each language config struct.

**`RustConfig`** in `crates/cli/src/config/mod.rs`:
```rust
pub struct RustConfig {
    // ... existing fields ...

    /// Per-language cloc settings.
    #[serde(default)]
    pub cloc: Option<LangClocConfig>,

    // Note: cloc_advice is deprecated in favor of cloc.advice
    #[serde(default)]
    pub cloc_advice: Option<String>,
}
```

**`GoConfig`** in `crates/cli/src/config/go.rs`:
```rust
pub struct GoConfig {
    // ... existing fields ...

    #[serde(default)]
    pub cloc: Option<LangClocConfig>,
}
```

**`JavaScriptConfig`** in `crates/cli/src/config/javascript.rs`:
```rust
pub struct JavaScriptConfig {
    // ... existing fields ...

    #[serde(default)]
    pub cloc: Option<LangClocConfig>,
}
```

**`ShellConfig`** in `crates/cli/src/config/shell.rs`:
```rust
pub struct ShellConfig {
    // ... existing fields ...

    #[serde(default)]
    pub cloc: Option<LangClocConfig>,
}
```

**Verification**: Unit tests that `[rust.cloc]`, `[golang.cloc]`, `[javascript.cloc]`, `[shell.cloc]` sections parse correctly.

### Phase 3: Implement Check Level Resolution

Add methods to `Config` for resolving per-language check levels in `crates/cli/src/config/mod.rs`:

```rust
impl Config {
    /// Get effective cloc check level for a language.
    ///
    /// Resolution order:
    /// 1. {lang}.cloc.check if set
    /// 2. check.cloc.check (global default)
    pub fn cloc_check_level_for_language(&self, language: &str) -> CheckLevel {
        let lang_level = match language {
            "rust" => self.rust.cloc.as_ref().and_then(|c| c.check),
            "go" => self.golang.cloc.as_ref().and_then(|c| c.check),
            "javascript" => self.javascript.cloc.as_ref().and_then(|c| c.check),
            "shell" => self.shell.cloc.as_ref().and_then(|c| c.check),
            _ => None,
        };
        lang_level.unwrap_or(self.check.cloc.check)
    }
}
```

Update `cloc_advice_for_language()` to check `{lang}.cloc.advice` first:

```rust
impl Config {
    pub fn cloc_advice_for_language(&self, language: &str) -> &str {
        match language {
            "rust" => {
                // Check {lang}.cloc.advice first, then cloc_advice, then default
                self.rust.cloc.as_ref()
                    .and_then(|c| c.advice.as_deref())
                    .or(self.rust.cloc_advice.as_deref())
                    .unwrap_or(RustConfig::default_cloc_advice())
            }
            "go" => {
                self.golang.cloc.as_ref()
                    .and_then(|c| c.advice.as_deref())
                    .or(self.golang.cloc_advice.as_deref())
                    .unwrap_or(GoConfig::default_cloc_advice())
            }
            // ... similar for javascript, shell
            _ => &self.check.cloc.advice,
        }
    }
}
```

**Verification**: Unit tests for check level resolution with various config combinations.

### Phase 4: Integrate Check Level in Cloc Check

Modify `crates/cli/src/checks/cloc.rs` to filter by per-language check level.

Current logic at line 47:
```rust
if cloc_config.check == CheckLevel::Off {
    return CheckResult::passed(self.name());
}
```

This global early-return stays for complete skip. Add per-file filtering:

```rust
// In the file iteration loop, after getting adapter_name (line 200):
let adapter_name = registry.adapter_for(relative_path).name();
let lang_check_level = ctx.config.cloc_check_level_for_language(adapter_name);

// Skip this file if language check level is Off
if lang_check_level == CheckLevel::Off {
    // Still count metrics but skip violation check
    continue; // Or skip only the violation creation, not metrics
}
```

For warn level, track which violations are warnings vs errors:

```rust
struct ViolationInfo {
    violation: Violation,
    check_level: CheckLevel,
}

// Later when building result:
let has_errors = violation_infos.iter().any(|v| v.check_level == CheckLevel::Error);
let result = if has_errors {
    CheckResult::failed(self.name(), violations)
} else if !violations.is_empty() {
    // All violations are warnings - pass but include them
    CheckResult::passed_with_warnings(self.name(), violations)
} else {
    CheckResult::passed(self.name())
};
```

**Note**: May need to add `passed_with_warnings` to `CheckResult` or use existing warning mechanism.

**Verification**: Behavioral specs in `tests/specs/checks/cloc_lang.rs` pass.

### Phase 5: Remove Ignored Markers and Final Testing

1. Remove `#[ignore = "TODO: Phase 1542 - Per-language cloc config"]` from all specs in `tests/specs/checks/cloc_lang.rs`
2. Run full test suite: `make check`
3. Verify all 11 behavioral specs pass

## Key Implementation Details

### Inheritance Behavior

```
Check level inheritance:
[{lang}.cloc.check] → [check.cloc.check] → default ("error")

Advice inheritance:
[{lang}.cloc.advice] → [{lang}.cloc_advice] → language default → [check.cloc.advice]
```

### Backwards Compatibility

The existing `cloc_advice` field in language configs is preserved for backwards compatibility. The new `{lang}.cloc.advice` takes precedence if both are set.

### Metrics Behavior

When a language is set to `check = "off"`, files are still counted in metrics (source_lines, test_lines, etc.) but violations are not generated. This ensures aggregate metrics remain accurate.

### CheckResult Warnings

The cloc check needs to distinguish between:
- **Pass**: No violations
- **Pass with warnings**: Violations exist but all are warn-level
- **Fail**: At least one error-level violation

The existing `CheckResult` may need a `warnings` field or similar mechanism. Check `crate::check::CheckResult` for existing support.

## Verification Plan

### Unit Tests

Add to `crates/cli/src/config/mod_tests.rs` (or similar):

1. `test_lang_cloc_config_deserializes()` - Basic deserialization
2. `test_rust_cloc_section_parses()` - `[rust.cloc]` parses
3. `test_golang_cloc_section_parses()` - `[golang.cloc]` parses
4. `test_cloc_check_level_inheritance()` - Unset inherits global
5. `test_cloc_advice_resolution_order()` - Precedence chain

### Behavioral Specs

All 11 specs in `tests/specs/checks/cloc_lang.rs`:

1. `rust_cloc_check_off_skips_rust_files`
2. `rust_cloc_check_warn_reports_without_failing`
3. `rust_cloc_advice_overrides_default`
4. `golang_cloc_check_off_skips_go_files`
5. `golang_cloc_check_warn_reports_without_failing`
6. `javascript_cloc_check_off_skips_js_files`
7. `shell_cloc_check_off_skips_shell_files`
8. `each_language_can_have_independent_cloc_check_level`
9. `mixed_levels_go_warn_rust_error`
10. `unset_lang_cloc_inherits_from_global`
11. `global_off_disables_all_unless_overridden`

### Integration

Run `make check` which executes:
- `cargo fmt --all -- --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --all`
- `cargo build --all`
- `cargo audit`
- `cargo deny check`
