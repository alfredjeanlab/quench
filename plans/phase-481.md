# Phase 481: Ruby Adapter - Behavioral Specs

## Overview

Write behavioral specifications (black-box tests) for the Ruby language adapter. These specs define expected behavior for auto-detection, default patterns, escape hatches, suppress directive checking, and lint policy enforcement. All specs will be marked `#[ignore]` since implementation occurs in Phases 483-487.

This phase establishes the contract for Ruby support through executable specifications, following the pattern from Go (`tests/specs/adapters/golang.rs`) and Rust (`tests/specs/adapters/rust.rs`) adapters.

## Project Structure

```
quench/
├── tests/
│   ├── specs.rs                      # Existing: add ruby module
│   ├── specs/
│   │   └── adapters/
│   │       ├── mod.rs                # Existing: add ruby module
│   │       └── ruby.rs               # New: Ruby adapter specs
│   └── fixtures/
│       └── ruby/                     # New: Ruby test fixtures
│           ├── auto-detect/          # Basic gem with Gemfile
│           ├── gemspec-detect/       # Gem with .gemspec
│           ├── config-ru-detect/     # Rack app with config.ru
│           ├── rails-detect/         # Rails app with config/application.rb
│           ├── eval-fail/            # eval( without comment
│           ├── eval-ok/              # eval( with METAPROGRAMMING comment
│           ├── instance-eval-fail/   # instance_eval without comment
│           ├── class-eval-fail/      # class_eval without comment
│           ├── binding-pry-fail/     # binding.pry in source
│           ├── byebug-fail/          # byebug in source
│           ├── debugger-fail/        # debugger in source
│           ├── debugger-test-ok/     # debugger in test file (allowed)
│           ├── rubocop-comment-fail/ # rubocop:disable without comment
│           ├── rubocop-comment-ok/   # rubocop:disable with comment
│           ├── rubocop-test-ok/      # rubocop:disable in test (allowed)
│           └── vendor-ignore/        # vendor/ should be ignored
```

## Dependencies

**None required** - This phase only adds spec files and fixtures.

**Implementation phases require:**
- Phase 201: Generic Language Adapter trait
- Phase 205-220: Escapes check framework

## Implementation Phases

### Phase 1: Spec Infrastructure

**Goal:** Create the spec file and register it in the test harness.

**Files:**
- `tests/specs/adapters/ruby.rs` - Create with module header
- `tests/specs/adapters/mod.rs` - Add `pub mod ruby;`

**Code:**
```rust
// tests/specs/adapters/ruby.rs
//! Behavioral specs for the Ruby language adapter.
//!
//! Tests that quench correctly:
//! - Detects Ruby projects via Gemfile, *.gemspec, config.ru, config/application.rb
//! - Applies default source/test/ignore patterns
//! - Applies Ruby-specific escape patterns (eval, debuggers)
//! - Checks rubocop:disable suppression directives
//!
//! Reference: docs/specs/langs/ruby.md

#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::prelude::*;
```

**Verification:**
```bash
cargo test --test specs -- adapters_ruby --list
```

### Phase 2: Auto-Detection Specs

**Goal:** Specify Ruby project detection from marker files.

**Specs (all ignored until Phase 483):**

| Spec | Marker File | Reference |
|------|-------------|-----------|
| `auto_detected_when_gemfile_present` | `Gemfile` | docs/specs/langs/ruby.md#detection |
| `auto_detected_when_gemspec_present` | `*.gemspec` | docs/specs/langs/ruby.md#detection |
| `auto_detected_when_config_ru_present` | `config.ru` | docs/specs/langs/ruby.md#detection |
| `auto_detected_when_rails_config_present` | `config/application.rb` | docs/specs/langs/ruby.md#detection |

