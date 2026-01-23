# Phase 325: Rust Adapter - Policy

**Root Feature:** `quench-a0ea`

## Overview

Implement lint configuration policy enforcement for the Rust adapter. When `lint_changes = "standalone"` is configured, changes to lint configuration files (rustfmt.toml, clippy.toml, etc.) must be in separate PRs from source/test code changes.

Additionally, implement Rust profile defaults for `quench init --profile rust` and Landing the Plane checklist items for combined agent+rust profiles.

| Feature | Description |
|---------|-------------|
| Policy config | `[rust.policy]` section with `lint_changes` and `lint_config` settings |
| Lint config detection | Identify changes to rustfmt.toml, clippy.toml variants |
| Mixed change detection | Detect lint config + source changes in same diff |
| Standalone violation | Generate error when policy violated |
| Profile defaults | Default escapes, suppress, and policy settings for `--profile rust` |
| Landing the Plane | Rust-specific checklist items (fmt, clippy, test, build) |

Reference docs:
- `docs/specs/langs/rust.md` (Policy section, Profile Defaults)
- `docs/specs/01-cli.md` (Profile Selection, Landing the Plane)

## Project Structure

```
quench/
├── crates/cli/src/
│   ├── adapter/
│   │   ├── rust.rs            # UPDATE: add policy checking
│   │   └── rust_tests.rs      # UPDATE: add policy tests
│   ├── config.rs              # UPDATE: add [rust.policy] section
│   ├── config_tests.rs        # UPDATE: add policy config tests
│   ├── checks/
│   │   ├── escapes.rs         # UPDATE: integrate policy check
│   │   └── escapes_tests.rs   # UPDATE: add policy tests
│   └── cli.rs                 # UPDATE: profile defaults, landing items
├── tests/
│   ├── specs/
│   │   └── adapters/rust.rs   # UPDATE: remove #[ignore] from policy specs
│   └── fixtures/
│       └── rust/
│           └── lint-policy/   # NEW: mixed lint+source changes
└── plans/
    └── phase-325.md
```

## Dependencies

No new external dependencies. Uses existing:
- `crate::checks::git` for changed file detection (via `--base`)
- `crate::adapter::rust` for file classification
- Phase 320's suppress infrastructure

## Implementation Phases

### Phase 1: Policy Config Schema

Add the `[rust.policy]` configuration section.

**Update `crates/cli/src/config.rs`:**

```rust
/// Rust language-specific configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct RustConfig {
    /// Split #[cfg(test)] blocks from source LOC (default: true).
    #[serde(default = "RustConfig::default_split_cfg_test")]
    pub split_cfg_test: bool,

    /// Lint suppression settings.
    #[serde(default)]
    pub suppress: SuppressConfig,

    /// Lint configuration policy.
    #[serde(default)]
    pub policy: RustPolicyConfig,
}

/// Rust lint policy configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct RustPolicyConfig {
    /// Lint config changes policy: "standalone" requires separate PRs.
    #[serde(default)]
    pub lint_changes: LintChangesPolicy,

    /// Files that trigger the standalone requirement.
    #[serde(default = "RustPolicyConfig::default_lint_config")]
    pub lint_config: Vec<String>,
}

impl Default for RustPolicyConfig {
    fn default() -> Self {
        Self {
            lint_changes: LintChangesPolicy::default(),
            lint_config: Self::default_lint_config(),
        }
    }
}

impl RustPolicyConfig {
    fn default_lint_config() -> Vec<String> {
        vec![
            "rustfmt.toml".to_string(),
            ".rustfmt.toml".to_string(),
            "clippy.toml".to_string(),
            ".clippy.toml".to_string(),
        ]
    }
}

/// Lint changes policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LintChangesPolicy {
    /// No policy - mixed changes allowed.
    #[default]
    None,
    /// Lint config changes must be in standalone PRs.
    Standalone,
}
```

**Update `parse_rust_config()` to parse policy section.**

**Milestone:** Config parses `[rust.policy]` section without errors.

**Verification:**
```bash
cargo build
cargo test config -- policy
```

---

### Phase 2: Policy Checking Logic

Add lint policy checking to the Rust adapter.

**Add to `crates/cli/src/adapter/rust.rs`:**

