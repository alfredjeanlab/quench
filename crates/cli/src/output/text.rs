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
        if result.passed {
            return Ok(false); // Silent on pass per spec
        }

        // Check name: bold
        self.stdout.set_color(&scheme::check_name())?;
        write!(self.stdout, "{}", result.name)?;
        self.stdout.reset()?;

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
        // Format based on violation type and available fields
        match (v.value, v.threshold) {
            (Some(val), Some(thresh)) => {
                format!("{} ({} vs {})", v.violation_type, val, thresh)
            }
            _ => v.violation_type.clone(),
        }
    }

    /// Write the summary line.
    pub fn write_summary(&mut self, output: &CheckOutput) -> std::io::Result<()> {
        let passed = output.checks.iter().filter(|c| c.passed).count();
        let failed = output.checks.len() - passed;

        if failed == 0 {
            writeln!(
                self.stdout,
                "{} check{} passed",
                passed,
                if passed == 1 { "" } else { "s" }
            )?;
        } else {
            writeln!(
                self.stdout,
                "{} check{} passed, {} failed",
                passed,
                if passed == 1 { "" } else { "s" },
                failed
            )?;
        }
        Ok(())
    }

    /// Write truncation message if applicable.
    pub fn write_truncation_message(&mut self, total: usize) -> std::io::Result<()> {
        if let Some(limit) = self.options.limit
            && self.truncated
            && total > limit
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
        self.truncated
    }

    /// Get the number of violations shown.
    pub fn violations_shown(&self) -> usize {
        self.violations_shown
    }
}

#[cfg(test)]
#[path = "text_tests.rs"]
mod tests;
