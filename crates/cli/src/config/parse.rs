// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Parse helper functions for configuration.

use std::path::Path;

use super::{
    CheckLevel, ClocConfig, EscapeAction, EscapePattern, EscapesConfig, LineMetric,
    LintChangesPolicy, RustConfig, RustPolicyConfig, SuppressConfig, SuppressLevel,
    SuppressScopeConfig,
};

/// Parse Rust-specific configuration from TOML value.
pub(super) fn parse_rust_config(value: Option<&toml::Value>) -> RustConfig {
    let Some(toml::Value::Table(t)) = value else {
        return RustConfig::default();
    };

    let cfg_test_split = t
        .get("cfg_test_split")
        .and_then(|v| v.as_bool())
        .unwrap_or_else(RustConfig::default_cfg_test_split);

    let suppress = parse_suppress_config(t.get("suppress"));
    let policy = parse_policy_config(t.get("policy"));

    RustConfig {
        cfg_test_split,
        suppress,
        policy,
    }
}

/// Parse lint policy configuration from TOML value.
fn parse_policy_config(value: Option<&toml::Value>) -> RustPolicyConfig {
    let Some(toml::Value::Table(t)) = value else {
        return RustPolicyConfig::default();
    };

    let lint_changes = match t.get("lint_changes").and_then(|v| v.as_str()) {
        Some("standalone") => LintChangesPolicy::Standalone,
        Some("none") | None => LintChangesPolicy::None,
        _ => LintChangesPolicy::None,
    };

    let lint_config = t
        .get("lint_config")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_else(RustPolicyConfig::default_lint_config);

    RustPolicyConfig {
        lint_changes,
        lint_config,
    }
}

/// Parse suppress configuration from TOML value.
fn parse_suppress_config(value: Option<&toml::Value>) -> SuppressConfig {
    let Some(toml::Value::Table(t)) = value else {
        return SuppressConfig::default();
    };

    let check = match t.get("check").and_then(|v| v.as_str()) {
        Some("forbid") => SuppressLevel::Forbid,
        Some("comment") => SuppressLevel::Comment,
        Some("allow") => SuppressLevel::Allow,
        _ => SuppressConfig::default_check(),
    };

    let comment = t.get("comment").and_then(|v| v.as_str()).map(String::from);

    let source = parse_suppress_scope_config(t.get("source"), false);
    let test = parse_suppress_scope_config(t.get("test"), true);

    SuppressConfig {
        check,
        comment,
        source,
        test,
    }
}

/// Parse scope-specific suppress configuration.
fn parse_suppress_scope_config(value: Option<&toml::Value>, is_test: bool) -> SuppressScopeConfig {
    let Some(toml::Value::Table(t)) = value else {
        return if is_test {
            SuppressScopeConfig::default_for_test()
        } else {
            SuppressScopeConfig::default()
        };
    };

    let check = match t.get("check").and_then(|v| v.as_str()) {
        Some("forbid") => Some(SuppressLevel::Forbid),
        Some("comment") => Some(SuppressLevel::Comment),
        Some("allow") => Some(SuppressLevel::Allow),
        _ if is_test => Some(SuppressLevel::Allow),
        _ => None,
    };

    let allow = t
        .get("allow")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    let forbid = t
        .get("forbid")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    SuppressScopeConfig {
        check,
        allow,
        forbid,
    }
}

/// Parse cloc configuration from TOML value.
pub(super) fn parse_cloc_config(value: Option<&toml::Value>) -> ClocConfig {
    let Some(toml::Value::Table(t)) = value else {
        return ClocConfig::default();
    };

    let max_lines = t
        .get("max_lines")
        .and_then(|v| v.as_integer())
        .map(|v| v as usize)
        .unwrap_or_else(ClocConfig::default_max_lines);

    let max_lines_test = t
        .get("max_lines_test")
        .and_then(|v| v.as_integer())
        .map(|v| v as usize)
        .unwrap_or_else(ClocConfig::default_max_lines_test);

    let check = match t.get("check").and_then(|v| v.as_str()) {
        Some("error") => CheckLevel::Error,
        Some("warn") => CheckLevel::Warn,
        Some("off") => CheckLevel::Off,
        _ => CheckLevel::default(),
    };

    let test_patterns = t
        .get("test_patterns")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_else(ClocConfig::default_test_patterns);

    let exclude = t
        .get("exclude")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    let max_tokens = t
        .get("max_tokens")
        .map(|v| {
            if v.as_bool() == Some(false) {
                None // max_tokens = false disables the check
            } else {
                v.as_integer().map(|n| n as usize)
            }
        })
        .unwrap_or_else(ClocConfig::default_max_tokens);

    let advice = t
        .get("advice")
        .and_then(|v| v.as_str())
        .map(String::from)
        .unwrap_or_else(ClocConfig::default_advice);

    let advice_test = t
        .get("advice_test")
        .and_then(|v| v.as_str())
        .map(String::from)
        .unwrap_or_else(ClocConfig::default_advice_test);

    let metric = match t.get("metric").and_then(|v| v.as_str()) {
        Some("nonblank") => LineMetric::Nonblank,
        _ => LineMetric::Lines,
    };

    ClocConfig {
        max_lines,
        max_lines_test,
        metric,
        check,
        test_patterns,
        exclude,
        max_tokens,
        advice,
        advice_test,
    }
}

/// Parse escapes configuration from TOML value.
pub(super) fn parse_escapes_config(value: Option<&toml::Value>) -> EscapesConfig {
    let Some(toml::Value::Table(t)) = value else {
        return EscapesConfig::default();
    };

    let check = match t.get("check").and_then(|v| v.as_str()) {
        Some("error") => CheckLevel::Error,
        Some("warn") => CheckLevel::Warn,
        Some("off") => CheckLevel::Off,
        _ => CheckLevel::default(),
    };

    let patterns = t
        .get("patterns")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(parse_escape_pattern).collect())
        .unwrap_or_default();

    EscapesConfig { check, patterns }
}

/// Parse a single escape pattern from TOML value.
fn parse_escape_pattern(value: &toml::Value) -> Option<EscapePattern> {
    let t = value.as_table()?;

    let name = t.get("name")?.as_str()?.to_string();
    let pattern = t.get("pattern")?.as_str()?.to_string();

    let action = match t.get("action").and_then(|v| v.as_str()) {
        Some("forbid") => EscapeAction::Forbid,
        Some("comment") => EscapeAction::Comment,
        Some("count") => EscapeAction::Count,
        _ => EscapeAction::default(),
    };

    let comment = t.get("comment").and_then(|v| v.as_str()).map(String::from);

    let threshold = t
        .get("threshold")
        .and_then(|v| v.as_integer())
        .map(|v| v as usize)
        .unwrap_or(0);

    let advice = t.get("advice").and_then(|v| v.as_str()).map(String::from);

    Some(EscapePattern {
        name,
        pattern,
        action,
        comment,
        threshold,
        advice,
    })
}

/// Warn about unknown configuration key.
pub(super) fn warn_unknown_key(path: &Path, key: &str) {
    eprintln!(
        "quench: warning: {}: unrecognized field `{}` (ignored)",
        path.display(),
        key
    );
}
