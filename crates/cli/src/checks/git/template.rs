// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Git commit message template generation.
//!
//! Generates `.gitmessage` content from configuration.

use crate::checks::git::parse::DEFAULT_TYPES;
use crate::config::GitCommitConfig;

/// Default template path.
pub const TEMPLATE_PATH: &str = ".gitmessage";

/// Generate .gitmessage content from configuration.
///
/// Template format:
/// ```text
/// # <type>(<scope>): <description>
/// #
/// # Types: feat, fix, chore, ...
/// # Scope: optional (api, cli, core)
/// #
/// # Examples:
/// #   feat(api): add export endpoint
/// #   fix: handle empty input
/// ```
pub fn generate_template(config: &GitCommitConfig) -> String {
    let types = effective_types(config);
    let scopes = config.scopes.as_ref();

    let mut lines = Vec::new();

    // Header with format reminder
    if scopes.is_some() {
        lines.push("# <type>(<scope>): <description>".to_string());
    } else {
        lines.push("# <type>: <description>".to_string());
    }
    lines.push("#".to_string());

    // Types line
    if types.is_empty() {
        lines.push("# Types: (any)".to_string());
    } else {
        lines.push(format!("# Types: {}", types.join(", ")));
    }

    // Scopes line (if configured)
    if let Some(scopes) = scopes {
        if scopes.is_empty() {
            lines.push("# Scope: optional".to_string());
        } else {
            lines.push(format!("# Scope: optional ({})", scopes.join(", ")));
        }
    }

    // Examples section
    lines.push("#".to_string());
    lines.push("# Examples:".to_string());

    let example_type = types.first().map(|s| s.as_str()).unwrap_or("feat");
    if let Some(scopes) = scopes
        && let Some(scope) = scopes.first()
    {
        lines.push(format!("#   {}({}): add new feature", example_type, scope));
    } else {
        lines.push(format!("#   {}: add new feature", example_type));
    }

    // Second example without scope
    let fix_type = if types.contains(&"fix".to_string()) {
        "fix"
    } else {
        types.get(1).map(|s| s.as_str()).unwrap_or("fix")
    };
    lines.push(format!("#   {}: handle edge case", fix_type));

    // Trailing newline for clean file
    lines.push(String::new());

    lines.join("\n")
}

/// Get effective types list for template.
fn effective_types(config: &GitCommitConfig) -> Vec<String> {
    match &config.types {
        Some(types) => types.clone(),
        None => DEFAULT_TYPES.iter().map(|s| s.to_string()).collect(),
    }
}

#[cfg(test)]
#[path = "template_tests.rs"]
mod tests;
