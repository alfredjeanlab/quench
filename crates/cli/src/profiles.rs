// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Profile defaults and templates for quench init command.
//!
//! Provides configuration templates for various languages and agent types.

use std::path::Path;

use crate::adapter::javascript::PackageManager;
use crate::adapter::python::PythonTooling;
use crate::init::{CursorMarker, DetectedAgent};

// =============================================================================
// PROFILE DEFAULTS
// =============================================================================

/// Claude agent profile configuration for quench init.
///
/// Sets up [check.agents] with CLAUDE.md as required.
pub fn claude_profile_defaults() -> &'static str {
    r#"[check.agents]
check = "error"
required = ["CLAUDE.md"]
"#
}

/// Cursor agent profile configuration for quench init.
///
/// Sets up [check.agents] with .cursorrules as required.
pub fn cursor_profile_defaults() -> &'static str {
    r#"[check.agents]
check = "error"
required = [".cursorrules"]
"#
}

/// JavaScript profile configuration for quench init.
pub fn javascript_profile_defaults() -> String {
    r#"# See: `quench config javascript`
[javascript]
# source = ["**/*.js", "**/*.jsx", "**/*.ts", "**/*.tsx", "**/*.mjs", "**/*.mts"]
# tests = ["**/*.test.*", "**/*.spec.*", "**/test/**", "**/tests/**", "**/__tests__/**"]

[javascript.suppress]
check = "comment"

[javascript.policy]
lint_changes = "standalone"
"#
    .to_string()
}

/// JavaScript-specific Landing the Plane checklist items.
///
/// Returns default npm-based items for consistency with other languages.
pub fn javascript_landing_items() -> &'static [&'static str] {
    &[
        "npm run lint",
        "npm run typecheck",
        "npm test",
        "npm run build",
    ]
}

/// JavaScript-specific Landing the Plane checklist items with package manager detection.
///
/// Detects the package manager from lock files and returns appropriate commands.
pub fn javascript_landing_items_for(root: &Path) -> Vec<String> {
    let pkg_mgr = PackageManager::detect(root);
    let exe = pkg_mgr.executable();

    // Helper to generate run commands - yarn uses `yarn <script>` without "run"
    let run_cmd = |script: &str| {
        if pkg_mgr == PackageManager::Yarn {
            format!("yarn {script}")
        } else {
            format!("{exe} run {script}")
        }
    };

    vec![
        run_cmd("lint"),
        run_cmd("typecheck"),
        format!("{exe} test"),
        run_cmd("build"),
    ]
}

/// Default Rust profile configuration for quench init.
pub fn rust_profile_defaults() -> String {
    r#"# See: `quench config rust`
[rust]
# source = ["**/*.rs"]
# tests = ["**/tests/**/*.rs", "**/*_test.rs", "**/*_tests.rs"]
cfg_test_split = "count"  # recommended: "require"

[rust.suppress]
check = "comment"

[rust.policy]
lint_changes = "standalone"
"#
    .to_string()
}

/// Rust-specific Landing the Plane checklist items.
pub fn rust_landing_items() -> &'static [&'static str] {
    &[
        "cargo fmt --check",
        "cargo clippy -- -D warnings",
        "cargo test",
        "cargo build",
    ]
}

/// Default Shell profile configuration for quench init.
pub fn shell_profile_defaults() -> String {
    r#"# See: `quench config shell`
[shell]
# source = ["**/*.sh", "**/*.bash"]
# tests = ["tests/**/*.bats", "test/**/*.bats", "*_test.sh", "**/*_test.sh"]

[shell.suppress]
check = "comment"

[shell.policy]
lint_changes = "standalone"
"#
    .to_string()
}

/// Shell-specific Landing the Plane checklist items.
pub fn shell_landing_items() -> &'static [&'static str] {
    &["shellcheck **/*.sh", "bats tests/"]
}

/// Default Go profile configuration for quench init.
pub fn golang_profile_defaults() -> String {
    r#"# See: `quench config golang`
[golang]
# source = ["**/*.go"]
# tests = ["**/*_test.go"]

[golang.suppress]
check = "comment"

[golang.policy]
lint_changes = "standalone"
"#
    .to_string()
}

/// Go-specific Landing the Plane checklist items.
pub fn golang_landing_items() -> &'static [&'static str] {
    &[
        "go fmt ./...",
        "go vet ./...",
        "golangci-lint run",
        "go test ./...",
        "go build ./...",
    ]
}

/// Ruby profile configuration for quench init.
pub fn ruby_profile_defaults() -> String {
    r#"# See: `quench config ruby`
[ruby]
# source = ["**/*.rb"]
# tests = ["**/test/**/*.rb", "**/spec/**/*.rb", "**/*_test.rb", "**/*_spec.rb"]

[ruby.suppress]
check = "comment"

[ruby.policy]
lint_changes = "standalone"
"#
    .to_string()
}

