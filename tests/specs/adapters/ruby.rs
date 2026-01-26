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

// =============================================================================
// AUTO-DETECTION SPECS
// =============================================================================

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

/// Spec: docs/specs/langs/ruby.md#detection
///
/// > Detected when `*.gemspec` exists in project root.
#[test]
#[ignore = "TODO: Phase 483 - Ruby Detection"]
fn auto_detected_when_gemspec_present() {
    let result = cli().on("ruby/gemspec-detect").json().passes();
    let checks = result.checks();

    assert!(
        checks
            .iter()
            .any(|c| c.get("name").and_then(|n| n.as_str()) == Some("escapes"))
    );
}

/// Spec: docs/specs/langs/ruby.md#detection
///
/// > Detected when `config.ru` exists in project root.
#[test]
#[ignore = "TODO: Phase 483 - Ruby Detection"]
fn auto_detected_when_config_ru_present() {
    let result = cli().on("ruby/config-ru-detect").json().passes();
    let checks = result.checks();

    assert!(
        checks
            .iter()
            .any(|c| c.get("name").and_then(|n| n.as_str()) == Some("escapes"))
    );
}

/// Spec: docs/specs/langs/ruby.md#detection
///
/// > Detected when `config/application.rb` exists (Rails).
#[test]
#[ignore = "TODO: Phase 483 - Ruby Detection"]
fn auto_detected_when_rails_config_present() {
    let result = cli().on("ruby/rails-detect").json().passes();
    let checks = result.checks();

    assert!(
        checks
            .iter()
            .any(|c| c.get("name").and_then(|n| n.as_str()) == Some("escapes"))
    );
}

// =============================================================================
// DEFAULT PATTERN SPECS
// =============================================================================

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
/// > source = ["**/*.rb", "**/*.rake", ...]
#[test]
#[ignore = "TODO: Phase 483 - Ruby Detection"]
fn default_source_pattern_matches_rake_files() {
    let cloc = check("cloc").on("ruby/rake-files").json().passes();
    let metrics = cloc.require("metrics");

    let source_lines = metrics
        .get("source_lines")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    assert!(source_lines > 0, "should count .rake files as source");
}

/// Spec: docs/specs/langs/ruby.md#default-patterns
///
/// > tests = ["spec/**/*_spec.rb", ...]
#[test]
#[ignore = "TODO: Phase 483 - Ruby Detection"]
fn default_test_pattern_matches_spec_files() {
    let cloc = check("cloc").on("ruby/auto-detect").json().passes();
    let metrics = cloc.require("metrics");

    let test_lines = metrics
        .get("test_lines")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    assert!(test_lines > 0, "should count spec/**/*_spec.rb as test");
}

/// Spec: docs/specs/langs/ruby.md#default-patterns
///
/// > tests = ["test/**/*_test.rb", ...]
#[test]
#[ignore = "TODO: Phase 483 - Ruby Detection"]
fn default_test_pattern_matches_test_files() {
    let cloc = check("cloc").on("ruby/test-unit").json().passes();
    let metrics = cloc.require("metrics");

    let test_lines = metrics
        .get("test_lines")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    assert!(test_lines > 0, "should count test/**/*_test.rb as test");
}

