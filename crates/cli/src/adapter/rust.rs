//! Rust language adapter.
//!
//! Provides Rust-specific behavior for checks:
//! - File classification (source vs test)
//! - Default patterns for Rust projects
//! - Inline test detection via #[cfg(test)]
//!
//! See docs/specs/langs/rust.md for specification.

use std::fs;
use std::ops::Range;
use std::path::Path;

use globset::{Glob, GlobSet, GlobSetBuilder};
use toml::Value;

use super::{Adapter, EscapeAction, EscapePattern, FileKind};
use crate::config::{LintChangesPolicy, RustPolicyConfig};

/// Default escape patterns for Rust.
const RUST_ESCAPE_PATTERNS: &[EscapePattern] = &[
    EscapePattern {
        name: "unsafe",
        pattern: r"unsafe\s*\{",
        action: EscapeAction::Comment,
        comment: Some("// SAFETY:"),
        advice: "Add a // SAFETY: comment explaining the invariants.",
    },
    EscapePattern {
        name: "unwrap",
        pattern: r"\.unwrap\(\)",
        action: EscapeAction::Forbid,
        comment: None,
        advice: "Use ? operator or handle the error explicitly.",
    },
    EscapePattern {
        name: "expect",
        pattern: r"\.expect\(",
        action: EscapeAction::Forbid,
        comment: None,
        advice: "Use ? operator or handle the error explicitly.",
    },
    EscapePattern {
        name: "transmute",
        // SAFETY: This string literal defines the pattern; it's not actual transmute usage.
        pattern: r"mem::transmute",
        action: EscapeAction::Comment,
        comment: Some("// SAFETY:"),
        advice: "Add a // SAFETY: comment explaining type compatibility.",
    },
];

/// Rust language adapter.
pub struct RustAdapter {
    source_patterns: GlobSet,
    test_patterns: GlobSet,
    ignore_patterns: GlobSet,
}

impl RustAdapter {
    /// Create a new Rust adapter with default patterns.
    pub fn new() -> Self {
        Self {
            source_patterns: build_glob_set(&["**/*.rs".to_string()]),
            test_patterns: build_glob_set(&[
                "tests/**".to_string(),
                "test/**/*.rs".to_string(),
                "*_test.rs".to_string(),
                "*_tests.rs".to_string(),
            ]),
            ignore_patterns: build_glob_set(&["target/**".to_string()]),
        }
    }

    /// Check if a path should be ignored (e.g., target/).
    pub fn should_ignore(&self, path: &Path) -> bool {
        self.ignore_patterns.is_match(path)
    }
}

impl Default for RustAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl Adapter for RustAdapter {
    fn name(&self) -> &'static str {
        "rust"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["rs"]
    }

    fn classify(&self, path: &Path) -> FileKind {
        // Ignored paths are "Other"
        if self.should_ignore(path) {
            return FileKind::Other;
        }

        // Test patterns take precedence
        if self.test_patterns.is_match(path) {
            return FileKind::Test;
        }

        // Source patterns
        if self.source_patterns.is_match(path) {
            return FileKind::Source;
        }

        FileKind::Other
    }

    fn default_escapes(&self) -> &'static [EscapePattern] {
        RUST_ESCAPE_PATTERNS
    }
}

/// Build a GlobSet from pattern strings.
fn build_glob_set(patterns: &[String]) -> GlobSet {
    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        if let Ok(glob) = Glob::new(pattern) {
            builder.add(glob);
        }
    }
    builder.build().unwrap_or_else(|_| GlobSet::empty())
}

// =============================================================================
// CFG(TEST) BLOCK DETECTION
// =============================================================================

/// Result of parsing a Rust file for #[cfg(test)] blocks.
#[derive(Debug, Default)]
pub struct CfgTestInfo {
    /// Line ranges (0-indexed) that are inside #[cfg(test)] blocks.
    pub test_ranges: Vec<Range<usize>>,
}