/// Ruby-specific Landing the Plane checklist items.
pub fn ruby_landing_items() -> &'static [&'static str] {
    &[
        "bundle exec rubocop",
        "bundle exec rspec",
        "bundle exec rake test",
    ]
}

/// Python profile configuration for quench init.
pub fn python_profile_defaults() -> String {
    r#"# See: `quench config python`
[python]
# source = ["**/*.py"]
# tests = ["tests/**/*.py", "test/**/*.py", "test_*.py", "*_test.py", "conftest.py"]

[python.suppress]
check = "comment"

[python.policy]
lint_changes = "standalone"
"#
    .to_string()
}

/// Python-specific Landing the Plane checklist items.
///
/// Returns default items assuming common tooling (ruff, mypy, pytest).
pub fn python_landing_items() -> &'static [&'static str] {
    &["ruff check .", "ruff format --check .", "mypy .", "pytest"]
}

/// Python-specific Landing the Plane checklist items with tooling detection.
///
/// Detects the configured tools and returns appropriate commands.
/// Only includes items for tools that are actually configured.
pub fn python_landing_items_for(root: &Path) -> Vec<String> {
    let tooling = PythonTooling::detect(root);
    let mut items = Vec::new();

    // Helper to prepend run prefix if needed
    let cmd = |tool: &str| -> String {
        if let Some(prefix) = tooling.package_manager.run_prefix() {
            format!("{} {}", prefix.join(" "), tool)
        } else {
            tool.to_string()
        }
    };

    // Linting: prefer ruff, fall back to flake8/pylint
    if tooling.has_ruff {
        items.push(cmd("ruff check ."));
    } else if tooling.has_flake8 {
        items.push(cmd("flake8"));
    }
    if tooling.has_pylint && !tooling.has_ruff {
        // Only add pylint if not using ruff (ruff replaces most pylint checks)
        items.push(cmd("pylint **/*.py"));
    }

    // Formatting: prefer ruff format, fall back to black
    if tooling.has_ruff {
        items.push(cmd("ruff format --check ."));
    } else if tooling.has_black {
        items.push(cmd("black --check ."));
    }

    // Type checking
    if tooling.has_mypy {
        items.push(cmd("mypy ."));
    }

    // Testing
    if tooling.has_pytest {
        items.push(cmd("pytest"));
    }

    // Building (only if build system is configured)
    if tooling.has_build {
        items.push(cmd("python -m build"));
    }

    // If nothing detected, return sensible defaults
    if items.is_empty() {
        return vec![
            cmd("ruff check ."),
            cmd("ruff format --check ."),
            cmd("pytest"),
        ];
    }

    items
}

// =============================================================================
// PROFILE REGISTRY
// =============================================================================

/// Profile registry for init command.
///
/// Centralizes profile lookup for maintainability and extensibility.
pub struct ProfileRegistry;

impl ProfileRegistry {
    /// Get all available profile names.
    pub fn available() -> &'static [&'static str] {
        &[
            "rust",
            "golang",
            "javascript",
            "ruby",
            "python",
            "shell",
            "claude",
            "cursor",
        ]
    }

    /// Get profile content by name.
    ///
    /// Returns the full profile configuration string for the given profile name,
    /// or None if the profile is not recognized.
    pub fn get(name: &str) -> Option<String> {
        match name.to_lowercase().as_str() {
            "rust" => Some(rust_profile_defaults()),
            "shell" => Some(shell_profile_defaults()),
            "golang" | "go" => Some(golang_profile_defaults()),
            "javascript" | "js" | "typescript" | "ts" => Some(javascript_profile_defaults()),
            "ruby" | "rb" => Some(ruby_profile_defaults()),
            "python" | "py" => Some(python_profile_defaults()),
            "claude" => Some(claude_profile_defaults().to_string()),
            "cursor" => Some(cursor_profile_defaults().to_string()),
            _ => None,
        }
    }

    /// Check if a profile name is valid.
    pub fn is_valid(name: &str) -> bool {
        Self::get(name).is_some()
    }

    /// Check if a profile is an agent profile (vs language profile).
    pub fn is_agent_profile(name: &str) -> bool {
        matches!(name.to_lowercase().as_str(), "claude" | "cursor")
    }

    /// Suggest similar profile names for typos.
    ///
    /// Uses simple prefix matching to suggest corrections.
    pub fn suggest(name: &str) -> Option<&'static str> {
        let lower = name.to_lowercase();

        // Check for common prefixes
        for &profile in Self::available() {
            if profile.starts_with(&lower) || lower.starts_with(profile) {
                return Some(profile);
            }
        }

        // Check for common aliases
        match lower.as_str() {
            "js" | "ts" | "typescript" | "node" => Some("javascript"),
            "go" => Some("golang"),
            "bash" | "zsh" | "sh" => Some("shell"),
            "rb" | "rails" | "rake" => Some("ruby"),
            "py" | "pip" | "poetry" | "uv" | "django" | "flask" | "fastapi" => Some("python"),
            _ => None,
        }
    }
}