/// Spec: docs/specs/langs/ruby.md#default-patterns
///
/// > tests = ["features/**/*.rb", ...]
#[test]
#[ignore = "TODO: Phase 483 - Ruby Detection"]
fn default_test_pattern_matches_features() {
    let cloc = check("cloc").on("ruby/cucumber-features").json().passes();
    let metrics = cloc.require("metrics");

    let test_lines = metrics
        .get("test_lines")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    assert!(test_lines > 0, "should count features/**/*.rb as test");
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

/// Spec: docs/specs/langs/ruby.md#default-patterns
///
/// > ignore = ["vendor/", "tmp/", ...]
#[test]
#[ignore = "TODO: Phase 483 - Ruby Detection"]
fn default_ignores_tmp_directory() {
    let cloc = check("cloc").on("ruby/tmp-ignore").json().passes();
    let files = cloc.get("files").and_then(|f| f.as_array());

    if let Some(files) = files {
        assert!(
            !files
                .iter()
                .any(|f| f.as_str().map(|s| s.contains("tmp/")).unwrap_or(false)),
            "tmp/ directory should be ignored"
        );
    }
}

/// Spec: docs/specs/langs/ruby.md#default-patterns
///
/// > ignore = ["vendor/", "tmp/", "log/", ...]
#[test]
#[ignore = "TODO: Phase 483 - Ruby Detection"]
fn default_ignores_log_directory() {
    let cloc = check("cloc").on("ruby/log-ignore").json().passes();
    let files = cloc.get("files").and_then(|f| f.as_array());

    if let Some(files) = files {
        assert!(
            !files
                .iter()
                .any(|f| f.as_str().map(|s| s.contains("log/")).unwrap_or(false)),
            "log/ directory should be ignored"
        );
    }
}

/// Spec: docs/specs/langs/ruby.md#default-patterns
///
/// > ignore = ["vendor/", "tmp/", "log/", "coverage/"]
#[test]
#[ignore = "TODO: Phase 483 - Ruby Detection"]
fn default_ignores_coverage_directory() {
    let cloc = check("cloc").on("ruby/coverage-ignore").json().passes();
    let files = cloc.get("files").and_then(|f| f.as_array());

    if let Some(files) = files {
        assert!(
            !files
                .iter()
                .any(|f| f.as_str().map(|s| s.contains("coverage/")).unwrap_or(false)),
            "coverage/ directory should be ignored"
        );
    }
}

// =============================================================================
// ESCAPE PATTERN SPECS - Metaprogramming
// =============================================================================

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

/// Spec: docs/specs/langs/ruby.md#default-escape-patterns
///
/// > `instance_eval` requires `# METAPROGRAMMING:` comment
#[test]
#[ignore = "TODO: Phase 485 - Ruby Escapes"]
fn instance_eval_without_metaprogramming_comment_fails() {
    check("escapes")
        .on("ruby/instance-eval-fail")
        .fails()
        .stdout_has("escapes: FAIL")
        .stdout_has("# METAPROGRAMMING:");
}

/// Spec: docs/specs/langs/ruby.md#default-escape-patterns
///
/// > `class_eval` requires `# METAPROGRAMMING:` comment
#[test]
#[ignore = "TODO: Phase 485 - Ruby Escapes"]
fn class_eval_without_metaprogramming_comment_fails() {
    check("escapes")
        .on("ruby/class-eval-fail")
        .fails()
        .stdout_has("escapes: FAIL")
        .stdout_has("# METAPROGRAMMING:");
}

/// Spec: docs/specs/langs/ruby.md#escapes-in-test-code
///
/// > Metaprogramming escapes allowed in test code by default
#[test]
#[ignore = "TODO: Phase 485 - Ruby Escapes"]
fn metaprogramming_in_test_code_allowed() {
    check("escapes").on("ruby/eval-test-ok").passes();
}

// =============================================================================
// ESCAPE PATTERN SPECS - Debuggers
// =============================================================================

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

/// Spec: docs/specs/langs/ruby.md#default-escape-patterns
///
/// > `byebug` is forbidden in source code
#[test]
#[ignore = "TODO: Phase 485 - Ruby Escapes"]
fn byebug_forbidden_in_source() {
    check("escapes")
        .on("ruby/byebug-fail")
        .fails()
        .stdout_has("escapes: FAIL")
        .stdout_has("forbidden")
        .stdout_has("byebug");
}

/// Spec: docs/specs/langs/ruby.md#default-escape-patterns
///
/// > `debugger` is forbidden in source code
#[test]
#[ignore = "TODO: Phase 485 - Ruby Escapes"]
fn debugger_forbidden_in_source() {
    check("escapes")
        .on("ruby/debugger-fail")
        .fails()
        .stdout_has("escapes: FAIL")
        .stdout_has("forbidden")
        .stdout_has("debugger");
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

// =============================================================================
// SUPPRESS DIRECTIVE SPECS
// =============================================================================

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

/// Spec: docs/specs/langs/ruby.md#supported-patterns
///
/// > # rubocop:disable A, B (multiple cops)
#[test]
#[ignore = "TODO: Phase 486 - Ruby Suppress"]
fn rubocop_disable_multiple_cops_detected() {
    check("escapes")
        .on("ruby/rubocop-multiple-fail")
        .fails()
        .stdout_has("rubocop:disable");
}

/// Spec: docs/specs/langs/ruby.md#supported-patterns
///
/// > # rubocop:todo (same as disable)
#[test]
#[ignore = "TODO: Phase 486 - Ruby Suppress"]
fn rubocop_todo_detected() {
    check("escapes")
        .on("ruby/rubocop-todo-fail")
        .fails()
        .stdout_has("rubocop:todo");
}

/// Spec: docs/specs/langs/ruby.md#supported-patterns
///
/// > # standard:disable (StandardRB variant)
#[test]
#[ignore = "TODO: Phase 486 - Ruby Suppress"]
fn standard_disable_detected() {
    check("escapes")
        .on("ruby/standard-disable-fail")
        .fails()
        .stdout_has("standard:disable");
}

/// Spec: docs/specs/langs/ruby.md#supported-patterns
///
/// > Same-line disable detected
#[test]
#[ignore = "TODO: Phase 486 - Ruby Suppress"]
fn rubocop_disable_inline_detected() {
    check("escapes")
        .on("ruby/rubocop-inline-fail")
        .fails()
        .stdout_has("rubocop:disable");
}

/// Spec: docs/specs/langs/ruby.md#suppress
///
/// > [ruby.suppress.test] check = "allow" - tests can suppress freely
#[test]
#[ignore = "TODO: Phase 486 - Ruby Suppress"]
fn rubocop_disable_in_test_allowed() {
    check("escapes").on("ruby/rubocop-test-ok").passes();
}

// =============================================================================
// LINT CONFIG POLICY SPECS
// =============================================================================

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
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(temp.path())
        .output()
        .unwrap();

    std::process::Command::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(temp.path())
        .output()
        .unwrap();

    std::process::Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(temp.path())
        .output()
        .unwrap();

    // Create initial commit with source
    temp.file("lib/app.rb", "class App; end\n");

    std::process::Command::new("git")
        .args(["add", "-A"])
        .current_dir(temp.path())
        .output()
        .unwrap();

    std::process::Command::new("git")
        .args(["commit", "-m", "initial"])
        .current_dir(temp.path())
        .output()
        .unwrap();

    // Add both lint config and source changes
    temp.file(".rubocop.yml", "AllCops:\n  TargetRubyVersion: 3.2\n");
    temp.file("lib/app.rb", "class App\n  def hello; end\nend\n");

    std::process::Command::new("git")
        .args(["add", "-A"])
        .current_dir(temp.path())
        .output()
        .unwrap();

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
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(temp.path())
        .output()
        .unwrap();

    std::process::Command::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(temp.path())
        .output()
        .unwrap();

    std::process::Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(temp.path())
        .output()
        .unwrap();

    // Create initial commit
    temp.file("lib/app.rb", "class App; end\n");

    std::process::Command::new("git")
        .args(["add", "-A"])
        .current_dir(temp.path())
        .output()
        .unwrap();

    std::process::Command::new("git")
        .args(["commit", "-m", "initial"])
        .current_dir(temp.path())
        .output()
        .unwrap();

    // Add ONLY lint config change (no source changes)
    temp.file(".rubocop.yml", "AllCops:\n  TargetRubyVersion: 3.2\n");

    std::process::Command::new("git")
        .args(["add", "-A"])
        .current_dir(temp.path())
        .output()
        .unwrap();

    // Should pass - only lint config changed
    check("escapes")
        .pwd(temp.path())
        .args(&["--base", "HEAD"])
        .passes();
}
