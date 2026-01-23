# Phase 420: Shell Adapter - Policy

**Root Feature:** `quench-64f6`

## Overview

Add lint policy enforcement for Shell projects, matching the pattern established for Rust. This includes:
- `lint_changes = "standalone"` policy checking for `.shellcheckrc`
- Shell profile defaults for `quench init --profile shell`
- Shell-specific Landing the Plane checklist items (shellcheck, bats)

## Project Structure

```
crates/cli/src/
├── adapter/
│   └── shell/
│       ├── mod.rs           # Add check_lint_policy method
│       └── policy.rs        # NEW: Shell-specific policy checking
│       └── policy_tests.rs  # NEW: Unit tests
├── config/
│   └── shell.rs             # Already has ShellPolicyConfig (no changes needed)
├── checks/
│   └── escapes/
│       └── mod.rs           # Wire up shell policy checking
└── cli.rs                   # Add shell_profile_defaults(), shell_landing_items()
```

## Dependencies

No new external dependencies required. Uses existing:
- `std::path::Path` for file classification
- Existing `LintChangesPolicy`, `ShellPolicyConfig` types

## Implementation Phases

### Phase 1: Shell Policy Module

**Goal:** Create policy checking infrastructure for Shell adapter.

**Files:**
- `crates/cli/src/adapter/shell/policy.rs` (new)
- `crates/cli/src/adapter/shell/policy_tests.rs` (new)
- `crates/cli/src/adapter/shell/mod.rs` (update)

**Implementation:**

Create `crates/cli/src/adapter/shell/policy.rs`:

```rust
//! Shell lint policy checking.

use std::path::Path;

use crate::adapter::FileKind;
use crate::config::{LintChangesPolicy, ShellPolicyConfig};

/// Result of checking shell lint policy.
#[derive(Debug, Default)]
pub struct ShellPolicyCheckResult {
    /// Lint config files that were changed.
    pub changed_lint_config: Vec<String>,
    /// Source/test files that were changed.
    pub changed_source: Vec<String>,
    /// Whether the standalone policy is violated.
    pub standalone_violated: bool,
}

/// Check shell lint policy against changed files.
pub fn check_lint_policy(
    changed_files: &[&Path],
    policy: &ShellPolicyConfig,
    classify: impl Fn(&Path) -> FileKind,
) -> ShellPolicyCheckResult {
    if policy.lint_changes == LintChangesPolicy::None {
        return ShellPolicyCheckResult::default();
    }

    let mut result = ShellPolicyCheckResult::default();

    for file in changed_files {
        let filename = file.file_name().and_then(|n| n.to_str()).unwrap_or("");

        // Check if it's a lint config file (e.g., .shellcheckrc)
        if policy
            .lint_config
            .iter()
            .any(|cfg| filename == cfg || file.to_string_lossy().ends_with(cfg))
        {
            result.changed_lint_config.push(file.display().to_string());
            continue;
        }

        // Check if it's a shell source or test file
        let kind = classify(file);
        if kind == FileKind::Source || kind == FileKind::Test {
            result.changed_source.push(file.display().to_string());
        }
    }

    // Standalone policy violated if both lint config AND source changed
    result.standalone_violated = policy.lint_changes == LintChangesPolicy::Standalone
        && !result.changed_lint_config.is_empty()
        && !result.changed_source.is_empty();

    result
}
```

Update `crates/cli/src/adapter/shell/mod.rs` to expose policy module:

```rust
mod policy;
pub use policy::{check_lint_policy, ShellPolicyCheckResult};

impl ShellAdapter {
    /// Check lint policy for changed files.
    pub fn check_lint_policy(
        &self,
        changed_files: &[&Path],
        policy: &ShellPolicyConfig,
    ) -> ShellPolicyCheckResult {
        policy::check_lint_policy(changed_files, policy, |p| self.classify(p))
    }
}
```

**Milestone:** `ShellAdapter::check_lint_policy()` correctly identifies mixed `.shellcheckrc` + source changes.

---

