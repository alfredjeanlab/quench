// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! License header check.
//!
//! Validates SPDX license headers per docs/specs/checks/license-headers.md.

use std::path::Path;
use std::sync::LazyLock;

use chrono::Datelike;
use globset::{Glob, GlobSetBuilder};
use regex::Regex;
use serde_json::json;

use crate::check::{Check, CheckContext, CheckResult, Violation};
use crate::file_reader::FileContent;

/// Regex pattern for matching SPDX-License-Identifier header lines.
#[allow(clippy::expect_used)]
static SPDX_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"SPDX-License-Identifier:\s*(\S+)").expect("valid regex"));

/// Regex pattern for matching Copyright header lines.
#[allow(clippy::expect_used)]
static COPYRIGHT_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"Copyright\s+\([cC]\)\s+(\d{4}(?:-\d{4})?)\s+(.+)").expect("valid regex")
});

/// The license check validates SPDX license headers in source files.
pub struct LicenseCheck;

impl Check for LicenseCheck {
    fn name(&self) -> &'static str {
        "license"
    }

    fn description(&self) -> &'static str {
        "License headers"
    }

    fn default_enabled(&self) -> bool {
        false
    }

    fn run(&self, ctx: &CheckContext) -> CheckResult {
        let config = &ctx.config.check.license;

        // License check only runs in CI mode
        if !ctx.ci_mode {
            return CheckResult::passed(self.name());
        }

        // Check if explicitly disabled
        if config.check.as_deref() == Some("off") {
            return CheckResult::passed(self.name());
        }

        // If license is not configured, skip silently (disabled by default)
        let expected_license = match &config.license {
            Some(l) => l.as_str(),
            None => return CheckResult::passed(self.name()),
        };

        // Copyright holder for fix mode
        let expected_copyright = config.copyright.as_deref().unwrap_or("Unknown");
        let current_year = chrono::Utc::now().year();

        // Build include patterns matcher (if patterns are configured)
        let include_matcher = build_include_matcher(&config.patterns);

        // Build exclude patterns matcher
        let exclude_matcher = build_exclude_matcher(&config.exclude);

        let mut violations = Vec::new();
        let mut fixes = LicenseFixes::new();
        let mut files_checked = 0;
        let mut files_with_headers = 0;
        let mut files_missing_headers = 0;
        let mut files_outdated_year = 0;
        let mut files_wrong_license = 0;

        for file in ctx.files {
            let relative_path = file.path.strip_prefix(ctx.root).unwrap_or(&file.path);

            // Check if file should be processed
            if !should_check_file(
                relative_path,
                &config.patterns,
                &include_matcher,
                &exclude_matcher,
            ) {
                continue;
            }

            // Read file content
            let file_content = match FileContent::read(&file.path) {
                Ok(c) => c,
                Err(_) => continue,
            };
            let Some(content) = file_content.as_str() else {
                continue; // Skip non-UTF-8 files
            };

            files_checked += 1;

            // Get file extension for comment syntax
            let ext = file.path.extension().and_then(|e| e.to_str()).unwrap_or("");

            // Get first N lines for header check (skip shebang)
            let header_lines = get_header_lines(content, 10);

            // Check for SPDX header
            let spdx_match = SPDX_PATTERN.captures(&header_lines);
            let copyright_match = COPYRIGHT_PATTERN.captures(&header_lines);

            match (spdx_match, copyright_match) {
                (None, _) | (_, None) => {
                    // Missing header
                    files_missing_headers += 1;

                    if ctx.fix {
                        // Generate and insert header
                        let header = generate_header(
                            expected_license,
                            expected_copyright,
                            current_year,
                            ext,
                        );
                        let new_content = insert_header_preserving_shebang(content, &header);

                        if !ctx.dry_run {
                            let _ = std::fs::write(&file.path, &new_content);
                        }
                        fixes
                            .headers_added
                            .push(relative_path.display().to_string());
                    } else {
                        violations.push(Violation::file_only(
                            relative_path,
                            "missing_header",
                            "missing license header. Add SPDX-License-Identifier and Copyright at file start.",
                        ));
                    }
                }
                (Some(spdx), Some(copyright)) => {
                    files_with_headers += 1;
                    let found_license = spdx.get(1).map(|m| m.as_str()).unwrap_or("");
                    let found_year = copyright.get(1).map(|m| m.as_str()).unwrap_or("");

                    // Check license identifier
                    if found_license != expected_license {
                        files_wrong_license += 1;
                        // Don't auto-fix wrong license (too risky), just report
                        violations.push(
                            Violation::file(
                                relative_path,
                                find_line_number(content, "SPDX-License-Identifier"),
                                "wrong_license",
                                format!(
                                    "Expected: {}, found: {}. Update or run --fix to correct.",
                                    expected_license, found_license
                                ),
                            )
                            .with_expected_found(expected_license, found_license),
                        );
                    }

                    // Check copyright year includes current year
                    if !year_includes_current(found_year, current_year) {
                        files_outdated_year += 1;

                        if ctx.fix {
                            // Update year in content
                            let new_content = update_copyright_year(content, current_year);

                            if !ctx.dry_run {
                                let _ = std::fs::write(&file.path, &new_content);
                            }
                            fixes
                                .years_updated
                                .push(relative_path.display().to_string());
                        } else {
                            violations.push(
                                Violation::file(
                                    relative_path,
                                    find_line_number(content, "Copyright"),
                                    "outdated_year",
                                    format!(
                                        "Expected: {}, found: {}. Update copyright year or run --fix.",
                                        current_year, found_year
                                    ),
                                )
                                .with_expected_found(current_year.to_string(), found_year),
                            );
                        }
                    }
                }
            }

            // Respect violation limit
            if let Some(limit) = ctx.limit
                && violations.len() >= limit
            {
                break;
            }
        }

        // Check LICENSE and README.md files for copyright year (unless limit reached)
        if ctx.limit.map_or(true, |limit| violations.len() < limit) {
            check_root_file(
                &ctx.root.join("LICENSE"),
                ctx.root,
                expected_copyright,
                current_year,
                ctx.fix,
                ctx.dry_run,
                &mut violations,
                &mut fixes,
                &mut files_checked,
                &mut files_outdated_year,
            );
        }

        if ctx.limit.map_or(true, |limit| violations.len() < limit) {
            check_root_file(
                &ctx.root.join("README.md"),
                ctx.root,
                expected_copyright,
                current_year,
                ctx.fix,
                ctx.dry_run,
                &mut violations,
                &mut fixes,
                &mut files_checked,
                &mut files_outdated_year,
            );
        }

        let metrics = json!({
            "files_checked": files_checked,
            "files_with_headers": files_with_headers,
            "files_missing_headers": files_missing_headers,
            "files_outdated_year": files_outdated_year,
            "files_wrong_license": files_wrong_license,
        });

        // Determine result based on violations and fixes
        if violations.is_empty() {
            if !fixes.is_empty() {
                CheckResult::fixed(self.name(), fixes.to_json()).with_metrics(metrics)
            } else {
                CheckResult::passed(self.name()).with_metrics(metrics)
            }
        } else {
            CheckResult::failed(self.name(), violations).with_metrics(metrics)
        }
    }
}

