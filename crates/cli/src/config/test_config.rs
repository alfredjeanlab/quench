//! Test suite configuration.

use serde::Deserialize;

use super::duration;

/// Tests check configuration.
#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TestsConfig {
    /// Check level: "error" | "warn" | "off"
    pub check: Option<String>,

    /// Commit message validation settings.
    #[serde(default)]
    pub commit: TestsCommitConfig,

    /// Test suites to run.
    #[serde(default)]
    pub suite: Vec<TestSuiteConfig>,

    /// Time limit checking.
    #[serde(default)]
    pub time: TestsTimeConfig,
}

/// Configuration for a single test suite.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TestSuiteConfig {
    /// Runner name: "cargo", "bats", "pytest", etc.
    pub runner: String,

    /// Name for custom runners (optional, defaults to runner).
    #[serde(default)]
    pub name: Option<String>,

    /// Test directory or file pattern.
    #[serde(default)]
    pub path: Option<String>,

    /// Command to run before tests.
    #[serde(default)]
    pub setup: Option<String>,

    /// Custom command for unsupported runners.
    #[serde(default)]
    pub command: Option<String>,

    /// Coverage targets (binary names or glob patterns).
    #[serde(default)]
    pub targets: Vec<String>,

    /// Only run in CI mode.
    #[serde(default)]
    pub ci: bool,

    /// Maximum total time for this suite.
    #[serde(default, deserialize_with = "duration::deserialize_option")]
    pub max_total: Option<std::time::Duration>,

    /// Maximum average time per test.
    #[serde(default, deserialize_with = "duration::deserialize_option")]
    pub max_avg: Option<std::time::Duration>,

    /// Maximum time for slowest individual test.
    #[serde(default, deserialize_with = "duration::deserialize_option")]
    pub max_test: Option<std::time::Duration>,
}

/// Time limit configuration for test suites.
#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TestsTimeConfig {
    /// Check level: "error" | "warn" | "off"
    #[serde(default = "TestsTimeConfig::default_check")]
    pub check: String,
}

impl Default for TestsTimeConfig {
    fn default() -> Self {
        Self {
            check: Self::default_check(),
        }
    }
}

impl TestsTimeConfig {
    fn default_check() -> String {
        "warn".to_string()
    }
}

/// Tests commit check configuration.
#[derive(Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TestsCommitConfig {
    /// Check level: "error" | "warn" | "off"
    #[serde(default = "TestsCommitConfig::default_check")]
    pub check: String,

    /// Scope: "branch" | "commit"
    #[serde(default = "TestsCommitConfig::default_scope")]
    pub scope: String,

    /// Placeholder handling: "allow" | "forbid"
    #[serde(default = "TestsCommitConfig::default_placeholders")]
    pub placeholders: String,

    /// Test file patterns (extends defaults).
    #[serde(default = "TestsCommitConfig::default_test_patterns")]
    pub test_patterns: Vec<String>,

    /// Source file patterns.
    #[serde(default = "TestsCommitConfig::default_source_patterns")]
    pub source_patterns: Vec<String>,

    /// Excluded patterns (never require tests).
    #[serde(default = "TestsCommitConfig::default_exclude")]
    pub exclude: Vec<String>,
}

impl Default for TestsCommitConfig {
    fn default() -> Self {
        Self {
            check: Self::default_check(),
            scope: Self::default_scope(),
            placeholders: Self::default_placeholders(),
            test_patterns: Self::default_test_patterns(),
            source_patterns: Self::default_source_patterns(),
            exclude: Self::default_exclude(),
        }
    }
}

impl TestsCommitConfig {
    fn default_check() -> String {
        "off".to_string()
    }

    fn default_scope() -> String {
        "branch".to_string()
    }

    fn default_placeholders() -> String {
        "allow".to_string()
    }

    fn default_test_patterns() -> Vec<String> {
        vec![
            // Directory-based patterns
            "tests/**/*".to_string(),
            "test/**/*".to_string(),
            "spec/**/*".to_string(),
            "**/__tests__/**".to_string(),
            // Suffix patterns (underscore)
            "**/*_test.*".to_string(),
            "**/*_tests.*".to_string(),
            // Suffix patterns (dot)
            "**/*.test.*".to_string(),
            "**/*.spec.*".to_string(),
            // Prefix patterns
            "**/test_*.*".to_string(),
        ]
    }

    fn default_source_patterns() -> Vec<String> {
        vec!["src/**/*".to_string()]
    }

    fn default_exclude() -> Vec<String> {
        vec![
            "**/mod.rs".to_string(),
            "**/lib.rs".to_string(),
            "**/main.rs".to_string(),
            "**/generated/**".to_string(),
        ]
    }
}