### Phase 2: Wire Shell Policy in Escapes Check

**Goal:** Integrate shell lint policy checking into the escapes check.

**Files:**
- `crates/cli/src/checks/escapes/mod.rs` (update)

**Implementation:**

Add shell policy checking alongside the existing Rust policy checking:

```rust
// In EscapesCheck::run(), update the policy checking section:

// Check lint policy for Rust projects (only when --base is provided)
let mut policy_violations = Vec::new();
match detect_language(ctx.root) {
    ProjectLanguage::Rust => {
        policy_violations = check_rust_lint_policy(ctx, &ctx.config.rust);
    }
    ProjectLanguage::Shell => {
        policy_violations = check_shell_lint_policy(ctx, &ctx.config.shell);
    }
    ProjectLanguage::Generic => {}
}
```

Add new function for shell policy checking:

```rust
/// Check shell lint policy and generate violations.
fn check_shell_lint_policy(ctx: &CheckContext, shell_config: &ShellConfig) -> Vec<Violation> {
    let mut violations = Vec::new();

    // Policy only applies when comparing against a base
    let Some(changed_files) = ctx.changed_files else {
        return violations;
    };

    // Check if standalone policy is enabled
    if shell_config.policy.lint_changes != LintChangesPolicy::Standalone {
        return violations;
    }

    let adapter = ShellAdapter::new();
    let file_refs: Vec<&Path> = changed_files.iter().map(|p| p.as_path()).collect();
    let result = adapter.check_lint_policy(&file_refs, &shell_config.policy);

    if result.standalone_violated {
        violations.push(Violation {
            file: None,
            line: None,
            violation_type: "lint_policy".to_string(),
            advice: format!(
                "Changed lint config: {}\nAlso changed source: {}\nSubmit lint config changes in a separate PR.",
                result.changed_lint_config.join(", "),
                truncate_list(&result.changed_source, 3),
            ),
            value: None,
            threshold: None,
            pattern: Some("lint_changes = standalone".to_string()),
            lines: None,
            nonblank: None,
        });
    }

    violations
}
```

Rename existing `check_lint_policy` to `check_rust_lint_policy` for clarity.

**Milestone:** `quench check --base main` on shell projects reports policy violations.

---

### Phase 3: Shell Profile Defaults

**Goal:** Provide opinionated defaults for `quench init --profile shell`.

**Files:**
- `crates/cli/src/cli.rs` (update)

**Implementation:**

Add `shell_profile_defaults()` function:

```rust
/// Default Shell profile configuration for quench init.
pub fn shell_profile_defaults() -> String {
    r#"[shell]
source = ["**/*.sh", "**/*.bash"]
tests = ["tests/**/*.bats", "test/**/*.bats", "*_test.sh", "**/*_test.sh"]

[shell.suppress]
check = "comment"
comment = "# OK:"

[shell.suppress.test]
check = "allow"

[shell.policy]
lint_changes = "standalone"
lint_config = [".shellcheckrc"]

[[check.escapes.patterns]]
name = "set_plus_e"
pattern = "set \\+e"
action = "comment"
comment = "# OK:"
advice = "Add a # OK: comment explaining why error checking is disabled."

[[check.escapes.patterns]]
name = "eval"
pattern = "\\beval\\s"
action = "comment"
comment = "# OK:"
advice = "Add a # OK: comment explaining why eval is safe here."

[[check.escapes.patterns]]
name = "rm_rf"
pattern = "rm\\s+-rf"
action = "comment"
comment = "# OK:"
advice = "Add a # OK: comment explaining the rm -rf is safe."
"#
    .to_string()
}
```

Update `InitArgs` handling in `main.rs` to recognize "shell" profile.

**Milestone:** `quench init --profile shell` generates valid config with shell-specific defaults.

---

### Phase 4: Shell Landing the Plane Checklist

**Goal:** Provide shell-specific pre-commit checklist items.

**Files:**
- `crates/cli/src/cli.rs` (update)

**Implementation:**

Add `shell_landing_items()` function:

