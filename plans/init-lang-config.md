# Per-Language Config Schema Implementation Plan

Add per-language `[lang.cloc]` and `[lang.policy].check` config schema (Phase 1540-1547).

## Overview

This plan covers adding per-language configuration for cloc (lines of code) checks and policy checks. Each supported language (rust, golang, javascript, shell) should be able to independently configure:

- `{lang}.cloc.check` - error | warn | off (inherits from `check.cloc.check` if unset)
- `{lang}.cloc.advice` - custom advice message for violations
- `{lang}.policy.check` - error | warn | off (defaults to error)

**Status: Implementation Complete** - All code, tests, and fixtures are already in place. This plan documents the verification steps.

## Project Structure

```
crates/cli/src/
├── config/
│   ├── mod.rs              # Config struct with cloc_check_level_for_language()
│   ├── checks.rs           # LangClocConfig struct
│   ├── go.rs               # GoConfig with cloc and policy fields
│   ├── javascript.rs       # JavaScriptConfig with cloc and policy fields
│   └── shell.rs            # ShellConfig with cloc and policy fields
├── checks/
│   ├── cloc.rs             # Uses per-language check levels
│   └── escapes/
│       └── lint_policy.rs  # Uses per-language policy check levels
tests/
├── specs/checks/
│   ├── cloc_lang.rs        # Per-language cloc specs (14 tests)
│   └── policy_lang.rs      # Per-language policy specs (8 tests)
└── fixtures/cloc-lang/     # Test fixtures for each language
```

## Dependencies

No new dependencies required. Uses existing:
- `serde` for config deserialization
- `toml` for parsing

## Implementation Phases

### Phase 1540: Config Schema - [lang.cloc] - Specs

**Status: Complete** - All specs written and passing.

Behavioral specs in `tests/specs/checks/cloc_lang.rs`:

| Spec | Test Function | Status |
|------|---------------|--------|
| `rust.cloc.check = "off"` skips Rust files | `rust_cloc_check_off_skips_rust_files` | ✅ Pass |
| `rust.cloc.check = "warn"` reports without failing | `rust_cloc_check_warn_reports_without_failing` | ✅ Pass |
| `rust.cloc.advice` overrides default | `rust_cloc_advice_overrides_default` | ✅ Pass |
| `golang.cloc.check = "off"` skips Go files | `golang_cloc_check_off_skips_go_files` | ✅ Pass |
| `golang.cloc.check = "warn"` reports without failing | `golang_cloc_check_warn_reports_without_failing` | ✅ Pass |
| `javascript.cloc.check = "off"` skips JS files | `javascript_cloc_check_off_skips_js_files` | ✅ Pass |
| `shell.cloc.check = "off"` skips shell files | `shell_cloc_check_off_skips_shell_files` | ✅ Pass |
| Independent check levels per language | `each_language_can_have_independent_cloc_check_level` | ✅ Pass |
| Mixed levels work correctly | `mixed_levels_go_warn_rust_error` | ✅ Pass |
| Unset inherits from global | `unset_lang_cloc_inherits_from_global` | ✅ Pass |
| Global off can be overridden | `global_off_disables_all_unless_overridden` | ✅ Pass |

### Phase 1542: Config Schema - [lang.cloc] - Implementation

**Status: Complete** - All implementation in place.

Key implementation details:

