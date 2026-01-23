//! Escapes (escape hatches) check.
//!
//! Detects patterns that bypass type safety or error handling.
//! See docs/specs/checks/escape-hatches.md.

use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::Ordering;

use serde_json::{Value as JsonValue, json};

use crate::adapter::{FileKind, GenericAdapter};
use crate::check::{Check, CheckContext, CheckResult, Violation};
use crate::config::{CheckLevel, EscapeAction, EscapesConfig};
use crate::pattern::{CompiledPattern, PatternError};

/// Compiled escape pattern ready for matching.
struct CompiledEscapePattern {
    name: String,
    matcher: CompiledPattern,
    action: EscapeAction,
    advice: String,
    /// Required comment pattern for action = comment.
    comment: Option<String>,
    /// Count threshold for action = count (default: 0).
    threshold: usize,
}

/// Metrics tracked during escapes check.
#[derive(Default)]
struct EscapesMetrics {
    /// Counts per pattern for source files.
    source: HashMap<String, usize>,
    /// Counts per pattern for test files.
    test: HashMap<String, usize>,
    /// Per-package breakdown (only if workspace configured).
    packages: HashMap<String, PackageMetrics>,
}

#[derive(Default)]
struct PackageMetrics {
    source: HashMap<String, usize>,
    test: HashMap<String, usize>,
}

impl EscapesMetrics {
    fn new() -> Self {
        Self::default()
    }

    fn increment(&mut self, pattern_name: &str, is_test: bool) {
        let map = if is_test {
            &mut self.test
        } else {
            &mut self.source
        };
        *map.entry(pattern_name.to_string()).or_insert(0) += 1;
    }

    fn increment_package(&mut self, package: &str, pattern_name: &str, is_test: bool) {
        let pkg = self.packages.entry(package.to_string()).or_default();
        let map = if is_test {
            &mut pkg.test
        } else {
            &mut pkg.source
        };
        *map.entry(pattern_name.to_string()).or_insert(0) += 1;
    }

    fn source_count(&self, pattern_name: &str) -> usize {
        self.source.get(pattern_name).copied().unwrap_or(0)
    }

    /// Convert to JSON metrics structure.
    fn to_json(&self, pattern_names: &[String]) -> JsonValue {
        // Include all configured patterns, even with 0 count
        let mut source_obj = serde_json::Map::new();
        let mut test_obj = serde_json::Map::new();

        for name in pattern_names {
            source_obj.insert(
                name.clone(),
                json!(self.source.get(name).copied().unwrap_or(0)),
            );
            test_obj.insert(
                name.clone(),
                json!(self.test.get(name).copied().unwrap_or(0)),
            );
        }

        json!({
            "source": source_obj,
            "test": test_obj
        })
    }

    /// Convert to by_package structure (only if packages exist).
    fn to_by_package(&self, pattern_names: &[String]) -> Option<HashMap<String, JsonValue>> {
        if self.packages.is_empty() {
            return None;
        }

        let mut result = HashMap::new();
        for (pkg_name, pkg_metrics) in &self.packages {
            let mut source_obj = serde_json::Map::new();
            let mut test_obj = serde_json::Map::new();

            for name in pattern_names {
                source_obj.insert(
                    name.clone(),
                    json!(pkg_metrics.source.get(name).copied().unwrap_or(0)),
                );
                test_obj.insert(
                    name.clone(),
                    json!(pkg_metrics.test.get(name).copied().unwrap_or(0)),
                );
            }

            result.insert(
                pkg_name.clone(),
                json!({
                    "source": source_obj,
                    "test": test_obj
                }),
            );
        }

        Some(result)
    }
}

/// The escapes check detects escape hatch patterns.
pub struct EscapesCheck;

