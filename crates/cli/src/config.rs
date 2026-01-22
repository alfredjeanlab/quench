//! Configuration parsing and validation.
//!
//! Handles quench.toml parsing with version validation and unknown key warnings.

use std::collections::BTreeSet;
use std::path::Path;

use serde::Deserialize;

use crate::error::{Error, Result};

/// Minimum config structure for version checking.
#[derive(Deserialize)]
struct VersionOnly {
    version: Option<i64>,
}

/// Config with flexible parsing that captures unknown keys.
#[derive(Deserialize)]
struct FlexibleConfig {
    version: i64,

    #[serde(default)]
    project: Option<toml::Value>,

    #[serde(flatten)]
    unknown: std::collections::BTreeMap<String, toml::Value>,
}

/// Full configuration.
#[derive(Debug, Default, Deserialize)]
pub struct Config {
    /// Config file version (must be 1).
    pub version: i64,

    /// Project configuration.
    #[serde(default)]
    pub project: ProjectConfig,
}

/// Project-level configuration.
#[derive(Debug, Default, Deserialize)]
pub struct ProjectConfig {
    /// Project name.
    pub name: Option<String>,

    /// Custom ignore patterns.
    #[serde(default)]
    pub ignore: IgnoreConfig,
}

/// Ignore pattern configuration.
#[derive(Debug, Default, Clone, Deserialize)]
pub struct IgnoreConfig {
    /// Glob patterns to ignore (e.g., "*.snapshot", "testdata/", "**/fixtures/**").
    #[serde(default)]
    pub patterns: Vec<String>,
}

/// Currently supported config version.
pub const SUPPORTED_VERSION: i64 = 1;

/// Known top-level keys in the config.
const KNOWN_KEYS: &[&str] = &["version", "project", "check"];

/// Known project keys in the config.
const KNOWN_PROJECT_KEYS: &[&str] = &["name", "ignore"];

/// Load and validate config from a file path.
pub fn load(path: &Path) -> Result<Config> {
    let content = std::fs::read_to_string(path).map_err(|e| Error::Io {
        path: path.to_path_buf(),
        source: e,
    })?;

    parse(&content, path)
}

/// Load config with warnings for unknown keys.
pub fn load_with_warnings(path: &Path) -> Result<Config> {
    let content = std::fs::read_to_string(path).map_err(|e| Error::Io {
        path: path.to_path_buf(),
        source: e,
    })?;

    parse_with_warnings(&content, path)
}

/// Parse config from string content (strict mode).
pub fn parse(content: &str, path: &Path) -> Result<Config> {
    // First check version
    let version_check: VersionOnly = toml::from_str(content).map_err(|e| Error::Config {
        message: e.to_string(),
        path: Some(path.to_path_buf()),
    })?;

    let version = version_check.version.ok_or_else(|| Error::Config {
        message: "missing required field: version".to_string(),
        path: Some(path.to_path_buf()),
    })?;

    if version != SUPPORTED_VERSION {
        return Err(Error::Config {
            message: format!(
                "unsupported config version {} (supported: {})\n  Upgrade quench to use this config.",
                version, SUPPORTED_VERSION
            ),
            path: Some(path.to_path_buf()),
        });
    }

    // Parse full config
    toml::from_str(content).map_err(|e| Error::Config {
        message: e.to_string(),
        path: Some(path.to_path_buf()),
    })
}

/// Parse config, warning on unknown keys.
pub fn parse_with_warnings(content: &str, path: &Path) -> Result<Config> {
    // First validate version
    let flexible: FlexibleConfig = toml::from_str(content).map_err(|e| Error::Config {
        message: e.to_string(),
        path: Some(path.to_path_buf()),
    })?;

    if flexible.version != SUPPORTED_VERSION {
        return Err(Error::Config {
            message: format!(
                "unsupported config version {} (supported: {})",
                flexible.version, SUPPORTED_VERSION
            ),
            path: Some(path.to_path_buf()),
        });
    }

    // Collect unknown keys
    let mut unknown_keys = BTreeSet::new();

    // Check top-level unknown keys
    for key in flexible.unknown.keys() {
        if !KNOWN_KEYS.contains(&key.as_str()) {
            unknown_keys.insert(key.clone());
        }
    }

    // Check unknown keys in check section (if present)
    if let Some(toml::Value::Table(check_table)) = flexible.unknown.get("check") {
        for key in check_table.keys() {
            unknown_keys.insert(format!("check.{}", key));
        }
    }

    // Warn about unknown keys
    for key in &unknown_keys {
        warn_unknown_key(path, key);
    }

    // Return a valid config with known fields
    let project = match flexible.project {
        Some(toml::Value::Table(t)) => {
            let name = t
                .get("name")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            // Parse ignore patterns
            let ignore = match t.get("ignore") {
                Some(toml::Value::Table(ignore_table)) => {
                    let patterns = ignore_table
                        .get("patterns")
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| v.as_str().map(String::from))
                                .collect()
                        })
                        .unwrap_or_default();

                    // Warn about unknown ignore fields
                    for key in ignore_table.keys() {
                        if key != "patterns" {
                            warn_unknown_key(path, &format!("project.ignore.{}", key));
                        }
                    }

                    IgnoreConfig { patterns }
                }
                _ => IgnoreConfig::default(),
            };

            // Warn about unknown project fields
            for key in t.keys() {
                if !KNOWN_PROJECT_KEYS.contains(&key.as_str()) {
                    warn_unknown_key(path, &format!("project.{}", key));
                }
            }

            ProjectConfig { name, ignore }
        }
        _ => ProjectConfig::default(),
    };

    Ok(Config {
        version: flexible.version,
        project,
    })
}

fn warn_unknown_key(path: &Path, key: &str) {
    eprintln!(
        "quench: warning: {}: unrecognized field `{}` (ignored)",
        path.display(),
        key
    );
}

#[cfg(test)]
#[path = "config_tests.rs"]
mod tests;