```rust
/// Result of checking lint policy.
#[derive(Debug, Default)]
pub struct PolicyCheckResult {
    /// Lint config files that were changed.
    pub changed_lint_config: Vec<String>,
    /// Source/test files that were changed.
    pub changed_source: Vec<String>,
    /// Whether the standalone policy is violated.
    pub standalone_violated: bool,
}

impl RustAdapter {
    /// Check lint policy against changed files.
    ///
    /// Returns policy check result with violation details.
    pub fn check_lint_policy(
        &self,
        changed_files: &[&Path],
        policy: &RustPolicyConfig,
    ) -> PolicyCheckResult {
        if policy.lint_changes == LintChangesPolicy::None {
            return PolicyCheckResult::default();
        }

        let mut result = PolicyCheckResult::default();

        for file in changed_files {
            let filename = file.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("");

            // Check if it's a lint config file
            if policy.lint_config.iter().any(|cfg| {
                filename == cfg || file.to_string_lossy().ends_with(cfg)
            }) {
                result.changed_lint_config.push(file.display().to_string());
                continue;
            }

            // Check if it's a source or test file
            let kind = self.classify(file);
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
}
```

**Milestone:** `RustAdapter::check_lint_policy()` correctly identifies mixed changes.

**Verification:**
```bash
cargo test adapter::rust -- policy
```

---

### Phase 3: Unit Tests for Policy Checking

**Add to `crates/cli/src/adapter/rust_tests.rs`:**

```rust
mod policy_checking {
    use super::*;
    use std::path::Path;

    fn default_policy() -> RustPolicyConfig {
        RustPolicyConfig {
            lint_changes: LintChangesPolicy::Standalone,
            lint_config: vec![
                "rustfmt.toml".to_string(),
                ".rustfmt.toml".to_string(),
                "clippy.toml".to_string(),
                ".clippy.toml".to_string(),
            ],
        }
    }

    #[test]
    fn no_violation_when_only_source_changed() {
        let adapter = RustAdapter::new();
        let policy = default_policy();
        let files = [Path::new("src/lib.rs"), Path::new("src/main.rs")];
        let file_refs: Vec<&Path> = files.iter().map(|p| *p).collect();

        let result = adapter.check_lint_policy(&file_refs, &policy);

        assert!(!result.standalone_violated);
        assert!(result.changed_lint_config.is_empty());
        assert_eq!(result.changed_source.len(), 2);
    }

    #[test]
    fn no_violation_when_only_lint_config_changed() {
        let adapter = RustAdapter::new();
        let policy = default_policy();
        let files = [Path::new("rustfmt.toml"), Path::new("clippy.toml")];
        let file_refs: Vec<&Path> = files.iter().map(|p| *p).collect();

        let result = adapter.check_lint_policy(&file_refs, &policy);

        assert!(!result.standalone_violated);
        assert_eq!(result.changed_lint_config.len(), 2);
        assert!(result.changed_source.is_empty());
    }

    #[test]
    fn violation_when_both_changed() {
        let adapter = RustAdapter::new();
        let policy = default_policy();
        let files = [
            Path::new("rustfmt.toml"),
            Path::new("src/lib.rs"),
        ];
        let file_refs: Vec<&Path> = files.iter().map(|p| *p).collect();

        let result = adapter.check_lint_policy(&file_refs, &policy);

        assert!(result.standalone_violated);
        assert_eq!(result.changed_lint_config.len(), 1);
        assert_eq!(result.changed_source.len(), 1);
    }

    #[test]
    fn no_violation_when_policy_disabled() {
        let adapter = RustAdapter::new();
        let policy = RustPolicyConfig {
            lint_changes: LintChangesPolicy::None,
            ..default_policy()
        };
        let files = [
            Path::new("rustfmt.toml"),
            Path::new("src/lib.rs"),
        ];
        let file_refs: Vec<&Path> = files.iter().map(|p| *p).collect();

        let result = adapter.check_lint_policy(&file_refs, &policy);

        assert!(!result.standalone_violated);
    }

    #[test]
    fn detects_hidden_lint_config_files() {
        let adapter = RustAdapter::new();
        let policy = default_policy();
        let files = [
            Path::new(".rustfmt.toml"),
            Path::new("src/lib.rs"),
        ];
        let file_refs: Vec<&Path> = files.iter().map(|p| *p).collect();

        let result = adapter.check_lint_policy(&file_refs, &policy);

        assert!(result.standalone_violated);
        assert_eq!(result.changed_lint_config, vec![".rustfmt.toml"]);
    }
}
```

