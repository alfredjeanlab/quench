// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Parse helper functions for configuration.

use std::path::Path;

use super::{
    AgentsConfig, AgentsScopeConfig, CheckLevel, ClocConfig, EscapeAction, EscapePattern,
    EscapesConfig, GoConfig, GoPolicyConfig, GoSuppressConfig, LineMetric, LintChangesPolicy,
    RustConfig, RustPolicyConfig, ShellConfig, ShellPolicyConfig, ShellSuppressConfig,
    SuppressConfig, SuppressLevel, SuppressScopeConfig,
};
use crate::checks::agents::config::{ContentRule, RequiredSection, SectionsConfig};

/// Parse a TOML array of strings into a Vec<String>.
fn parse_string_array(value: Option<&toml::Value>) -> Option<Vec<String>> {
    value?.as_array().map(|arr| {
        arr.iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect()
    })
}

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

/// Parse Shell-specific configuration from TOML value.
pub(super) fn parse_shell_config(value: Option<&toml::Value>) -> ShellConfig {
    let Some(toml::Value::Table(t)) = value else {
        return ShellConfig::default();
    };

    let source = parse_string_array(t.get("source")).unwrap_or_else(ShellConfig::default_source);
    let tests = parse_string_array(t.get("tests")).unwrap_or_else(ShellConfig::default_tests);

    // Parse suppress config
    let suppress = parse_shell_suppress_config(t.get("suppress"));

    // Parse policy config
    let policy = parse_shell_policy_config(t.get("policy"));

    ShellConfig {
        source,
        tests,
        suppress,
        policy,
    }
}

/// Parse shell suppress configuration.
fn parse_shell_suppress_config(value: Option<&toml::Value>) -> ShellSuppressConfig {
    let Some(toml::Value::Table(t)) = value else {
        return ShellSuppressConfig::default();
    };

    let check = match t.get("check").and_then(|v| v.as_str()) {
        Some("forbid") => SuppressLevel::Forbid,
        Some("comment") => SuppressLevel::Comment,
        Some("allow") => SuppressLevel::Allow,
        _ => ShellSuppressConfig::default_check(),
    };

    let comment = t.get("comment").and_then(|v| v.as_str()).map(String::from);

    // Shell uses empty defaults (forbid level doesn't need patterns)
    let source = parse_suppress_scope_config_with_defaults(
        t.get("source"),
        ShellSuppressConfig::default_source(),
    );
    let test = t
        .get("test")
        .map(|v| {
            parse_suppress_scope_config_with_defaults(Some(v), ShellSuppressConfig::default_test())
        })
        .unwrap_or_else(ShellSuppressConfig::default_test);

    ShellSuppressConfig {
        check,
        comment,
        source,
        test,
    }
}

/// Parse shell policy configuration.
fn parse_shell_policy_config(value: Option<&toml::Value>) -> ShellPolicyConfig {
    let Some(toml::Value::Table(t)) = value else {
        return ShellPolicyConfig::default();
    };

    let lint_changes = match t.get("lint_changes").and_then(|v| v.as_str()) {
        Some("standalone") => LintChangesPolicy::Standalone,
        Some("none") | None => LintChangesPolicy::None,
        _ => LintChangesPolicy::None,
    };

    let lint_config = parse_string_array(t.get("lint_config"))
        .unwrap_or_else(ShellPolicyConfig::default_lint_config);
    ShellPolicyConfig {
        lint_changes,
        lint_config,
    }
}

/// Parse Go-specific configuration from TOML value.
pub(super) fn parse_go_config(value: Option<&toml::Value>) -> GoConfig {
    let Some(toml::Value::Table(t)) = value else {
        return GoConfig::default();
    };
    let source = parse_string_array(t.get("source")).unwrap_or_else(GoConfig::default_source);
    let tests = parse_string_array(t.get("tests")).unwrap_or_else(GoConfig::default_tests);

    // Parse suppress config
    let suppress = parse_go_suppress_config(t.get("suppress"));

    // Parse policy config
    let policy = parse_go_policy_config(t.get("policy"));

    GoConfig {
        source,
        tests,
        suppress,
        policy,
    }
}