/// Build include matcher from patterns config.
fn build_include_matcher(
    patterns: &std::collections::HashMap<String, Vec<String>>,
) -> Option<globset::GlobSet> {
    if patterns.is_empty() {
        return None;
    }

    let mut builder = GlobSetBuilder::new();
    for globs in patterns.values() {
        for pattern in globs {
            if let Ok(glob) = Glob::new(pattern) {
                builder.add(glob);
            }
        }
    }

    builder.build().ok()
}

/// Build exclude matcher from exclude patterns.
fn build_exclude_matcher(patterns: &[String]) -> Option<globset::GlobSet> {
    if patterns.is_empty() {
        return None;
    }

    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        if let Ok(glob) = Glob::new(pattern) {
            builder.add(glob);
        }
    }

    builder.build().ok()
}

/// Check if a file should be processed based on patterns config.
fn should_check_file(
    path: &Path,
    patterns: &std::collections::HashMap<String, Vec<String>>,
    include_matcher: &Option<globset::GlobSet>,
    exclude_matcher: &Option<globset::GlobSet>,
) -> bool {
    // Check exclusions first
    if let Some(matcher) = exclude_matcher
        && matcher.is_match(path)
    {
        return false;
    }

    // If patterns are configured, use them
    if let Some(matcher) = include_matcher {
        return matcher.is_match(path);
    }

    // If no patterns configured, use default extension-based matching
    if patterns.is_empty() {
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        return is_supported_extension(ext);
    }

    false
}

