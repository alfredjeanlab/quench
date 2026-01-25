//! Behavioral specs for `quench init` command.

use crate::prelude::*;

// =============================================================================
// Phase 1: Basic Creation Specs
// =============================================================================

/// Spec: docs/specs/01-cli.md#quench-init
///
/// > quench init creates quench.toml in current directory
#[test]
fn init_creates_quench_toml_in_current_directory() {
    let temp = Project::empty();

    quench_cmd()
        .args(["init"])
        .current_dir(temp.path())
        .assert()
        .success();

    assert!(temp.path().join("quench.toml").exists());
}

// =============================================================================
// Force Flag Specs
// =============================================================================

/// Spec: docs/specs/01-cli.md#quench-init
///
/// > Refuses to overwrite existing quench.toml without --force
#[test]
fn init_refuses_to_overwrite_without_force() {
    let temp = Project::empty();
    temp.file("quench.toml", "version = 1\n# existing\n");

    quench_cmd()
        .args(["init"])
        .current_dir(temp.path())
        .assert()
        .code(2)
        .stderr(predicates::str::contains("already exists"))
        .stderr(predicates::str::contains("--force"));
}

/// Spec: docs/specs/01-cli.md#quench-init
///
/// > --force overwrites existing quench.toml
#[test]
fn init_force_overwrites_existing_config() {
    let temp = Project::empty();
    temp.file("quench.toml", "version = 1\n# existing content\n");

    quench_cmd()
        .args(["init", "--force"])
        .current_dir(temp.path())
        .assert()
        .success();

    let config = std::fs::read_to_string(temp.path().join("quench.toml")).unwrap();
    assert!(!config.contains("# existing content"), "should overwrite");
    assert!(config.contains("version = 1"));
}

// =============================================================================
// Explicit Profile Specs
// =============================================================================

/// Spec: docs/specs/01-cli.md#explicit-profiles
///
/// > --with rust configures Rust defaults
#[test]
fn init_with_rust_configures_rust_defaults() {
    let temp = Project::empty();

    quench_cmd()
        .args(["init", "--with", "rust"])
        .current_dir(temp.path())
        .assert()
        .success()
        .stdout(predicates::str::contains("rust"));

    let config = std::fs::read_to_string(temp.path().join("quench.toml")).unwrap();
    assert!(config.contains("[rust]"));
    assert!(config.contains("[rust.suppress]"));
    assert!(config.contains("[rust.policy]"));
    assert!(
        config.contains("unsafe"),
        "should have unsafe escape pattern"
    );
}

/// Spec: docs/specs/01-cli.md#explicit-profiles
///
/// > --with claude configures CLAUDE.md defaults
#[test]
fn init_with_claude_configures_claude_defaults() {
    let temp = Project::empty();

    quench_cmd()
        .args(["init", "--with", "claude"])
        .current_dir(temp.path())
        .assert()
        .success();

    let config = std::fs::read_to_string(temp.path().join("quench.toml")).unwrap();
    assert!(config.contains("[check.agents]"));
    assert!(config.contains("CLAUDE.md"));
}

/// Spec: docs/specs/01-cli.md#explicit-profiles
///
/// > --with cursor configures .cursorrules defaults
#[test]
fn init_with_cursor_configures_cursor_defaults() {
    let temp = Project::empty();

    quench_cmd()
        .args(["init", "--with", "cursor"])
        .current_dir(temp.path())
        .assert()
        .success();

    let config = std::fs::read_to_string(temp.path().join("quench.toml")).unwrap();
    assert!(config.contains("[check.agents]"));
    assert!(config.contains(".cursorrules"));
}

// =============================================================================
// Shell Detection Specs (additional locations)
// =============================================================================

/// Spec: docs/specs/01-cli.md#auto-detection
///
/// > Auto-detects Shell when *.sh in root
#[test]
fn init_auto_detects_shell_from_root_sh() {
    let temp = Project::empty();
    temp.file("build.sh", "#!/bin/bash\necho hello\n");

    quench_cmd()
        .args(["init"])
        .current_dir(temp.path())
        .assert()
        .success();

    let config = std::fs::read_to_string(temp.path().join("quench.toml")).unwrap();
    assert!(config.contains("[shell]"));
}

