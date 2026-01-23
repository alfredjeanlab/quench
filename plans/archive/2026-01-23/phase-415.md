# Phase 415: Shell Adapter - Suppress

**Root Feature:** `quench-7f3f-2`

## Overview

Add `# shellcheck disable=` detection and suppression policy enforcement to the Shell adapter. Unlike Rust's default "comment" policy (which requires justification), Shell defaults to "forbid" - shellcheck suppressions should be fixed rather than justified. This encourages cleaner shell code and leverages shellcheck's comprehensive linting.

## Project Structure

Key files to modify/create:

```
quench/
├── crates/cli/src/
│   ├── adapter/
│   │   ├── shell/
│   │   │   ├── mod.rs             # Add suppress-related methods
│   │   │   └── suppress.rs        # NEW: Parse # shellcheck disable= comments
│   │   └── shell_tests.rs         # Add suppress parsing tests
│   ├── config/
│   │   ├── mod.rs                 # Add ShellConfig, ShellSuppressConfig
│   │   └── parse.rs               # Add parse_shell_config()
│   └── checks/escapes/
│       └── mod.rs                 # Wire up shell suppress checking
├── tests/
│   ├── specs/adapters/
│   │   └── shell.rs               # Enable shellcheck suppress specs
│   └── fixtures/
│       ├── shell/shellcheck-forbid/
│       ├── shell/shellcheck-test/
│       └── shell/shellcheck-allow/
└── docs/specs/langs/shell.md      # Reference (already specifies behavior)
```

## Dependencies

No new dependencies required. Uses existing:
- `regex` - Pattern matching for `# shellcheck disable=SCxxxx` parsing
- `globset` - File classification (already used by ShellAdapter)

## Implementation Phases

### Phase 1: Add Shell Configuration Structs

Add Shell-specific configuration types mirroring the Rust adapter pattern.

**File:** `crates/cli/src/config/mod.rs`

```rust
/// Shell language-specific configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct ShellConfig {
    /// Source file patterns.
    #[serde(default = "ShellConfig::default_source")]
    pub source: Vec<String>,

    /// Test file patterns.
    #[serde(default = "ShellConfig::default_tests")]
    pub tests: Vec<String>,

    /// Lint suppression settings.
    #[serde(default)]
    pub suppress: ShellSuppressConfig,

    /// Lint configuration policy.
    #[serde(default)]
    pub policy: ShellPolicyConfig,
}

impl ShellConfig {
    pub(crate) fn default_source() -> Vec<String> {
        vec!["**/*.sh".to_string(), "**/*.bash".to_string()]
    }

    pub(crate) fn default_tests() -> Vec<String> {
        vec![
            "tests/**/*.bats".to_string(),
            "test/**/*.bats".to_string(),
            "*_test.sh".to_string(),
            "**/*_test.sh".to_string(),
        ]
    }
}

/// Shell suppress configuration (defaults to "forbid" unlike Rust's "comment").
#[derive(Debug, Clone, Deserialize)]
pub struct ShellSuppressConfig {
    /// Check level: forbid, comment, or allow (default: "forbid").
    #[serde(default = "ShellSuppressConfig::default_check")]
    pub check: SuppressLevel,

    /// Optional comment pattern required (default: any comment).
    #[serde(default)]
    pub comment: Option<String>,

    /// Source-specific settings.
    #[serde(default)]
    pub source: SuppressScopeConfig,

    /// Test-specific settings (overrides base settings for test code).
    #[serde(default = "ShellSuppressConfig::default_test")]
    pub test: SuppressScopeConfig,
}

impl Default for ShellSuppressConfig {
    fn default() -> Self {
        Self {
            check: Self::default_check(),
            comment: None,
            source: SuppressScopeConfig::default(),
            test: Self::default_test(),
        }
    }
}

impl ShellSuppressConfig {
    pub(crate) fn default_check() -> SuppressLevel {
        SuppressLevel::Forbid  // Shell defaults to forbid, not comment
    }

    pub(crate) fn default_test() -> SuppressScopeConfig {
        SuppressScopeConfig {
            check: Some(SuppressLevel::Allow),
            allow: Vec::new(),
            forbid: Vec::new(),
        }
    }
}
```

