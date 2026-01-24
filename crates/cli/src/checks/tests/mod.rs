// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Tests check implementation.
//!
//! Reference: docs/specs/checks/tests.md

pub mod correlation;
pub mod diff;

#[cfg(test)]
#[path = "mod_tests.rs"]
mod unit_tests;

use std::sync::Arc;

use serde_json::json;

use crate::check::{Check, CheckContext, CheckResult, Violation};
use crate::config::TestsCommitConfig;

use self::correlation::{
    CorrelationConfig, analyze_correlation, has_inline_test_changes, has_placeholder_test,
};
use self::diff::{get_base_changes, get_staged_changes};

pub struct TestsCheck;

impl TestsCheck {
    pub fn new() -> Arc<Self> {
        Arc::new(Self)
    }
}

impl Check for TestsCheck {
    fn name(&self) -> &'static str {
        "tests"
    }

    fn description(&self) -> &'static str {
        "Test correlation"
    }

    fn run(&self, ctx: &CheckContext) -> CheckResult {
        let config = &ctx.config.check.tests.commit;

        // Skip if disabled
        if config.check == "off" {
            return CheckResult::passed(self.name());
        }

        // Need either --staged or --base for change detection
        let changes = if ctx.staged {
            match get_staged_changes(ctx.root) {
                Ok(c) => c,
                Err(e) => return CheckResult::skipped(self.name(), e),
            }
        } else if let Some(base) = ctx.base_branch {
            match get_base_changes(ctx.root, base) {
                Ok(c) => c,
                Err(e) => return CheckResult::skipped(self.name(), e),
            }
        } else {
            // No change context available - pass silently
            return CheckResult::passed(self.name());
        };

        // Build correlation config from user settings
        let correlation_config = build_correlation_config(config);

        // Analyze correlation
        let mut result = analyze_correlation(&changes, &correlation_config, ctx.root);

        // Check for inline test changes in Rust files
        let base_ref = if ctx.staged { None } else { ctx.base_branch };
        let allow_placeholders = config.placeholders == "allow";

        result.without_tests.retain(|path| {
            // If the file has inline test changes, move it to with_tests
            if path.extension().is_some_and(|e| e == "rs")
                && has_inline_test_changes(path, ctx.root, base_ref)
            {
                return false; // Remove from without_tests
            }

            // If placeholders are allowed, check for placeholder tests
            if allow_placeholders && let Some(base_name) = path.file_stem().and_then(|s| s.to_str())
            {
                // Check common test file locations for placeholders
                let test_paths = [
                    format!("tests/{}_tests.rs", base_name),
                    format!("tests/{}_test.rs", base_name),
                    format!("tests/{}.rs", base_name),
                    format!("test/{}_tests.rs", base_name),
                    format!("test/{}.rs", base_name),
                ];

                for test_path in &test_paths {
                    let test_file = std::path::Path::new(test_path);
                    if ctx.root.join(test_file).exists()
                        && has_placeholder_test(test_file, base_name, ctx.root).unwrap_or(false)
                    {
                        return false; // Placeholder test satisfies requirement
                    }
                }
            }

            true // Keep in without_tests
        });

        // Build violations for source files without tests
        let mut violations = Vec::new();
        for path in &result.without_tests {
            let change = changes
                .iter()
                .find(|c| c.path.strip_prefix(ctx.root).unwrap_or(&c.path).eq(path));

            let file_stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("file");

            let advice = format!(
                "Add tests in tests/{}_tests.rs or update inline #[cfg(test)] block",
                file_stem
            );

            let mut v = Violation::file_only(path, "missing_tests", advice);

            if let Some(c) = change {
                v.lines = Some(c.lines_changed() as i64);
            }

            violations.push(v);

            if ctx.limit.is_some_and(|l| violations.len() >= l) {
                break;
            }
        }

        // Build metrics
        let metrics = json!({
            "source_files_changed": result.with_tests.len() + result.without_tests.len(),
            "with_test_changes": result.with_tests.len(),
            "without_test_changes": result.without_tests.len(),
            "scope": config.scope,
        });

        if violations.is_empty() {
            CheckResult::passed(self.name()).with_metrics(metrics)
        } else if config.check == "warn" {
            CheckResult::passed_with_warnings(self.name(), violations).with_metrics(metrics)
        } else {
            CheckResult::failed(self.name(), violations).with_metrics(metrics)
        }
    }

    fn default_enabled(&self) -> bool {
        true
    }
}

fn build_correlation_config(config: &TestsCommitConfig) -> CorrelationConfig {
    CorrelationConfig {
        test_patterns: config.test_patterns.clone(),
        source_patterns: config.source_patterns.clone(),
        exclude_patterns: config.exclude.clone(),
    }
}
