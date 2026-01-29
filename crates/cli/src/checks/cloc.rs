// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Cloc (count lines of code) check.
//!
//! Validates file size limits per docs/specs/checks/cloc.md.

use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::Ordering;

use globset::GlobSet;
use serde_json::json;

use crate::adapter::glob::build_glob_set;
use crate::adapter::rust::CfgTestInfo;
use crate::adapter::{AdapterRegistry, FileKind, RustAdapter};
use crate::check::{Check, CheckContext, CheckResult, Violation};
use crate::config::{CfgTestSplitMode, CheckLevel, LineMetric};
use crate::file_reader::FileContent;

/// Parameters for creating a line-count violation.
struct LineViolationInfo {
    metric: LineMetric,
    value: usize,
    threshold: usize,
    total_lines: usize,
    nonblank_lines: usize,
}

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
        let packages = &ctx.config.project.packages;

        // NOTE: We don't early-return on global check.cloc.check = "off" because
        // individual languages can override this setting. The per-file check level
        // resolution handles "off" correctly by skipping files with that level.

        // Build adapter registry for file classification
        // Uses config-aware pattern resolution with hierarchy:
        // 1. [<language>].tests - Language-specific override
        // 2. [project].tests - Project-wide patterns
        // 3. Adapter defaults - Built-in convention
        let registry = AdapterRegistry::for_project_with_config(ctx.root, ctx.config);

        // Get Rust config for cfg_test_split
        let rust_config = &ctx.config.rust;
        // Only create adapter for modes that need parsing
        // Uses config-aware patterns for consistent test file classification
        let rust_adapter = match rust_config.cfg_test_split {
            CfgTestSplitMode::Count | CfgTestSplitMode::Require => {
                use crate::config::RustConfig;
                let fallback_test = if !ctx.config.project.tests.is_empty() {
                    ctx.config.project.tests.clone()
                } else {
                    RustConfig::default_tests()
                };
                let patterns = crate::adapter::ResolvedPatterns {
                    source: if !rust_config.source.is_empty() {
                        rust_config.source.clone()
                    } else {
                        RustConfig::default_source()
                    },
                    test: if !rust_config.tests.is_empty() {
                        rust_config.tests.clone()
                    } else {
                        fallback_test
                    },
                    exclude: if !rust_config.exclude.is_empty() {
                        rust_config.exclude.clone()
                    } else {
                        RustConfig::default_exclude()
                    },
                };
                Some(RustAdapter::with_patterns(patterns))
            }
            CfgTestSplitMode::Off => None,
        };

        // Build pattern matcher for exclude patterns only
        let exclude_matcher = ExcludeMatcher::new(&cloc_config.exclude);

        // Track violations along with whether they are errors (true) or warnings (false)
        let mut violation_infos: Vec<(Violation, bool)> = Vec::new();
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

            match count_file_metrics(&file.path) {
                Ok(metrics) => {
                    let total_lines = metrics.lines;
                    let nonblank_lines = metrics.nonblank_lines;
                    let token_count = metrics.tokens;
                    let relative_path = file.path.strip_prefix(ctx.root).unwrap_or(&file.path);
                    let file_kind = registry.classify(relative_path);
                    let is_excluded = exclude_matcher.is_excluded(&file.path, ctx.root);

                    // Check if this is a Rust source file that might have inline tests
                    let is_rust_source = file.path.extension().and_then(|e| e.to_str())
                        == Some("rs")
                        && file_kind == FileKind::Source
                        && rust_adapter.is_some();

                    // Determine source/test line counts
                    let (file_source_lines, file_test_lines, is_test) =
                        if let (true, Some(adapter)) = (is_rust_source, rust_adapter.as_ref()) {
                            // Use line-level classification for Rust source files
                            // (uses mmap for large files per performance spec)
                            let file_content = match FileContent::read(&file.path) {
                                Ok(c) => c,
                                Err(_) => {
                                    // Fallback to whole-file classification on read error
                                    continue;
                                }
                            };
                            let Some(content) = file_content.as_str() else {
                                // Skip non-UTF-8 files
                                continue;
                            };

                            match rust_config.cfg_test_split {
                                CfgTestSplitMode::Require => {
                                    // Check for inline tests and generate violation
                                    // Note: This respects rust.cloc.check level
                                    let rust_check_level =
                                        ctx.config.cloc_check_level_for_language("rust");
                                    if rust_check_level != CheckLevel::Off {
                                        let cfg_info = CfgTestInfo::parse(content);
                                        if cfg_info.has_inline_tests()
                                            && let Some(line) = cfg_info.first_inline_test_line()
                                        {
                                            let is_error = rust_check_level == CheckLevel::Error;
                                            violation_infos.push((
                                                create_inline_cfg_test_violation(
                                                    ctx,
                                                    &file.path,
                                                    line as u32 + 1,
                                                ),
                                                is_error,
                                            ));
                                        }
                                    }
                                    // Still count as source (no splitting)
                                    (nonblank_lines, 0, false)
                                }
                                CfgTestSplitMode::Count => {
                                    // Existing behavior: split source/test
                                    let classification =
                                        adapter.classify_lines(relative_path, content);
                                    // File is considered "test" for size limits if it has more test than source
                                    let is_test =
                                        classification.test_lines > classification.source_lines;
                                    (
                                        classification.source_lines,
                                        classification.test_lines,
                                        is_test,
                                    )
                                }
                                CfgTestSplitMode::Off => unreachable!(), // Adapter is None
                            }
                        } else {
                            // Use whole-file classification
                            let is_test = file_kind == FileKind::Test;
                            if is_test {
                                (0, nonblank_lines, true)
                            } else {
                                (nonblank_lines, 0, false)
                            }
                        };

                    // Accumulate global metrics
                    source_lines += file_source_lines;
                    test_lines += file_test_lines;

                    // File counts: count file in source if any source lines, test if any test lines
                    if file_source_lines > 0 {
                        source_files += 1;
                        source_tokens += token_count * file_source_lines / nonblank_lines.max(1);
                    }
                    if file_test_lines > 0 {
                        test_files += 1;
                        test_tokens += token_count * file_test_lines / nonblank_lines.max(1);
                    }

                    // Accumulate per-package metrics
                    if !packages.is_empty()
                        && let Some(pkg_name) = file_package(&file.path, ctx.root, packages)
                    {
                        let pkg = package_metrics.entry(pkg_name).or_default();
                        pkg.source_lines += file_source_lines;
                        pkg.test_lines += file_test_lines;
                        if file_source_lines > 0 {
                            pkg.source_files += 1;
                            pkg.source_tokens +=
                                token_count * file_source_lines / nonblank_lines.max(1);
                        }
                        if file_test_lines > 0 {
                            pkg.test_files += 1;
                            pkg.test_tokens +=
                                token_count * file_test_lines / nonblank_lines.max(1);
                        }
                    }

                    // Size limit check (skip excluded files)
                    // For files with both source and test lines, check source portion against source limit
                    if !is_excluded {
                        // Get language-specific check level and advice
                        // Use file extension for language detection in mixed-language projects
                        // where only the primary language adapter is registered
                        let adapter_name = registry.adapter_for(relative_path).name();
                        let lang_key = if adapter_name == "generic" {
                            // Fall back to file extension for per-language config lookup
                            file.path
                                .extension()
                                .and_then(|e| e.to_str())
                                .unwrap_or(adapter_name)
                        } else {
                            adapter_name
                        };
                        let lang_check_level = ctx.config.cloc_check_level_for_language(lang_key);

                        // Skip violation generation if this language is disabled
                        if lang_check_level == CheckLevel::Off {
                            continue;
                        }

                        let is_error = lang_check_level == CheckLevel::Error;

                        let max_lines = if is_test {
                            cloc_config.max_lines_test
                        } else {
                            cloc_config.max_lines
                        };

                        // Use configured line metric for size limit check
                        let line_count = match cloc_config.metric {
                            LineMetric::Lines => total_lines,
                            LineMetric::Nonblank => nonblank_lines,
                        };

                        let source_advice =
                            ctx.config.cloc_advice_for_language(lang_key, max_lines);

                        if line_count > max_lines {
                            let info = LineViolationInfo {
                                metric: cloc_config.metric,
                                value: line_count,
                                threshold: max_lines,
                                total_lines,
                                nonblank_lines,
                            };
                            match try_create_line_violation(
                                ctx,
                                &file.path,
                                is_test,
                                &source_advice,
                                &cloc_config.advice_test,
                                &info,
                            ) {
                                Some(v) => violation_infos.push((v, is_error)),
                                None => break,
                            }
                        }

                        // Token limit check
                        if let Some(max_tokens) = cloc_config.max_tokens
                            && token_count > max_tokens
                        {
                            match try_create_token_violation(
                                ctx,
                                &file.path,
                                is_test,
                                &source_advice,
                                &cloc_config.advice_test,
                                token_count,
                                max_tokens,
                            ) {
                                Some(v) => violation_infos.push((v, is_error)),
                                None => break,
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("failed to count lines in {}: {}", file.path.display(), e);
                }
            }
        }

        // Separate errors and warnings, then build result
        let violations: Vec<Violation> = violation_infos.iter().map(|(v, _)| v.clone()).collect();
        let has_errors = violation_infos.iter().any(|(_, is_error)| *is_error);

        let result = if violations.is_empty() {
            CheckResult::passed(self.name())
        } else if has_errors {
            CheckResult::failed(self.name(), violations)
        } else {
            // All violations are warnings - pass but include them for reporting
            CheckResult::passed_with_warnings(self.name(), violations)
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
            let package_names = &ctx.config.project.package_names;
            let by_package: HashMap<String, serde_json::Value> = package_metrics
                .into_iter()
                .map(|(path, metrics)| {
                    // Use package name from mapping if available, otherwise use path
                    let name = package_names.get(&path).cloned().unwrap_or(path);
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
///
/// Uses `.ok()?` for strip_prefix because if the path isn't under root,
/// it semantically cannot belong to any configured package.
fn file_package(path: &Path, root: &Path, packages: &[String]) -> Option<String> {
    let relative = path.strip_prefix(root).ok()?;

    for pkg in packages {
        // Special case: "." means root package (all files belong to it)
        if pkg == "." {
            return Some(pkg.clone());
        }
        // Check if file is under the package directory
        if relative.starts_with(pkg) {
            return Some(pkg.clone());
        }
    }
    None
}

/// Pattern matcher for exclude patterns.
struct ExcludeMatcher {
    exclude_patterns: GlobSet,
}

impl ExcludeMatcher {
    /// Create a new exclude matcher from config patterns.
    fn new(exclude_patterns: &[String]) -> Self {
        Self {
            exclude_patterns: build_glob_set(exclude_patterns),
        }
    }

    /// Check if a file should be excluded from violations.
    fn is_excluded(&self, path: &Path, root: &Path) -> bool {
        let relative = path.strip_prefix(root).unwrap_or(path);
        self.exclude_patterns.is_match(relative)
    }
}

/// Check violation limit and create a line count violation if under the limit.
/// Returns `Some(violation)` if under limit, `None` if limit exceeded.
fn try_create_line_violation(
    ctx: &CheckContext,
    file_path: &Path,
    is_test: bool,
    advice: &str,
    advice_test: &str,
    info: &LineViolationInfo,
) -> Option<Violation> {
    let current = ctx.violation_count.fetch_add(1, Ordering::SeqCst);
    if let Some(limit) = ctx.limit
        && current >= limit
    {
        return None;
    }

    let display_path = file_path.strip_prefix(ctx.root).unwrap_or(file_path);
    let advice = if is_test { advice_test } else { advice };

    // Use violation type that indicates which metric was checked
    let violation_type = match info.metric {
        LineMetric::Lines => "file_too_large",
        LineMetric::Nonblank => "file_too_large_nonblank",
    };

    Some(
        Violation::file_only(display_path, violation_type, advice)
            .with_threshold(info.value as i64, info.threshold as i64)
            .with_line_counts(info.total_lines as i64, info.nonblank_lines as i64),
    )
}

/// Check violation limit and create a token count violation if under the limit.
/// Returns `Some(violation)` if under limit, `None` if limit exceeded.
fn try_create_token_violation(
    ctx: &CheckContext,
    file_path: &Path,
    is_test: bool,
    advice: &str,
    advice_test: &str,
    value: usize,
    threshold: usize,
) -> Option<Violation> {
    let current = ctx.violation_count.fetch_add(1, Ordering::SeqCst);
    if let Some(limit) = ctx.limit
        && current >= limit
    {
        return None;
    }

    let display_path = file_path.strip_prefix(ctx.root).unwrap_or(file_path);
    let advice = if is_test { advice_test } else { advice };

    Some(
        Violation::file_only(display_path, "file_too_large", advice)
            .with_threshold(value as i64, threshold as i64),
    )
}

/// Create a violation for inline #[cfg(test)] block.
fn create_inline_cfg_test_violation(ctx: &CheckContext, file_path: &Path, line: u32) -> Violation {
    let display_path = file_path.strip_prefix(ctx.root).unwrap_or(file_path);
    Violation::file(
        display_path,
        line,
        "inline_cfg_test",
        "Move tests to a sibling _tests.rs file.",
    )
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
        | "sh" | "bash" | "zsh" | "fish" | "bats" | "ps1" | "bat" | "cmd"
        // Web (code only)
        | "vue" | "svelte"
        // SQL
        | "sql"
    )
}

/// Metrics computed from a single file read.
struct FileMetrics {
    /// Total lines (matches `wc -l`), used for size limit thresholds.
    lines: usize,
    /// Non-blank lines (lines with at least one non-whitespace character).
    nonblank_lines: usize,
    tokens: usize,
}

/// Count lines and tokens from a single file read.
/// - `lines`: total line count (matches `wc -l`)
/// - `nonblank_lines`: lines with at least one non-whitespace character
/// - `tokens`: chars/4 approximation (standard LLM heuristic)
fn count_file_metrics(path: &Path) -> std::io::Result<FileMetrics> {
    let content = std::fs::read(path)?;
    // Try UTF-8, fall back to lossy conversion for encoding issues
    let text = String::from_utf8(content)
        .unwrap_or_else(|e| String::from_utf8_lossy(e.as_bytes()).into_owned());

    let lines = text.lines().count();
    let nonblank_lines = text.lines().filter(|l| !l.trim().is_empty()).count();
    let tokens = text.chars().count() / 4;

    Ok(FileMetrics {
        lines,
        nonblank_lines,
        tokens,
    })
}

#[cfg(test)]
#[path = "cloc_tests.rs"]
mod tests;
