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

    #[serde(default)]
    check: Option<toml::Value>,

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

    /// Check configurations.
    #[serde(default)]
    pub check: CheckConfig,
}

/// Check-specific configurations.
#[derive(Debug, Default, Deserialize)]
pub struct CheckConfig {
    /// Cloc (count lines of code) check configuration.
    #[serde(default)]
    pub cloc: ClocConfig,
}

/// Cloc check configuration.
#[derive(Debug, Deserialize)]
pub struct ClocConfig {
    /// Maximum lines per file (default: 750).
    #[serde(default = "ClocConfig::default_max_lines")]
    pub max_lines: usize,

    /// Maximum lines per test file (default: 1100).
    #[serde(default = "ClocConfig::default_max_lines_test")]
    pub max_lines_test: usize,

    /// Check level: error, warn, or off.
    #[serde(default)]
    pub check: CheckLevel,
}

impl Default for ClocConfig {
    fn default() -> Self {
        Self {
            max_lines: Self::default_max_lines(),
            max_lines_test: Self::default_max_lines_test(),
            check: CheckLevel::default(),
        }
    }
}

impl ClocConfig {
    fn default_max_lines() -> usize {
        750
    }

    fn default_max_lines_test() -> usize {
        1100
    }
}

/// Check level: error, warn, or off.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CheckLevel {
    #[default]
    Error,
    Warn,
    Off,
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

    // Parse check config
    let check = match flexible.check {
        Some(toml::Value::Table(t)) => {
            // Known check types
            const KNOWN_CHECKS: &[&str] = &[
                "cloc", "escapes", "agents", "docs", "tests", "git", "build", "license",
            ];

            // Warn about unknown check types
            for key in t.keys() {
                if !KNOWN_CHECKS.contains(&key.as_str()) {
                    warn_unknown_key(path, &format!("check.{}", key));
                }
            }

            // Parse cloc config
            let cloc = match t.get("cloc") {
                Some(toml::Value::Table(cloc_table)) => {
                    let max_lines = cloc_table
                        .get("max_lines")
                        .and_then(|v| v.as_integer())
                        .map(|v| v as usize)
                        .unwrap_or_else(ClocConfig::default_max_lines);

                    let max_lines_test = cloc_table
                        .get("max_lines_test")
                        .and_then(|v| v.as_integer())
                        .map(|v| v as usize)
                        .unwrap_or_else(ClocConfig::default_max_lines_test);

                    let check = match cloc_table.get("check").and_then(|v| v.as_str()) {
                        Some("error") => CheckLevel::Error,
                        Some("warn") => CheckLevel::Warn,
                        Some("off") => CheckLevel::Off,
                        _ => CheckLevel::default(),
                    };

                    ClocConfig {
                        max_lines,
                        max_lines_test,
                        check,
                    }
                }
                _ => ClocConfig::default(),
            };

            CheckConfig { cloc }
        }
        _ => CheckConfig::default(),
    };

    Ok(Config {
        version: flexible.version,
        project,
        check,
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
