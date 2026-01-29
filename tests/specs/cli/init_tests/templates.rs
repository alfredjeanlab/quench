// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Template matching and output format specs.

use crate::prelude::*;

// =============================================================================
// Output Format Specs
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

/// Spec: Enforce quench init matches docs/specs/templates/init.default.toml
///
/// > This test ensures the default output of quench init exactly matches
/// > the documented spec template file.
#[test]
fn init_default_output_matches_spec_template() {
    let temp = Project::empty();

    quench_cmd()
        .args(["init"])
        .current_dir(temp.path())
        .assert()
        .success();

    let generated = std::fs::read_to_string(temp.path().join("quench.toml")).unwrap();

    // Read the spec template file (relative to workspace root)
    let spec_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../docs/specs/templates/init.default.toml"
    );
    let spec_template = std::fs::read_to_string(spec_path)
        .expect("docs/specs/templates/init.default.toml should exist");

    // Normalize whitespace: trim lines and overall content
    let normalize = |s: &str| {
        s.lines()
            .map(|line| line.trim_end())
            .collect::<Vec<_>>()
            .join("\n")
            .trim()
            .to_string()
    };

    let generated_normalized = normalize(&generated);
    let spec_normalized = normalize(&spec_template);

    assert_eq!(
        generated_normalized, spec_normalized,
        "\n\nquench init output does not match docs/specs/templates/init.default.toml\n\
         \n\
         The spec is the source of truth. Update the code in profiles.rs\n\
         (default_template_base/suffix functions) to match the spec.\n\
         \n\
         Generated:\n{}\n\n\
         Spec file:\n{}\n",
        generated_normalized, spec_normalized
    );
}

/// Spec: Enforce quench init --with rust matches docs/specs/templates/init.rust.toml
///
/// > This test ensures the Rust profile output exactly matches the spec template.
#[test]
fn init_rust_profile_matches_spec_template() {
    let temp = Project::empty();

    quench_cmd()
        .args(["init", "--with", "rust"])
        .current_dir(temp.path())
        .assert()
        .success();

    let generated = std::fs::read_to_string(temp.path().join("quench.toml")).unwrap();

    // Read base template and language-specific template
    let base_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../docs/specs/templates/init.default.toml"
    );
    let lang_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../docs/specs/templates/init.rust.toml"
    );

    let base_template = std::fs::read_to_string(base_path)
        .expect("docs/specs/templates/init.default.toml should exist");
    let lang_template = std::fs::read_to_string(lang_path)
        .expect("docs/specs/templates/init.rust.toml should exist");

    let expected = format!("{}\n{}", base_template, lang_template);

    // Normalize whitespace: trim lines and overall content
    let normalize = |s: &str| {
        s.lines()
            .map(|line| line.trim_end())
            .collect::<Vec<_>>()
            .join("\n")
            .trim()
            .to_string()
    };

    let generated_normalized = normalize(&generated);
    let expected_normalized = normalize(&expected);

    assert_eq!(
        generated_normalized, expected_normalized,
        "\n\nquench init --with rust output does not match spec templates\n\
         \n\
         The spec is the source of truth. Update rust_profile_defaults() in profiles.rs\n\
         to match docs/specs/templates/init.rust.toml.\n\
         \n\
         Generated:\n{}\n\n\
         Expected:\n{}\n",
        generated_normalized, expected_normalized
    );
}

/// Spec: Enforce quench init --with golang matches docs/specs/templates/init.golang.toml
///
/// > This test ensures the Go profile output exactly matches the spec template.
#[test]
fn init_golang_profile_matches_spec_template() {
    let temp = Project::empty();

    quench_cmd()
        .args(["init", "--with", "golang"])
        .current_dir(temp.path())
        .assert()
        .success();

    let generated = std::fs::read_to_string(temp.path().join("quench.toml")).unwrap();

    let base_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../docs/specs/templates/init.default.toml"
    );
    let lang_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../docs/specs/templates/init.golang.toml"
    );

    let base_template = std::fs::read_to_string(base_path)
        .expect("docs/specs/templates/init.default.toml should exist");
    let lang_template = std::fs::read_to_string(lang_path)
        .expect("docs/specs/templates/init.golang.toml should exist");

    let expected = format!("{}\n{}", base_template, lang_template);

    let normalize = |s: &str| {
        s.lines()
            .map(|line| line.trim_end())
            .collect::<Vec<_>>()
            .join("\n")
            .trim()
            .to_string()
    };

    let generated_normalized = normalize(&generated);
    let expected_normalized = normalize(&expected);

    assert_eq!(
        generated_normalized, expected_normalized,
        "\n\nquench init --with golang output does not match spec templates\n\
         \n\
         The spec is the source of truth. Update golang_profile_defaults() in profiles.rs\n\
         to match docs/specs/templates/init.golang.toml.\n\
         \n\
         Generated:\n{}\n\n\
         Expected:\n{}\n",
        generated_normalized, expected_normalized
    );
}

