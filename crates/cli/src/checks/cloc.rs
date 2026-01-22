//! Cloc (count lines of code) check.
//!
//! Validates file size limits per docs/specs/checks/cloc.md.

use std::io::BufRead;
use std::path::Path;
use std::sync::atomic::Ordering;

use serde_json::json;

use crate::check::{Check, CheckContext, CheckResult, Violation};
use crate::config::CheckLevel;

/// The cloc check validates file size limits.
pub struct ClocCheck;

impl Check for ClocCheck {
    fn name(&self) -> &'static str {
        "cloc"
    }

    fn description(&self) -> &'static str {
        "Lines of code and file size limits"
    }

    fn run(&self, ctx: &CheckContext) -> CheckResult {
        let cloc_config = &ctx.config.check.cloc;

        // Skip if disabled
        if cloc_config.check == CheckLevel::Off {
            return CheckResult::passed(self.name());
        }

        let mut violations = Vec::new();
        let mut source_lines: usize = 0;
        let mut test_lines: usize = 0;

        for file in ctx.files {
            // Skip non-text files
            if !is_text_file(&file.path) {
                continue;
            }

            match count_lines(&file.path) {
                Ok(line_count) => {
                    let is_test = is_test_file(&file.path);

                    // Accumulate metrics
                    if is_test {
                        test_lines += line_count;
                    } else {
                        source_lines += line_count;
                    }

                    let max_lines = if is_test {
                        cloc_config.max_lines_test
                    } else {
                        cloc_config.max_lines
                    };

                    if line_count > max_lines {
                        // Check violation limit
                        let current = ctx.violation_count.fetch_add(1, Ordering::SeqCst);
                        if let Some(limit) = ctx.limit
                            && current >= limit
                        {
                            break;
                        }

                        let display_path = file.path.strip_prefix(ctx.root).unwrap_or(&file.path);
                        violations.push(
                            Violation::file_only(
                                display_path,
                                "file_too_large",
                                format!(
                                    "Split into smaller modules. {} lines exceeds {} line limit.",
                                    line_count, max_lines
                                ),
                            )
                            .with_threshold(line_count as i64, max_lines as i64),
                        );
                    }
                }
                Err(e) => {
                    tracing::warn!("failed to count lines in {}: {}", file.path.display(), e);
                }
            }
        }

        let result = if violations.is_empty() {
            CheckResult::passed(self.name())
        } else {
            CheckResult::failed(self.name(), violations)
        };

        // Add metrics
        let ratio = if source_lines > 0 {
            test_lines as f64 / source_lines as f64
        } else {
            0.0
        };

        result.with_metrics(json!({
            "source_lines": source_lines,
            "test_lines": test_lines,
            "ratio": (ratio * 100.0).round() / 100.0,
        }))
    }

    fn default_enabled(&self) -> bool {
        true
    }
}

/// Check if a file appears to be a text file (not binary).
fn is_text_file(path: &Path) -> bool {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    matches!(
        ext.as_str(),
        "rs" | "py"
            | "js"
            | "ts"
            | "jsx"
            | "tsx"
            | "go"
            | "c"
            | "cpp"
            | "h"
            | "hpp"
            | "java"
            | "kt"
            | "scala"
            | "rb"
            | "php"
            | "cs"
            | "swift"
            | "m"
            | "mm"
            | "sh"
            | "bash"
            | "zsh"
            | "fish"
            | "ps1"
            | "bat"
            | "cmd"
            | "lua"
            | "pl"
            | "pm"
            | "r"
            | "sql"
            | "md"
            | "txt"
            | "toml"
            | "yaml"
            | "yml"
            | "json"
            | "xml"
            | "html"
            | "css"
            | "scss"
            | "sass"
            | "less"
            | "vue"
            | "svelte"
    )
}

/// Check if a file is a test file based on its filename.
fn is_test_file(path: &Path) -> bool {
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .map(|s| s.to_lowercase())
        .unwrap_or_default();

    file_name.contains("_test.")
        || file_name.contains("_tests.")
        || file_name.contains(".test.")
        || file_name.contains(".spec.")
        || file_name.ends_with("_test.rs")
        || file_name.ends_with("_tests.rs")
        || file_name.ends_with("_test.go")
        || file_name.ends_with("_test.py")
        || file_name.ends_with(".test.js")
        || file_name.ends_with(".test.ts")
        || file_name.ends_with(".test.tsx")
        || file_name.ends_with(".spec.js")
        || file_name.ends_with(".spec.ts")
        || file_name.ends_with(".spec.tsx")
}

/// Count the number of lines in a file.
fn count_lines(path: &Path) -> std::io::Result<usize> {
    let file = std::fs::File::open(path)?;
    let reader = std::io::BufReader::new(file);
    Ok(reader.lines().count())
}

#[cfg(test)]
#[path = "cloc_tests.rs"]
mod tests;
