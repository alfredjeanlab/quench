# Phase 497: JavaScript Adapter - Lint Policy

**Root Feature:** `quench-5c10`

## Overview

Implement lint configuration hygiene policy for JavaScript/TypeScript projects. When `lint_changes = "standalone"` is configured, changes to lint config files (ESLint, Biome) must be in separate PRs from source/test changes. This prevents lint rule adjustments from being bundled with code changes, ensuring deliberate lint configuration decisions.

**Scope:**
- ESLint config detection: `.eslintrc*`, `eslint.config.*`
- Biome config detection: `biome.json`, `biome.jsonc`
- Mixed change detection (lint config + source in same branch)
- Standalone PR requirement violation generation
- Integration with existing common policy infrastructure

## Project Structure

```
crates/cli/src/
├── adapter/
│   └── javascript/
│       ├── mod.rs                   # UPDATE: add policy module, check_lint_policy method
│       ├── policy.rs                # NEW: policy checking (delegates to common)
│       └── policy_tests.rs          # NEW: unit tests
├── checks/
│   └── escapes/
│       └── lint_policy.rs           # UPDATE: add JavaScript case
└── config/
    └── javascript.rs                # UPDATE: implement PolicyConfig trait

tests/
├── specs/
│   └── adapters/
│       └── javascript.rs            # UPDATE: remove #[ignore] from policy tests
└── fixtures/
    └── javascript/
        ├── lint-config-only/        # NEW: lint config change only (passes)
        ├── lint-config-mixed/       # NEW: lint config + source (fails)
        └── source-only/             # EXISTING: source changes only (passes)
```

## Dependencies

No new external dependencies required. Uses existing:
- Common policy checking infrastructure (`adapter/common/policy.rs`)
- Existing `JavaScriptAdapter` for file classification
- Existing `JavaScriptPolicyConfig` structure

## Implementation Phases

### Phase 1: Implement PolicyConfig Trait for JavaScript

**Goal:** Enable JavaScript config to use common policy checking.

**Files:**
- `crates/cli/src/config/javascript.rs` (update)

**Changes:**

```rust
// Add trait implementation at end of file
impl crate::adapter::common::policy::PolicyConfig for JavaScriptPolicyConfig {
    fn lint_changes(&self) -> LintChangesPolicy {
        self.lint_changes
    }

    fn lint_config(&self) -> &[String] {
        &self.lint_config
    }
}
```

**Verify default lint_config includes spec files:**

Per spec, defaults should include:
- `.eslintrc`, `.eslintrc.js`, `.eslintrc.json`, `.eslintrc.yml`
- `eslint.config.js`, `eslint.config.mjs`
- `biome.json`, `biome.jsonc`

The existing `default_lint_config()` includes most but needs `.eslintrc.yml` and `biome.jsonc`. Update as needed.

**Milestone:** Trait compiles, defaults match specification.

---

### Phase 2: Create JavaScript Policy Module

**Goal:** Create policy checking module following Go/Rust/Shell patterns.

**Files:**
- `crates/cli/src/adapter/javascript/policy.rs` (new)
- `crates/cli/src/adapter/javascript/policy_tests.rs` (new)
- `crates/cli/src/adapter/javascript/mod.rs` (update)

**policy.rs:**

```rust
// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! JavaScript lint policy checking.
//!
//! Checks that lint configuration changes follow the project's policy.

use std::path::Path;

use crate::adapter::FileKind;
use crate::config::JavaScriptPolicyConfig;

// Re-export from common
pub use crate::adapter::common::policy::PolicyCheckResult;

/// Check JavaScript lint policy against changed files.
///
/// Takes a classifier closure to allow testing without a full adapter.
pub fn check_lint_policy(
    changed_files: &[&Path],
    policy: &JavaScriptPolicyConfig,
    classify: impl Fn(&Path) -> FileKind,
) -> PolicyCheckResult {
    crate::adapter::common::policy::check_lint_policy(changed_files, policy, classify)
}

#[cfg(test)]
#[path = "policy_tests.rs"]
mod tests;
```