/// Spec: docs/specs/01-cli.md#auto-detection
///
/// > Auto-detects Shell when *.sh in bin/
#[test]
fn init_auto_detects_shell_from_bin_dir() {
    let temp = Project::empty();
    temp.file("bin/run.sh", "#!/bin/bash\n");

    quench_cmd()
        .args(["init"])
        .current_dir(temp.path())
        .assert()
        .success();

    let config = std::fs::read_to_string(temp.path().join("quench.toml")).unwrap();
    assert!(config.contains("[shell]"));
}

// =============================================================================
// Profile Name Validation Specs
// =============================================================================

/// Spec: docs/specs/01-cli.md#explicit-profiles
///
/// > Valid profile names: rust, shell, claude, cursor (plus golang, javascript)
#[test]
fn init_accepts_valid_profile_names() {
    for profile in ["rust", "shell", "claude", "cursor"] {
        let temp = Project::empty();

        quench_cmd()
            .args(["init", "--with", profile])
            .current_dir(temp.path())
            .assert()
            .success();

        assert!(temp.path().join("quench.toml").exists());
    }
}

/// Spec: docs/specs/01-cli.md#explicit-profiles
///
/// > Unknown profile names produce warning
#[test]
fn init_warns_on_unknown_profile() {
    let temp = Project::empty();

    quench_cmd()
        .args(["init", "--with", "python"])
        .current_dir(temp.path())
        .assert()
        .success()
        .stderr(predicates::str::contains("unknown profile"));
}

/// Spec: docs/specs/01-cli.md#explicit-profiles
///
/// > Profile names are case-insensitive
#[test]
fn init_profile_names_case_insensitive() {
    let temp = Project::empty();

    quench_cmd()
        .args(["init", "--with", "RUST"])
        .current_dir(temp.path())
        .assert()
        .success();

    let config = std::fs::read_to_string(temp.path().join("quench.toml")).unwrap();
    assert!(config.contains("[rust]"));
}

// =============================================================================
// Existing Shell Profile Specs
// =============================================================================

/// Spec: docs/specs/01-cli.md#profile-selection-recommended
///
/// > quench init --profile shell - Shell project defaults
#[test]
fn init_shell_profile_generates_config() {
    let temp = Project::empty();

    quench_cmd()
        .args(["init", "--with", "shell"])
        .current_dir(temp.path())
        .assert()
        .success()
        .stdout(predicates::str::contains("Created quench.toml"));

    let config = std::fs::read_to_string(temp.path().join("quench.toml")).unwrap();
    assert!(
        config.contains("[shell]"),
        "config should have [shell] section"
    );
    assert!(
        config.contains("[shell.suppress]"),
        "config should have [shell.suppress] section"
    );
    assert!(
        config.contains("[shell.policy]"),
        "config should have [shell.policy] section"
    );
}

/// Spec: docs/specs/01-cli.md#profile-selection-recommended
///
/// > Shell profile includes escape patterns for set +e, eval, rm -rf
#[test]
fn init_shell_profile_includes_escape_patterns() {
    let temp = Project::empty();

    quench_cmd()
        .args(["init", "--with", "shell"])
        .current_dir(temp.path())
        .assert()
        .success();

    let config = std::fs::read_to_string(temp.path().join("quench.toml")).unwrap();
    assert!(
        config.contains("set \\\\+e") || config.contains("set \\+e"),
        "config should have set +e escape pattern"
    );
    assert!(
        config.contains("eval"),
        "config should have eval escape pattern"
    );
    assert!(
        config.contains("# OK:"),
        "config should have # OK: comment marker"
    );
}

/// Spec: docs/specs/01-cli.md#profile-selection-recommended
///
/// > quench init --profile rust,shell - Multi-language project
#[test]
fn init_combined_profiles_generates_both() {
    let temp = Project::empty();

    quench_cmd()
        .args(["init", "--with", "rust,shell"])
        .current_dir(temp.path())
        .assert()
        .success()
        .stdout(predicates::str::contains("rust, shell"));

    let config = std::fs::read_to_string(temp.path().join("quench.toml")).unwrap();
    assert!(
        config.contains("[rust]"),
        "config should have [rust] section"
    );
    assert!(
        config.contains("[shell]"),
        "config should have [shell] section"
    );
}

