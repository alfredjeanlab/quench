// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Violation generation for escape hatch detection.
//!
//! Creates violation objects for various escape hatch scenarios.

use std::sync::atomic::Ordering;

use crate::check::{CheckContext, Violation};
use crate::config::EscapeAction;

/// Default advice message for an escape action.
pub(super) fn default_advice(action: &EscapeAction) -> String {
    match action {
        EscapeAction::Forbid => "Remove this escape hatch from production code.".to_string(),
        EscapeAction::Comment => "Add a justification comment.".to_string(),
        EscapeAction::Count => "Reduce escape hatch usage.".to_string(),
    }
}

/// Try to create a violation, respecting the violation limit.
///
/// Returns `Some(violation)` if under limit, `None` if limit reached.
pub(super) fn try_create_violation(
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

/// Create a threshold exceeded violation for count-based patterns.
pub(super) fn create_threshold_violation(
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
        other_file: None,
        section: None,
        commit: None,
        message: None,
        expected_docs: None,
        area: None,
        area_match: None,
        path: None,
        target: None,
        change_type: None,
        lines_changed: None,
        scope: None,
        expected: None,
        found: None,
    })
}

/// Format comment advice with the required pattern.
pub(super) fn format_comment_advice(custom_advice: &str, comment_pattern: &str) -> String {
    if custom_advice.is_empty() || custom_advice == default_advice(&EscapeAction::Comment) {
        format!(
            "Add a {} comment explaining why this is necessary.",
            comment_pattern
        )
    } else {
        custom_advice.to_string()
    }
}