**Milestone:** All policy unit tests pass.

**Verification:**
```bash
cargo test adapter::rust::tests::policy_checking
```

---

### Phase 4: Integrate Policy Check into Escapes

Add policy violation reporting to the escapes check when `--base` is provided.

**Update `crates/cli/src/checks/escapes.rs`:**

```rust
use crate::adapter::rust::{RustAdapter, PolicyCheckResult};
use crate::config::LintChangesPolicy;

/// Check lint policy and generate violations.
fn check_lint_policy(
    ctx: &CheckContext,
    rust_config: &RustConfig,
) -> Vec<Violation> {
    let mut violations = Vec::new();

    // Policy only applies when comparing against a base
    let Some(changed_files) = &ctx.changed_files else {
        return violations;
    };

    // Check if standalone policy is enabled
    if rust_config.policy.lint_changes != LintChangesPolicy::Standalone {
        return violations;
    }

    let adapter = RustAdapter::new();
    let file_refs: Vec<&Path> = changed_files.iter().map(|p| p.as_path()).collect();
    let result = adapter.check_lint_policy(&file_refs, &rust_config.policy);

    if result.standalone_violated {
        // Create a single policy violation
        violations.push(Violation {
            path: PathBuf::from("rust.policy"),
            line: 0,
            pattern: "lint_changes = standalone".to_string(),
            message: "lint config changes must be standalone".to_string(),
            advice: format!(
                "Changed: {}\nAlso changed: {}\nSubmit lint config changes in a separate PR.",
                result.changed_lint_config.join(", "),
                truncate_list(&result.changed_source, 3),
            ),
        });
    }

    violations
}

/// Truncate a list for display, showing "and N more" if needed.
fn truncate_list(items: &[String], max: usize) -> String {
    if items.len() <= max {
        items.join(", ")
    } else {
        let shown: Vec<_> = items.iter().take(max).cloned().collect();
        format!("{} and {} more", shown.join(", "), items.len() - max)
    }
}
```

**Update `EscapesCheck::run()` to call `check_lint_policy()` for Rust projects.**

**Milestone:** Policy violations reported when lint config and source both change.

**Verification:**
```bash
cargo test checks::escapes -- policy
```

---

### Phase 5: Profile Defaults for quench init

Add Rust profile defaults to CLI initialization.

**Add to `crates/cli/src/cli.rs` or appropriate init module:**

```rust
/// Default Rust profile configuration for quench init.
pub fn rust_profile_defaults() -> String {
    r#"[rust]
split_cfg_test = true
binary_size = true
build_time = true

[rust.suppress]
check = "comment"

[rust.suppress.test]
check = "allow"

[rust.policy]
lint_changes = "standalone"
lint_config = ["rustfmt.toml", ".rustfmt.toml", "clippy.toml", ".clippy.toml"]

[[check.escapes.patterns]]
name = "unsafe"
pattern = "unsafe\\s*\\{"
action = "comment"
comment = "// SAFETY:"
advice = "Add a // SAFETY: comment explaining the invariants."

[[check.escapes.patterns]]
name = "unwrap"
pattern = "\\.unwrap\\(\\)"
action = "forbid"
advice = "Use ? operator or handle the error explicitly."

[[check.escapes.patterns]]
name = "expect"
pattern = "\\.expect\\("
action = "forbid"
advice = "Use ? operator or handle the error explicitly."

[[check.escapes.patterns]]
name = "transmute"
pattern = "mem::transmute"
action = "comment"
comment = "// SAFETY:"
advice = "Add a // SAFETY: comment explaining type compatibility."
"#
    .to_string()
}

/// Rust-specific Landing the Plane checklist items.
pub fn rust_landing_items() -> Vec<&'static str> {
    vec![
        "cargo fmt --check",
        "cargo clippy -- -D warnings",
        "cargo test",
        "cargo build",
    ]
}
```

**Milestone:** `quench init --profile rust` generates complete config.

**Verification:**
```bash
cargo build
./target/debug/quench init --profile rust
cat quench.toml  # Verify Rust-specific settings
```

---

### Phase 6: Test Fixtures and Enable Specs