/// Spec: docs/specs/01-cli.md#profile-selection-recommended
///
/// > Shell profile output message includes profile name
#[test]
fn init_shell_profile_message() {
    let temp = Project::empty();

    quench_cmd()
        .args(["init", "--with", "shell"])
        .current_dir(temp.path())
        .assert()
        .success()
        .stdout(predicates::str::contains("shell"));
}

// =============================================================================
// Phase 2: --with Flag Specs
// =============================================================================

/// Spec: docs/specs/commands/quench-init.md#--with-flag
///
/// > --with flag accepts comma-separated profiles
#[test]
fn init_with_accepts_comma_separated_profiles() {
    let temp = Project::empty();
    quench_cmd()
        .args(["init", "--with", "rust,shell"])
        .current_dir(temp.path())
        .assert()
        .success();

    let config = std::fs::read_to_string(temp.path().join("quench.toml")).unwrap();
    assert!(config.contains("[rust]"));
    assert!(config.contains("[shell]"));
}

/// Spec: docs/specs/commands/quench-init.md#--with-flag
///
/// > --with skips auto-detection
#[test]
fn init_with_skips_auto_detection() {
    let temp = Project::empty();
    temp.file("Cargo.toml", "[package]\nname = \"test\"\n");
    temp.file("go.mod", "module test\n");

    quench_cmd()
        .args(["init", "--with", "shell"])
        .current_dir(temp.path())
        .assert()
        .success();

    let config = std::fs::read_to_string(temp.path().join("quench.toml")).unwrap();
    assert!(config.contains("[shell]"));
    // Check for actual section headers, not comments
    assert!(
        !config.lines().any(|l| l.trim() == "[rust]"),
        "--with should skip rust detection"
    );
    assert!(
        !config.lines().any(|l| l.trim() == "[golang]"),
        "--with should skip go detection"
    );
}

/// Spec: docs/specs/commands/quench-init.md#language-detection
///
/// > no --with triggers auto-detection
#[test]
fn init_without_with_triggers_auto_detection() {
    let temp = Project::empty();
    temp.file("Cargo.toml", "[package]\nname = \"test\"\n");

    quench_cmd()
        .args(["init"])
        .current_dir(temp.path())
        .assert()
        .success();

    let config = std::fs::read_to_string(temp.path().join("quench.toml")).unwrap();
    assert!(config.contains("[rust]"), "should auto-detect rust");
}

// =============================================================================
// Phase 3: Language Detection Specs
// =============================================================================

/// Spec: docs/specs/commands/quench-init.md#language-detection
///
/// > Cargo.toml -> rust
#[test]
fn init_detects_rust_from_cargo_toml() {
    let temp = Project::empty();
    temp.file("Cargo.toml", "[package]\nname = \"test\"\n");

    quench_cmd()
        .args(["init"])
        .current_dir(temp.path())
        .assert()
        .success();

    let config = std::fs::read_to_string(temp.path().join("quench.toml")).unwrap();
    assert!(config.contains("[rust]"));
    assert!(config.contains("rust.cloc.check"));
}

/// Spec: docs/specs/commands/quench-init.md#language-detection
///
/// > go.mod -> golang
#[test]
fn init_detects_golang_from_go_mod() {
    let temp = Project::empty();
    temp.file("go.mod", "module test\n");

    quench_cmd()
        .args(["init"])
        .current_dir(temp.path())
        .assert()
        .success();

    let config = std::fs::read_to_string(temp.path().join("quench.toml")).unwrap();
    assert!(config.contains("[golang]"));
}

/// Spec: docs/specs/commands/quench-init.md#language-detection
///
/// > package.json -> javascript
#[test]
fn init_detects_javascript_from_package_json() {
    let temp = Project::empty();
    temp.file("package.json", "{\"name\": \"test\"}\n");

    quench_cmd()
        .args(["init"])
        .current_dir(temp.path())
        .assert()
        .success();

    let config = std::fs::read_to_string(temp.path().join("quench.toml")).unwrap();
    assert!(config.contains("[javascript]"));
}

/// Spec: docs/specs/commands/quench-init.md#language-detection
///
/// > *.sh in root/bin/scripts -> shell
#[test]
fn init_detects_shell_from_scripts_dir() {
    let temp = Project::empty();
    temp.file("scripts/build.sh", "#!/bin/bash\necho hello\n");

    quench_cmd()
        .args(["init"])
        .current_dir(temp.path())
        .assert()
        .success();

    let config = std::fs::read_to_string(temp.path().join("quench.toml")).unwrap();
    assert!(config.contains("[shell]"));
}

