// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Text output formatter.
//!
//! Format per docs/specs/03-output.md#text-format:
//! ```text
//! <check-name>: FAIL
//!   <file>:<line>: <brief violation description>
//!     <advice>
//! ```

use std::io::Write;
use termcolor::{ColorChoice, StandardStream, WriteColor};

use super::FormatOptions;
use crate::check::{CheckOutput, CheckResult, Violation};
use crate::color::scheme;

/// Text output formatter with color support.
pub struct TextFormatter {
    stdout: StandardStream,
    options: FormatOptions,
    violations_shown: usize,
    truncated: bool,
}

impl TextFormatter {
    /// Create a new text formatter.
    pub fn new(color_choice: ColorChoice, options: FormatOptions) -> Self {
        Self {
            stdout: StandardStream::stdout(color_choice),
            options,
            violations_shown: 0,
            truncated: false,
        }
    }

    /// Write a single check result (streaming).
    /// Returns true if output was truncated.
    pub fn write_check(&mut self, result: &CheckResult) -> std::io::Result<bool> {
        if result.passed && !result.fixed {
            return Ok(false); // Silent on pass per spec
        }

        // Check name: bold
        self.stdout.set_color(&scheme::check_name())?;
        write!(self.stdout, "{}", result.name)?;
        self.stdout.reset()?;

        if result.fixed {
            // ": FIXED" in green
            write!(self.stdout, ": ")?;
            self.stdout.set_color(&scheme::fixed())?;
            write!(self.stdout, "FIXED")?;
            self.stdout.reset()?;
            writeln!(self.stdout)?;

            // Show fix summary
            if let Some(ref summary) = result.fix_summary {
                self.write_fix_summary(summary)?;
            }

            return Ok(false);
        }

        if result.skipped {
            // ": SKIP" for skipped checks
            write!(self.stdout, ": ")?;
            self.stdout.set_color(&scheme::skip())?;
            write!(self.stdout, "SKIP")?;
            self.stdout.reset()?;
            writeln!(self.stdout)?;

            // Show skip reason
            if let Some(ref error) = result.error {
                writeln!(self.stdout, "  {}", error)?;
            }

            return Ok(false);
        }

        // ": FAIL" in red
        write!(self.stdout, ": ")?;
        self.stdout.set_color(&scheme::fail())?;
        write!(self.stdout, "FAIL")?;
        self.stdout.reset()?;
        writeln!(self.stdout)?;

        // Violations
        for violation in &result.violations {
            if let Some(limit) = self.options.limit
                && self.violations_shown >= limit
            {
                self.truncated = true;
                return Ok(true); // Truncated
            }
            self.write_violation(violation)?;
            self.violations_shown += 1;
        }

        Ok(false)
    }

    fn write_fix_summary(&mut self, summary: &serde_json::Value) -> std::io::Result<()> {
        // Show files_synced for actual fixes
        if let Some(synced) = summary.get("files_synced").and_then(|s| s.as_array()) {
            for entry in synced {
                let file = entry.get("file").and_then(|f| f.as_str()).unwrap_or("?");
                let source = entry.get("source").and_then(|s| s.as_str()).unwrap_or("?");
                let sections = entry.get("sections").and_then(|n| n.as_i64()).unwrap_or(0);
                writeln!(
                    self.stdout,
                    "  Synced {} from {} ({} sections updated)",
                    file, source, sections
                )?;
            }
        }

        // Show previews for dry-run
        if let Some(previews) = summary.get("previews").and_then(|p| p.as_array()) {
            for entry in previews {
                self.write_diff_preview(entry)?;
            }
        }
        Ok(())
    }

    fn write_diff_preview(&mut self, preview: &serde_json::Value) -> std::io::Result<()> {
        let file = preview.get("file").and_then(|f| f.as_str()).unwrap_or("?");
        let source = preview
            .get("source")
            .and_then(|s| s.as_str())
            .unwrap_or("?");
        let old_content = preview
            .get("old_content")
            .and_then(|c| c.as_str())
            .unwrap_or("");
        let new_content = preview
            .get("new_content")
            .and_then(|c| c.as_str())
            .unwrap_or("");
        let sections = preview
            .get("sections")
            .and_then(|n| n.as_i64())
            .unwrap_or(0);

        // Header
        writeln!(
            self.stdout,
            "  Would sync {} from {} ({} sections)",
            file, source, sections
        )?;

        // Unified diff
        self.write_unified_diff(file, old_content, new_content)?;

        Ok(())
    }

    fn write_unified_diff(&mut self, file: &str, old: &str, new: &str) -> std::io::Result<()> {
        // Unified diff headers with descriptive labels
        self.stdout.set_color(&scheme::diff_remove())?;
        writeln!(self.stdout, "  --- {} (original)", file)?;
        self.stdout.reset()?;
        self.stdout.set_color(&scheme::diff_add())?;
        writeln!(self.stdout, "  +++ {} (synced)", file)?;
        self.stdout.reset()?;

        let old_lines: Vec<_> = old.lines().collect();
        let new_lines: Vec<_> = new.lines().collect();

        // Hunk header showing line counts
        writeln!(
            self.stdout,
            "  @@ -1,{} +1,{} @@",
            old_lines.len(),
            new_lines.len()
        )?;

        // Show removed lines (old content)
        for line in &old_lines {
            self.stdout.set_color(&scheme::diff_remove())?;
            writeln!(self.stdout, "  -{}", line)?;
            self.stdout.reset()?;
        }

        // Show added lines (new content)
        for line in &new_lines {
            self.stdout.set_color(&scheme::diff_add())?;
            writeln!(self.stdout, "  +{}", line)?;
            self.stdout.reset()?;
        }

        Ok(())
    }

