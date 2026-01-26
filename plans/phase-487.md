# Phase 487: Ruby Adapter - Policy

Add lint policy enforcement for Ruby projects, requiring lint configuration changes
(`.rubocop.yml`, `.rubocop_todo.yml`, `.standard.yml`) to be submitted in standalone PRs
when `lint_changes = "standalone"` is configured.

## Overview

This phase adds:
- `RubyPolicyConfig` struct with `lint_changes` and `lint_config` fields
- `RubyConfig` struct (if not already present from Phase 483)
- Policy check implementation using the common `PolicyConfig` trait
- Integration with the escapes check `lint_policy` module
- Unit tests using the `policy_test_cases!` macro

## Project Structure

```
crates/cli/src/
├── adapter/
│   └── ruby/
│       ├── mod.rs           # Ruby adapter (may exist from 483)
│       ├── policy.rs        # NEW: Policy check wrapper
│       └── policy_tests.rs  # NEW: Policy unit tests
└── config/
    ├── mod.rs               # UPDATE: Add RubyConfig, RubyPolicyConfig exports
    └── ruby.rs              # NEW: Ruby configuration structs

tests/fixtures/
└── ruby/
    └── lint-policy/         # NEW: Test fixture
        ├── quench.toml
        ├── Gemfile
        ├── .rubocop.yml
        └── lib/
            └── example.rb
```

## Dependencies

**Internal (from prior phases):**
- `crate::adapter::common::policy::{PolicyConfig, check_lint_policy}` - Phase 325
- `crate::config::LintChangesPolicy` - existing enum

**External crates:**
- None new (uses existing serde, globset)

## Implementation Phases

### Phase 1: Configuration Structs

Add Ruby configuration to `crates/cli/src/config/`.

**1.1 Create `crates/cli/src/config/ruby.rs`:**

```rust
//! Ruby language-specific configuration.

use serde::Deserialize;
use super::{CheckLevel, LangClocConfig, LintChangesPolicy, SuppressLevel, SuppressScopeConfig};

/// Ruby language-specific configuration.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RubyConfig {
    /// Source file patterns.
    #[serde(default = "RubyConfig::default_source")]
    pub source: Vec<String>,

    /// Test file patterns.
    #[serde(default = "RubyConfig::default_tests")]
    pub tests: Vec<String>,

    /// Ignore patterns.
    #[serde(default = "RubyConfig::default_ignore")]
    pub ignore: Vec<String>,

    /// Lint suppression settings.
    #[serde(default)]
    pub suppress: RubySuppressConfig,

    /// Lint configuration policy.
    #[serde(default)]
    pub policy: RubyPolicyConfig,

    /// Per-language cloc settings.
    #[serde(default)]
    pub cloc: Option<LangClocConfig>,
}

impl RubyConfig {
    pub(crate) fn default_source() -> Vec<String> {
        vec![
            "**/*.rb".to_string(),
            "**/*.rake".to_string(),
            "Rakefile".to_string(),
            "Gemfile".to_string(),
            "*.gemspec".to_string(),
        ]
    }

    pub(crate) fn default_tests() -> Vec<String> {
        vec![
            "spec/**/*_spec.rb".to_string(),
            "test/**/*_test.rb".to_string(),
            "test/**/test_*.rb".to_string(),
            "features/**/*.rb".to_string(),
        ]
    }

    pub(crate) fn default_ignore() -> Vec<String> {
        vec![
            "vendor/".to_string(),
            "tmp/".to_string(),
            "log/".to_string(),
            "coverage/".to_string(),
        ]
    }
}

/// Ruby lint policy configuration.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RubyPolicyConfig {
    /// Check level: "error" | "warn" | "off".
    #[serde(default)]
    pub check: Option<CheckLevel>,

    /// Lint config changes policy: "standalone" requires separate PRs.
    #[serde(default)]
    pub lint_changes: LintChangesPolicy,

    /// Files that trigger the standalone requirement.
    #[serde(default = "RubyPolicyConfig::default_lint_config")]
    pub lint_config: Vec<String>,
}

impl Default for RubyPolicyConfig {
    fn default() -> Self {
        Self {
            check: None,
            lint_changes: LintChangesPolicy::default(),
            lint_config: Self::default_lint_config(),
        }
    }
}

impl RubyPolicyConfig {
    pub(crate) fn default_lint_config() -> Vec<String> {
        vec![
            ".rubocop.yml".to_string(),
            ".rubocop_todo.yml".to_string(),
            ".standard.yml".to_string(),
        ]
    }
}

impl crate::adapter::common::policy::PolicyConfig for RubyPolicyConfig {
    fn lint_changes(&self) -> LintChangesPolicy {
        self.lint_changes
    }

    fn lint_config(&self) -> &[String] {
        &self.lint_config
    }
}
```