**Spec Pattern:**
```rust
/// Spec: docs/specs/langs/ruby.md#detection
///
/// > Detected when `Gemfile` exists in project root.
#[test]
#[ignore = "TODO: Phase 483 - Ruby Detection"]
fn auto_detected_when_gemfile_present() {
    let result = cli().on("ruby/auto-detect").json().passes();
    let checks = result.checks();

    // escapes check should have Ruby-specific patterns active
    assert!(
        checks
            .iter()
            .any(|c| c.get("name").and_then(|n| n.as_str()) == Some("escapes"))
    );
}
```

**Fixtures:**
```
tests/fixtures/ruby/auto-detect/
├── Gemfile              # source 'https://rubygems.org'
├── lib/
│   └── example.rb       # module Example; end
├── spec/
│   └── example_spec.rb  # RSpec.describe Example
└── quench.toml          # version = 1

tests/fixtures/ruby/gemspec-detect/
├── example.gemspec      # Gem::Specification.new { |s| ... }
├── lib/
│   └── example.rb
└── quench.toml

tests/fixtures/ruby/config-ru-detect/
├── config.ru            # run MyApp
├── app.rb               # class MyApp; end
└── quench.toml

tests/fixtures/ruby/rails-detect/
├── config/
│   └── application.rb   # module MyApp; class Application < Rails::Application
├── app/
│   └── models/
│       └── user.rb      # class User; end
└── quench.toml
```

### Phase 3: Default Pattern Specs

**Goal:** Specify default source, test, and ignore patterns.

**Specs (all ignored until Phase 483):**

| Spec | Pattern | Reference |
|------|---------|-----------|
| `default_source_pattern_matches_rb_files` | `**/*.rb` | docs/specs/langs/ruby.md#default-patterns |
| `default_source_pattern_matches_rake_files` | `**/*.rake` | docs/specs/langs/ruby.md#default-patterns |
| `default_test_pattern_matches_spec_files` | `spec/**/*_spec.rb` | docs/specs/langs/ruby.md#default-patterns |
| `default_test_pattern_matches_test_files` | `test/**/*_test.rb` | docs/specs/langs/ruby.md#default-patterns |
| `default_test_pattern_matches_features` | `features/**/*.rb` | docs/specs/langs/ruby.md#default-patterns |
| `default_ignores_vendor_directory` | `vendor/` | docs/specs/langs/ruby.md#default-patterns |
| `default_ignores_tmp_directory` | `tmp/` | docs/specs/langs/ruby.md#default-patterns |
| `default_ignores_log_directory` | `log/` | docs/specs/langs/ruby.md#default-patterns |
| `default_ignores_coverage_directory` | `coverage/` | docs/specs/langs/ruby.md#default-patterns |

**Spec Pattern:**
```rust
/// Spec: docs/specs/langs/ruby.md#default-patterns
///
/// > source = ["**/*.rb", "**/*.rake", "Rakefile", "Gemfile", "*.gemspec"]
#[test]
#[ignore = "TODO: Phase 483 - Ruby Detection"]
fn default_source_pattern_matches_rb_files() {
    let cloc = check("cloc").on("ruby/auto-detect").json().passes();
    let metrics = cloc.require("metrics");

    let source_lines = metrics
        .get("source_lines")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    assert!(source_lines > 0, "should count .rb files as source");
}

/// Spec: docs/specs/langs/ruby.md#default-patterns
///
/// > ignore = ["vendor/", "tmp/", "log/", "coverage/"]
#[test]
#[ignore = "TODO: Phase 483 - Ruby Detection"]
fn default_ignores_vendor_directory() {
    let cloc = check("cloc").on("ruby/vendor-ignore").json().passes();
    let files = cloc.get("files").and_then(|f| f.as_array());

    if let Some(files) = files {
        assert!(
            !files
                .iter()
                .any(|f| f.as_str().map(|s| s.contains("vendor/")).unwrap_or(false)),
            "vendor/ directory should be ignored"
        );
    }
}
```