Create fixtures and enable behavioral specs.

**Create `tests/fixtures/rust/lint-policy/`:**

This fixture requires a git repository with mixed changes. Create programmatically in the spec test using `temp_project()`.

**Update `tests/specs/adapters/rust.rs`:**

Remove `#[ignore]` from:
- `rust_adapter_lint_config_changes_with_source_fails_standalone_policy`
- `rust_adapter_lint_config_standalone_passes`

Update the tests to properly set up git state:

```rust
/// Spec: docs/specs/langs/rust.md#policy
///
/// > lint_changes = "standalone" - lint config changes must be standalone PRs
#[test]
fn rust_adapter_lint_config_changes_with_source_fails_standalone_policy() {
    let dir = temp_project();

    // Setup quench.toml with standalone policy
    std::fs::write(
        dir.path().join("quench.toml"),
        r#"
version = 1
[rust.policy]
lint_changes = "standalone"
lint_config = ["rustfmt.toml"]
"#,
    ).unwrap();

    // Setup Cargo.toml
    std::fs::write(
        dir.path().join("Cargo.toml"),
        "[package]\nname = \"test\"\nversion = \"0.1.0\"\n",
    ).unwrap();

    // Initialize git repo
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    std::process::Command::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    std::process::Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    // Create initial commit with source
    std::fs::create_dir_all(dir.path().join("src")).unwrap();
    std::fs::write(dir.path().join("src/lib.rs"), "pub fn f() {}").unwrap();

    std::process::Command::new("git")
        .args(["add", "-A"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    std::process::Command::new("git")
        .args(["commit", "-m", "initial"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    // Add both lint config and source changes
    std::fs::write(dir.path().join("rustfmt.toml"), "max_width = 100\n").unwrap();
    std::fs::write(dir.path().join("src/lib.rs"), "pub fn f() {}\npub fn g() {}").unwrap();

    std::process::Command::new("git")
        .args(["add", "-A"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    // Check with --base HEAD should detect mixed changes
    check("escapes")
        .pwd(dir.path())
        .args(&["--base", "HEAD"])
        .fails()
        .stdout_has("lint config changes must be standalone");
}

/// Spec: docs/specs/langs/rust.md#policy
///
/// > lint_config = ["rustfmt.toml", ...] files that trigger standalone requirement
#[test]
fn rust_adapter_lint_config_standalone_passes() {
    let dir = temp_project();

    // Setup quench.toml with standalone policy
    std::fs::write(
        dir.path().join("quench.toml"),
        r#"
version = 1
[rust.policy]
lint_changes = "standalone"
lint_config = ["rustfmt.toml"]
"#,
    ).unwrap();

    // Setup Cargo.toml
    std::fs::write(
        dir.path().join("Cargo.toml"),
        "[package]\nname = \"test\"\nversion = \"0.1.0\"\n",
    ).unwrap();

    // Initialize git repo
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    std::process::Command::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    std::process::Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    // Create initial commit
    std::fs::create_dir_all(dir.path().join("src")).unwrap();
    std::fs::write(dir.path().join("src/lib.rs"), "pub fn f() {}").unwrap();

    std::process::Command::new("git")
        .args(["add", "-A"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    std::process::Command::new("git")
        .args(["commit", "-m", "initial"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    // Add ONLY lint config change (no source changes)
    std::fs::write(dir.path().join("rustfmt.toml"), "max_width = 100\n").unwrap();

    std::process::Command::new("git")
        .args(["add", "-A"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    // Should pass - only lint config changed
    check("escapes")
        .pwd(dir.path())
        .args(&["--base", "HEAD"])
        .passes();
}
```

**Milestone:** Two policy specs pass without `#[ignore]`.

**Verification:**
```bash
cargo test --test specs rust_adapter_lint_config
```

---

## Key Implementation Details

### Policy Enforcement Flow

1. User runs `quench check --base main` (or `--ci` auto-detects base)
2. Git check determines changed files since base
3. For Rust projects with `lint_changes = "standalone"`:
   - Categorize changed files into lint config vs source/test
   - If both categories have changes, report violation
4. Violation output follows spec format:
   ```
   rust: FAIL
     lint config changes must be standalone
       Changed: rustfmt.toml
       Also changed: src/parser.rs, src/lexer.rs
     Submit lint config changes in a separate PR.
   ```