**1.2 Update `crates/cli/src/config/mod.rs`:**
- Add `mod ruby;`
- Add `pub use ruby::{RubyConfig, RubyPolicyConfig, RubySuppressConfig};`
- Add `ruby: RubyConfig` field to `Config` struct
- Add Ruby cases to `cloc_check_level_for_language()` and `policy_check_level_for_language()`

**Verification:** `cargo check` passes

---

### Phase 2: Ruby Adapter Policy Module

Add policy checking to the Ruby adapter.

**2.1 Create `crates/cli/src/adapter/ruby/` directory structure:**

If the Ruby adapter doesn't exist from Phase 483, create minimal `mod.rs`:

```rust
//! Ruby language adapter.
//!
//! See docs/specs/langs/ruby.md for specification.

use std::path::Path;
use globset::GlobSet;

mod policy;

pub use policy::{PolicyCheckResult, check_lint_policy};

use super::glob::build_glob_set;
use super::{Adapter, EscapePattern, FileKind};
use crate::config::RubyPolicyConfig;

/// Ruby language adapter.
pub struct RubyAdapter {
    source_patterns: GlobSet,
    test_patterns: GlobSet,
    ignore_patterns: GlobSet,
}

impl RubyAdapter {
    pub fn new() -> Self {
        Self {
            source_patterns: build_glob_set(&crate::config::RubyConfig::default_source()),
            test_patterns: build_glob_set(&crate::config::RubyConfig::default_tests()),
            ignore_patterns: build_glob_set(&crate::config::RubyConfig::default_ignore()),
        }
    }

    pub fn with_patterns(patterns: super::ResolvedPatterns) -> Self {
        Self {
            source_patterns: build_glob_set(&patterns.source),
            test_patterns: build_glob_set(&patterns.test),
            ignore_patterns: build_glob_set(&patterns.ignore),
        }
    }

    pub fn should_ignore(&self, path: &Path) -> bool {
        self.ignore_patterns.is_match(path)
    }

    /// Check lint policy against changed files.
    pub fn check_lint_policy(
        &self,
        changed_files: &[&Path],
        policy: &RubyPolicyConfig,
    ) -> PolicyCheckResult {
        policy::check_lint_policy(changed_files, policy, |p| self.classify(p))
    }
}

impl Adapter for RubyAdapter {
    fn name(&self) -> &'static str {
        "ruby"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["rb", "rake"]
    }

    fn classify(&self, path: &Path) -> FileKind {
        if self.should_ignore(path) {
            return FileKind::Other;
        }
        if self.test_patterns.is_match(path) {
            return FileKind::Test;
        }
        if self.source_patterns.is_match(path) {
            return FileKind::Source;
        }
        FileKind::Other
    }
}
```

**2.2 Create `crates/cli/src/adapter/ruby/policy.rs`:**

```rust
//! Ruby lint policy checking.

use std::path::Path;

use crate::adapter::FileKind;
use crate::config::RubyPolicyConfig;

pub use crate::adapter::common::policy::PolicyCheckResult;

/// Check Ruby lint policy against changed files.
pub fn check_lint_policy(
    changed_files: &[&Path],
    policy: &RubyPolicyConfig,
    classify: impl Fn(&Path) -> FileKind,
) -> PolicyCheckResult {
    crate::adapter::common::policy::check_lint_policy(changed_files, policy, classify)
}

#[cfg(test)]
#[path = "policy_tests.rs"]
mod tests;
```

**2.3 Create `crates/cli/src/adapter/ruby/policy_tests.rs`:**

