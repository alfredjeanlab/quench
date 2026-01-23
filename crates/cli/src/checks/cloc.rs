//! Cloc (count lines of code) check.
//!
//! Validates file size limits per docs/specs/checks/cloc.md.

use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::Ordering;

use globset::{Glob, GlobSet, GlobSetBuilder};
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
        let packages = &ctx.config.workspace.packages;

        // Skip if disabled
        if cloc_config.check == CheckLevel::Off {
            return CheckResult::passed(self.name());
        }

        // Build pattern matchers
        let matcher = PatternMatcher::new(&cloc_config.test_patterns, &cloc_config.exclude);

        let mut violations = Vec::new();
        let mut source_lines: usize = 0;
        let mut source_files: usize = 0;
        let mut source_tokens: usize = 0;
        let mut test_lines: usize = 0;
        let mut test_files: usize = 0;
        let mut test_tokens: usize = 0;

        // Per-package metrics (only tracked if packages are configured)
        let mut package_metrics: HashMap<String, PackageMetrics> = HashMap::new();

        for file in ctx.files {
            // Skip non-text files
            if !is_text_file(&file.path) {
                continue;
            }

            match count_nonblank_lines(&file.path) {
                Ok(line_count) => {
                    let is_test = matcher.is_test_file(&file.path, ctx.root);
                    let is_excluded = matcher.is_excluded(&file.path, ctx.root);
                    let token_count = count_tokens(&file.path).unwrap_or(0);

                    // Accumulate global metrics
                    if is_test {
                        test_lines += line_count;
                        test_files += 1;
                        test_tokens += token_count;
                    } else {
                        source_lines += line_count;
                        source_files += 1;
                        source_tokens += token_count;
                    }

                    // Accumulate per-package metrics
                    if !packages.is_empty()
                        && let Some(pkg_name) = file_package(&file.path, ctx.root, packages)
                    {
                        let pkg = package_metrics.entry(pkg_name).or_default();
                        if is_test {
                            pkg.test_lines += line_count;
                            pkg.test_files += 1;
                            pkg.test_tokens += token_count;
                        } else {
                            pkg.source_lines += line_count;
                            pkg.source_files += 1;
                            pkg.source_tokens += token_count;
                        }
                    }

                    // Size limit check (skip excluded files)
                    if !is_excluded {
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

                            let display_path =
                                file.path.strip_prefix(ctx.root).unwrap_or(&file.path);
                            let advice = if is_test {
                                cloc_config.advice_test.clone()
                            } else {
                                cloc_config.advice.clone()
                            };
                            violations.push(
                                Violation::file_only(display_path, "file_too_large", advice)
                                    .with_threshold(line_count as i64, max_lines as i64),
                            );
                        }

                        // Token limit check
                        if let Some(max_tokens) = cloc_config.max_tokens
                            && token_count > max_tokens
                        {
                            let current = ctx.violation_count.fetch_add(1, Ordering::SeqCst);
                            if let Some(limit) = ctx.limit
                                && current >= limit
                            {
                                break;
                            }

                            let display_path =
                                file.path.strip_prefix(ctx.root).unwrap_or(&file.path);
                            let advice = if is_test {
                                cloc_config.advice_test.clone()
                            } else {
                                cloc_config.advice.clone()
                            };
                            violations.push(
                                Violation::file_only(display_path, "file_too_large", advice)
                                    .with_threshold(token_count as i64, max_tokens as i64),
                            );
                        }
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

        // Calculate ratio
        let ratio = if source_lines > 0 {
            test_lines as f64 / source_lines as f64
        } else {
            0.0
        };

        let result = result.with_metrics(json!({
            "source_lines": source_lines,
            "source_files": source_files,
            "source_tokens": source_tokens,
            "test_lines": test_lines,
            "test_files": test_files,
            "test_tokens": test_tokens,
            "ratio": (ratio * 100.0).round() / 100.0,
        }));

        // Add per-package metrics if packages are configured
        if !package_metrics.is_empty() {
            let by_package: HashMap<String, serde_json::Value> = package_metrics
                .into_iter()
                .map(|(name, metrics)| {
                    let ratio = metrics.ratio();
                    (
                        name,
                        json!({
                            "source_lines": metrics.source_lines,
                            "source_files": metrics.source_files,
                            "source_tokens": metrics.source_tokens,
                            "test_lines": metrics.test_lines,
                            "test_files": metrics.test_files,
                            "test_tokens": metrics.test_tokens,
                            "ratio": (ratio * 100.0).round() / 100.0,
                        }),
                    )
                })
                .collect();
            result.with_by_package(by_package)
        } else {
            result
        }
    }

    fn default_enabled(&self) -> bool {
        true
    }
}

/// Per-package metrics.
#[derive(Default)]
struct PackageMetrics {
    source_lines: usize,
    source_files: usize,
    source_tokens: usize,
    test_lines: usize,
    test_files: usize,
    test_tokens: usize,
}

impl PackageMetrics {
    fn ratio(&self) -> f64 {
        if self.source_lines > 0 {
            self.test_lines as f64 / self.source_lines as f64
        } else {
            0.0
        }
    }
}

/// Determine which package a file belongs to.
fn file_package(path: &Path, root: &Path, packages: &[String]) -> Option<String> {
    let relative = path.strip_prefix(root).ok()?;

    for pkg in packages {
        // Check if file is under the package directory
        if relative.starts_with(pkg) {
            return Some(pkg.clone());
        }
    }
    None
}

/// Pattern matcher for test file and exclude patterns.
struct PatternMatcher {
    test_patterns: GlobSet,
    exclude_patterns: GlobSet,
}

impl PatternMatcher {
    /// Create a new pattern matcher from config patterns.
    fn new(test_patterns: &[String], exclude_patterns: &[String]) -> Self {
        Self {
            test_patterns: build_glob_set(test_patterns),
            exclude_patterns: build_glob_set(exclude_patterns),
        }
    }

    /// Check if a file matches test patterns.
    fn is_test_file(&self, path: &Path, root: &Path) -> bool {
        let relative = path.strip_prefix(root).unwrap_or(path);
        self.test_patterns.is_match(relative)
    }

    /// Check if a file should be excluded from violations.
    fn is_excluded(&self, path: &Path, root: &Path) -> bool {
        let relative = path.strip_prefix(root).unwrap_or(path);
        self.exclude_patterns.is_match(relative)
    }
}

/// Build a GlobSet from pattern strings.
fn build_glob_set(patterns: &[String]) -> GlobSet {
    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        if let Ok(glob) = Glob::new(pattern) {
            builder.add(glob);
        } else {
            tracing::warn!("invalid glob pattern: {}", pattern);
        }
    }
    builder.build().unwrap_or_else(|_| GlobSet::empty())
}

