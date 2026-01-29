// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

use std::path::Path;

use crate::adapter::FileKind;
use crate::config::{JavaScriptPolicyConfig, LintChangesPolicy};

fn default_policy() -> JavaScriptPolicyConfig {
    JavaScriptPolicyConfig {
        check: None,
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
    if path_str.contains(".test.")
        || path_str.contains(".spec.")
        || path_str.contains("__tests__")
        || path_str.contains("_test.")
        || path_str.contains("_tests.")
        || path_str.starts_with("test_")
    {
        FileKind::Test
    } else if path_str.ends_with(".ts")
        || path_str.ends_with(".js")
        || path_str.ends_with(".tsx")
        || path_str.ends_with(".jsx")
        || path_str.ends_with(".mjs")
        || path_str.ends_with(".mts")
        || path_str.ends_with(".cjs")
        || path_str.ends_with(".cts")
    {
        FileKind::Source
    } else {
        FileKind::Other
    }
}

// Generate standard policy tests
crate::policy_test_cases! {
    policy_type: JavaScriptPolicyConfig,
    default_policy: default_policy,
    classifier: js_classifier,
    source_files: ["src/app.ts", "src/utils.js"],
    lint_config_file: ".eslintrc",
    test_file: "src/app.test.ts",
}

// =============================================================================
// JavaScript-specific tests (not covered by standard pattern)
// =============================================================================

#[test]
fn recognizes_eslint_config_variants() {
    use crate::adapter::common::test_utils::assert_violation;

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

    for config in &[".eslintrc.json", ".eslintrc.yml", "eslint.config.mjs"] {
        assert_violation(&[*config, "src/app.ts"], &policy, js_classifier);
    }
}

#[test]
fn recognizes_biome_config_variants() {
    use crate::adapter::common::test_utils::assert_violation;

    let policy = JavaScriptPolicyConfig {
        lint_config: vec!["biome.json".to_string(), "biome.jsonc".to_string()],
        ..default_policy()
    };

    for config in &["biome.json", "biome.jsonc"] {
        assert_violation(&[*config, "src/app.ts"], &policy, js_classifier);
    }
}

#[test]
fn recognizes_commonjs_extensions() {
    use crate::adapter::common::test_utils::check_policy;

    let policy = default_policy();
    let result = check_policy(
        &["src/config.cjs", "src/types.cts", ".eslintrc"],
        &policy,
        js_classifier,
    );

    assert!(result.standalone_violated);
    assert_eq!(result.changed_source.len(), 2);
}