/// Check if extension is supported for license header checking (default behavior).
fn is_supported_extension(ext: &str) -> bool {
    matches!(
        ext,
        // Rust/TypeScript/Go/C/C++
        "rs" | "ts" | "tsx" | "js" | "jsx" | "go" | "c" | "cpp" | "h"
        // Shell/Python/Ruby/YAML
        | "sh" | "bash" | "py" | "rb" | "yaml" | "yml"
    )
}

/// Get header lines from content, skipping shebang if present.
fn get_header_lines(content: &str, max_lines: usize) -> String {
    let lines: Vec<&str> = content.lines().take(max_lines + 1).collect();

    // Skip shebang line
    let start = if lines.first().map(|l| l.starts_with("#!")).unwrap_or(false) {
        1
    } else {
        0
    };

    lines[start..]
        .iter()
        .take(max_lines)
        .copied()
        .collect::<Vec<_>>()
        .join("\n")
}

/// Find line number (1-indexed) of first occurrence of pattern.
fn find_line_number(content: &str, pattern: &str) -> u32 {
    for (idx, line) in content.lines().enumerate() {
        if line.contains(pattern) {
            return (idx + 1) as u32;
        }
    }
    1
}

/// Check if year string includes the current year.
/// Handles formats: "2026" or "2020-2026"
fn year_includes_current(year_str: &str, current_year: i32) -> bool {
    if year_str.contains('-') {
        // Range format: "2020-2026"
        let parts: Vec<&str> = year_str.split('-').collect();
        if parts.len() == 2
            && let (Ok(start), Ok(end)) = (parts[0].parse::<i32>(), parts[1].parse::<i32>())
        {
            return current_year >= start && current_year <= end;
        }
        false
    } else {
        // Single year: "2026"
        year_str
            .parse::<i32>()
            .map(|y| y == current_year)
            .unwrap_or(false)
    }
}

/// Get comment prefix for a file based on extension.
fn comment_prefix_for_extension(ext: &str) -> &'static str {
    match ext {
        // Line comment languages
        "rs" | "ts" | "tsx" | "js" | "jsx" | "go" | "c" | "cpp" | "h" => "// ",
        // Hash comment languages
        "sh" | "bash" | "py" | "rb" | "yaml" | "yml" => "# ",
        // Default to line comments
        _ => "// ",
    }
}

/// Generate a license header for a file.
fn generate_header(license: &str, copyright_holder: &str, year: i32, ext: &str) -> String {
    let prefix = comment_prefix_for_extension(ext);
    format!(
        "{prefix}SPDX-License-Identifier: {license}\n\
         {prefix}Copyright (c) {year} {copyright_holder}\n"
    )
}

/// Insert header at file start, preserving shebang if present.
fn insert_header_preserving_shebang(content: &str, header: &str) -> String {
    if content.starts_with("#!") {
        // Find end of shebang line
        if let Some(newline_pos) = content.find('\n') {
            let shebang = &content[..=newline_pos];
            let rest = &content[newline_pos + 1..];
            return format!("{shebang}{header}\n{rest}");
        }
        // Shebang only, no newline
        return format!("{content}\n{header}\n");
    }
    // No shebang, prepend header
    format!("{header}\n{content}")
}