```rust
/// Shell-specific Landing the Plane checklist items.
pub fn shell_landing_items() -> &'static [&'static str] {
    &[
        "shellcheck **/*.sh",
        "bats tests/",
    ]
}
```

**Note:** These commands are advisory - actual invocation depends on project structure.

**Milestone:** Shell landing items are documented and accessible.

---

### Phase 5: Integration Tests

**Goal:** End-to-end verification of shell policy enforcement.

**Files:**
- `tests/specs/shell_policy_spec.rs` (new)
- `tests/fixtures/shell-policy/` (new fixture)

**Test fixture structure:**

```
tests/fixtures/shell-policy/
├── .shellcheckrc
├── quench.toml
├── build.sh
└── tests/
    └── test.bats
```

**Test cases:**

1. **Standalone violation detected:**
   - Change both `.shellcheckrc` and `build.sh`
   - Expect violation with `lint_changes = "standalone"`

2. **Config-only change allowed:**
   - Change only `.shellcheckrc`
   - No violation (standalone only triggers with mixed changes)

3. **Source-only change allowed:**
   - Change only `build.sh`
   - No violation

4. **Policy disabled:**
   - Set `lint_changes = "none"`
   - Mixed changes should not trigger violation

**Milestone:** All integration tests pass; `make check` succeeds.

---

### Phase 6: Documentation and Cleanup

**Goal:** Update documentation and ensure consistency.

**Files:**
- Update docstrings in modified files
- Ensure consistent naming (e.g., `check_rust_lint_policy` vs `check_lint_policy`)

**Tasks:**
1. Add doc comments to new public APIs
2. Update any existing shell-related documentation
3. Run `make check` to verify no regressions

**Milestone:** `make check` passes; documentation is consistent.

---

## Key Implementation Details

### Policy Config Detection

The shell lint policy uses `.shellcheckrc` by default (configured in `ShellPolicyConfig::default_lint_config()`). The detection logic matches files by:
1. Exact filename match: `filename == ".shellcheckrc"`
2. Path suffix match: `path.ends_with(".shellcheckrc")`

This allows for both root-level and nested config files.

### Pattern Reuse

The policy checking logic is nearly identical between Rust and Shell. Consider extracting a generic `check_lint_policy_generic<P>()` function in a future refactoring phase if more languages are added.

### Escape Pattern Defaults

Shell escape patterns differ from Rust:
- `set +e` - disables error checking
- `eval` - dynamic code execution
- `rm -rf` - potentially dangerous file deletion

All use `# OK:` comment convention (vs Rust's `// SAFETY:`).

### Test File Classification

Shell tests are identified by:
- `tests/**/*.bats` - bats test files
- `*_test.sh` - shell test scripts

This is already implemented in `ShellAdapter::new()` and `ShellConfig::default_tests()`.

## Verification Plan

### Unit Tests

```bash
# Run shell policy unit tests
cargo test -p quench -- shell::policy

# Run all adapter tests
cargo test -p quench -- adapter
```

### Integration Tests

```bash
# Run shell policy specs
cargo test -p quench -- shell_policy_spec

# Run all specs
cargo test -p quench -- specs
```

### Manual Verification

```bash
# 1. Initialize shell project
mkdir /tmp/shell-test && cd /tmp/shell-test
quench init --profile shell
cat quench.toml  # Verify shell config generated

# 2. Test policy detection
echo '#!/bin/bash' > build.sh
echo 'enable=all' > .shellcheckrc
git init && git add -A && git commit -m "init"

# Modify both files
echo 'echo "test"' >> build.sh
echo '# comment' >> .shellcheckrc

# Should report violation
quench check --base HEAD~1

# 3. Run full check suite
make check
```

### CI Validation

The existing CI pipeline (`make check`) will validate:
- `cargo fmt --check` - formatting
- `cargo clippy -- -D warnings` - lints
- `cargo test --all` - all tests pass
- `cargo build --all` - builds successfully
- `./scripts/bootstrap` - conventions enforced
