// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Escape pattern compilation and merging utilities.

use std::collections::HashSet;
use std::path::Path;

use crate::adapter::{
    EscapePattern as AdapterEscapePattern, GoAdapter, ProjectLanguage, RustAdapter, ShellAdapter,
    detect_language,
};
use crate::config::{EscapeAction, EscapePattern as ConfigEscapePattern};
use crate::pattern::{CompiledPattern, PatternError};

use super::{CompiledEscapePattern, default_advice};

/// Default test patterns for file classification.
pub(super) fn default_test_patterns() -> Vec<String> {
    vec![
        "**/tests/**".to_string(),
        "**/test/**".to_string(),
        "**/benches/**".to_string(),
        "benches/**".to_string(),
        "**/test_utils.*".to_string(),
        "test_utils.*".to_string(),
        "**/*_test.*".to_string(),
        "**/*_tests.*".to_string(),
        "**/*.test.*".to_string(),
        "**/*.spec.*".to_string(),
    ]
}

/// Get escape patterns from the adapter for the detected language.
pub(super) fn get_adapter_escape_patterns(root: &Path) -> Vec<ConfigEscapePattern> {
    use crate::adapter::Adapter;

    let mut patterns = Vec::new();

    // Check project language and get adapter defaults
    match detect_language(root) {
        ProjectLanguage::Rust => {
            let rust_adapter = RustAdapter::new();
            patterns.extend(convert_adapter_patterns(rust_adapter.default_escapes()));
        }
        ProjectLanguage::Go => {
            let go_adapter = GoAdapter::new();
            patterns.extend(convert_adapter_patterns(go_adapter.default_escapes()));
        }
        ProjectLanguage::Shell => {
            let shell_adapter = ShellAdapter::new();
            patterns.extend(convert_adapter_patterns(shell_adapter.default_escapes()));
        }
        ProjectLanguage::Generic => {
            // No default patterns for generic projects
        }
    }

    patterns
}

/// Convert adapter escape patterns to config format.
fn convert_adapter_patterns(adapter_patterns: &[AdapterEscapePattern]) -> Vec<ConfigEscapePattern> {
    adapter_patterns
        .iter()
        .map(|p| ConfigEscapePattern {
            name: p.name.to_string(),
            pattern: p.pattern.to_string(),
            action: adapter_action_to_config(p.action),
            comment: p.comment.map(String::from),
            advice: Some(p.advice.to_string()),
            threshold: 0,
        })
        .collect()
}

/// Convert adapter EscapeAction to config EscapeAction.
fn adapter_action_to_config(action: crate::adapter::EscapeAction) -> EscapeAction {
    match action {
        crate::adapter::EscapeAction::Count => EscapeAction::Count,
        crate::adapter::EscapeAction::Comment => EscapeAction::Comment,
        crate::adapter::EscapeAction::Forbid => EscapeAction::Forbid,
    }
}

/// Merge user config patterns with adapter defaults.
/// User patterns override defaults by name.
pub(super) fn merge_patterns(
    config_patterns: &[ConfigEscapePattern],
    adapter_patterns: &[ConfigEscapePattern],
) -> Vec<ConfigEscapePattern> {
    let mut merged = Vec::new();
    let config_names: HashSet<_> = config_patterns.iter().map(|p| &p.name).collect();

    // Add adapter defaults not overridden by config
    for pattern in adapter_patterns {
        if !config_names.contains(&pattern.name) {
            merged.push(pattern.clone());
        }
    }

    // Add all config patterns (they take precedence)
    merged.extend(config_patterns.iter().cloned());

    merged
}

/// Compile merged patterns into matchers.
pub(super) fn compile_merged_patterns(
    patterns: &[ConfigEscapePattern],
) -> Result<Vec<CompiledEscapePattern>, PatternError> {
    patterns
        .iter()
        .map(|p| {
            let matcher = CompiledPattern::compile(&p.pattern)?;
            let advice = p
                .advice
                .clone()
                .unwrap_or_else(|| default_advice(&p.action));
            Ok(CompiledEscapePattern {
                name: p.name.clone(),
                matcher,
                action: p.action,
                advice,
                comment: p.comment.clone(),
                threshold: p.threshold,
            })
        })
        .collect()
}