/// Update copyright year in content to include current year.
fn update_copyright_year(content: &str, current_year: i32) -> String {
    let mut result = String::with_capacity(content.len() + 10);
    for line in content.lines() {
        if let Some(caps) = COPYRIGHT_PATTERN.captures(line) {
            let year_str = caps.get(1).map(|m| m.as_str()).unwrap_or("");
            let new_year = if year_str.contains('-') {
                // Range format: "2020-2025" -> "2020-2026"
                let parts: Vec<&str> = year_str.split('-').collect();
                if parts.len() == 2 {
                    format!("{}-{}", parts[0], current_year)
                } else {
                    current_year.to_string()
                }
            } else {
                // Single year: "2020" -> "2020-2026"
                format!("{}-{}", year_str, current_year)
            };
            let new_line = line.replace(year_str, &new_year);
            result.push_str(&new_line);
        } else {
            result.push_str(line);
        }
        result.push('\n');
    }
    // Remove trailing newline if original didn't have one
    if !content.ends_with('\n') && result.ends_with('\n') {
        result.pop();
    }
    result
}

/// Check LICENSE or README.md file for outdated copyright year.
#[allow(clippy::too_many_arguments)]
fn check_root_file(
    file_path: &Path,
    root: &Path,
    expected_copyright: &str,
    current_year: i32,
    fix: bool,
    dry_run: bool,
    violations: &mut Vec<Violation>,
    fixes: &mut LicenseFixes,
    files_checked: &mut usize,
    files_outdated_year: &mut usize,
) {
    // Check if file exists
    if !file_path.exists() {
        return;
    }

    // Read file content
    let Ok(content) = std::fs::read_to_string(file_path) else {
        return;
    };

    *files_checked += 1;

    let relative_path = file_path.strip_prefix(root).unwrap_or(file_path);

    // Check if copyright line exists and includes current year
    if let Some(caps) = COPYRIGHT_PATTERN.captures(&content) {
        let found_year = caps.get(1).map(|m| m.as_str()).unwrap_or("");
        let found_holder = caps.get(2).map(|m| m.as_str()).unwrap_or("");

        // Check if copyright year includes current year
        if !year_includes_current(found_year, current_year) {
            *files_outdated_year += 1;

            if fix {
                // Update year in content
                let new_content = update_copyright_year(&content, current_year);

                if !dry_run {
                    let _ = std::fs::write(file_path, &new_content);
                }
                fixes
                    .years_updated
                    .push(relative_path.display().to_string());
            } else {
                violations.push(
                    Violation::file(
                        relative_path,
                        find_line_number(&content, "Copyright"),
                        "outdated_year",
                        format!(
                            "Expected: {}, found: {}. Update copyright year or run --fix.",
                            current_year, found_year
                        ),
                    )
                    .with_expected_found(current_year.to_string(), found_year),
                );
            }
        }

        // Optionally check copyright holder matches expected
        if found_holder != expected_copyright {
            // Note: Not reporting this as a violation, just updating if in fix mode
            if fix {
                // Could add logic to update copyright holder, but that's more invasive
                // For now, we only update the year
            }
        }
    }
}

/// Track fixes applied during check execution.
#[derive(Debug, Default)]
struct LicenseFixes {
    headers_added: Vec<String>,
    years_updated: Vec<String>,
}

impl LicenseFixes {
    fn new() -> Self {
        Self::default()
    }

    fn is_empty(&self) -> bool {
        self.headers_added.is_empty() && self.years_updated.is_empty()
    }

    fn to_json(&self) -> serde_json::Value {
        json!({
            "headers_added": self.headers_added.len(),
            "years_updated": self.years_updated.len(),
            "files": {
                "added": self.headers_added,
                "updated": self.years_updated,
            }
        })
    }
}

#[cfg(test)]
#[path = "license_tests.rs"]
mod tests;