    fn write_violation(&mut self, v: &Violation) -> std::io::Result<()> {
        write!(self.stdout, "  ")?;

        // File path in cyan
        if let Some(ref file) = v.file {
            self.stdout.set_color(&scheme::path())?;
            write!(self.stdout, "{}", file.display())?;
            self.stdout.reset()?;

            // Line number in yellow
            if let Some(line) = v.line {
                write!(self.stdout, ":")?;
                self.stdout.set_color(&scheme::line_number())?;
                write!(self.stdout, "{}", line)?;
                self.stdout.reset()?;
            }
            write!(self.stdout, ": ")?;
        }

        // Violation description (includes type-specific info)
        writeln!(self.stdout, "{}", self.format_violation_desc(v))?;

        // Advice (4-space indent)
        writeln!(self.stdout, "    {}", v.advice)?;

        Ok(())
    }

    fn format_violation_desc(&self, v: &Violation) -> String {
        match v.violation_type.as_str() {
            // Agents check - human-readable descriptions
            "missing_file" => "missing required file".to_string(),
            "forbidden_file" => "forbidden file exists".to_string(),
            "out_of_sync" => {
                if let Some(ref other) = v.other_file {
                    format!("out of sync with {}", other.display())
                } else {
                    "out of sync".to_string()
                }
            }
            "missing_section" => "missing required section".to_string(),
            "forbidden_section" => "forbidden section found".to_string(),
            "forbidden_table" => "forbidden table".to_string(),
            "forbidden_diagram" => "forbidden box diagram".to_string(),
            "forbidden_mermaid" => "forbidden mermaid block".to_string(),
            "file_too_large" => {
                // Agents check sets value/threshold but not lines
                // Cloc check sets lines/nonblank - use default format with "lines:" prefix
                match (v.value, v.threshold, v.lines) {
                    (Some(val), Some(thresh), None) => {
                        format!("file too large ({} vs {})", val, thresh)
                    }
                    _ => self.format_default_desc(v),
                }
            }
            // Other checks - existing behavior
            _ => self.format_default_desc(v),
        }
    }

    fn format_default_desc(&self, v: &Violation) -> String {
        let base = match (v.value, v.threshold) {
            (Some(val), Some(thresh)) => {
                // Use labeled format for cloc line violations
                let label = match v.violation_type.as_str() {
                    "file_too_large" => "lines: ",
                    "file_too_large_nonblank" => "nonblank: ",
                    _ => "",
                };
                format!("{} ({}{} vs {})", v.violation_type, label, val, thresh)
            }
            _ => v.violation_type.clone(),
        };

        // Append pattern if present (for escape/suppress violations)
        if let Some(ref pattern) = v.pattern {
            format!("{}: {}", base, pattern)
        } else {
            base
        }
    }

    /// Write the summary listing each check by status.
    pub fn write_summary(&mut self, output: &CheckOutput) -> std::io::Result<()> {
        let passed: Vec<_> = output
            .checks
            .iter()
            .filter(|c| c.passed && !c.stub)
            .map(|c| c.name.as_str())
            .collect();
        let failed: Vec<_> = output
            .checks
            .iter()
            .filter(|c| !c.passed && !c.skipped && !c.stub)
            .map(|c| c.name.as_str())
            .collect();
        let skipped: Vec<_> = output
            .checks
            .iter()
            .filter(|c| c.skipped && !c.stub)
            .map(|c| c.name.as_str())
            .collect();

        if !passed.is_empty() {
            writeln!(self.stdout, "PASS: {}", passed.join(", "))?;
        }
        if !failed.is_empty() {
            writeln!(self.stdout, "FAIL: {}", failed.join(", "))?;
        }
        if !skipped.is_empty() {
            writeln!(self.stdout, "SKIP: {}", skipped.join(", "))?;
        }
        Ok(())
    }

    /// Write truncation message if applicable.
    pub fn write_truncation_message(&mut self, _total: usize) -> std::io::Result<()> {
        if let Some(limit) = self.options.limit
            && self.was_truncated()
        {
            writeln!(
                self.stdout,
                "Stopped after {} violations. Use --no-limit to see all.",
                limit
            )?;
        }
        Ok(())
    }

    /// Check if output was truncated.
    pub fn was_truncated(&self) -> bool {
        // Truncated if we either explicitly stopped writing, or if we hit the limit
        self.truncated
            || self
                .options
                .limit
                .is_some_and(|limit| self.violations_shown >= limit)
    }

    /// Get the number of violations shown.
    pub fn violations_shown(&self) -> usize {
        self.violations_shown
    }
}

#[cfg(test)]
#[path = "text_tests.rs"]
mod tests;