**mod.rs updates:**

```rust
mod policy;

pub use policy::{PolicyCheckResult, check_lint_policy};

// Add method to JavaScriptAdapter impl block:
impl JavaScriptAdapter {
    /// Check lint policy against changed files.
    ///
    /// Returns policy check result with violation details.
    pub fn check_lint_policy(
        &self,
        changed_files: &[&Path],
        policy: &JavaScriptPolicyConfig,
    ) -> PolicyCheckResult {
        policy::check_lint_policy(changed_files, policy, |p| self.classify(p))
    }
}
```

**Milestone:** Module compiles and is exported from adapter.

---

### Phase 3: Policy Unit Tests

**Goal:** Verify policy checking logic with JavaScript-specific patterns.

**Files:**
- `crates/cli/src/adapter/javascript/policy_tests.rs` (new)

**Test cases:**

```rust
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::Path;

use crate::adapter::FileKind;
use crate::config::{JavaScriptPolicyConfig, LintChangesPolicy};

use super::check_lint_policy;

fn default_policy() -> JavaScriptPolicyConfig {
    JavaScriptPolicyConfig {
        lint_changes: LintChangesPolicy::Standalone,
        lint_config: vec![
            ".eslintrc".to_string(),
            ".eslintrc.js".to_string(),
            "eslint.config.js".to_string(),
            "biome.json".to_string(),
        ],
    }
}

fn js_classifier(path: &Path) -> FileKind {
    let path_str = path.to_string_lossy();
    if path_str.contains(".test.") || path_str.contains(".spec.") || path_str.contains("__tests__") {
        FileKind::Test
    } else if path_str.ends_with(".ts") || path_str.ends_with(".js") ||
              path_str.ends_with(".tsx") || path_str.ends_with(".jsx") {
        FileKind::Source
    } else {
        FileKind::Other
    }
}

#[test]
fn no_policy_allows_mixed_changes() {
    let policy = JavaScriptPolicyConfig {
        lint_changes: LintChangesPolicy::None,
        ..default_policy()
    };
    let files = [Path::new(".eslintrc"), Path::new("src/app.ts")];
    let file_refs: Vec<&Path> = files.to_vec();

    let result = check_lint_policy(&file_refs, &policy, js_classifier);
    assert!(!result.standalone_violated);
}

#[test]
fn standalone_policy_allows_lint_only() {
    let policy = default_policy();
    let files = [Path::new(".eslintrc"), Path::new("eslint.config.js")];
    let file_refs: Vec<&Path> = files.to_vec();

    let result = check_lint_policy(&file_refs, &policy, js_classifier);
    assert!(!result.standalone_violated);
    assert_eq!(result.changed_lint_config.len(), 2);
}

#[test]
fn standalone_policy_allows_source_only() {
    let policy = default_policy();
    let files = [Path::new("src/app.ts"), Path::new("src/utils.test.ts")];
    let file_refs: Vec<&Path> = files.to_vec();

    let result = check_lint_policy(&file_refs, &policy, js_classifier);
    assert!(!result.standalone_violated);
    assert_eq!(result.changed_source.len(), 2);
}

#[test]
fn standalone_policy_fails_mixed_changes() {
    let policy = default_policy();
    let files = [Path::new(".eslintrc"), Path::new("src/app.ts")];
    let file_refs: Vec<&Path> = files.to_vec();

    let result = check_lint_policy(&file_refs, &policy, js_classifier);
    assert!(result.standalone_violated);
}

#[test]
fn recognizes_eslint_config_variants() {
    let policy = JavaScriptPolicyConfig {
        lint_config: vec![
            ".eslintrc".to_string(),
            ".eslintrc.js".to_string(),
            ".eslintrc.json".to_string(),
            ".eslintrc.yml".to_string(),
            "eslint.config.js".to_string(),
            "eslint.config.mjs".to_string(),
        ],
        ..default_policy()
    };

    // Test each variant triggers policy
    for config in &[".eslintrc.json", ".eslintrc.yml", "eslint.config.mjs"] {
        let files = [Path::new(*config), Path::new("src/app.ts")];
        let file_refs: Vec<&Path> = files.to_vec();
        let result = check_lint_policy(&file_refs, &policy, js_classifier);
        assert!(result.standalone_violated, "Expected violation for {}", config);
    }
}

#[test]
fn recognizes_biome_config_variants() {
    let policy = JavaScriptPolicyConfig {
        lint_config: vec!["biome.json".to_string(), "biome.jsonc".to_string()],
        ..default_policy()
    };

    for config in &["biome.json", "biome.jsonc"] {
        let files = [Path::new(*config), Path::new("src/app.ts")];
        let file_refs: Vec<&Path> = files.to_vec();
        let result = check_lint_policy(&file_refs, &policy, js_classifier);
        assert!(result.standalone_violated, "Expected violation for {}", config);
    }
}
```

