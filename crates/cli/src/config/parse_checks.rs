// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Parsing functions for check configurations (agents, docs, toc).

use super::{AgentsConfig, AgentsScopeConfig, CheckLevel, DocsConfig, TocConfig};
use crate::checks::agents::config::{ContentRule, RequiredSection, SectionsConfig};

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

/// Parse docs configuration from TOML value.
pub(super) fn parse_docs_config(value: Option<&toml::Value>) -> DocsConfig {
    let Some(toml::Value::Table(t)) = value else {
        return DocsConfig::default();
    };

    let check = t.get("check").and_then(|v| v.as_str()).map(String::from);

    let toc = parse_toc_config(t.get("toc"));

    DocsConfig { check, toc }
}

/// Parse TOC configuration from TOML value.
fn parse_toc_config(value: Option<&toml::Value>) -> TocConfig {
    let Some(toml::Value::Table(t)) = value else {
        return TocConfig::default();
    };

    let check = t.get("check").and_then(|v| v.as_str()).map(String::from);

    let include = parse_string_array(t.get("include")).unwrap_or_else(TocConfig::default_include);

    let exclude = parse_string_array(t.get("exclude")).unwrap_or_else(TocConfig::default_exclude);

    TocConfig {
        check,
        include,
        exclude,
    }
}

/// Parse a TOML array of strings into a Vec<String>.
fn parse_string_array(value: Option<&toml::Value>) -> Option<Vec<String>> {
    value?.as_array().map(|arr| {
        arr.iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect()
    })
}
