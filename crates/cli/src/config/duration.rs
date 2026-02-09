// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Duration string parsing for test runner time limits.
//!
//! Supports formats:
//! - `"30s"` → 30 seconds
//! - `"500ms"` → 500 milliseconds
//! - `"1m"` → 1 minute
//! - `"1.5s"` → 1.5 seconds

use std::time::Duration;

use serde::{Deserialize, Deserializer};

/// Parse a duration string into a Duration.
///
/// Supports formats:
/// - "30s" → 30 seconds
/// - "500ms" → 500 milliseconds
/// - "1m" → 1 minute
/// - "1.5s" → 1.5 seconds (fractional seconds)
pub fn parse_duration(s: &str) -> Result<Duration, String> {
    let s = s.trim();

    if s.is_empty() {
        return Err("empty duration string".to_string());
    }

    // Check for milliseconds first (longer suffix)
    if let Some(ms) = s.strip_suffix("ms") {
        let n: u64 = ms.trim().parse().map_err(|_| format!("invalid duration: {s}"))?;
        return Ok(Duration::from_millis(n));
    }

    // Check for seconds (supports fractional)
    if let Some(secs) = s.strip_suffix('s') {
        let n: f64 = secs.trim().parse().map_err(|_| format!("invalid duration: {s}"))?;
        if n < 0.0 {
            return Err(format!("negative duration: {s}"));
        }
        return Ok(Duration::from_secs_f64(n));
    }

    // Check for minutes
    if let Some(mins) = s.strip_suffix('m') {
        let n: u64 = mins.trim().parse().map_err(|_| format!("invalid duration: {s}"))?;
        return Ok(Duration::from_secs(n * 60));
    }

    Err(format!("invalid duration format: {s} (use 30s, 500ms, or 1m)"))
}

/// Deserialize an optional duration string.
pub fn deserialize_option<'de, D>(deserializer: D) -> Result<Option<Duration>, D::Error>
where
    D: Deserializer<'de>,
{
    let opt: Option<String> = Option::deserialize(deserializer)?;
    match opt {
        None => Ok(None),
        Some(s) => parse_duration(&s).map(Some).map_err(serde::de::Error::custom),
    }
}

#[cfg(test)]
#[path = "duration_tests.rs"]
mod tests;