impl Check for EscapesCheck {
    fn name(&self) -> &'static str {
        "escapes"
    }

    fn description(&self) -> &'static str {
        "Escape hatch detection"
    }

    fn run(&self, ctx: &CheckContext) -> CheckResult {
        let config = &ctx.config.check.escapes;

        if config.check == CheckLevel::Off {
            return CheckResult::passed(self.name());
        }

        // No patterns configured = nothing to check
        if config.patterns.is_empty() {
            return CheckResult::passed(self.name());
        }

        // Compile patterns once
        let patterns = match compile_patterns(config) {
            Ok(p) => p,
            Err(e) => return CheckResult::skipped(self.name(), e.to_string()),
        };

        // Collect pattern names for metrics output
        let pattern_names: Vec<String> = patterns.iter().map(|p| p.name.clone()).collect();

        // Get workspace packages for by_package tracking
        let packages = &ctx.config.workspace.packages;

        // Use project test patterns or defaults
        let test_patterns = if ctx.config.project.tests.is_empty() {
            default_test_patterns()
        } else {
            ctx.config.project.tests.clone()
        };

        let mut violations = Vec::new();
        let mut metrics = EscapesMetrics::new();
        let mut limit_reached = false;

        for file in ctx.files {
            if limit_reached {
                break;
            }

            // Skip non-source files (configs, docs, etc.)
            if !is_source_file(&file.path) {
                continue;
            }

            // Read file content
            let content = match std::fs::read_to_string(&file.path) {
                Ok(c) => c,
                Err(_) => continue,
            };

            let relative = file.path.strip_prefix(ctx.root).unwrap_or(&file.path);

            // Classify file as source or test
            let is_test = classify_file(&file.path, ctx.root, &test_patterns) == FileKind::Test;
            let package = find_package(&file.path, ctx.root, packages);

            // Find matches for each pattern
            for pattern in &patterns {
                let matches = pattern.matcher.find_all_with_lines(&content);

                for m in matches {
                    // Always track metrics (both source and test)
                    metrics.increment(&pattern.name, is_test);
                    if let Some(ref pkg) = package {
                        metrics.increment_package(pkg, &pattern.name, is_test);
                    }

                    // Test code: tracked in metrics but no violations
                    if is_test {
                        continue;
                    }

                    // Source code: apply action logic
                    match pattern.action {
                        EscapeAction::Count => {
                            // Just count - threshold check happens after all files
                        }
                        EscapeAction::Comment => {
                            let comment_pattern =
                                pattern.comment.as_deref().unwrap_or("// JUSTIFIED:");

                            if !has_justification_comment(&content, m.line, comment_pattern) {
                                let advice =
                                    format_comment_advice(&pattern.advice, comment_pattern);
                                if let Some(v) = try_create_violation(
                                    ctx,
                                    relative,
                                    m.line,
                                    "missing_comment",
                                    &advice,
                                    &pattern.name,
                                ) {
                                    violations.push(v);
                                } else {
                                    limit_reached = true;
                                    break;
                                }
                            }
                        }
                        EscapeAction::Forbid => {
                            if let Some(v) = try_create_violation(
                                ctx,
                                relative,
                                m.line,
                                "forbidden",
                                &pattern.advice,
                                &pattern.name,
                            ) {
                                violations.push(v);
                            } else {
                                limit_reached = true;
                                break;
                            }
                        }
                    }
                }

                if limit_reached {
                    break;
                }
            }
        }

        // Check count thresholds after scanning all files (uses metrics)
        for pattern in &patterns {
            if pattern.action == EscapeAction::Count {
                let count = metrics.source_count(&pattern.name);
                if count > pattern.threshold
                    && let Some(v) = create_threshold_violation(
                        ctx,
                        &pattern.name,
                        count,
                        pattern.threshold,
                        &pattern.advice,
                    )
                {
                    violations.push(v);
                }
            }
        }

        // Build result with metrics
        let result = if violations.is_empty() {
            CheckResult::passed(self.name())
        } else {
            CheckResult::failed(self.name(), violations)
        };

        // Add metrics to result
        let result = result.with_metrics(metrics.to_json(&pattern_names));

        // Add by_package if workspace configured
        if let Some(by_package) = metrics.to_by_package(&pattern_names) {
            result.with_by_package(by_package)
        } else {
            result
        }
    }

    fn default_enabled(&self) -> bool {
        true
    }
}

fn compile_patterns(config: &EscapesConfig) -> Result<Vec<CompiledEscapePattern>, PatternError> {
    config
        .patterns
        .iter()
        .map(|p| {
            let matcher = CompiledPattern::compile(&p.pattern)?;
            let advice = p
                .advice
                .clone()
                .unwrap_or_else(|| default_advice(&p.action));
            Ok(CompiledEscapePattern {
                name: p.name.clone(),
                matcher,
                action: p.action,
                advice,
                comment: p.comment.clone(),
                threshold: p.threshold,
            })
        })
        .collect()
}

fn default_advice(action: &EscapeAction) -> String {
    match action {
        EscapeAction::Forbid => "Remove this escape hatch from production code.".to_string(),
        EscapeAction::Comment => "Add a justification comment.".to_string(),
        EscapeAction::Count => "Reduce escape hatch usage.".to_string(),
    }
}