Update `Config` struct to include `shell: ShellConfig`.

Update `FlexibleConfig` and `KNOWN_KEYS` to include "shell".

**Milestone:** Config structs compile, default values correct.

### Phase 2: Parse Shell Configuration

Add config parsing for the `[shell]` section.

**File:** `crates/cli/src/config/parse.rs`

```rust
/// Parse shell configuration from TOML value.
pub fn parse_shell_config(value: Option<&toml::Value>) -> ShellConfig {
    let Some(toml::Value::Table(t)) = value else {
        return ShellConfig::default();
    };

    // Parse source patterns
    let source = t
        .get("source")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
        .unwrap_or_else(ShellConfig::default_source);

    // Parse test patterns
    let tests = t
        .get("tests")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
        .unwrap_or_else(ShellConfig::default_tests);

    // Parse suppress config
    let suppress = parse_shell_suppress_config(t.get("suppress"));

    // Parse policy config
    let policy = parse_shell_policy_config(t.get("policy"));

    ShellConfig { source, tests, suppress, policy }
}

/// Parse shell suppress configuration.
fn parse_shell_suppress_config(value: Option<&toml::Value>) -> ShellSuppressConfig {
    let Some(toml::Value::Table(t)) = value else {
        return ShellSuppressConfig::default();
    };

    let check = t
        .get("check")
        .and_then(|v| v.as_str())
        .and_then(parse_suppress_level)
        .unwrap_or(ShellSuppressConfig::default_check());

    let comment = t
        .get("comment")
        .and_then(|v| v.as_str())
        .map(String::from);

    let source = parse_suppress_scope_config(t.get("source"));
    let test = t
        .get("test")
        .map(|v| parse_suppress_scope_config(Some(v)))
        .unwrap_or_else(ShellSuppressConfig::default_test);

    ShellSuppressConfig { check, comment, source, test }
}
```

**Milestone:** `[shell]` config section parses correctly.

### Phase 3: Implement Shellcheck Suppress Parser

Create a new module to parse `# shellcheck disable=` comments.

**File:** `crates/cli/src/adapter/shell/suppress.rs`

```rust
//! Shellcheck suppress directive parsing.
//!
//! Parses `# shellcheck disable=SC2034,SC2086` comments in shell scripts.

/// Shellcheck suppress directive found in source code.
#[derive(Debug, Clone)]
pub struct ShellcheckSuppress {
    /// Line number (0-indexed).
    pub line: usize,
    /// Shellcheck codes being suppressed (e.g., ["SC2034", "SC2086"]).
    pub codes: Vec<String>,
    /// Whether a justification comment was found.
    pub has_comment: bool,
    /// The comment text if found.
    pub comment_text: Option<String>,
}

/// Parse shellcheck suppress directives from shell source.
pub fn parse_shellcheck_suppresses(
    content: &str,
    comment_pattern: Option<&str>,
) -> Vec<ShellcheckSuppress> {
    let mut suppresses = Vec::new();
    let lines: Vec<&str> = content.lines().collect();

    for (line_idx, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        // Match # shellcheck disable=...
        if let Some(codes) = parse_shellcheck_disable(trimmed) {
            let (has_comment, comment_text) =
                check_justification_comment(&lines, line_idx, comment_pattern);

            suppresses.push(ShellcheckSuppress {
                line: line_idx,
                codes,
                has_comment,
                comment_text,
            });
        }
    }

    suppresses
}