### Lint Config File Detection

Files checked against `lint_config` list:
- Exact filename match: `rustfmt.toml`
- Hidden variants: `.rustfmt.toml`
- In any directory: `crates/foo/rustfmt.toml`

Default config files:
```toml
lint_config = [
  "rustfmt.toml",
  ".rustfmt.toml",
  "clippy.toml",
  ".clippy.toml",
]
```

### Integration with --base Flag

The policy check only runs when:
1. `--base` flag is provided (explicit or via `--ci`)
2. A Rust project is detected (Cargo.toml exists)
3. `lint_changes = "standalone"` is configured

Without `--base`, there's no diff to check against.

### Profile Defaults Structure

When `quench init --profile rust` is run:

```toml
version = 1

[rust]
split_cfg_test = true
binary_size = true
build_time = true

[rust.suppress]
check = "comment"

[rust.suppress.test]
check = "allow"

[rust.policy]
lint_changes = "standalone"
lint_config = ["rustfmt.toml", ".rustfmt.toml", "clippy.toml", ".clippy.toml"]

[[check.escapes.patterns]]
name = "unsafe"
pattern = "unsafe\\s*\\{"
action = "comment"
comment = "// SAFETY:"
advice = "Add a // SAFETY: comment explaining the invariants."

# ... more patterns ...
```

### Landing the Plane Items

When combined with agent profile (`quench init --profile rust,claude`):

```markdown
## Landing the Plane

Before completing work:

- [ ] Run `quench check`
- [ ] Run `cargo fmt --check`
- [ ] Run `cargo clippy -- -D warnings`
- [ ] Run `cargo test`
- [ ] Run `cargo build`
```

## Verification Plan

### After Each Phase

```bash
# Compile check
cargo build

# Run relevant unit tests
cargo test config -- policy
cargo test adapter::rust -- policy
cargo test checks::escapes -- policy

# Check lints
cargo clippy --all-targets --all-features -- -D warnings
```

### End-to-End Verification

```bash
# Run policy specs
cargo test --test specs rust_adapter_lint_config

# Full quality gates
make check
```

### Test Matrix

| Test Case | Input | Expected |
|-----------|-------|----------|
| Only source changed | src/lib.rs modified | PASS |
| Only lint config changed | rustfmt.toml added | PASS |
| Both changed | rustfmt.toml + src/lib.rs | FAIL |
| Policy disabled | lint_changes = "none" | PASS (no check) |
| No --base flag | No git comparison | PASS (no check) |
| Hidden config file | .rustfmt.toml + src | FAIL |
| Custom lint_config | Only check specified files | Per config |

### Manual Verification

```bash
# Create test project
mkdir /tmp/policy-test && cd /tmp/policy-test
git init

cat > quench.toml << 'EOF'
version = 1
[rust.policy]
lint_changes = "standalone"
EOF

cat > Cargo.toml << 'EOF'
[package]
name = "test"
version = "0.1.0"
EOF

mkdir src
echo 'pub fn f() {}' > src/lib.rs
git add -A && git commit -m "initial"

# Add mixed changes
echo 'max_width = 100' > rustfmt.toml
echo 'pub fn g() {}' >> src/lib.rs
git add -A

# Run quench with base comparison
cargo run -- check --escapes --base HEAD

# Expected output:
# escapes: FAIL
#   lint config changes must be standalone
#     Changed: rustfmt.toml
#     Also changed: src/lib.rs
#   Submit lint config changes in a separate PR.
```

## Summary

| Phase | Task | Key Files | Status |
|-------|------|-----------|--------|
| 1 | Policy config schema | `config.rs` | [ ] Pending |
| 2 | Policy checking logic | `adapter/rust.rs` | [ ] Pending |
| 3 | Policy unit tests | `adapter/rust_tests.rs` | [ ] Pending |
| 4 | Escapes integration | `checks/escapes.rs` | [ ] Pending |
| 5 | Profile defaults | `cli.rs` | [ ] Pending |
| 6 | Enable specs | `tests/specs/adapters/rust.rs` | [ ] Pending |

## Future Phases

- **Phase 330**: Shell adapter escape patterns (`# shellcheck disable=`)
- **Phase 335**: `#[expect]` vs `#[allow]` differentiation
- **Phase 340**: Build metrics (binary size, build time)