**Milestone:** All unit tests pass.

---

### Phase 4: Integration with Escapes Check

**Goal:** Wire JavaScript policy checking into the escapes check lint_policy module.

**Files:**
- `crates/cli/src/checks/escapes/lint_policy.rs` (update)

**Changes:**

Update imports:
```rust
use crate::adapter::{GoAdapter, JavaScriptAdapter, ProjectLanguage, RustAdapter, ShellAdapter, detect_language};
use crate::config::{GoConfig, JavaScriptConfig, LintChangesPolicy, RustConfig, ShellConfig};
```

Replace TODO with implementation:
```rust
pub fn check_lint_policy(ctx: &CheckContext) -> Vec<Violation> {
    match detect_language(ctx.root) {
        ProjectLanguage::Rust => check_rust_lint_policy(ctx, &ctx.config.rust),
        ProjectLanguage::Go => check_go_lint_policy(ctx, &ctx.config.golang),
        ProjectLanguage::Shell => check_shell_lint_policy(ctx, &ctx.config.shell),
        ProjectLanguage::JavaScript => check_javascript_lint_policy(ctx, &ctx.config.javascript),
        ProjectLanguage::Generic => Vec::new(),
    }
}
```

Add JavaScript checking function:
```rust
/// Check JavaScript lint policy and generate violations.
fn check_javascript_lint_policy(ctx: &CheckContext, js_config: &JavaScriptConfig) -> Vec<Violation> {
    if js_config.policy.lint_changes != LintChangesPolicy::Standalone {
        return Vec::new();
    }
    let Some(changed_files) = ctx.changed_files else {
        return Vec::new();
    };

    let adapter = JavaScriptAdapter::new();
    let file_refs: Vec<&Path> = changed_files.iter().map(|p| p.as_path()).collect();
    let result = adapter.check_lint_policy(&file_refs, &js_config.policy);
    make_policy_violation(
        result.standalone_violated,
        &result.changed_lint_config,
        &result.changed_source,
    )
}
```

**Milestone:** JavaScript policy violations are generated when appropriate.

---

### Phase 5: Test Fixtures and Behavioral Tests

**Goal:** Add fixtures and enable spec tests.

**Files:**
- `tests/fixtures/javascript/lint-config-only/` (new)
- `tests/fixtures/javascript/lint-config-mixed/` (new)
- `tests/specs/adapters/javascript.rs` (update)