fn try_create_violation(
    ctx: &CheckContext,
    path: &std::path::Path,
    line: u32,
    violation_type: &str,
    advice: &str,
    pattern_name: &str,
) -> Option<Violation> {
    let current = ctx.violation_count.fetch_add(1, Ordering::SeqCst);
    if let Some(limit) = ctx.limit
        && current >= limit
    {
        return None;
    }

    Some(Violation::file(path, line, violation_type, advice).with_pattern(pattern_name))
}

fn create_threshold_violation(
    ctx: &CheckContext,
    pattern_name: &str,
    count: usize,
    threshold: usize,
    advice: &str,
) -> Option<Violation> {
    let current = ctx.violation_count.fetch_add(1, Ordering::SeqCst);
    if let Some(limit) = ctx.limit
        && current >= limit
    {
        return None;
    }

    Some(Violation {
        file: None,
        line: None,
        violation_type: "threshold_exceeded".to_string(),
        advice: advice.to_string(),
        value: Some(count as i64),
        threshold: Some(threshold as i64),
        pattern: Some(pattern_name.to_string()),
    })
}

fn format_comment_advice(custom_advice: &str, comment_pattern: &str) -> String {
    if custom_advice.is_empty() || custom_advice == default_advice(&EscapeAction::Comment) {
        format!(
            "Add a {} comment explaining why this is necessary.",
            comment_pattern
        )
    } else {
        custom_advice.to_string()
    }
}

/// Search upward from a line for a required comment pattern.
///
/// Searches:
/// 1. Same line as the match
/// 2. Preceding lines, stopping at non-blank/non-comment lines
///
/// Returns true if comment pattern is found.
fn has_justification_comment(content: &str, match_line: u32, comment_pattern: &str) -> bool {
    let lines: Vec<&str> = content.lines().collect();
    let line_idx = (match_line - 1) as usize;

    // Check same line first
    if line_idx < lines.len() && lines[line_idx].contains(comment_pattern) {
        return true;
    }

    // Search upward through comments and blank lines
    if line_idx > 0 {
        for i in (0..line_idx).rev() {
            let line = lines[i].trim();

            // Check for comment pattern
            if line.contains(comment_pattern) {
                return true;
            }

            // Stop at non-blank, non-comment lines
            if !line.is_empty() && !is_comment_line(line) {
                break;
            }
        }
    }

    false
}

/// Check if a line is a comment line (language-agnostic heuristics).
fn is_comment_line(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.starts_with("//")      // C-style
        || trimmed.starts_with('#')   // Shell/Python/Ruby
        || trimmed.starts_with("/*")  // C block comment start
        || trimmed.starts_with('*')   // C block comment continuation
        || trimmed.starts_with("--")  // SQL/Lua
        || trimmed.starts_with(";;") // Lisp
}

/// Classify file as source or test.
fn classify_file(path: &Path, root: &Path, test_patterns: &[String]) -> FileKind {
    use crate::adapter::Adapter;
    let adapter = GenericAdapter::new(&[], test_patterns);
    let relative = path.strip_prefix(root).unwrap_or(path);
    adapter.classify(relative)
}

/// Find which package a file belongs to, if any.
fn find_package(path: &Path, root: &Path, packages: &[String]) -> Option<String> {
    let relative = path.strip_prefix(root).ok()?;
    let relative_str = relative.to_string_lossy();

    // Check if file is under any package directory
    for pkg in packages {
        // Handle wildcard patterns like "crates/*"
        let prefix = pkg.trim_end_matches("/*");
        if relative_str.starts_with(prefix) {
            // Extract package name from path
            let rest = relative_str.strip_prefix(prefix)?.trim_start_matches('/');
            let parts: Vec<&str> = rest.split('/').collect();

            if pkg.ends_with("/*") {
                // Wildcard: first component after prefix is package name
                if let Some(name) = parts.first()
                    && !name.is_empty()
                {
                    return Some((*name).to_string());
                }
            } else {
                // Exact path: use the last component of the pattern
                return Some(pkg.split('/').next_back().unwrap_or(pkg).to_string());
            }
        }
    }

    None
}

/// Default test patterns for file classification.
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

/// Check if a file is a source code file (for escape pattern checking).
/// Excludes configuration files, documentation, and data files.
fn is_source_file(path: &Path) -> bool {
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
        | "sh" | "bash" | "zsh"
        // Web
        | "html" | "css" | "vue" | "svelte"
        // Other
        | "sql" | "ex" | "exs" | "erl" | "clj" | "hs" | "ml"
    )
}

#[cfg(test)]
#[path = "escapes_tests.rs"]
mod tests;
