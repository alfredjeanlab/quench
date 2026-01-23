//! Escapes (escape hatches) check.
//!
//! Detects patterns that bypass type safety or error handling.
//! See docs/specs/checks/escape-hatches.md.

use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::atomic::Ordering;

use serde_json::{Value as JsonValue, json};

use crate::adapter::{
    CfgTestInfo, EscapePattern as AdapterEscapePattern, FileKind, GenericAdapter, ProjectLanguage,
    RustAdapter, detect_language, parse_suppress_attrs,
};
use crate::check::{Check, CheckContext, CheckResult, Violation};
use crate::config::{
    CheckLevel, EscapeAction, EscapePattern as ConfigEscapePattern, LintChangesPolicy, RustConfig,
    SuppressConfig, SuppressLevel,
};
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

        // Check lint policy for Rust projects (only when --base is provided)
        let mut policy_violations = Vec::new();
        if detect_language(ctx.root) == ProjectLanguage::Rust {
            policy_violations = check_lint_policy(ctx, &ctx.config.rust);
        }

        // Get adapter default patterns for the detected language
        let adapter_patterns = get_adapter_escape_patterns(ctx.root);

        // Merge patterns: config patterns override adapter defaults by name
        let merged_patterns = merge_patterns(&config.patterns, &adapter_patterns);

        // No patterns to check = pass
        if merged_patterns.is_empty() {
            return CheckResult::passed(self.name());
        }

        // Compile patterns once
        let patterns = match compile_merged_patterns(&merged_patterns) {
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

        // Create adapter once for file classification (optimization: avoid per-file allocation)
        let file_adapter = GenericAdapter::new(&[], &test_patterns);

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
            let is_test_file = classify_file(&file_adapter, &file.path, ctx.root) == FileKind::Test;
            let package = find_package(&file.path, ctx.root, packages);

            // Check for Rust suppress attribute violations
            if is_rust_file(&file.path) {
                let cfg_info = CfgTestInfo::parse(&content);
                let suppress_violations = check_suppress_violations(
                    ctx,
                    relative,
                    &content,
                    &ctx.config.rust.suppress,
                    is_test_file,
                    &cfg_info,
                    &mut limit_reached,
                );
                violations.extend(suppress_violations);

                if limit_reached {
                    break;
                }
            }

            // Find matches for each pattern
            for pattern in &patterns {
                let matches = pattern.matcher.find_all_with_lines(&content);

                // Deduplicate matches by line - keep only first match per line
                // This prevents duplicate violations when pattern appears multiple
                // times on same line (e.g., in code AND in a comment)
                let mut seen_lines = HashSet::new();
                let unique_matches: Vec<_> = matches
                    .into_iter()
                    .filter(|m| seen_lines.insert(m.line))
                    .collect();

                for m in unique_matches {
                    // Always track metrics (both source and test)
                    metrics.increment(&pattern.name, is_test_file);
                    if let Some(ref pkg) = package {
                        metrics.increment_package(pkg, &pattern.name, is_test_file);
                    }

                    // Test code: tracked in metrics but no violations
                    if is_test_file {
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

        // Add policy violations to the main violations list
        violations.extend(policy_violations);

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
        lines: None,
        nonblank: None,
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
/// 1. Same line as the match (pattern must be at start of inline comment)
/// 2. Preceding lines, stopping at non-blank/non-comment lines
///
/// The pattern must appear at the start of a comment's content, not embedded
/// within other text. For example, `// SAFETY:` embedded inside
/// `// VIOLATION: missing // SAFETY: comment` should NOT match.
///
/// Returns true if comment pattern is found at a valid comment boundary.
fn has_justification_comment(content: &str, match_line: u32, comment_pattern: &str) -> bool {
    let lines: Vec<&str> = content.lines().collect();
    let line_idx = (match_line - 1) as usize;

    // Check same line first - look for pattern at start of inline comment
    if line_idx < lines.len() {
        let line = lines[line_idx];
        if let Some(comment_start) = find_comment_start(line) {
            let comment = &line[comment_start..];
            if comment_starts_with_pattern(comment, comment_pattern) {
                return true;
            }
        }
    }

    // Search upward through comments and blank lines
    if line_idx > 0 {
        for i in (0..line_idx).rev() {
            let line = lines[i].trim();

            // Check for comment pattern at start of comment
            if is_comment_line(line) && comment_starts_with_pattern(line, comment_pattern) {
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

/// Check if comment content starts with the pattern (after comment marker).
///
/// Normalizes both the comment and pattern by stripping comment markers,
/// then checks if the comment content starts with the pattern content.
fn comment_starts_with_pattern(comment: &str, pattern: &str) -> bool {
    let comment_content = strip_comment_markers(comment);
    let pattern_content = strip_comment_markers(pattern);
    comment_content.starts_with(&pattern_content)
}

/// Strip comment markers and leading whitespace to get the content.
fn strip_comment_markers(s: &str) -> String {
    let trimmed = s.trim();
    // Strip various comment markers
    let content = trimmed
        .strip_prefix("///")
        .or_else(|| trimmed.strip_prefix("//!"))
        .or_else(|| trimmed.strip_prefix("//"))
        .or_else(|| trimmed.strip_prefix("/*"))
        .or_else(|| trimmed.strip_prefix('#'))
        .or_else(|| trimmed.strip_prefix("--"))
        .or_else(|| trimmed.strip_prefix(";;"))
        .or_else(|| trimmed.strip_prefix('*'))
        .unwrap_or(trimmed);
    content.trim_start().to_string()
}

/// Find the start of a comment in a line (returns byte offset of comment marker).
fn find_comment_start(line: &str) -> Option<usize> {
    // Find // comment (most common)
    if let Some(pos) = line.find("//") {
        return Some(pos);
    }
    // Find # comment (shell/Python) - avoid matching # in strings
    if let Some(pos) = line.find('#') {
        // Simple heuristic: # at start or preceded by whitespace
        if pos == 0 || line.as_bytes().get(pos.saturating_sub(1)) == Some(&b' ') {
            return Some(pos);
        }
    }
    None
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

/// Classify file as source or test using a pre-built adapter.
fn classify_file(adapter: &GenericAdapter, path: &Path, root: &Path) -> FileKind {
    use crate::adapter::Adapter;
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
        "**/benches/**".to_string(),
        "benches/**".to_string(),
        "**/test_utils.*".to_string(),
        "test_utils.*".to_string(),
        "**/*_test.*".to_string(),
        "**/*_tests.*".to_string(),
        "**/*.test.*".to_string(),
        "**/*.spec.*".to_string(),
    ]
}

/// Get escape patterns from the adapter for the detected language.
fn get_adapter_escape_patterns(root: &Path) -> Vec<ConfigEscapePattern> {
    use crate::adapter::Adapter;

    let mut patterns = Vec::new();

    // Check project language and get adapter defaults
    match detect_language(root) {
        ProjectLanguage::Rust => {
            let rust_adapter = RustAdapter::new();
            patterns.extend(convert_adapter_patterns(rust_adapter.default_escapes()));
        }
        ProjectLanguage::Shell => {
            // Shell escape patterns will be added in Phase 406
        }
        ProjectLanguage::Generic => {
            // No default patterns for generic projects
        }
    }

    patterns
}

/// Convert adapter escape patterns to config format.
fn convert_adapter_patterns(adapter_patterns: &[AdapterEscapePattern]) -> Vec<ConfigEscapePattern> {
    adapter_patterns
        .iter()
        .map(|p| ConfigEscapePattern {
            name: p.name.to_string(),
            pattern: p.pattern.to_string(),
            action: adapter_action_to_config(p.action),
            comment: p.comment.map(String::from),
            advice: Some(p.advice.to_string()),
            threshold: 0,
        })
        .collect()
}

/// Convert adapter EscapeAction to config EscapeAction.
fn adapter_action_to_config(action: crate::adapter::EscapeAction) -> EscapeAction {
    match action {
        crate::adapter::EscapeAction::Count => EscapeAction::Count,
        crate::adapter::EscapeAction::Comment => EscapeAction::Comment,
        crate::adapter::EscapeAction::Forbid => EscapeAction::Forbid,
    }
}

/// Merge user config patterns with adapter defaults.
/// User patterns override defaults by name.
fn merge_patterns(
    config_patterns: &[ConfigEscapePattern],
    adapter_patterns: &[ConfigEscapePattern],
) -> Vec<ConfigEscapePattern> {
    let mut merged = Vec::new();
    let config_names: HashSet<_> = config_patterns.iter().map(|p| &p.name).collect();

    // Add adapter defaults not overridden by config
    for pattern in adapter_patterns {
        if !config_names.contains(&pattern.name) {
            merged.push(pattern.clone());
        }
    }

    // Add all config patterns (they take precedence)
    merged.extend(config_patterns.iter().cloned());

    merged
}

/// Compile merged patterns into matchers.
fn compile_merged_patterns(
    patterns: &[ConfigEscapePattern],
) -> Result<Vec<CompiledEscapePattern>, PatternError> {
    patterns
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

/// Check if a file is a Rust source file.
fn is_rust_file(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("rs"))
        .unwrap_or(false)
}

/// Check lint policy and generate violations.
fn check_lint_policy(ctx: &CheckContext, rust_config: &RustConfig) -> Vec<Violation> {
    let mut violations = Vec::new();

    // Policy only applies when comparing against a base
    let Some(changed_files) = ctx.changed_files else {
        return violations;
    };

    // Check if standalone policy is enabled
    if rust_config.policy.lint_changes != LintChangesPolicy::Standalone {
        return violations;
    }

    let adapter = RustAdapter::new();
    let file_refs: Vec<&Path> = changed_files.iter().map(|p| p.as_path()).collect();
    let result = adapter.check_lint_policy(&file_refs, &rust_config.policy);

    if result.standalone_violated {
        // Create a single policy violation
        violations.push(Violation {
            file: None,
            line: None,
            violation_type: "lint_policy".to_string(),
            advice: format!(
                "Changed lint config: {}\nAlso changed source: {}\nSubmit lint config changes in a separate PR.",
                result.changed_lint_config.join(", "),
                truncate_list(&result.changed_source, 3),
            ),
            value: None,
            threshold: None,
            pattern: Some("lint_changes = standalone".to_string()),
            lines: None,
            nonblank: None,
        });
    }

    violations
}

/// Truncate a list for display, showing "and N more" if needed.
fn truncate_list(items: &[String], max: usize) -> String {
    if items.len() <= max {
        items.join(", ")
    } else {
        let shown: Vec<_> = items.iter().take(max).cloned().collect();
        format!("{} and {} more", shown.join(", "), items.len() - max)
    }
}

/// Check suppress attributes in a Rust file.
fn check_suppress_violations(
    ctx: &CheckContext,
    path: &Path,
    content: &str,
    config: &SuppressConfig,
    is_test_file: bool,
    cfg_info: &CfgTestInfo,
    limit_reached: &mut bool,
) -> Vec<Violation> {
    let mut violations = Vec::new();

    // Determine effective check level based on source vs test
    let effective_check = if is_test_file {
        config.test.check.unwrap_or(SuppressLevel::Allow)
    } else {
        config.check
    };

    // If allow, no checking needed
    if effective_check == SuppressLevel::Allow {
        return violations;
    }

    // Parse suppress attributes
    let attrs = parse_suppress_attrs(content, config.comment.as_deref());

    for attr in attrs {
        if *limit_reached {
            break;
        }

        // Check if this line is in test code (inline #[cfg(test)])
        let is_test_line = cfg_info.is_test_line(attr.line);

        if is_test_line {
            // Use test policy
            let test_check = config.test.check.unwrap_or(SuppressLevel::Allow);
            if test_check == SuppressLevel::Allow {
                continue;
            }
        }

        // Get scope config (source or test)
        let scope_config = if is_test_file || is_test_line {
            &config.test
        } else {
            &config.source
        };

        // Get the effective check level for this scope
        let scope_check = if is_test_file || is_test_line {
            config.test.check.unwrap_or(SuppressLevel::Allow)
        } else {
            scope_config.check.unwrap_or(config.check)
        };

        // Check each lint code
        for code in &attr.codes {
            if *limit_reached {
                break;
            }

            let pattern = format!("#[{}({})]", attr.kind, code);

            // Check forbid list first
            if scope_config.forbid.contains(code) {
                let advice = format!(
                    "Suppressing `{}` is forbidden. Remove the suppression or address the issue.",
                    code
                );
                if let Some(v) = try_create_violation(
                    ctx,
                    path,
                    (attr.line + 1) as u32,
                    "suppress_forbidden",
                    &advice,
                    &pattern,
                ) {
                    violations.push(v);
                } else {
                    *limit_reached = true;
                }
                continue;
            }

            // Check allow list (skip comment check)
            if scope_config.allow.contains(code) {
                continue;
            }

            // Check if comment is required
            if scope_check == SuppressLevel::Comment && !attr.has_comment {
                let advice = if let Some(ref pat) = config.comment {
                    format!(
                        "Lint suppression requires justification. Add a {} comment.",
                        pat
                    )
                } else {
                    "Lint suppression requires justification. Add a comment above the attribute."
                        .into()
                };
                if let Some(v) = try_create_violation(
                    ctx,
                    path,
                    (attr.line + 1) as u32,
                    "suppress_missing_comment",
                    &advice,
                    &pattern,
                ) {
                    violations.push(v);
                } else {
                    *limit_reached = true;
                }
            }

            // Forbid level means all suppressions fail
            if scope_check == SuppressLevel::Forbid {
                let advice =
                    "Lint suppressions are forbidden. Remove and fix the underlying issue.";
                if let Some(v) = try_create_violation(
                    ctx,
                    path,
                    (attr.line + 1) as u32,
                    "suppress_forbidden",
                    advice,
                    &pattern,
                ) {
                    violations.push(v);
                } else {
                    *limit_reached = true;
                }
            }
        }
    }

    violations
}

#[cfg(test)]
#[path = "escapes_tests.rs"]
mod tests;
