// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Shared test utilities for adapter policy tests.
//!
//! Reduces boilerplate across JS, Go, Rust, and Shell policy tests.

use std::path::Path;

use crate::adapter::FileKind;
use crate::adapter::common::policy::{PolicyCheckResult, PolicyConfig, check_lint_policy};

/// Run policy check with string paths (convenience wrapper).
pub fn check_policy<P: PolicyConfig>(
    files: &[&str],
    policy: &P,
    classify: impl Fn(&Path) -> FileKind,
) -> PolicyCheckResult {
    let paths: Vec<&Path> = files.iter().map(|f| Path::new(*f)).collect();
    check_lint_policy(&paths, policy, classify)
}

/// Assert that a policy check does NOT result in a violation.
pub fn assert_no_violation<P: PolicyConfig>(
    files: &[&str],
    policy: &P,
    classify: impl Fn(&Path) -> FileKind,
) {
    let result = check_policy(files, policy, classify);
    assert!(
        !result.standalone_violated,
        "expected no violation, but got one for files: {:?}",
        files
    );
}

/// Assert that a policy check DOES result in a violation.
pub fn assert_violation<P: PolicyConfig>(
    files: &[&str],
    policy: &P,
    classify: impl Fn(&Path) -> FileKind,
) {
    let result = check_policy(files, policy, classify);
    assert!(result.standalone_violated, "expected violation, but got none for files: {:?}", files);
}

/// Macro to generate standard policy tests for an adapter.
///
/// This generates the core policy tests that all adapters share:
/// - no_violation_source_only: Only source files, no lint config → no violation
/// - no_violation_lint_only: Only lint config, no source → no violation
/// - violation_when_both: Source + lint config → violation
/// - no_violation_when_disabled: Policy disabled → no violation
/// - test_files_trigger_violation: Test files count as source for policy
/// - non_source_ignored: README, Makefile, etc. don't trigger violation
///
/// # Usage
///
/// ```ignore
/// policy_test_cases! {
///     policy_type: JavaScriptPolicyConfig,
///     default_policy: || JavaScriptPolicyConfig {
///         lint_changes: LintChangesPolicy::Standalone,
///         lint_config: vec![".eslintrc".to_string()],
///     },
///     classifier: |path| js_classify(path),
///     source_files: ["src/app.ts", "src/utils.js"],
///     lint_config_file: ".eslintrc",
///     test_file: "src/app.test.ts",
/// }
/// ```
#[macro_export]
macro_rules! policy_test_cases {
    (
        policy_type: $policy_type:ty,
        default_policy: $default_policy:expr,
        classifier: $classifier:expr,
        source_files: [$($src:expr),+ $(,)?],
        lint_config_file: $lint_config:expr,
        test_file: $test_file:expr $(,)?
    ) => {
        #[test]
        fn no_violation_source_only() {
            use $crate::adapter::common::test_utils::assert_no_violation;
            let policy = $default_policy();
            assert_no_violation(&[$($src),+], &policy, $classifier);
        }

        #[test]
        fn no_violation_lint_only() {
            use $crate::adapter::common::test_utils::assert_no_violation;
            let policy = $default_policy();
            assert_no_violation(&[$lint_config], &policy, $classifier);
        }

        #[test]
        fn violation_when_both() {
            use $crate::adapter::common::test_utils::assert_violation;
            let policy = $default_policy();
            let first_src: &str = [$($src),+][0];
            assert_violation(&[$lint_config, first_src], &policy, $classifier);
        }

        #[test]
        fn no_violation_when_disabled() {
            use $crate::adapter::common::test_utils::assert_no_violation;
            use $crate::config::LintChangesPolicy;
            let mut policy = $default_policy();
            policy.lint_changes = LintChangesPolicy::None;
            let first_src: &str = [$($src),+][0];
            assert_no_violation(&[$lint_config, first_src], &policy, $classifier);
        }

        #[test]
        fn test_files_trigger_violation() {
            use $crate::adapter::common::test_utils::assert_violation;
            let policy = $default_policy();
            assert_violation(&[$lint_config, $test_file], &policy, $classifier);
        }

        #[test]
        fn non_source_files_ignored() {
            use $crate::adapter::common::test_utils::assert_no_violation;
            let policy = $default_policy();
            assert_no_violation(&[$lint_config, "README.md", "Cargo.toml"], &policy, $classifier);
        }
    };
}

// Re-export for use in test modules
pub use policy_test_cases;