/// Parse shellcheck disable directive from a single line.
/// Returns list of codes if found (e.g., ["SC2034", "SC2086"]).
fn parse_shellcheck_disable(line: &str) -> Option<Vec<String>> {
    // Match: # shellcheck disable=SC2034 or # shellcheck disable=SC2034,SC2086
    // Also handles: #shellcheck disable=... (no space after #)
    let line = line.trim_start_matches('#').trim();

    if !line.starts_with("shellcheck") {
        return None;
    }

    let rest = line.strip_prefix("shellcheck")?.trim();
    let codes_str = rest.strip_prefix("disable=")?;

    let codes: Vec<String> = codes_str
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    if codes.is_empty() {
        return None;
    }

    Some(codes)
}

/// Check if there's a justification comment above the directive.
fn check_justification_comment(
    lines: &[&str],
    directive_line: usize,
    required_pattern: Option<&str>,
) -> (bool, Option<String>) {
    // Look at preceding lines for a comment
    let mut check_line = directive_line;

    while check_line > 0 {
        check_line -= 1;
        let line = lines[check_line].trim();

        // Stop at blank lines or non-comment code
        if line.is_empty() {
            break;
        }

        // Check for comment (but not shellcheck directive itself)
        if line.starts_with('#') && !line.contains("shellcheck") {
            let comment_text = line.trim_start_matches('#').trim();

            // If a specific pattern is required, check for it
            if let Some(pattern) = required_pattern {
                let pattern_prefix = pattern.trim_start_matches('#').trim();
                if comment_text.starts_with(pattern_prefix) {
                    return (true, Some(comment_text.to_string()));
                }
                continue;
            }

            // Any non-empty comment counts as justification
            if !comment_text.is_empty() {
                return (true, Some(comment_text.to_string()));
            }
        } else if !line.starts_with('#') {
            // Stop at non-comment line
            break;
        }
    }

    (false, None)
}

#[cfg(test)]
#[path = "suppress_tests.rs"]
mod tests;
```

**Milestone:** Parser correctly extracts shellcheck codes and justification comments.

### Phase 4: Wire Suppress Check into Escapes Check

Integrate shellcheck suppress detection into the escapes check.

**File:** `crates/cli/src/checks/escapes/mod.rs`

Add shellcheck suppress checking alongside existing escape pattern checking:

```rust
/// Check shell files for shellcheck suppress directives.
fn check_shell_suppresses(
    content: &str,
    file_path: &Path,
    is_test: bool,
    config: &ShellSuppressConfig,
) -> Vec<EscapeViolation> {
    let mut violations = Vec::new();

    // Get effective config for source vs test
    let (effective_check, allow_list, forbid_list) = if is_test {
        let check = config.test.check.unwrap_or(config.check);
        (&check, &config.test.allow, &config.test.forbid)
    } else {
        let check = config.source.check.unwrap_or(config.check);
        (&check, &config.source.allow, &config.source.forbid)
    };

    // Skip if allowed
    if *effective_check == SuppressLevel::Allow {
        return violations;
    }

    let suppresses = parse_shellcheck_suppresses(content, config.comment.as_deref());

    for suppress in suppresses {
        for code in &suppress.codes {
            // Check forbid list first (always fails)
            if forbid_list.contains(code) {
                violations.push(EscapeViolation {
                    file: file_path.to_path_buf(),
                    line: suppress.line + 1,
                    pattern: "shellcheck_disable".to_string(),
                    code: Some(code.clone()),
                    reason: ViolationReason::Forbidden,
                    advice: format!("Shellcheck {} is forbidden. Fix the underlying issue.", code),
                });
                continue;
            }

            // Check allow list (skip if allowed)
            if allow_list.contains(code) {
                continue;
            }

            // Apply check level
            match effective_check {
                SuppressLevel::Forbid => {
                    violations.push(EscapeViolation {
                        file: file_path.to_path_buf(),
                        line: suppress.line + 1,
                        pattern: "shellcheck_disable".to_string(),
                        code: Some(code.clone()),
                        reason: ViolationReason::Forbidden,
                        advice: format!(
                            "Fix the shellcheck warning {} instead of disabling it.",
                            code
                        ),
                    });
                }
                SuppressLevel::Comment if !suppress.has_comment => {
                    violations.push(EscapeViolation {
                        file: file_path.to_path_buf(),
                        line: suppress.line + 1,
                        pattern: "shellcheck_disable".to_string(),
                        code: Some(code.clone()),
                        reason: ViolationReason::MissingComment,
                        advice: "Add a comment above explaining why this warning is suppressed."
                            .to_string(),
                    });
                }
                _ => {}
            }
        }
    }

    violations
}
```

**Milestone:** Shellcheck suppresses detected and violations generated.

### Phase 5: Add Test Fixtures and Enable Specs

Create test fixtures and enable the ignored behavioral specs.

**Fixtures:**

```
tests/fixtures/shell/shellcheck-forbid/
├── quench.toml         # version = 1
└── scripts/
    └── deploy.sh       # Has # shellcheck disable=SC2034