/// Check if a file is a source code file (for LOC counting).
/// Excludes configuration files, documentation, and data files.
fn is_text_file(path: &Path) -> bool {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    matches!(
        ext.as_str(),
        // Systems languages
        "rs" | "c" | "cpp" | "h" | "hpp" | "go"
        // JVM languages
        | "java" | "kt" | "scala"
        // Dynamic languages
        | "py" | "rb" | "php" | "lua" | "pl" | "pm" | "r"
        // JavaScript/TypeScript
        | "js" | "ts" | "jsx" | "tsx"
        // Apple platforms
        | "swift" | "m" | "mm"
        // .NET
        | "cs"
        // Shell scripts
        | "sh" | "bash" | "zsh" | "fish" | "ps1" | "bat" | "cmd"
        // Web (code only)
        | "vue" | "svelte"
        // SQL
        | "sql"
    )
}

/// Count non-blank lines in a file.
/// A line is counted if it contains at least one non-whitespace character.
fn count_nonblank_lines(path: &Path) -> std::io::Result<usize> {
    let content = std::fs::read(path)?;
    // Try UTF-8, fall back to lossy conversion for encoding issues
    let text = String::from_utf8(content)
        .unwrap_or_else(|e| String::from_utf8_lossy(e.as_bytes()).into_owned());

    Ok(text.lines().filter(|l| !l.trim().is_empty()).count())
}

/// Count tokens in a file using chars/4 approximation.
/// This matches typical LLM tokenization behavior.
fn count_tokens(path: &Path) -> std::io::Result<usize> {
    let content = std::fs::read(path)?;
    let text = String::from_utf8(content)
        .unwrap_or_else(|e| String::from_utf8_lossy(e.as_bytes()).into_owned());

    // chars / 4 approximation (standard LLM heuristic)
    Ok(text.chars().count() / 4)
}

#[cfg(test)]
#[path = "cloc_tests.rs"]
mod tests;
