//! JSON output formatter.
//!
//! Produces output conforming to docs/specs/output.schema.json.
//! JSON is buffered and written at the end (not streamed).

use std::io::Write;

use chrono::Utc;

use crate::check::{CheckOutput, CheckResult};

/// JSON output formatter.
pub struct JsonFormatter<W: Write> {
    writer: W,
}

impl<W: Write> JsonFormatter<W> {
    /// Create a new JSON formatter.
    pub fn new(writer: W) -> Self {
        Self { writer }
    }

    /// Write the complete JSON output.
    pub fn write(&mut self, output: &CheckOutput) -> std::io::Result<()> {
        let json = serde_json::to_string_pretty(output).map_err(std::io::Error::other)?;
        writeln!(self.writer, "{}", json)
    }
}

/// Create CheckOutput with current timestamp.
pub fn create_output(checks: Vec<CheckResult>) -> CheckOutput {
    CheckOutput::new(
        Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        checks,
    )
}

#[cfg(test)]
#[path = "json_tests.rs"]
mod tests;
