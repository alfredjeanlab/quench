// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Check name suggestions for config validation.

use std::path::Path;

/// Known check names for suggestions.
const KNOWN_CHECK_NAMES: &[&str] = &[
    "cloc", "escapes", "agents", "docs", "tests", "git", "build", "license",
];

/// Suggest a check name for a typo.
pub fn suggest_check_name(unknown: &str) -> Option<&'static str> {
    // Empty strings can't be typos
    if unknown.is_empty() {
        return None;
    }

    // Common typos and variations
    let suggestion = match unknown {
        // escapes
        "escape" | "escaps" | "escap" | "esc" => Some("escapes"),
        // agents
        "agent" | "claude" | "cursor" | "agnt" => Some("agents"),
        // tests
        "test" | "testing" | "tst" => Some("tests"),
        // docs
        "doc" | "documentation" | "readme" => Some("docs"),
        // cloc
        "loc" | "lines" | "code" | "sloc" => Some("cloc"),
        // git
        "commit" | "commits" | "gitcheck" => Some("git"),
        // build
        "builds" | "binary" | "compile" => Some("build"),
        // license
        "licenses" | "lic" | "header" | "headers" => Some("license"),
        _ => None,
    };

    if suggestion.is_some() {
        return suggestion;
    }

    // Try prefix matching (require at least 2 chars to avoid false positives)
    if unknown.len() >= 2 {
        for &name in KNOWN_CHECK_NAMES {
            if name.starts_with(unknown) || unknown.starts_with(name) {
                return Some(name);
            }
        }
    }

    None
}

/// Warn about unknown check key with suggestion.
pub fn warn_unknown_check(path: &Path, key: &str) {
    let suggestion = suggest_check_name(key);
    if let Some(suggested) = suggestion {
        eprintln!(
            "quench: warning: {}: unknown check `{}`. Did you mean `{}`?",
            path.display(),
            key,
            suggested
        );
    } else {
        eprintln!(
            "quench: warning: {}: unknown check `{}`\n  Valid checks: {}",
            path.display(),
            key,
            KNOWN_CHECK_NAMES.join(", ")
        );
    }
}

#[cfg(test)]
#[path = "suggest_tests.rs"]
mod tests;