tests/fixtures/shell/shellcheck-test/
├── quench.toml         # version = 1
├── scripts/
│   └── build.sh        # Clean shell script
└── tests/
    └── test.bats       # Has # shellcheck disable (allowed in test)

tests/fixtures/shell/shellcheck-allow/
├── quench.toml         # [shell.suppress.source] allow = ["SC2034"]
└── scripts/
    └── vars.sh         # Has # shellcheck disable=SC2034 (allowed via config)
```

**File:** `tests/specs/adapters/shell.rs`

Remove `#[ignore]` from these specs:
- `shell_adapter_shellcheck_disable_forbidden_by_default`
- `shell_adapter_shellcheck_disable_allowed_in_tests`
- `shell_adapter_shellcheck_disable_with_comment_when_configured`
- `shell_adapter_shellcheck_allow_list_skips_check`

**Milestone:** All behavioral specs pass.

### Phase 6: Add Unit Tests and Verify

Add unit tests for the suppress parser and config parsing.

**File:** `crates/cli/src/adapter/shell/suppress_tests.rs`

```rust
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;

#[test]
fn parse_single_code() {
    let content = "# shellcheck disable=SC2034\nUNUSED=1";
    let suppresses = parse_shellcheck_suppresses(content, None);

    assert_eq!(suppresses.len(), 1);
    assert_eq!(suppresses[0].codes, vec!["SC2034"]);
    assert_eq!(suppresses[0].line, 0);
}

#[test]
fn parse_multiple_codes() {
    let content = "# shellcheck disable=SC2034,SC2086\necho $var";
    let suppresses = parse_shellcheck_suppresses(content, None);

    assert_eq!(suppresses.len(), 1);
    assert_eq!(suppresses[0].codes, vec!["SC2034", "SC2086"]);
}

#[test]
fn parse_no_space_after_hash() {
    let content = "#shellcheck disable=SC2034\nUNUSED=1";
    let suppresses = parse_shellcheck_suppresses(content, None);

    assert_eq!(suppresses.len(), 1);
    assert_eq!(suppresses[0].codes, vec!["SC2034"]);
}

#[test]
fn detects_justification_comment() {
    let content = "# This variable is used by subprocesses\n# shellcheck disable=SC2034\nUNUSED=1";
    let suppresses = parse_shellcheck_suppresses(content, None);

    assert_eq!(suppresses.len(), 1);
    assert!(suppresses[0].has_comment);
    assert_eq!(
        suppresses[0].comment_text.as_deref(),
        Some("This variable is used by subprocesses")
    );
}

#[test]
fn no_comment_when_blank_line_separates() {
    let content = "# Comment\n\n# shellcheck disable=SC2034\nUNUSED=1";
    let suppresses = parse_shellcheck_suppresses(content, None);

    assert_eq!(suppresses.len(), 1);
    assert!(!suppresses[0].has_comment);
}

#[test]
fn requires_specific_pattern_when_configured() {
    let content = "# Random comment\n# shellcheck disable=SC2034\nUNUSED=1";
    let suppresses = parse_shellcheck_suppresses(content, Some("# OK:"));

    assert_eq!(suppresses.len(), 1);
    assert!(!suppresses[0].has_comment, "should require # OK: pattern");
}

#[test]
fn matches_specific_pattern() {
    let content = "# OK: exported for subprocesses\n# shellcheck disable=SC2034\nUNUSED=1";
    let suppresses = parse_shellcheck_suppresses(content, Some("# OK:"));

    assert_eq!(suppresses.len(), 1);
    assert!(suppresses[0].has_comment);
}
```