1. **LangClocConfig struct** (`crates/cli/src/config/checks.rs:487-503`):
```rust
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

2. **Per-language cloc field** (in each lang config):
```rust
// Example from RustConfig
pub cloc: Option<LangClocConfig>,
```

3. **Resolution method** (`crates/cli/src/config/mod.rs:165-190`):
```rust
pub fn cloc_check_level_for_language(&self, language: &str) -> CheckLevel {
    let lang_level = match language {
        "rust" | "rs" => self.rust.cloc.as_ref().and_then(|c| c.check),
        "go" => self.golang.cloc.as_ref().and_then(|c| c.check),
        // ... other languages
        _ => None,
    };
    lang_level.unwrap_or(self.check.cloc.check)
}
```

4. **Cloc check integration** (`crates/cli/src/checks/cloc.rs:231-245`):
   - Gets adapter name for each file
   - Calls `config.cloc_check_level_for_language(lang_key)`
   - Skips violation generation if level is Off
   - Uses appropriate severity for Warn vs Error

### Phase 1545: Config Schema - [lang.policy].check - Specs

**Status: Complete** - All specs written and passing.

Behavioral specs in `tests/specs/checks/policy_lang.rs`:

| Spec | Test Function | Status |
|------|---------------|--------|
| `rust.policy.check = "off"` disables policy | `rust_policy_check_off_disables_policy` | ✅ Pass |
| `rust.policy.check = "warn"` reports without failing | `rust_policy_check_warn_reports_without_failing` | ✅ Pass |
| `golang.policy.check = "off"` disables policy | `golang_policy_check_off_disables_policy` | ✅ Pass |
| `golang.policy.check = "warn"` reports without failing | `golang_policy_check_warn_reports_without_failing` | ✅ Pass |
| `javascript.policy.check = "off"` disables policy | `javascript_policy_check_off_disables_policy` | ✅ Pass |
| `shell.policy.check = "off"` disables policy | `shell_policy_check_off_disables_policy` | ✅ Pass |
| Independent check levels per language | `each_language_can_have_independent_policy_check_level` | ✅ Pass |
| Mixed levels work correctly | `mixed_levels_go_warn_rust_error` | ✅ Pass |

### Phase 1547: Config Schema - [lang.policy].check - Implementation

**Status: Complete** - All implementation in place.

Key implementation details:

1. **Check field in policy configs** (example from `RustPolicyConfig`):
```rust
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RustPolicyConfig {
    /// Check level: "error" | "warn" | "off" (default: inherits from global).
    #[serde(default)]
    pub check: Option<CheckLevel>,
    // ... other fields
}
```

2. **Resolution method** (`crates/cli/src/config/mod.rs:251-267`):
```rust
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
```

3. **Lint policy integration** (`crates/cli/src/checks/escapes/lint_policy.rs`):
   - Calls `config.policy_check_level_for_language(language)`
   - Respects Off to skip, Warn for non-failing violations

## Key Implementation Details

### Resolution Hierarchy

Both cloc and policy use a resolution hierarchy:

1. **Language-specific override**: `[rust.cloc].check` or `[rust.policy].check`
2. **Global default**: `[check.cloc].check` (for cloc) or `CheckLevel::Error` (for policy)

### Language Key Mapping

The config methods accept both adapter names and file extensions:
- `"rust"` or `"rs"` → Rust config
- `"go"` → Go config
- `"javascript"`, `"js"`, `"jsx"`, `"ts"`, `"tsx"`, etc. → JavaScript config
- `"shell"`, `"sh"`, `"bash"`, `"zsh"`, `"fish"`, `"bats"` → Shell config

### Test Fixtures

Located in `tests/fixtures/cloc-lang/`:
- `rust-off/` - Rust files with `rust.cloc.check = "off"`
- `rust-warn/` - Rust files with `rust.cloc.check = "warn"`
- `rust-advice/` - Custom advice via `rust.cloc.advice`
- `golang-off/`, `golang-warn/` - Go equivalents
- `javascript-off/` - JavaScript equivalent
- `shell-off/` - Shell equivalent
- `mixed-levels/` - Multiple languages with different levels
- `inherits/` - No lang-specific config, inherits global

## Verification Plan

### 1. Run Full Test Suite

```bash
make check
```

Expected: All tests pass, including the 22 per-language config specs.

### 2. Run Specific Spec Suites

```bash
cargo test --test specs cloc_lang
cargo test --test specs policy_lang
```

Expected: All 22 tests pass (14 cloc_lang + 8 policy_lang).

### 3. Manual Config Validation

Test that invalid config produces helpful errors:

```bash
# Create test config with typo
echo 'version = 1
[rust.cloc]
chek = "off"  # typo
' > /tmp/quench.toml

cd /tmp && quench check
# Expected: unknown field `chek`, expected one of: check, advice
```

### 4. Verify Documentation Accuracy

Check that `docs/specs/02-config.md` accurately reflects the schema:
- `[rust.cloc]` section with `check` and `advice` fields
- `[rust.policy]` section with `check` field
- Same for golang, javascript, shell

### Checkpoint: Phases 1540-1547 Complete

- [x] `{lang}.cloc.check = "off"` disables cloc for that language's files
- [x] `{lang}.cloc.check = "warn"` reports but doesn't fail
- [x] `{lang}.cloc.advice` overrides default advice for that language
- [x] Each language can have independent cloc check level
- [x] Unset `{lang}.cloc.check` inherits from `check.cloc.check`
- [x] `{lang}.policy.check = "off"` disables policy for that language
- [x] `{lang}.policy.check = "warn"` reports but doesn't fail
- [x] Each language can have independent policy check level
- [x] All behavioral specs pass
- [x] Fixtures exist for all test scenarios