/// Spec: docs/specs/commands/quench-init.md#language-detection
///
/// > Detection is additive (multiple languages/agents)
#[test]
fn init_detection_is_additive() {
    let temp = Project::empty();
    temp.file("Cargo.toml", "[package]\nname = \"test\"\n");
    temp.file("scripts/deploy.sh", "#!/bin/bash\n");

    quench_cmd()
        .args(["init"])
        .current_dir(temp.path())
        .assert()
        .success();

    let config = std::fs::read_to_string(temp.path().join("quench.toml")).unwrap();
    assert!(config.contains("[rust]"), "should detect rust");
    assert!(config.contains("[shell]"), "should detect shell");
}

// =============================================================================
// Phase 4: Agent Detection Specs
// =============================================================================

/// Spec: docs/specs/commands/quench-init.md#agent-detection
///
/// > CLAUDE.md -> claude
#[test]
fn init_detects_claude_from_claude_md() {
    let temp = Project::empty();
    temp.file("CLAUDE.md", "# Project\n");

    quench_cmd()
        .args(["init"])
        .current_dir(temp.path())
        .assert()
        .success();

    let config = std::fs::read_to_string(temp.path().join("quench.toml")).unwrap();
    assert!(config.contains("[check.agents]"));
    assert!(config.contains("required") && config.contains("CLAUDE.md"));
}

/// Spec: docs/specs/commands/quench-init.md#agent-detection
///
/// > .cursorrules -> cursor
#[test]
fn init_detects_cursor_from_cursorrules() {
    let temp = Project::empty();
    temp.file(".cursorrules", "# Cursor rules\n");

    quench_cmd()
        .args(["init"])
        .current_dir(temp.path())
        .assert()
        .success();

    let config = std::fs::read_to_string(temp.path().join("quench.toml")).unwrap();
    assert!(config.contains("[check.agents]"));
    assert!(config.contains("required") && config.contains(".cursorrules"));
}

/// Spec: docs/specs/commands/quench-init.md#agent-detection
///
/// > .cursor/rules/*.md[c] -> cursor
#[test]
fn init_detects_cursor_from_mdc_rules() {
    let temp = Project::empty();
    temp.file(".cursor/rules/project.mdc", "# Cursor rules\n");

    quench_cmd()
        .args(["init"])
        .current_dir(temp.path())
        .assert()
        .success();

    let config = std::fs::read_to_string(temp.path().join("quench.toml")).unwrap();
    assert!(config.contains("[check.agents]"));
    // Should detect cursor agent presence
}

// =============================================================================
// Phase 5: Output Format Specs
// =============================================================================

/// Spec: docs/specs/commands/quench-init.md#default-output
///
/// > Output matches templates/init.default.toml format
#[test]
fn init_output_matches_template_format() {
    let temp = Project::empty();

    quench_cmd()
        .args(["init"])
        .current_dir(temp.path())
        .assert()
        .success();

    let config = std::fs::read_to_string(temp.path().join("quench.toml")).unwrap();

    // Base template fields
    assert!(config.contains("version = 1"));
    assert!(config.contains("[check.cloc]"));
    assert!(config.contains("[check.escapes]"));
    assert!(config.contains("[check.agents]"));
    assert!(config.contains("[check.docs]"));
    assert!(config.contains("# Supported Languages:"));
}

/// Spec: docs/specs/commands/quench-init.md#language-detection
///
/// > Detected language appends [lang] section with dotted keys
#[test]
fn init_detected_language_uses_dotted_keys() {
    let temp = Project::empty();
    temp.file("Cargo.toml", "[package]\nname = \"test\"\n");

    quench_cmd()
        .args(["init"])
        .current_dir(temp.path())
        .assert()
        .success();

    let config = std::fs::read_to_string(temp.path().join("quench.toml")).unwrap();

    assert!(config.contains("[rust]"));
    assert!(config.contains("rust.cloc.check"));
    assert!(config.contains("rust.policy.check"));
    assert!(config.contains("rust.suppress.check"));
}