/// Spec: Enforce quench init --with javascript matches docs/specs/templates/init.javascript.toml
///
/// > This test ensures the JavaScript profile output exactly matches the spec template.
#[test]
fn init_javascript_profile_matches_spec_template() {
    let temp = Project::empty();

    quench_cmd()
        .args(["init", "--with", "javascript"])
        .current_dir(temp.path())
        .assert()
        .success();

    let generated = std::fs::read_to_string(temp.path().join("quench.toml")).unwrap();

    let base_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../docs/specs/templates/init.default.toml"
    );
    let lang_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../docs/specs/templates/init.javascript.toml"
    );

    let base_template = std::fs::read_to_string(base_path)
        .expect("docs/specs/templates/init.default.toml should exist");
    let lang_template = std::fs::read_to_string(lang_path)
        .expect("docs/specs/templates/init.javascript.toml should exist");

    let expected = format!("{}\n{}", base_template, lang_template);

    let normalize = |s: &str| {
        s.lines()
            .map(|line| line.trim_end())
            .collect::<Vec<_>>()
            .join("\n")
            .trim()
            .to_string()
    };

    let generated_normalized = normalize(&generated);
    let expected_normalized = normalize(&expected);

    assert_eq!(
        generated_normalized, expected_normalized,
        "\n\nquench init --with javascript output does not match spec templates\n\
         \n\
         The spec is the source of truth. Update javascript_profile_defaults() in profiles.rs\n\
         to match docs/specs/templates/init.javascript.toml.\n\
         \n\
         Generated:\n{}\n\n\
         Expected:\n{}\n",
        generated_normalized, expected_normalized
    );
}

/// Spec: Enforce quench init --with shell matches docs/specs/templates/init.shell.toml
///
/// > This test ensures the Shell profile output exactly matches the spec template.
#[test]
fn init_shell_profile_matches_spec_template() {
    let temp = Project::empty();

    quench_cmd()
        .args(["init", "--with", "shell"])
        .current_dir(temp.path())
        .assert()
        .success();

    let generated = std::fs::read_to_string(temp.path().join("quench.toml")).unwrap();

    let base_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../docs/specs/templates/init.default.toml"
    );
    let lang_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../docs/specs/templates/init.shell.toml"
    );

    let base_template = std::fs::read_to_string(base_path)
        .expect("docs/specs/templates/init.default.toml should exist");
    let lang_template = std::fs::read_to_string(lang_path)
        .expect("docs/specs/templates/init.shell.toml should exist");

    let expected = format!("{}\n{}", base_template, lang_template);

    let normalize = |s: &str| {
        s.lines()
            .map(|line| line.trim_end())
            .collect::<Vec<_>>()
            .join("\n")
            .trim()
            .to_string()
    };

    let generated_normalized = normalize(&generated);
    let expected_normalized = normalize(&expected);

    assert_eq!(
        generated_normalized, expected_normalized,
        "\n\nquench init --with shell output does not match spec templates\n\
         \n\
         The spec is the source of truth. Update shell_profile_defaults() in profiles.rs\n\
         to match docs/specs/templates/init.shell.toml.\n\
         \n\
         Generated:\n{}\n\n\
         Expected:\n{}\n",
        generated_normalized, expected_normalized
    );
}

/// Spec: Enforce quench init --with ruby matches docs/specs/templates/init.ruby.toml
///
/// > This test ensures the Ruby profile output exactly matches the spec template.
#[test]
fn init_ruby_profile_matches_spec_template() {
    let temp = Project::empty();

    quench_cmd()
        .args(["init", "--with", "ruby"])
        .current_dir(temp.path())
        .assert()
        .success();

    let generated = std::fs::read_to_string(temp.path().join("quench.toml")).unwrap();

    let base_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../docs/specs/templates/init.default.toml"
    );
    let lang_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../docs/specs/templates/init.ruby.toml"
    );

    let base_template = std::fs::read_to_string(base_path)
        .expect("docs/specs/templates/init.default.toml should exist");
    let lang_template = std::fs::read_to_string(lang_path)
        .expect("docs/specs/templates/init.ruby.toml should exist");

    let expected = format!("{}\n{}", base_template, lang_template);

    let normalize = |s: &str| {
        s.lines()
            .map(|line| line.trim_end())
            .collect::<Vec<_>>()
            .join("\n")
            .trim()
            .to_string()
    };

    let generated_normalized = normalize(&generated);
    let expected_normalized = normalize(&expected);

    assert_eq!(
        generated_normalized, expected_normalized,
        "\n\nquench init --with ruby output does not match spec templates\n\
         \n\
         The spec is the source of truth. Update ruby_profile_defaults() in profiles.rs\n\
         to match docs/specs/templates/init.ruby.toml.\n\
         \n\
         Generated:\n{}\n\n\
         Expected:\n{}\n",
        generated_normalized, expected_normalized
    );
}

/// Spec: Enforce quench init --with python matches docs/specs/templates/init.python.toml
///
/// > This test ensures the Python profile output exactly matches the spec template.
#[test]
fn init_python_profile_matches_spec_template() {
    let temp = Project::empty();

    quench_cmd()
        .args(["init", "--with", "python"])
        .current_dir(temp.path())
        .assert()
        .success();

    let generated = std::fs::read_to_string(temp.path().join("quench.toml")).unwrap();

    let base_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../docs/specs/templates/init.default.toml"
    );
    let lang_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../docs/specs/templates/init.python.toml"
    );

    let base_template = std::fs::read_to_string(base_path)
        .expect("docs/specs/templates/init.default.toml should exist");
    let lang_template = std::fs::read_to_string(lang_path)
        .expect("docs/specs/templates/init.python.toml should exist");

    let expected = format!("{}\n{}", base_template, lang_template);

    let normalize = |s: &str| {
        s.lines()
            .map(|line| line.trim_end())
            .collect::<Vec<_>>()
            .join("\n")
            .trim()
            .to_string()
    };

    let generated_normalized = normalize(&generated);
    let expected_normalized = normalize(&expected);

    assert_eq!(
        generated_normalized, expected_normalized,
        "\n\nquench init --with python output does not match spec templates\n\
         \n\
         The spec is the source of truth. Update python_profile_defaults() in profiles.rs\n\
         to match docs/specs/templates/init.python.toml.\n\
         \n\
         Generated:\n{}\n\n\
         Expected:\n{}\n",
        generated_normalized, expected_normalized
    );
}