```rust
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::Path;

use crate::adapter::FileKind;
use crate::config::{RubyPolicyConfig, LintChangesPolicy};

fn default_policy() -> RubyPolicyConfig {
    RubyPolicyConfig {
        check: None,
        lint_changes: LintChangesPolicy::Standalone,
        lint_config: vec![
            ".rubocop.yml".to_string(),
            ".rubocop_todo.yml".to_string(),
            ".standard.yml".to_string(),
        ],
    }
}

fn ruby_classifier(path: &Path) -> FileKind {
    let path_str = path.to_string_lossy();
    if path_str.contains("spec/") && path_str.ends_with("_spec.rb") {
        FileKind::Test
    } else if path_str.contains("test/") && (path_str.ends_with("_test.rb") || path_str.contains("test_")) {
        FileKind::Test
    } else if path_str.ends_with(".rb") || path_str.ends_with(".rake") {
        FileKind::Source
    } else {
        FileKind::Other
    }
}

// Generate standard policy tests
crate::policy_test_cases! {
    policy_type: RubyPolicyConfig,
    default_policy: default_policy,
    classifier: ruby_classifier,
    source_files: ["lib/parser.rb", "lib/lexer.rb"],
    lint_config_file: ".rubocop.yml",
    test_file: "spec/parser_spec.rb",
}

// Ruby-specific tests

#[test]
fn recognizes_rubocop_todo() {
    use crate::adapter::common::test_utils::assert_violation;
    let policy = default_policy();
    assert_violation(&[".rubocop_todo.yml", "lib/parser.rb"], &policy, ruby_classifier);
}

#[test]
fn recognizes_standard_yml() {
    use crate::adapter::common::test_utils::assert_violation;
    let policy = default_policy();
    assert_violation(&[".standard.yml", "lib/parser.rb"], &policy, ruby_classifier);
}

#[test]
fn multiple_lint_configs() {
    use crate::adapter::common::test_utils::assert_violation;
    let policy = default_policy();
    // Should still violate even with multiple config files changed
    assert_violation(
        &[".rubocop.yml", ".rubocop_todo.yml", "lib/parser.rb"],
        &policy,
        ruby_classifier,
    );
}
```

**Verification:** `cargo test -p quench-cli adapter::ruby::policy` passes

---

### Phase 3: Integration with Escapes Check

Update `lint_policy.rs` to include Ruby.

**3.1 Update `crates/cli/src/checks/escapes/lint_policy.rs`:**

Add Ruby case to `check_lint_policy()`:

```rust
use crate::adapter::{
    GoAdapter, JavaScriptAdapter, ProjectLanguage, RubyAdapter, RustAdapter, ShellAdapter,
    detect_language,
};
use crate::config::{
    CheckLevel, GoConfig, JavaScriptConfig, LintChangesPolicy, RubyConfig, RustConfig, ShellConfig,
};

pub fn check_lint_policy(ctx: &CheckContext) -> PolicyCheckResult {
    match detect_language(ctx.root) {
        ProjectLanguage::Ruby => check_ruby_lint_policy(ctx, &ctx.config.ruby),
        // ... existing cases
    }
}

fn check_ruby_lint_policy(ctx: &CheckContext, ruby_config: &RubyConfig) -> PolicyCheckResult {
    let check_level = ctx.config.policy_check_level_for_language("ruby");

    if check_level == CheckLevel::Off {
        return PolicyCheckResult {
            violations: Vec::new(),
            check_level,
        };
    }

    if ruby_config.policy.lint_changes != LintChangesPolicy::Standalone {
        return PolicyCheckResult {
            violations: Vec::new(),
            check_level,
        };
    }

    let Some(changed_files) = ctx.changed_files else {
        return PolicyCheckResult {
            violations: Vec::new(),
            check_level,
        };
    };

    let adapter = RubyAdapter::new();
    let file_refs: Vec<&Path> = changed_files.iter().map(|p| p.as_path()).collect();
    let result = adapter.check_lint_policy(&file_refs, &ruby_config.policy);
    PolicyCheckResult {
        violations: make_policy_violation(
            result.standalone_violated,
            &result.changed_lint_config,
            &result.changed_source,
        ),
        check_level,
    }
}
```

**3.2 Update `crates/cli/src/adapter/mod.rs`:**

- Add `pub mod ruby;`
- Add `Ruby` variant to `ProjectLanguage` enum
- Update `detect_language()` to detect Ruby projects
- Add `RubyAdapter` to `for_project()` and `for_project_with_config()`

```rust
pub fn detect_language(root: &Path) -> ProjectLanguage {
    // ... existing checks ...

    // Ruby detection
    if root.join("Gemfile").exists()
        || has_gemspec(root)
        || root.join("config.ru").exists()
        || root.join("config/application.rb").exists()
    {
        return ProjectLanguage::Ruby;
    }

    // ... remaining checks ...
}

fn has_gemspec(root: &Path) -> bool {
    root.read_dir()
        .ok()
        .map(|entries| {
            entries.filter_map(|e| e.ok()).any(|entry| {
                entry.path().extension().and_then(|e| e.to_str()) == Some("gemspec")
            })
        })
        .unwrap_or(false)
}
```

