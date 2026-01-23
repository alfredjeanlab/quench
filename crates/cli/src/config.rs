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
    workspace: Option<toml::Value>,

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

    /// Workspace configuration (for monorepos).
    #[serde(default)]
    pub workspace: WorkspaceConfig,

    /// Check configurations.
    #[serde(default)]
    pub check: CheckConfig,
}

/// Workspace configuration for monorepos.
#[derive(Debug, Default, Clone, Deserialize)]
pub struct WorkspaceConfig {
    /// Package directories within the workspace.
    #[serde(default)]
    pub packages: Vec<String>,

    /// Package name lookup (path -> name).
    /// Auto-populated when detecting Rust workspaces.
    #[serde(default)]
    pub package_names: std::collections::HashMap<String, String>,
}

/// Check-specific configurations.
#[derive(Debug, Default, Deserialize)]
pub struct CheckConfig {
    /// Cloc (count lines of code) check configuration.
    #[serde(default)]
    pub cloc: ClocConfig,

    /// Escapes (escape hatches) check configuration.
    #[serde(default)]
    pub escapes: EscapesConfig,
}

/// Escapes check configuration.
#[derive(Debug, Default, Deserialize)]
pub struct EscapesConfig {
    /// Check level: error, warn, or off.
    #[serde(default)]
    pub check: CheckLevel,

    /// Patterns to detect (overrides defaults).
    #[serde(default)]
    pub patterns: Vec<EscapePattern>,
}

/// A single escape hatch pattern definition.
#[derive(Debug, Clone, Deserialize)]
pub struct EscapePattern {
    /// Unique name for this pattern (e.g., "unwrap", "unsafe").
    pub name: String,

    /// Regex pattern to match.
    pub pattern: String,

    /// Action to take: count, comment, or forbid.
    #[serde(default)]
    pub action: EscapeAction,

    /// Required comment pattern for action = "comment".
    #[serde(default)]
    pub comment: Option<String>,

    /// Count threshold for action = "count" (default: 0).
    #[serde(default)]
    pub threshold: usize,

    /// Custom advice message for violations.
    #[serde(default)]
    pub advice: Option<String>,
}

/// Action to take when pattern is matched.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EscapeAction {
    #[default]
    Forbid,
    Comment,
    Count,
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

    /// Test file patterns (default: common test directory/file patterns).
    #[serde(default = "ClocConfig::default_test_patterns")]
    pub test_patterns: Vec<String>,

    /// Patterns to exclude from size limit checks.
    #[serde(default)]
    pub exclude: Vec<String>,

    /// Maximum tokens per file (default: 20000, None = disabled).
    #[serde(default = "ClocConfig::default_max_tokens")]
    pub max_tokens: Option<usize>,

    /// Advice message for source file violations.
    #[serde(default = "ClocConfig::default_advice")]
    pub advice: String,

    /// Advice message for test file violations.
    #[serde(default = "ClocConfig::default_advice_test")]
    pub advice_test: String,
}

impl Default for ClocConfig {
    fn default() -> Self {
        Self {
            max_lines: Self::default_max_lines(),
            max_lines_test: Self::default_max_lines_test(),
            check: CheckLevel::default(),
            test_patterns: Self::default_test_patterns(),
            exclude: Vec::new(),
            max_tokens: Self::default_max_tokens(),
            advice: Self::default_advice(),
            advice_test: Self::default_advice_test(),
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

    fn default_max_tokens() -> Option<usize> {
        Some(20000)
    }

    fn default_test_patterns() -> Vec<String> {
        vec![
            "**/tests/**".to_string(),
            "**/test/**".to_string(),
            "**/*_test.*".to_string(),
            "**/*_tests.*".to_string(),
            "**/*.test.*".to_string(),
            "**/*.spec.*".to_string(),
            "**/test_*.*".to_string(),
        ]
    }

    fn default_advice() -> String {
        "Can the code be made more concise? If not, split large source files into sibling modules or submodules in a folder; consider refactoring to be more unit testable.".to_string()
    }

    fn default_advice_test() -> String {
        "Can tests be parameterized or use shared fixtures to be more concise? If not, split large test files into a folder.".to_string()
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

    /// Source file patterns (default: empty = all non-test files are source).
    #[serde(default)]
    pub source: Vec<String>,

    /// Test file patterns (default: common test directory/file patterns).
    #[serde(default = "ProjectConfig::default_test_patterns")]
    pub tests: Vec<String>,

    /// Custom ignore patterns.
    #[serde(default)]
    pub ignore: IgnoreConfig,
}

impl ProjectConfig {
    /// Default test patterns matching common conventions.
    fn default_test_patterns() -> Vec<String> {
        vec![
            "**/tests/**".to_string(),
            "**/test/**".to_string(),
            "**/*_test.*".to_string(),
            "**/*_tests.*".to_string(),
            "**/*.test.*".to_string(),
            "**/*.spec.*".to_string(),
        ]
    }
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
const KNOWN_KEYS: &[&str] = &["version", "project", "workspace", "check"];

/// Known project keys in the config.
const KNOWN_PROJECT_KEYS: &[&str] = &["name", "source", "tests", "ignore"];

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

            // Parse source patterns
            let source = t
                .get("source")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();

            // Parse test patterns
            let tests = t
                .get("tests")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_else(ProjectConfig::default_test_patterns);

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

            ProjectConfig {
                name,
                source,
                tests,
                ignore,
            }
        }
        _ => ProjectConfig::default(),
    };

    // Parse workspace config
    let workspace = match flexible.workspace {
        Some(toml::Value::Table(t)) => {
            let packages = t
                .get("packages")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();

            WorkspaceConfig {
                packages,
                package_names: std::collections::HashMap::new(),
            }
        }
        _ => WorkspaceConfig::default(),
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

                    let test_patterns = cloc_table
                        .get("test_patterns")
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| v.as_str().map(String::from))
                                .collect()
                        })
                        .unwrap_or_else(ClocConfig::default_test_patterns);

                    let exclude = cloc_table
                        .get("exclude")
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| v.as_str().map(String::from))
                                .collect()
                        })
                        .unwrap_or_default();

                    let max_tokens = cloc_table
                        .get("max_tokens")
                        .map(|v| {
                            if v.as_bool() == Some(false) {
                                None // max_tokens = false disables the check
                            } else {
                                v.as_integer().map(|n| n as usize)
                            }
                        })
                        .unwrap_or_else(ClocConfig::default_max_tokens);

                    let advice = cloc_table
                        .get("advice")
                        .and_then(|v| v.as_str())
                        .map(String::from)
                        .unwrap_or_else(ClocConfig::default_advice);

                    let advice_test = cloc_table
                        .get("advice_test")
                        .and_then(|v| v.as_str())
                        .map(String::from)
                        .unwrap_or_else(ClocConfig::default_advice_test);

                    ClocConfig {
                        max_lines,
                        max_lines_test,
                        check,
                        test_patterns,
                        exclude,
                        max_tokens,
                        advice,
                        advice_test,
                    }
                }
                _ => ClocConfig::default(),
            };

            // Parse escapes config
            let escapes = parse_escapes_config(t.get("escapes"));

            CheckConfig { cloc, escapes }
        }
        _ => CheckConfig::default(),
    };

    Ok(Config {
        version: flexible.version,
        project,
        workspace,
        check,
    })
}

/// Parse escapes configuration from TOML value.
fn parse_escapes_config(value: Option<&toml::Value>) -> EscapesConfig {
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
