// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Tolerance value parsing for ratcheting.
//!
//! Parses human-readable duration and size strings for performance metric tolerances.

use std::time::Duration;

/// Parse a duration string like "5s", "1m30s", "500ms".
pub fn parse_duration(s: &str) -> Result<Duration, ParseError> {
    let s = s.trim();

    // Handle milliseconds first (before checking for 'm' in "1m30s" pattern)
    if let Some(ms_str) = s.strip_suffix("ms") {
        let ms: u64 = ms_str.parse()?;
        return Ok(Duration::from_millis(ms));
    }

    // Handle combined format: "1m30s" or "2m"
    if let Some((min_part, sec_part)) = s.split_once('m') {
        if !sec_part.is_empty() && !sec_part.ends_with('s') {
            return Err(ParseError::InvalidFormat(s.to_string()));
        }
        let mins: u64 = min_part.parse()?;
        let secs: f64 = if sec_part.is_empty() {
            0.0
        } else {
            let sec_str = sec_part.trim_end_matches('s');
            if sec_str.is_empty() { 0.0 } else { sec_str.parse()? }
        };
        return Ok(Duration::from_secs(mins * 60) + Duration::from_secs_f64(secs));
    }

    // Handle seconds: "5s"
    if let Some(s_str) = s.strip_suffix('s') {
        let secs: f64 = s_str.parse()?;
        return Ok(Duration::from_secs_f64(secs));
    }

    // Plain number = seconds
    let secs: f64 = s.parse()?;
    Ok(Duration::from_secs_f64(secs))
}

/// Parse a size string like "100KB", "5MB", "1GB", "100 bytes".
pub fn parse_size(s: &str) -> Result<u64, ParseError> {
    let s = s.trim().to_uppercase();

    let (num_str, multiplier) = if let Some(n) = s.strip_suffix("GB") {
        (n, 1024 * 1024 * 1024)
    } else if let Some(n) = s.strip_suffix("MB") {
        (n, 1024 * 1024)
    } else if let Some(n) = s.strip_suffix("KB") {
        (n, 1024)
    } else if let Some(n) = s.strip_suffix("BYTES") {
        (n, 1)
    } else if let Some(n) = s.strip_suffix('B') {
        (n, 1)
    } else {
        // Plain number = bytes
        (s.as_str(), 1)
    };

    let num: f64 = num_str.trim().parse()?;
    Ok((num * multiplier as f64) as u64)
}

/// Errors that can occur during tolerance parsing.
#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("invalid number: {0}")]
    InvalidNumber(#[from] std::num::ParseFloatError),

    #[error("invalid integer: {0}")]
    InvalidInt(#[from] std::num::ParseIntError),

    #[error("invalid format: {0}")]
    InvalidFormat(String),
}

#[cfg(test)]
#[path = "tolerance_tests.rs"]
mod tests;