**Fixture: lint-config-only/**

```
lint-config-only/
├── quench.toml
├── package.json
└── .eslintrc.json
```

`quench.toml`:
```toml
version = 1

[javascript.policy]
lint_changes = "standalone"
```

This fixture simulates a PR with only lint config changes (should pass).

**Fixture: lint-config-mixed/**

```
lint-config-mixed/
├── quench.toml
├── package.json
├── .eslintrc.json
└── src/
    └── app.ts
```

Same `quench.toml` but with source files present (should fail when both modified).

**Spec tests to add:**

```rust
#[test]
fn javascript_lint_policy_standalone_allows_config_only() {
    cli()
        .on("javascript/lint-config-only")
        .with_changed(&[".eslintrc.json"])
        .passes()
        .stdout_has_none("lint_policy");
}

#[test]
fn javascript_lint_policy_standalone_allows_source_only() {
    cli()
        .on("javascript/lint-config-mixed")
        .with_changed(&["src/app.ts"])
        .passes()
        .stdout_has_none("lint_policy");
}

#[test]
fn javascript_lint_policy_standalone_fails_mixed() {
    cli()
        .on("javascript/lint-config-mixed")
        .with_changed(&[".eslintrc.json", "src/app.ts"])
        .fails()
        .stdout_has("lint_policy")
        .stdout_has("lint config changes must be standalone")
        .stdout_has(".eslintrc.json")
        .stdout_has("src/app.ts");
}
```

**Milestone:** Spec tests pass, fixtures work correctly.

---

### Phase 6: Final Verification

**Goal:** Ensure all tests pass and implementation matches specification.

**Verification steps:**

1. Run unit tests:
   ```bash
   cargo test --package quench -- javascript::policy
   ```

2. Run spec tests:
   ```bash
   cargo test --package quench --test specs -- javascript
   ```

3. Run full check suite:
   ```bash
   make check
   ```

4. Verify output format matches spec example:
   ```
   javascript: FAIL
     lint config changes must be standalone
       Changed: eslint.config.js
       Also changed: src/parser.ts, src/lexer.ts
     Submit lint config changes in a separate PR.
   ```

**Milestone:** `make check` succeeds, all tests pass.

## Key Implementation Details

### Lint Config File Patterns

Per specification, these files trigger the standalone requirement:

| Tool | Config Files |
|------|--------------|
| ESLint | `.eslintrc`, `.eslintrc.js`, `.eslintrc.json`, `.eslintrc.yml`, `eslint.config.js`, `eslint.config.mjs` |
| Biome | `biome.json`, `biome.jsonc` |

The defaults in `JavaScriptPolicyConfig::default_lint_config()` should match these exactly.

### File Classification

Uses existing `JavaScriptAdapter::classify()` which:
1. Checks ignore patterns (node_modules/, dist/, etc.)
2. Checks test patterns (*.test.ts, *.spec.js, __tests__/, etc.)
3. Checks source patterns (*.js, *.ts, *.jsx, *.tsx, etc.)

### Common Policy Reuse

The implementation delegates to `crate::adapter::common::policy::check_lint_policy()` which:
1. Returns early if policy is `None`
2. Classifies each changed file as lint config, source, test, or other
3. Reports violation if both lint config AND source/test files changed

### Violation Output Format

The `make_policy_violation()` function in `lint_policy.rs` generates:
- `violation_type`: "lint_policy"
- `pattern`: "lint_changes = standalone"
- `advice`: Lists changed lint config files and source files with guidance

## Verification Plan

### Unit Tests

1. **No policy allows mixed**: `LintChangesPolicy::None` permits any combination
2. **Standalone allows lint only**: Config changes alone pass
3. **Standalone allows source only**: Source changes alone pass
4. **Standalone fails mixed**: Config + source fails
5. **ESLint variants recognized**: All `.eslintrc*` and `eslint.config.*` patterns
6. **Biome variants recognized**: `biome.json` and `biome.jsonc`

### Behavioral Tests

1. Lint config only change passes
2. Source only change passes
3. Mixed change fails with correct output format
4. Correct files listed in violation message

### Integration Verification

```bash
make check
```

Ensures:
- Code compiles without warnings
- All tests pass
- Clippy lints pass
- Bootstrap checks pass