// =============================================================================
// DETECTED AGENT SECTIONS
// =============================================================================

/// Generate [check.agents] section with detected agents.
///
/// Returns the TOML section with required files based on detected agents.
pub fn agents_detected_section(agents: &[DetectedAgent]) -> String {
    if agents.is_empty() {
        return String::new();
    }

    let required: Vec<&str> = agents
        .iter()
        .map(|a| match a {
            DetectedAgent::Claude => "CLAUDE.md",
            DetectedAgent::Cursor(CursorMarker::Cursorrules) => ".cursorrules",
            DetectedAgent::Cursor(CursorMarker::CursorRulesDir) => ".cursor/rules/*.mdc",
        })
        .collect();

    format!(
        r#"[check.agents]
check = "error"
required = {:?}
"#,
        required
    )
}

// =============================================================================
// DETECTED LANGUAGE SECTIONS
// =============================================================================

/// Minimal Rust section for auto-detection output.
///
/// Uses dotted keys per spec: docs/specs/commands/quench-init.md
pub fn rust_detected_section() -> &'static str {
    r#"[rust]
cloc.check = "error"
policy.check = "error"
suppress.check = "comment"
"#
}

/// Minimal Go section for auto-detection output.
pub fn golang_detected_section() -> &'static str {
    r#"[golang]
cloc.check = "error"
policy.check = "error"
suppress.check = "comment"
"#
}

/// Minimal JavaScript section for auto-detection output.
pub fn javascript_detected_section() -> &'static str {
    r#"[javascript]
cloc.check = "error"
policy.check = "error"
suppress.check = "comment"
"#
}

/// Minimal Shell section for auto-detection output.
///
/// Note: Shell uses "forbid" for suppress by default.
pub fn shell_detected_section() -> &'static str {
    r#"[shell]
cloc.check = "error"
policy.check = "error"
suppress.check = "forbid"
"#
}

/// Minimal Ruby section for auto-detection output.
pub fn ruby_detected_section() -> &'static str {
    r#"[ruby]
cloc.check = "error"
policy.check = "error"
suppress.check = "comment"
"#
}

/// Minimal Python section for auto-detection output.
pub fn python_detected_section() -> &'static str {
    r#"[python]
cloc.check = "error"
policy.check = "error"
suppress.check = "comment"
"#
}

// =============================================================================
// DEFAULT TEMPLATE
// =============================================================================

/// Base template without [check.agents] section.
/// The agents section is generated separately to support required field.
pub fn default_template_base() -> &'static str {
    r#"# Quench configuration
# Run `quench config` for reference documentation
version = 1

# Baseline source for ratcheting (default: git notes)
# Use "notes" for per-commit history, or a file path for committed baseline
[git]
baseline = "notes"    # e.g.  ".quench/baseline.json"

# Validates conventional commit format: type(scope): description
# See: `quench config git`
[git.commit]
check = "error"

[ratchet]
check = "error"       # error | warn | off
coverage = true       # Coverage can't drop
escapes = true        # Escape counts can't increase

# See: `quench config cloc`
[check.cloc]
check = "error"

# See: `quench config escapes`
[check.escapes]
check = "error"

"#
}

/// Portion of template after agents section.
pub fn default_template_suffix() -> &'static str {
    r#"
# See: `quench config docs`
[check.docs]
check = "error"

# Verify source changes have corresponding test changes, tracks test coverage and timing for ratcheting
# See: `quench config tests`
[check.tests]
check = "off"
auto = false         # auto discover test suites

# Validate SPDX license headers and copyright
# See: `quench config license`
[check.license]
check = "off"

# Supported Languages:
# [rust], [golang], [javascript], [shell]
"#
}

/// Generate [check.agents] section with optional required field.
pub fn agents_section(agents: &[DetectedAgent]) -> String {
    if agents.is_empty() {
        return "# See: `quench config agents`\n[check.agents]\ncheck = \"error\"\n".to_string();
    }

    let required: Vec<&str> = agents
        .iter()
        .map(|a| match a {
            DetectedAgent::Claude => "CLAUDE.md",
            DetectedAgent::Cursor(CursorMarker::Cursorrules) => ".cursorrules",
            DetectedAgent::Cursor(CursorMarker::CursorRulesDir) => ".cursor/rules/*.mdc",
        })
        .collect();

    format!(
        "# See: `quench config agents`\n[check.agents]\ncheck = \"error\"\nrequired = {:?}\n",
        required
    )
}

/// Full default template for quench init without profiles.
///
/// Matches docs/specs/templates/init.default.toml
pub fn default_template() -> String {
    format!(
        "{}{}{}",
        default_template_base(),
        agents_section(&[]),
        default_template_suffix()
    )
}

#[cfg(test)]
#[path = "profiles_tests.rs"]
mod tests;
