//! Behavioral specs for config validation.
//!
//! Tests that quench correctly handles:
//! - Unknown config keys (errors)
//! - Unknown nested keys (errors)
//! - Valid config (no errors)
//!
//! Reference: docs/specs/02-config.md#validation

#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::prelude::*;

// =============================================================================
// CONFIG VALIDATION SPECS
// =============================================================================

/// Spec: docs/specs/02-config.md#validation
///
/// > Unknown keys are errors
#[test]
fn unknown_config_key_fails() {
    let temp = Project::empty();
    temp.config(
        r#"version = 1
unknown_key = true

[check.agents]
required = []
"#,
    );

    quench_cmd()
        .arg("check")
        .current_dir(temp.path())
        .assert()
        .failure()
        .stderr(predicates::str::contains("unknown field"));
}

/// Spec: docs/specs/02-config.md#validation
///
/// > Unknown nested keys are errors
#[test]
fn unknown_nested_config_key_fails() {
    let temp = Project::empty();
    temp.config(&format!(
        r#"{MINIMAL_CONFIG}
[check.unknown]
field = "value"
"#
    ));

    quench_cmd()
        .arg("check")
        .current_dir(temp.path())
        .assert()
        .failure()
        .stderr(predicates::str::contains("unknown field"));
}

/// Spec: docs/specs/02-config.md#validation
///
/// > Valid config produces no errors
#[test]
fn valid_config_no_errors() {
    let temp = Project::empty();
    temp.config(MINIMAL_CONFIG);

    quench_cmd()
        .arg("check")
        .current_dir(temp.path())
        .assert()
        .success()
        .stderr(predicates::str::is_empty());
}

/// Spec: All template files in docs/specs/templates/ must parse as valid configs
///
/// > This ensures all documented template files are syntactically valid
/// > and conform to quench's config schema
#[test]
fn all_template_files_parse_correctly() {
    let templates_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/specs/templates");

    // Read all .toml files in the templates directory
    let entries =
        std::fs::read_dir(templates_dir).expect("docs/specs/templates directory should exist");

    let mut tested_count = 0;
    let mut errors = Vec::new();

    for entry in entries {
        let entry = entry.unwrap();
        let path = entry.path();

        // Only test init.*.toml template files
        if path.extension().is_some_and(|ext| ext == "toml")
            && path
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("init."))
        {
            let file_name = path.file_name().unwrap().to_str().unwrap();
            tested_count += 1;

            // For language-specific templates, combine with base template
            let content = if file_name != "init.default.toml" {
                let base_path = format!("{}/init.default.toml", templates_dir);
                let base =
                    std::fs::read_to_string(&base_path).expect("init.default.toml should exist");
                let lang_template = std::fs::read_to_string(&path)
                    .unwrap_or_else(|e| panic!("Failed to read {}: {}", file_name, e));
                format!("{}\n{}", base, lang_template)
            } else {
                std::fs::read_to_string(&path)
                    .unwrap_or_else(|e| panic!("Failed to read {}: {}", file_name, e))
            };

            // Try to parse it with quench
            let temp = Project::empty();
            temp.config(&content);
            // Add CLAUDE.md to satisfy agents check
            temp.file("CLAUDE.md", "# Test Project\n");

            let output = quench_cmd()
                .arg("check")
                .current_dir(temp.path())
                .output()
                .unwrap_or_else(|e| panic!("Failed to run quench for {}: {}", file_name, e));

            // Only treat as parse error if stderr contains config/parse error messages
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("Failed to parse") || stderr.contains("Invalid configuration") {
                errors.push(format!("{}: Parse failed\n{}", file_name, stderr));
            }
        }
    }

    // Ensure we actually tested some templates
    assert!(
        tested_count >= 7,
        "Expected at least 7 template files, found {}",
        tested_count
    );

    // Report all errors at once
    if !errors.is_empty() {
        panic!(
            "\n\n{} template(s) failed to parse:\n\n{}",
            errors.len(),
            errors.join("\n---\n")
        );
    }
}