**Verification:** `cargo test -p quench-cli checks::escapes` passes

---

### Phase 4: Test Fixture

Create a test fixture for Ruby lint policy.

**4.1 Create `tests/fixtures/ruby/lint-policy/`:**

`quench.toml`:
```toml
version = 1

[ruby.policy]
lint_changes = "standalone"
```

`Gemfile`:
```ruby
source "https://rubygems.org"
gem "rubocop"
```

`.rubocop.yml`:
```yaml
AllCops:
  TargetRubyVersion: 3.2
```

`lib/example.rb`:
```ruby
# frozen_string_literal: true

class Example
  def greet
    "Hello, world!"
  end
end
```

**4.2 Add spec test in `tests/specs/adapters/ruby.rs`:**

```rust
#[test]
fn lint_policy_standalone_violation() {
    cli()
        .on("ruby/lint-policy")
        .with_changed(&[".rubocop.yml", "lib/example.rb"])
        .fails()
        .stdout_has("lint_policy")
        .stdout_has("standalone");
}

#[test]
fn lint_policy_lint_only_ok() {
    cli()
        .on("ruby/lint-policy")
        .with_changed(&[".rubocop.yml"])
        .succeeds();
}

#[test]
fn lint_policy_source_only_ok() {
    cli()
        .on("ruby/lint-policy")
        .with_changed(&["lib/example.rb"])
        .succeeds();
}
```

**Verification:** `cargo test -p quench-cli --test specs adapters::ruby` passes

---

### Phase 5: Config Method Updates

Update Config methods to include Ruby.

**5.1 Update `policy_check_level_for_language()`:**

```rust
pub fn policy_check_level_for_language(&self, language: &str) -> CheckLevel {
    let lang_level = match language {
        "rust" => self.rust.policy.check,
        "go" | "golang" => self.golang.policy.check,
        "javascript" | "js" => self.javascript.policy.check,
        "shell" | "sh" => self.shell.policy.check,
        "ruby" | "rb" => self.ruby.policy.check,  // NEW
        _ => None,
    };
    lang_level.unwrap_or(CheckLevel::Error)
}
```

**5.2 Update `cloc_check_level_for_language()` and `cloc_advice_for_language()`:**

Add Ruby cases for completeness.

**Verification:** `make check` passes

---

## Key Implementation Details

### Lint Config Files

Ruby projects use three common lint configuration files:

| File | Tool | Purpose |
|------|------|---------|
| `.rubocop.yml` | RuboCop | Main linter config |
| `.rubocop_todo.yml` | RuboCop | Auto-generated TODOs |
| `.standard.yml` | Standard Ruby | Simplified RuboCop |

All three are included in the default `lint_config` list.

### Detection Priority

Ruby detection should occur after JavaScript but before Shell to avoid
false positives from JavaScript projects that may contain Ruby tooling.

Detection order in `detect_language()`:
1. Rust (Cargo.toml)
2. Go (go.mod)
3. JavaScript (package.json, tsconfig.json)
4. **Ruby (Gemfile, *.gemspec, config.ru, config/application.rb)**
5. Shell (*.sh files)

### Test File Classification

Ruby test files are identified by:
- `spec/**/*_spec.rb` - RSpec
- `test/**/*_test.rb` - Minitest (suffix style)
- `test/**/test_*.rb` - Minitest (prefix style)
- `features/**/*.rb` - Cucumber step definitions

## Verification Plan

### Unit Tests
- [ ] `cargo test -p quench-cli adapter::ruby::policy` - Policy tests
- [ ] `cargo test -p quench-cli config::` - Config parsing tests

### Integration Tests
- [ ] `cargo test -p quench-cli --test specs adapters::ruby` - Spec tests

### Manual Verification
```bash
# Create test scenario
mkdir -p /tmp/ruby-test && cd /tmp/ruby-test
echo 'source "https://rubygems.org"' > Gemfile
echo 'AllCops: {}' > .rubocop.yml
mkdir lib && echo 'class Foo; end' > lib/foo.rb
echo 'version = 1
[ruby.policy]
lint_changes = "standalone"' > quench.toml

# Test with mock changed files
quench check --base=HEAD~1  # Should fail if both .rubocop.yml and lib/foo.rb changed
```

### Full Check
```bash
make check  # Runs fmt, clippy, test, build, audit, deny
```