/// Parse Go suppress configuration.
fn parse_go_suppress_config(value: Option<&toml::Value>) -> GoSuppressConfig {
    let Some(toml::Value::Table(t)) = value else {
        return GoSuppressConfig::default();
    };

    let check = match t.get("check").and_then(|v| v.as_str()) {
        Some("forbid") => SuppressLevel::Forbid,
        Some("comment") => SuppressLevel::Comment,
        Some("allow") => SuppressLevel::Allow,
        _ => GoSuppressConfig::default_check(),
    };

    let comment = t.get("comment").and_then(|v| v.as_str()).map(String::from);

    // Go uses empty defaults (no per-lint patterns yet)
    let source = parse_suppress_scope_config_with_defaults(
        t.get("source"),
        GoSuppressConfig::default_source(),
    );
    let test = t
        .get("test")
        .map(|v| {
            parse_suppress_scope_config_with_defaults(Some(v), GoSuppressConfig::default_test())
        })
        .unwrap_or_else(GoSuppressConfig::default_test);

    GoSuppressConfig {
        check,
        comment,
        source,
        test,
    }
}

/// Parse Go policy configuration.
fn parse_go_policy_config(value: Option<&toml::Value>) -> GoPolicyConfig {
    let Some(toml::Value::Table(t)) = value else {
        return GoPolicyConfig::default();
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
        .unwrap_or_else(GoPolicyConfig::default_lint_config);

    GoPolicyConfig {
        lint_changes,
        lint_config,
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

/// Parse scope-specific suppress configuration with language-specific defaults.
fn parse_suppress_scope_config(value: Option<&toml::Value>, is_test: bool) -> SuppressScopeConfig {
    let defaults = if is_test {
        SuppressScopeConfig::default_for_test()
    } else {
        SuppressScopeConfig::default_for_source()
    };
    parse_suppress_scope_config_with_defaults(value, defaults)
}

/// Parse scope-specific suppress configuration with explicit defaults.
fn parse_suppress_scope_config_with_defaults(
    value: Option<&toml::Value>,
    defaults: SuppressScopeConfig,
) -> SuppressScopeConfig {
    let Some(toml::Value::Table(t)) = value else {
        return defaults;
    };

    let is_test = defaults.check == Some(SuppressLevel::Allow);

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

    // Parse per-lint-code comment patterns.
    // Supports both:
    //   - Table form: [rust.suppress.source.dead_code] comment = "..."
    //   - Inline form: dead_code = ["// KEEP:", "// NOTE:"] or dead_code = "// KEEP:"
    let mut patterns = defaults.patterns;

    for (key, val) in t.iter() {
        // Skip known fields
        if matches!(key.as_str(), "check" | "allow" | "forbid") {
            continue;
        }

        match val {
            // Table form: [rust.suppress.source.dead_code] comment = "..."
            toml::Value::Table(lint_table) => {
                if let Some(comment_val) = lint_table.get("comment") {
                    let comment_patterns = parse_pattern_value(comment_val);
                    if !comment_patterns.is_empty() {
                        patterns.insert(key.clone(), comment_patterns);
                    }
                }
            }
            // Inline form: dead_code = "..." or dead_code = ["...", "..."]
            _ => {
                let comment_patterns = parse_pattern_value(val);
                if !comment_patterns.is_empty() {
                    patterns.insert(key.clone(), comment_patterns);
                }
            }
        }
    }

    SuppressScopeConfig {
        check,
        allow,
        forbid,
        patterns,
    }
}

/// Parse a pattern value that can be either a string or array of strings.
fn parse_pattern_value(value: &toml::Value) -> Vec<String> {
    match value {
        toml::Value::String(s) => vec![s.clone()],
        toml::Value::Array(arr) => arr
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect(),
        _ => Vec::new(),
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

/// Parse agents configuration from TOML value.
pub(super) fn parse_agents_config(value: Option<&toml::Value>) -> AgentsConfig {
    let Some(toml::Value::Table(t)) = value else {
        return AgentsConfig::default();
    };

    let check = match t.get("check").and_then(|v| v.as_str()) {
        Some("error") => CheckLevel::Error,
        Some("warn") => CheckLevel::Warn,
        Some("off") => CheckLevel::Off,
        _ => CheckLevel::default(),
    };

    let files = t
        .get("files")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_else(AgentsConfig::default_files);

    let required = t
        .get("required")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    let optional = t
        .get("optional")
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

    let sync = t.get("sync").and_then(|v| v.as_bool()).unwrap_or(false);

    let sync_source = t
        .get("sync_source")
        .and_then(|v| v.as_str())
        .map(String::from);

    let sections = parse_sections_config(t.get("sections"));

    let tables = parse_content_rule(t.get("tables")).unwrap_or_default();
    let box_diagrams = parse_content_rule(t.get("box_diagrams")).unwrap_or_else(ContentRule::allow);
    let mermaid = parse_content_rule(t.get("mermaid")).unwrap_or_else(ContentRule::allow);

    let max_lines = t
        .get("max_lines")
        .and_then(|v| v.as_integer())
        .map(|v| v as usize);

    let max_tokens = t
        .get("max_tokens")
        .and_then(|v| v.as_integer())
        .map(|v| v as usize);

    let root = t.get("root").map(parse_agents_scope_config);
    let package = t.get("package").map(parse_agents_scope_config);
    let module = t.get("module").map(parse_agents_scope_config);

    AgentsConfig {
        check,
        files,
        required,
        optional,
        forbid,
        sync,
        sync_source,
        sections,
        tables,
        box_diagrams,
        mermaid,
        max_lines,
        max_tokens,
        root,
        package,
        module,
    }
}

/// Parse a content rule from TOML value.
fn parse_content_rule(value: Option<&toml::Value>) -> Option<ContentRule> {
    let s = value?.as_str()?;
    match s {
        "allow" => Some(ContentRule::Allow),
        "forbid" => Some(ContentRule::Forbid),
        _ => None,
    }
}

/// Parse a scope-specific agents configuration.
fn parse_agents_scope_config(value: &toml::Value) -> AgentsScopeConfig {
    let Some(t) = value.as_table() else {
        return AgentsScopeConfig::default();
    };

    let required = t
        .get("required")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    let optional = t
        .get("optional")
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

    let max_lines = t
        .get("max_lines")
        .and_then(|v| v.as_integer())
        .map(|v| v as usize);

    let max_tokens = t
        .get("max_tokens")
        .and_then(|v| v.as_integer())
        .map(|v| v as usize);

    AgentsScopeConfig {
        required,
        optional,
        forbid,
        max_lines,
        max_tokens,
    }
}

/// Parse sections configuration from TOML value.
fn parse_sections_config(value: Option<&toml::Value>) -> SectionsConfig {
    let Some(toml::Value::Table(t)) = value else {
        return SectionsConfig::default();
    };

    let required = t
        .get("required")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(parse_required_section).collect())
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

    SectionsConfig { required, forbid }
}

/// Parse a single required section from TOML value.
fn parse_required_section(value: &toml::Value) -> Option<RequiredSection> {
    match value {
        // Simple form: just a string name
        toml::Value::String(name) => Some(RequiredSection {
            name: name.clone(),
            advice: None,
        }),
        // Extended form: table with name and advice
        toml::Value::Table(t) => {
            let name = t.get("name")?.as_str()?.to_string();
            let advice = t.get("advice").and_then(|v| v.as_str()).map(String::from);
            Some(RequiredSection { name, advice })
        }
        _ => None,
    }
}