**Fixture:**
```
tests/fixtures/ruby/vendor-ignore/
├── Gemfile
├── lib/
│   └── app.rb           # Main source (counted)
├── vendor/
│   └── bundle/
│       └── dep.rb       # Should be ignored
└── quench.toml
```

### Phase 4: Escape Pattern Specs - Metaprogramming

**Goal:** Specify eval/metaprogramming escape patterns requiring `# METAPROGRAMMING:` comment.

**Specs (all ignored until Phase 485):**

| Spec | Pattern | Reference |
|------|---------|-----------|
| `eval_without_metaprogramming_comment_fails` | `eval(` | docs/specs/langs/ruby.md#default-escape-patterns |
| `eval_with_metaprogramming_comment_passes` | `eval(` | docs/specs/langs/ruby.md#default-escape-patterns |
| `instance_eval_without_metaprogramming_comment_fails` | `instance_eval` | docs/specs/langs/ruby.md#default-escape-patterns |
| `class_eval_without_metaprogramming_comment_fails` | `class_eval` | docs/specs/langs/ruby.md#default-escape-patterns |
| `metaprogramming_in_test_code_allowed` | (in spec/) | docs/specs/langs/ruby.md#escapes-in-test-code |

**Spec Pattern:**
```rust
/// Spec: docs/specs/langs/ruby.md#default-escape-patterns
///
/// > `eval(` requires `# METAPROGRAMMING:` comment
#[test]
#[ignore = "TODO: Phase 485 - Ruby Escapes"]
fn eval_without_metaprogramming_comment_fails() {
    check("escapes")
        .on("ruby/eval-fail")
        .fails()
        .stdout_has("escapes: FAIL")
        .stdout_has("# METAPROGRAMMING:");
}

/// Spec: docs/specs/langs/ruby.md#default-escape-patterns
///
/// > `eval(` with `# METAPROGRAMMING:` comment passes
#[test]
#[ignore = "TODO: Phase 485 - Ruby Escapes"]
fn eval_with_metaprogramming_comment_passes() {
    check("escapes").on("ruby/eval-ok").passes();
}
```

**Fixtures:**
```
tests/fixtures/ruby/eval-fail/
├── Gemfile
├── lib/
│   └── dynamic.rb       # eval(code_string) without comment
└── quench.toml

tests/fixtures/ruby/eval-ok/
├── Gemfile
├── lib/
│   └── dynamic.rb       # METAPROGRAMMING: DSL builder
                         # eval(code_string)
└── quench.toml

tests/fixtures/ruby/instance-eval-fail/
├── Gemfile
├── lib/
│   └── dsl.rb           # obj.instance_eval { } without comment
└── quench.toml

tests/fixtures/ruby/class-eval-fail/
├── Gemfile
├── lib/
│   └── macro.rb         # klass.class_eval { } without comment
└── quench.toml
```

### Phase 5: Escape Pattern Specs - Debuggers

**Goal:** Specify debugger statement detection (forbidden in source).

**Specs (all ignored until Phase 485):**

| Spec | Pattern | Reference |
|------|---------|-----------|
| `binding_pry_forbidden_in_source` | `binding.pry` | docs/specs/langs/ruby.md#default-escape-patterns |
| `byebug_forbidden_in_source` | `byebug` | docs/specs/langs/ruby.md#default-escape-patterns |
| `debugger_forbidden_in_source` | `debugger` | docs/specs/langs/ruby.md#default-escape-patterns |
| `debugger_allowed_in_test_code` | (in spec/) | docs/specs/langs/ruby.md#escapes-in-test-code |

**Spec Pattern:**
```rust
/// Spec: docs/specs/langs/ruby.md#default-escape-patterns
///
/// > `binding.pry` is forbidden in source code
#[test]
#[ignore = "TODO: Phase 485 - Ruby Escapes"]
fn binding_pry_forbidden_in_source() {
    check("escapes")
        .on("ruby/binding-pry-fail")
        .fails()
        .stdout_has("escapes: FAIL")
        .stdout_has("forbidden")
        .stdout_has("binding.pry");
}

/// Spec: docs/specs/langs/ruby.md#escapes-in-test-code
///
/// > Debuggers: Forbidden even in tests by default (common source of CI failures)
#[test]
#[ignore = "TODO: Phase 485 - Ruby Escapes"]
fn debugger_forbidden_even_in_test() {
    // Unlike metaprogramming, debuggers should fail in test code too
    check("escapes")
        .on("ruby/debugger-test-fail")
        .fails()
        .stdout_has("forbidden");
}

/// Spec: docs/specs/langs/ruby.md#profile-defaults
///
/// > in_tests = "allow" for debugger patterns (configurable)
#[test]
#[ignore = "TODO: Phase 485 - Ruby Escapes"]
fn debugger_allowed_in_test_when_configured() {
    // With in_tests = "allow", debuggers pass in test files
    check("escapes").on("ruby/debugger-test-ok").passes();
}
```

**Fixtures:**
```
tests/fixtures/ruby/binding-pry-fail/
├── Gemfile
├── lib/
│   └── app.rb           # def debug; binding.pry; end
└── quench.toml

tests/fixtures/ruby/byebug-fail/
├── Gemfile
├── lib/
│   └── app.rb           # def debug; byebug; end
└── quench.toml

tests/fixtures/ruby/debugger-fail/
├── Gemfile
├── lib/
│   └── app.rb           # def debug; debugger; end
└── quench.toml

tests/fixtures/ruby/debugger-test-ok/
├── Gemfile
├── lib/
│   └── app.rb           # Clean source
├── spec/
│   └── debug_spec.rb    # binding.pry in test (allowed with config)
└── quench.toml          # [check.escapes.patterns] in_tests = "allow"
```

### Phase 6: Suppress Directive Specs

**Goal:** Specify rubocop:disable directive checking.

**Specs (all ignored until Phase 486):**

| Spec | Directive | Reference |
|------|-----------|-----------|
| `rubocop_disable_without_comment_fails` | `# rubocop:disable` | docs/specs/langs/ruby.md#suppress |
| `rubocop_disable_with_comment_passes` | `# rubocop:disable` | docs/specs/langs/ruby.md#suppress |
| `rubocop_disable_multiple_cops_detected` | `# rubocop:disable A, B` | docs/specs/langs/ruby.md#supported-patterns |
| `rubocop_todo_detected` | `# rubocop:todo` | docs/specs/langs/ruby.md#supported-patterns |
| `standard_disable_detected` | `# standard:disable` | docs/specs/langs/ruby.md#supported-patterns |
| `rubocop_disable_inline_detected` | Same-line disable | docs/specs/langs/ruby.md#supported-patterns |
| `rubocop_disable_in_test_allowed` | (in spec/) | docs/specs/langs/ruby.md#suppress |

**Spec Pattern:**
```rust
/// Spec: docs/specs/langs/ruby.md#suppress
///
/// > "comment" - Requires justification comment (default for source)
#[test]
#[ignore = "TODO: Phase 486 - Ruby Suppress"]
fn rubocop_disable_without_comment_fails() {
    check("escapes")
        .on("ruby/rubocop-comment-fail")
        .fails()
        .stdout_has("suppress_missing_comment")
        .stdout_has("rubocop:disable");
}

/// Spec: docs/specs/langs/ruby.md#suppress
///
/// > Requires justification comment
#[test]
#[ignore = "TODO: Phase 486 - Ruby Suppress"]
fn rubocop_disable_with_comment_passes() {
    check("escapes").on("ruby/rubocop-comment-ok").passes();
}

/// Spec: docs/specs/langs/ruby.md#suppress
///
/// > [ruby.suppress.test] check = "allow" - tests can suppress freely
#[test]
#[ignore = "TODO: Phase 486 - Ruby Suppress"]
fn rubocop_disable_in_test_allowed() {
    check("escapes").on("ruby/rubocop-test-ok").passes();
}
```

**Fixtures:**
```
tests/fixtures/ruby/rubocop-comment-fail/
├── Gemfile
├── lib/
│   └── parser.rb        # rubocop:disable Style/Documentation
                         # class Parser; end
                         # rubocop:enable Style/Documentation
└── quench.toml          # [ruby.suppress] check = "comment"

tests/fixtures/ruby/rubocop-comment-ok/
├── Gemfile
├── lib/
│   └── parser.rb        # OK: Internal helper class
                         # rubocop:disable Style/Documentation
                         # class Parser; end
                         # rubocop:enable Style/Documentation
└── quench.toml          # [ruby.suppress] check = "comment"

tests/fixtures/ruby/rubocop-test-ok/
├── Gemfile
├── lib/
│   └── app.rb           # Clean source
├── spec/
│   └── app_spec.rb      # rubocop:disable without comment (allowed in tests)
└── quench.toml          # [ruby.suppress.test] check = "allow"
```

### Phase 7: Lint Policy Specs

**Goal:** Specify standalone lint config policy enforcement.

**Specs (all ignored until Phase 487):**

| Spec | Scenario | Reference |
|------|----------|-----------|
| `lint_config_changes_with_source_fails_standalone_policy` | Mixed changes | docs/specs/langs/ruby.md#policy |
| `lint_config_standalone_passes` | Lint-only changes | docs/specs/langs/ruby.md#policy |

**Spec Pattern:**
```rust
/// Spec: docs/specs/langs/ruby.md#policy
///
/// > lint_changes = "standalone" - lint config changes must be standalone PRs
#[test]
#[ignore = "TODO: Phase 487 - Ruby Policy"]
fn lint_config_changes_with_source_fails_standalone_policy() {
    let temp = Project::empty();

    temp.config(
        r#"[ruby.policy]
lint_changes = "standalone"
lint_config = [".rubocop.yml"]
"#,
    );

    temp.file("Gemfile", "source 'https://rubygems.org'\n");

    // Initialize git repo
    git_init(&temp);

    // Create initial commit with source
    temp.file("lib/app.rb", "class App; end");
    git_initial_commit(&temp);

    // Add both lint config and source changes
    temp.file(".rubocop.yml", "AllCops:\n  TargetRubyVersion: 3.2\n");
    temp.file("lib/app.rb", "class App\n  def hello; end\nend");

    git_add_all(&temp);

    // Check with --base HEAD should detect mixed changes
    check("escapes")
        .pwd(temp.path())
        .args(&["--base", "HEAD"])
        .fails()
        .stdout_has("lint config")
        .stdout_has("separate PR");
}

/// Spec: docs/specs/langs/ruby.md#policy
///
/// > Lint config changes only (no source) passes standalone policy.
#[test]
#[ignore = "TODO: Phase 487 - Ruby Policy"]
fn lint_config_standalone_passes() {
    let temp = Project::empty();

    temp.config(
        r#"[ruby.policy]
lint_changes = "standalone"
lint_config = [".rubocop.yml"]
"#,
    );

    temp.file("Gemfile", "source 'https://rubygems.org'\n");

    // Initialize git repo
    git_init(&temp);

    // Create initial commit
    temp.file("lib/app.rb", "class App; end");
    git_initial_commit(&temp);

    // Add ONLY lint config change (no source changes)
    temp.file(".rubocop.yml", "AllCops:\n  TargetRubyVersion: 3.2\n");

    git_add_all(&temp);

    // Should pass - only lint config changed
    check("escapes")
        .pwd(temp.path())
        .args(&["--base", "HEAD"])
        .passes();
}
```

## Key Implementation Details

### Spec File Header

```rust
//! Behavioral specs for the Ruby language adapter.
//!
//! Tests that quench correctly:
//! - Detects Ruby projects via Gemfile, *.gemspec, config.ru, config/application.rb
//! - Applies default source/test/ignore patterns
//! - Applies Ruby-specific escape patterns (eval, instance_eval, class_eval, debuggers)
//! - Checks rubocop:disable suppression directives
//! - Enforces lint policy for standalone config changes
//!
//! Reference: docs/specs/langs/ruby.md

#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::prelude::*;
```

### Fixture quench.toml Pattern

Minimal fixture config:
```toml
version = 1
```

Config with Ruby-specific settings:
```toml
version = 1

[ruby.suppress]
check = "comment"

[ruby.suppress.test]
check = "allow"
```

### Escape Pattern Output Format

Expected violation output format (from escapes check):
```
escapes: FAIL
  lib/dynamic.rb:5: missing_comment: eval
    Add a # METAPROGRAMMING: comment explaining why eval is necessary.
FAIL: escapes
```

### Suppress Output Format

Expected suppress violation output:
```
escapes: FAIL
  lib/parser.rb:2: suppress_missing_comment: # rubocop:disable Style/Documentation
    Lint suppression requires justification.
    Should this class have documentation?
    Add a comment above the directive.

FAIL: escapes
```

## Verification Plan

### Compile Check

Verify specs compile (all will be ignored):
```bash
cargo test --test specs -- adapters::ruby --list
```

Expected output:
```
ruby::auto_detected_when_gemfile_present: test [ignored]
ruby::auto_detected_when_gemspec_present: test [ignored]
...
```

### Fixture Validation

Verify fixtures are well-formed:
```bash
# Each fixture should have required files
for d in tests/fixtures/ruby/*/; do
  echo "=== $d ==="
  ls -la "$d"
done
```

### Module Registration

Verify `tests/specs/adapters/mod.rs` includes:
```rust
pub mod ruby;
```

### Checklist

- [ ] `tests/specs/adapters/ruby.rs` created with all specs
- [ ] `tests/specs/adapters/mod.rs` updated with `pub mod ruby;`
- [ ] All specs marked with `#[ignore = "TODO: Phase N - ..."]`
- [ ] All specs reference `docs/specs/langs/ruby.md` sections
- [ ] Test fixtures created under `tests/fixtures/ruby/`
- [ ] Each fixture has minimal `quench.toml` and Ruby marker file
- [ ] `cargo test --test specs -- adapters::ruby --list` shows all specs
- [ ] `make check` passes

### Exit Criteria

The following specs exist and compile (all ignored):

**Auto-Detection (Phase 483):**
1. `auto_detected_when_gemfile_present`
2. `auto_detected_when_gemspec_present`
3. `auto_detected_when_config_ru_present`
4. `auto_detected_when_rails_config_present`

**Default Patterns (Phase 483):**
5. `default_source_pattern_matches_rb_files`
6. `default_source_pattern_matches_rake_files`
7. `default_test_pattern_matches_spec_files`
8. `default_test_pattern_matches_test_files`
9. `default_ignores_vendor_directory`

**Escape Patterns (Phase 485):**
10. `eval_without_metaprogramming_comment_fails`
11. `eval_with_metaprogramming_comment_passes`
12. `instance_eval_without_metaprogramming_comment_fails`
13. `class_eval_without_metaprogramming_comment_fails`
14. `binding_pry_forbidden_in_source`
15. `byebug_forbidden_in_source`
16. `debugger_forbidden_in_source`

**Suppress Directives (Phase 486):**
17. `rubocop_disable_without_comment_fails`
18. `rubocop_disable_with_comment_passes`
19. `rubocop_disable_in_test_allowed`

**Lint Policy (Phase 487):**
20. `lint_config_changes_with_source_fails_standalone_policy`
21. `lint_config_standalone_passes`