**File:** `crates/cli/src/config/mod_tests.rs` (add to existing)

```rust
#[test]
fn shell_suppress_defaults_to_forbid() {
    let config = parse_shell_config(None);
    assert_eq!(config.suppress.check, SuppressLevel::Forbid);
}

#[test]
fn shell_suppress_test_defaults_to_allow() {
    let config = parse_shell_config(None);
    assert_eq!(config.suppress.test.check, Some(SuppressLevel::Allow));
}
```

Run full verification:

```bash
make check
```

**Milestone:** All tests pass, `make check` succeeds.

## Key Implementation Details

### Shellcheck Directive Patterns

| Pattern | Example | Notes |
|---------|---------|-------|
| Single code | `# shellcheck disable=SC2034` | Most common |
| Multiple codes | `# shellcheck disable=SC2034,SC2086` | Comma-separated |
| No space | `#shellcheck disable=SC2034` | Also valid |
| Source directive | `# shellcheck source=./lib.sh` | NOT a suppress (ignore) |

### Default Check Levels

| Language | Default `check` | Rationale |
|----------|-----------------|-----------|
| Rust | `"comment"` | Suppressions common, justify with `// REASON:` |
| Shell | `"forbid"` | Suppressions indicate unfixed issues |

### Per-Code Allow/Forbid Lists

The allow/forbid lists work as follows:

1. **Forbid list checked first** - codes in `forbid = [...]` always fail
2. **Allow list checked second** - codes in `allow = [...]` always pass
3. **Check level applied last** - remaining codes subject to `check` level

```toml
[shell.suppress]
check = "comment"        # Default for non-listed codes

[shell.suppress.source]
allow = ["SC2034"]       # Unused variable OK without comment
forbid = ["SC2006"]      # Backticks never allowed

[shell.suppress.test]
check = "allow"          # Tests can suppress freely
```

### Source vs Test Policy Resolution

```rust
// Get effective config for current scope
let (check_level, allow_list, forbid_list) = if is_test_file {
    (
        config.test.check.unwrap_or(config.check),  // Override or inherit
        &config.test.allow,
        &config.test.forbid,
    )
} else {
    (
        config.source.check.unwrap_or(config.check),
        &config.source.allow,
        &config.source.forbid,
    )
};
```

### Comment Detection Algorithm

Same as Rust adapter:
1. Search upward from suppress directive
2. Stop at blank lines or non-comment code
3. Skip other shellcheck directives (not justification)
4. Match required pattern if configured

```bash
# This variable is exported for child processes    <- Justification found
# shellcheck disable=SC2034
EXPORTED_VAR=1

# shellcheck source=./lib.sh                       <- Not justification (directive)
# shellcheck disable=SC2034                        <- No justification
OTHER_VAR=1
```

## Verification Plan

1. **Unit tests:** `cargo test shell` - suppress parser tests pass
2. **Config tests:** `cargo test config` - shell config parsing works
3. **Behavioral tests:** `cargo test specs::adapters::shell` - all specs pass
4. **Integration test:**
   ```bash
   cd tests/fixtures/shell/shellcheck-forbid
   cargo run -- check --escapes -o json
   # Should show violation for # shellcheck disable=
   ```
5. **Full suite:** `make check` - all gates pass