impl CfgTestInfo {
    /// Parse a Rust source file to find #[cfg(test)] block ranges.
    ///
    /// Uses a simplified brace-counting approach:
    /// 1. Scan for #[cfg(test)] attribute
    /// 2. Count { and } to track block depth
    /// 3. Block ends when brace depth returns to 0
    ///
    /// Limitations (acceptable for v1):
    /// - Braces in string literals may confuse the parser
    /// - Multi-line attributes not fully supported
    /// - `mod tests;` (external module) declarations need file-level classification
    pub fn parse(content: &str) -> Self {
        let mut info = Self::default();
        let mut in_cfg_test = false;
        let mut brace_depth: i32 = 0;
        let mut block_start = 0;

        for (line_idx, line) in content.lines().enumerate() {
            let trimmed = line.trim();

            // Check for #[cfg(test)] attribute
            if !in_cfg_test && is_cfg_test_attr(trimmed) {
                in_cfg_test = true;
                block_start = line_idx;
                brace_depth = 0;
                continue;
            }

            if in_cfg_test {
                // Count braces to track block depth
                for ch in trimmed.chars() {
                    match ch {
                        '{' => brace_depth += 1,
                        '}' => {
                            brace_depth -= 1;
                            if brace_depth == 0 {
                                // End of #[cfg(test)] block
                                info.test_ranges.push(block_start..line_idx + 1);
                                in_cfg_test = false;
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        info
    }

    /// Check if a line (0-indexed) is inside a #[cfg(test)] block.
    pub fn is_test_line(&self, line_idx: usize) -> bool {
        self.test_ranges.iter().any(|r| r.contains(&line_idx))
    }
}

/// Check if a line is a #[cfg(test)] attribute.
fn is_cfg_test_attr(line: &str) -> bool {
    // Match #[cfg(test)] with optional whitespace
    line.starts_with("#[cfg(test)]")
        || line.starts_with("#[cfg( test )]")
        || line.contains("#[cfg(test)]")
}

/// Result of classifying lines within a single file.
#[derive(Debug, Default)]
pub struct LineClassification {
    pub source_lines: usize,
    pub test_lines: usize,
}

impl RustAdapter {
    /// Parse a file and return line-level classification.
    ///
    /// Returns a struct with source and test line counts.
    pub fn classify_lines(&self, path: &Path, content: &str) -> LineClassification {
        // First check if the whole file is a test file
        let file_kind = self.classify(path);

        if file_kind == FileKind::Test {
            // Entire file is test code
            let total_lines = content.lines().filter(|l| !l.trim().is_empty()).count();
            return LineClassification {
                source_lines: 0,
                test_lines: total_lines,
            };
        }

        if file_kind != FileKind::Source {
            return LineClassification::default();
        }

        // Parse for #[cfg(test)] blocks
        let cfg_info = CfgTestInfo::parse(content);

        let mut source_lines = 0;
        let mut test_lines = 0;

        for (idx, line) in content.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }

            if cfg_info.is_test_line(idx) {
                test_lines += 1;
            } else {
                source_lines += 1;
            }
        }

        LineClassification {
            source_lines,
            test_lines,
        }
    }
}

// =============================================================================
// LINT POLICY CHECKING
// =============================================================================

/// Result of checking lint policy.
#[derive(Debug, Default)]
pub struct PolicyCheckResult {
    /// Lint config files that were changed.
    pub changed_lint_config: Vec<String>,
    /// Source/test files that were changed.
    pub changed_source: Vec<String>,
    /// Whether the standalone policy is violated.
    pub standalone_violated: bool,
}

impl RustAdapter {
    /// Check lint policy against changed files.
    ///
    /// Returns policy check result with violation details.
    pub fn check_lint_policy(
        &self,
        changed_files: &[&Path],
        policy: &RustPolicyConfig,
    ) -> PolicyCheckResult {
        if policy.lint_changes == LintChangesPolicy::None {
            return PolicyCheckResult::default();
        }

        let mut result = PolicyCheckResult::default();

        for file in changed_files {
            let filename = file.file_name().and_then(|n| n.to_str()).unwrap_or("");

            // Check if it's a lint config file
            if policy
                .lint_config
                .iter()
                .any(|cfg| filename == cfg || file.to_string_lossy().ends_with(cfg))
            {
                result.changed_lint_config.push(file.display().to_string());
                continue;
            }

            // Check if it's a source or test file
            let kind = self.classify(file);
            if kind == FileKind::Source || kind == FileKind::Test {
                result.changed_source.push(file.display().to_string());
            }
        }

        // Standalone policy violated if both lint config AND source changed
        result.standalone_violated = policy.lint_changes == LintChangesPolicy::Standalone
            && !result.changed_lint_config.is_empty()
            && !result.changed_source.is_empty();

        result
    }
}

// =============================================================================
// WORKSPACE PARSING
// =============================================================================

/// Cargo workspace metadata.
#[derive(Debug, Clone, Default)]
pub struct CargoWorkspace {
    /// Is this a workspace root?
    pub is_workspace: bool,
    /// Package names in the workspace.
    pub packages: Vec<String>,
    /// Member glob patterns (e.g., "crates/*").
    pub member_patterns: Vec<String>,
}

impl CargoWorkspace {
    /// Parse workspace info from Cargo.toml at the given root.
    pub fn from_root(root: &Path) -> Self {
        let cargo_toml = root.join("Cargo.toml");
        if !cargo_toml.exists() {
            return Self::default();
        }

        let content = match fs::read_to_string(&cargo_toml) {
            Ok(c) => c,
            Err(_) => return Self::default(),
        };

        let value: Value = match toml::from_str(&content) {
            Ok(v) => v,
            Err(_) => return Self::default(),
        };

        Self::from_toml(&value, root)
    }

    fn from_toml(value: &Value, root: &Path) -> Self {
        let workspace = value.get("workspace");

        if workspace.is_none() {
            // Single package, not a workspace
            if let Some(pkg) = value.get("package").and_then(|p| p.get("name")) {
                return Self {
                    is_workspace: false,
                    packages: vec![pkg.as_str().unwrap_or("").to_string()],
                    member_patterns: vec![],
                };
            }
            return Self::default();
        }

        // Safe: we checked workspace.is_none() above
        let Some(workspace) = workspace else {
            return Self::default();
        };
        let members = workspace
            .get("members")
            .and_then(|m| m.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        // Expand member patterns to find actual packages
        let packages = expand_workspace_members(&members, root);

        Self {
            is_workspace: true,
            packages,
            member_patterns: members,
        }
    }
}

/// Expand workspace member patterns to package names.
fn expand_workspace_members(patterns: &[String], root: &Path) -> Vec<String> {
    let mut packages = Vec::new();

    for pattern in patterns {
        // Handle glob patterns like "crates/*"
        if pattern.contains('*') {
            if let Some(base) = pattern.strip_suffix("/*") {
                let dir = root.join(base);
                if let Ok(entries) = fs::read_dir(&dir) {
                    for entry in entries.flatten() {
                        if entry.path().is_dir()
                            && let Some(name) = read_package_name(&entry.path())
                        {
                            packages.push(name);
                        }
                    }
                }
            }
        } else {
            // Direct path to package
            let pkg_dir = root.join(pattern);
            if let Some(name) = read_package_name(&pkg_dir) {
                packages.push(name);
            }
        }
    }

    packages.sort();
    packages
}

/// Read package name from a directory's Cargo.toml.
fn read_package_name(dir: &Path) -> Option<String> {
    let cargo_toml = dir.join("Cargo.toml");
    let content = fs::read_to_string(&cargo_toml).ok()?;
    let value: Value = toml::from_str(&content).ok()?;
    value
        .get("package")?
        .get("name")?
        .as_str()
        .map(String::from)
}

// =============================================================================
// SUPPRESS ATTRIBUTE PARSING
// =============================================================================

/// Suppress attribute found in source code.
#[derive(Debug, Clone)]
pub struct SuppressAttr {
    /// Line number (0-indexed).
    pub line: usize,
    /// Attribute type: "allow" or "expect".
    pub kind: &'static str,
    /// Lint codes being suppressed (e.g., ["dead_code", "unused"]).
    pub codes: Vec<String>,
    /// Whether a justification comment was found.
    pub has_comment: bool,
    /// The comment text if found.
    pub comment_text: Option<String>,
}

/// Parse suppress attributes from Rust source.
pub fn parse_suppress_attrs(content: &str, comment_pattern: Option<&str>) -> Vec<SuppressAttr> {
    let mut attrs = Vec::new();
    let lines: Vec<&str> = content.lines().collect();

    for (line_idx, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        // Match #[allow(...)] or #[expect(...)]
        if let Some(attr) = parse_suppress_line(trimmed) {
            // Check for justification comment above
            let (has_comment, comment_text) =
                check_justification_comment(&lines, line_idx, comment_pattern);

            attrs.push(SuppressAttr {
                line: line_idx,
                kind: attr.kind,
                codes: attr.codes,
                has_comment,
                comment_text,
            });
        }
    }

    attrs
}

/// Parsed attribute info from a single line.
struct ParsedAttr {
    kind: &'static str,
    codes: Vec<String>,
}

/// Parse a single line for suppress attribute.
fn parse_suppress_line(line: &str) -> Option<ParsedAttr> {
    // Match #[allow(code1, code2)] or #[expect(code1, code2)]
    let kind = if line.starts_with("#[allow(") {
        "allow"
    } else if line.starts_with("#[expect(") {
        "expect"
    } else {
        return None;
    };

    // Extract codes between parentheses
    let start = line.find('(')? + 1;
    let end = line.rfind(')')?;
    if start >= end {
        return None;
    }

    let codes_str = &line[start..end];
    let codes: Vec<String> = codes_str
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    Some(ParsedAttr { kind, codes })
}

/// Check if there's a justification comment above the attribute.
fn check_justification_comment(
    lines: &[&str],
    attr_line: usize,
    required_pattern: Option<&str>,
) -> (bool, Option<String>) {
    // Look at preceding lines for a comment
    let mut check_line = attr_line;

    while check_line > 0 {
        check_line -= 1;
        let line = lines[check_line].trim();

        // Stop at blank lines or non-comment code
        if line.is_empty() {
            break;
        }

        // Check for comment
        if line.starts_with("//") {
            let comment_text = line.trim_start_matches('/').trim();

            // If a specific pattern is required, check for it
            if let Some(pattern) = required_pattern {
                let pattern_prefix = pattern.trim_start_matches('/').trim();
                if comment_text.starts_with(pattern_prefix) || line.starts_with(pattern) {
                    return (true, Some(comment_text.to_string()));
                }
                // Continue looking for the pattern
                continue;
            }

            // Any comment counts as justification
            if !comment_text.is_empty() {
                return (true, Some(comment_text.to_string()));
            }
        } else if !line.starts_with('#') {
            // Stop at non-attribute, non-comment line
            break;
        }
    }

    (false, None)
}

#[cfg(test)]
#[path = "rust_tests.rs"]
mod tests;
