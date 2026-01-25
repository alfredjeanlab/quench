// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Placeholders check: detects placeholder tests that need implementation.
//!
//! Detects patterns like:
//! - Rust: `#[ignore]` attribute on tests, `todo!()` macro in test bodies
//! - JavaScript/TypeScript: `test.todo()`, `it.todo()`, `test.fixme()`, `it.fixme()`

pub mod javascript;
pub mod rust;

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;

use std::path::Path;
use std::sync::atomic::Ordering;

use serde_json::json;

use crate::adapter::{Adapter, FileKind, GenericAdapter};
use crate::check::{Check, CheckContext, CheckResult, Violation};
use crate::config::CheckLevel;
use crate::file_reader::FileContent;

/// Placeholders check implementation.
pub struct PlaceholdersCheck;

impl Check for PlaceholdersCheck {
    fn name(&self) -> &'static str {
        "placeholders"
    }

    fn description(&self) -> &'static str {
        "Placeholder test detection"
    }

    fn run(&self, ctx: &CheckContext) -> CheckResult {
        let config = &ctx.config.check.placeholders;

        if config.check == CheckLevel::Off {
            return CheckResult::passed(self.name());
        }

        // Build file adapter for test file detection
        let test_patterns = if ctx.config.project.tests.is_empty() {
            default_test_patterns()
        } else {
            ctx.config.project.tests.clone()
        };
        let file_adapter = GenericAdapter::new(&[], &test_patterns);

        let mut violations = Vec::new();
        let mut metrics = Metrics::default();

        for file in ctx.files {
            // Only check test files
            let rel_path = file.path.strip_prefix(ctx.root).unwrap_or(&file.path);
            if file_adapter.classify(rel_path) != FileKind::Test {
                continue;
            }

            // Read file content (uses mmap for large files per performance spec)
            let file_content = match FileContent::read(&file.path) {
                Ok(c) => c,
                Err(_) => continue,
            };
            let Some(content) = file_content.as_str() else {
                continue; // Skip non-UTF-8 files
            };

            // Detect based on file extension
            let ext = file.path.extension().and_then(|e| e.to_str()).unwrap_or("");

            match ext {
                "rs" => {
                    let placeholders = rust::find_rust_placeholders(content, &config.patterns.rust);

                    for p in placeholders {
                        metrics.increment_rust(p.kind);

                        let advice =
                            format!("Implement test `{}` or remove placeholder.", p.test_name);

                        if let Some(v) =
                            try_create_violation(ctx, rel_path, p.line, p.kind.as_str(), &advice)
                        {
                            violations.push(v);
                        } else {
                            break; // Limit reached
                        }
                    }
                }
                "js" | "jsx" | "ts" | "tsx" | "mjs" | "mts" => {
                    let placeholders =
                        javascript::find_js_placeholders(content, &config.patterns.javascript);

                    for p in placeholders {
                        metrics.increment_js(p.kind);

                        let advice = format!(
                            "Implement test \"{}\" or remove placeholder.",
                            p.description
                        );

                        if let Some(v) =
                            try_create_violation(ctx, rel_path, p.line, p.kind.as_str(), &advice)
                        {
                            violations.push(v);
                        } else {
                            break; // Limit reached
                        }
                    }
                }
                _ => {}
            }

            // Check violation limit
            if ctx.limit.is_some_and(|l| violations.len() >= l) {
                break;
            }
        }

        let result = if violations.is_empty() {
            CheckResult::passed(self.name())
        } else if config.check == CheckLevel::Warn {
            CheckResult::passed_with_warnings(self.name(), violations)
        } else {
            CheckResult::failed(self.name(), violations)
        };

        result.with_metrics(metrics.to_json())
    }

    fn default_enabled(&self) -> bool {
        false // Disabled by default
    }
}

fn try_create_violation(
    ctx: &CheckContext,
    path: &Path,
    line: u32,
    violation_type: &str,
    advice: &str,
) -> Option<Violation> {
    let current = ctx.violation_count.fetch_add(1, Ordering::SeqCst);
    if ctx.limit.is_some_and(|l| current >= l) {
        return None;
    }

    Some(Violation::file(path, line, violation_type, advice))
}

fn default_test_patterns() -> Vec<String> {
    vec![
        "**/tests/**".to_string(),
        "**/test/**".to_string(),
        "**/*_test.*".to_string(),
        "**/*_tests.*".to_string(),
        "**/*.test.*".to_string(),
        "**/*.spec.*".to_string(),
    ]
}

#[derive(Default)]
struct Metrics {
    rust_ignore: usize,
    rust_todo: usize,
    js_todo: usize,
    js_fixme: usize,
}

impl Metrics {
    fn increment_rust(&mut self, kind: rust::RustPlaceholderKind) {
        match kind {
            rust::RustPlaceholderKind::Ignore => self.rust_ignore += 1,
            rust::RustPlaceholderKind::Todo => self.rust_todo += 1,
        }
    }

    fn increment_js(&mut self, kind: javascript::JsPlaceholderKind) {
        match kind {
            javascript::JsPlaceholderKind::Todo => self.js_todo += 1,
            javascript::JsPlaceholderKind::Fixme => self.js_fixme += 1,
            javascript::JsPlaceholderKind::Skip => {} // Not counted separately
        }
    }

    fn to_json(&self) -> serde_json::Value {
        json!({
            "rust": {
                "ignore": self.rust_ignore,
                "todo": self.rust_todo,
            },
            "javascript": {
                "todo": self.js_todo,
                "fixme": self.js_fixme,
            }
        })
    }
}
