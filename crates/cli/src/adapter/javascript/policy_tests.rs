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
    if path_str.contains(".test.") || path_str.contains(".spec.") || path_str.contains("__tests__")
    {
        FileKind::Test
    } else if path_str.ends_with(".ts")
        || path_str.ends_with(".js")
        || path_str.ends_with(".tsx")
        || path_str.ends_with(".jsx")
    {
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
        assert!(
            result.standalone_violated,
            "Expected violation for {}",
            config
        );
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
        assert!(
            result.standalone_violated,
            "Expected violation for {}",
            config
        );
    }
}
