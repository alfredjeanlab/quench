# Phase 486: Ruby Adapter - Suppress

## Overview

Add Ruby lint suppression directive detection to quench. This phase implements parsing and validation of RuboCop and Standard Ruby disable comments, with configurable check levels, per-cop allow/forbid lists, and separate source vs test policies.

Supported patterns:
- `# rubocop:disable Cop/Name` (single cop)
- `# rubocop:disable Cop1, Cop2` (multiple cops)
- `# rubocop:todo Cop/Name` (todo-style disable)
- `# standard:disable ...` (Standard Ruby linter)

## Project Structure

```
crates/cli/src/
├── adapter/
│   ├── mod.rs                      # Add Ruby exports, detection
│   └── ruby/
│       ├── mod.rs                  # Ruby adapter definition
│       ├── suppress.rs             # Directive parsing (rubocop, standard)
│       └── suppress_tests.rs       # Unit tests
├── checks/escapes/
│   ├── mod.rs                      # Add Ruby suppress integration
│   ├── ruby_suppress.rs            # Suppress violation checking
│   └── suppress_common.rs          # Add Ruby lint guidance
└── config/
    ├── mod.rs                      # Add Ruby config exports
    └── ruby.rs                     # RubyConfig, RubySuppressConfig

tests/
├── specs/adapters/ruby.rs          # Behavioral specs (spec tests)
└── fixtures/violations/
    └── ruby/                       # Ruby violation test files
```

## Dependencies

- **Internal**: Uses existing `suppress_common.rs` shared logic
- **No new external crates required**

## Implementation Phases

### Phase 1: Ruby Suppress Configuration

Add `RubyConfig` and `RubySuppressConfig` types for Ruby-specific settings.

**Files to create/modify:**

1. **`crates/cli/src/config/ruby.rs`** (new file)

```rust
//! Ruby language-specific configuration.

use serde::Deserialize;
use super::{CheckLevel, LangClocConfig, LintChangesPolicy, SuppressLevel, SuppressScopeConfig};

/// Ruby language-specific configuration.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RubyConfig {
    /// Source file patterns.
    #[serde(default = "RubyConfig::default_source")]
    pub source: Vec<String>,

    /// Test file patterns.
    #[serde(default = "RubyConfig::default_tests")]
    pub tests: Vec<String>,

    /// Ignore patterns.
    #[serde(default = "RubyConfig::default_ignore")]
    pub ignore: Vec<String>,

    /// Lint suppression settings.
    #[serde(default)]
    pub suppress: RubySuppressConfig,

    /// Lint configuration policy.
    #[serde(default)]
    pub policy: RubyPolicyConfig,

    /// Per-language cloc settings.
    #[serde(default)]
    pub cloc: Option<LangClocConfig>,
}

impl RubyConfig {
    pub(crate) fn default_source() -> Vec<String> {
        vec![
            "**/*.rb".to_string(),
            "**/*.rake".to_string(),
            "Rakefile".to_string(),
            "Gemfile".to_string(),
            "*.gemspec".to_string(),
        ]
    }

    pub(crate) fn default_tests() -> Vec<String> {
        vec![
            "spec/**/*_spec.rb".to_string(),
            "test/**/*_test.rb".to_string(),
            "test/**/test_*.rb".to_string(),
            "features/**/*.rb".to_string(),
        ]
    }

    pub(crate) fn default_ignore() -> Vec<String> {
        vec![
            "vendor/**".to_string(),
            "tmp/**".to_string(),
            "log/**".to_string(),
            "coverage/**".to_string(),
        ]
    }
}

/// Ruby suppress configuration.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RubySuppressConfig {
    /// Check level: forbid, comment, or allow (default: "comment").
    #[serde(default = "RubySuppressConfig::default_check")]
    pub check: SuppressLevel,

    /// Optional comment pattern required (default: any comment).
    #[serde(default)]
    pub comment: Option<String>,

    /// Source-specific settings.
    #[serde(default)]
    pub source: SuppressScopeConfig,

    /// Test-specific settings (overrides base settings for test code).
    #[serde(default = "RubySuppressConfig::default_test")]
    pub test: SuppressScopeConfig,
}

impl RubySuppressConfig {
    pub(crate) fn default_check() -> SuppressLevel {
        SuppressLevel::Comment
    }

    pub(crate) fn default_test() -> SuppressScopeConfig {
        SuppressScopeConfig {
            check: Some(SuppressLevel::Allow),
            allow: Vec::new(),
            forbid: Vec::new(),
            patterns: std::collections::HashMap::new(),
        }
    }
}
```

2. **`crates/cli/src/config/mod.rs`** - Add Ruby module and exports

**Verification:** `cargo check` passes with new config types.

---

### Phase 2: Ruby Suppress Directive Parsing

Implement the parser for RuboCop and Standard Ruby disable comments.

**Files to create:**

1. **`crates/cli/src/adapter/ruby/mod.rs`**

```rust
//! Ruby language adapter.

mod suppress;

pub use suppress::{RubocopDirective, parse_rubocop_directives};

use std::path::Path;
use crate::adapter::{Adapter, EscapePattern, FileKind, ResolvedPatterns};
use crate::adapter::patterns::resolve_patterns;

/// Ruby language adapter.
pub struct RubyAdapter {
    patterns: ResolvedPatterns,
}

impl RubyAdapter {
    pub fn new() -> Self {
        Self::with_patterns(ResolvedPatterns::default_for::<crate::config::RubyConfig>())
    }

    pub fn with_patterns(patterns: ResolvedPatterns) -> Self {
        Self { patterns }
    }
}

impl Adapter for RubyAdapter {
    fn name(&self) -> &'static str { "ruby" }
    fn extensions(&self) -> &'static [&'static str] { &["rb", "rake"] }

    fn classify(&self, path: &Path) -> FileKind {
        // Pattern-based classification
        self.patterns.classify(path)
    }
}
```

2. **`crates/cli/src/adapter/ruby/suppress.rs`**

```rust
//! Ruby lint suppression directive parsing.
//!
//! Parses RuboCop (`# rubocop:disable`, `# rubocop:todo`) and
//! Standard Ruby (`# standard:disable`) directives.

use crate::adapter::common::suppress::{CommentStyle, check_justification_comment};

/// A parsed RuboCop/Standard directive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RubocopDirective {
    /// Line number (0-indexed).
    pub line: usize,
    /// Directive kind: "disable", "todo", "enable".
    pub kind: &'static str,
    /// Tool: "rubocop" or "standard".
    pub tool: &'static str,
    /// Cop names being suppressed.
    pub cops: Vec<String>,
    /// Whether a justification comment was found.
    pub has_comment: bool,
    /// The comment text if found.
    pub comment_text: Option<String>,
}

/// Ruby comment style.
const RUBY_COMMENT_STYLE: CommentStyle = CommentStyle {
    prefix: "#",
    directive_patterns: &["rubocop:", "standard:"],
};

/// Parse all RuboCop/Standard directives from content.
pub fn parse_rubocop_directives(
    content: &str,
    comment_pattern: Option<&str>,
) -> Vec<RubocopDirective> {
    let mut directives = Vec::new();
    let lines: Vec<&str> = content.lines().collect();

    for (line_idx, line) in lines.iter().enumerate() {
        if let Some(parsed) = parse_directive_line(line) {
            // Check for justification comment above
            let (has_comment, comment_text) = check_justification_comment(
                &lines, line_idx, comment_pattern, &RUBY_COMMENT_STYLE
            );

            directives.push(RubocopDirective {
                line: line_idx,
                kind: parsed.kind,
                tool: parsed.tool,
                cops: parsed.cops,
                has_comment,
                comment_text,
            });
        }
    }

    directives
}

struct ParsedDirective {
    kind: &'static str,
    tool: &'static str,
    cops: Vec<String>,
}

fn parse_directive_line(line: &str) -> Option<ParsedDirective> {
    let trimmed = line.trim();

    // Must start with # (comment)
    let rest = trimmed.strip_prefix('#')?.trim();

    // Check for rubocop or standard
    let (tool, rest) = if let Some(r) = rest.strip_prefix("rubocop:") {
        ("rubocop", r)
    } else if let Some(r) = rest.strip_prefix("standard:") {
        ("standard", r)
    } else {
        return None;
    };

    // Parse kind (disable, todo, enable)
    let (kind, cops_str) = if let Some(r) = rest.strip_prefix("disable") {
        ("disable", r.trim_start())
    } else if let Some(r) = rest.strip_prefix("todo") {
        ("todo", r.trim_start())
    } else if let Some(r) = rest.strip_prefix("enable") {
        ("enable", r.trim_start())
    } else {
        return None;
    };

    // Skip "enable" directives - they're just closing blocks
    if kind == "enable" {
        return None;
    }

    // Parse cop names (comma-separated)
    let cops: Vec<String> = cops_str
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty() && s.chars().next().map_or(false, |c| c.is_ascii_alphabetic()))
        .collect();

    Some(ParsedDirective { kind, tool, cops })
}
```

3. **`crates/cli/src/adapter/ruby/suppress_tests.rs`** - Unit tests

**Verification:** `cargo test adapter::ruby` passes.

---

### Phase 3: Ruby Suppress Checking Integration

Integrate Ruby suppress checking into the escapes check.

**Files to create/modify:**

1. **`crates/cli/src/checks/escapes/ruby_suppress.rs`** (new file)

```rust
//! Ruby lint suppression directive checking for the escapes check.

use std::path::Path;

use crate::adapter::ruby::parse_rubocop_directives;
use crate::check::{CheckContext, Violation};
use crate::config::{RubySuppressConfig, SuppressLevel};

use super::suppress_common::{
    SuppressAttrInfo, SuppressCheckParams, SuppressViolationKind,
    build_suppress_missing_comment_advice, check_suppress_attr,
};
use super::try_create_violation;

/// Check Ruby suppress directives and return violations.
pub fn check_ruby_suppress_violations(
    ctx: &CheckContext,
    path: &Path,
    content: &str,
    config: &RubySuppressConfig,
    is_test_file: bool,
    limit_reached: &mut bool,
) -> Vec<Violation> {
    let mut violations = Vec::new();

    // Determine effective check level
    let effective_check = if is_test_file {
        config.test.check.unwrap_or(SuppressLevel::Allow)
    } else {
        config.source.check.unwrap_or(config.check)
    };

    if effective_check == SuppressLevel::Allow {
        return violations;
    }

    // Parse directives
    let directives = parse_rubocop_directives(content, config.comment.as_deref());

    // Get scope config
    let (scope_config, scope_check) = if is_test_file {
        (&config.test, config.test.check.unwrap_or(SuppressLevel::Allow))
    } else {
        (&config.source, config.source.check.unwrap_or(config.check))
    };

    if scope_check == SuppressLevel::Allow {
        return violations;
    }

    for directive in directives {
        if *limit_reached {
            break;
        }

        let params = SuppressCheckParams {
            scope_config,
            scope_check,
            global_comment: config.comment.as_deref(),
        };

        let attr_info = SuppressAttrInfo {
            codes: &directive.cops,
            has_comment: directive.has_comment,
            comment_text: directive.comment_text.as_deref(),
        };

        if let Some(violation_kind) = check_suppress_attr(&params, &attr_info) {
            let pattern = format_directive_pattern(&directive);

            let (violation_type, advice) = match violation_kind {
                SuppressViolationKind::Forbidden { ref code } => {
                    let advice = format!(
                        "Suppressing `{}` is forbidden. Remove the suppression or address the issue.",
                        code
                    );
                    ("suppress_forbidden", advice)
                }
                SuppressViolationKind::MissingComment { ref lint_code, ref required_patterns } => {
                    let advice = build_suppress_missing_comment_advice(
                        "ruby", lint_code.as_deref(), required_patterns
                    );
                    ("suppress_missing_comment", advice)
                }
                SuppressViolationKind::AllForbidden => {
                    ("suppress_forbidden", "Lint suppressions are forbidden.".to_string())
                }
            };

            if let Some(v) = try_create_violation(
                ctx, path, (directive.line + 1) as u32, violation_type, &advice, &pattern
            ) {
                violations.push(v);
            } else {
                *limit_reached = true;
            }
        }
    }

    violations
}

fn format_directive_pattern(directive: &crate::adapter::ruby::RubocopDirective) -> String {
    if directive.cops.is_empty() {
        format!("# {}:{}", directive.tool, directive.kind)
    } else {
        format!("# {}:{} {}", directive.tool, directive.kind, directive.cops.join(", "))
    }
}
```

2. **`crates/cli/src/checks/escapes/mod.rs`** - Add Ruby integration:
   - Add `mod ruby_suppress;`
   - Add `use ruby_suppress::check_ruby_suppress_violations;`
   - Add Ruby file extension check in the file loop (similar to Go/JS)

3. **`crates/cli/src/checks/escapes/suppress_common.rs`** - Add Ruby lint guidance:

```rust
/// Get lint-specific guidance for Ruby lints.
fn get_ruby_lint_guidance(lint_code: &str) -> &'static str {
    match lint_code {
        s if s.starts_with("Metrics/MethodLength") => "Can this method be refactored into smaller pieces?",
        s if s.starts_with("Metrics/AbcSize") => "Can this method's complexity be reduced?",
        s if s.starts_with("Metrics/CyclomaticComplexity") => "Can conditional logic be simplified?",
        s if s.starts_with("Security/") => "Is this security finding a false positive?",
        s if s.starts_with("Style/Documentation") => "Should this class have documentation?",
        s if s.starts_with("Lint/UselessAssignment") => "Is this variable used elsewhere?",
        _ => "Is this suppression necessary?",
    }
}
```

**Verification:** `cargo test checks::escapes` passes with Ruby suppress checking.

---

### Phase 4: Adapter Registry Integration

Add Ruby to the adapter registry and project language detection.

**Files to modify:**

1. **`crates/cli/src/adapter/mod.rs`**:
   - Add `pub mod ruby;`
   - Add `pub use ruby::{RubyAdapter, RubocopDirective, parse_rubocop_directives};`
   - Add `Ruby` variant to `ProjectLanguage` enum
   - Add Ruby detection in `detect_language()` (check for Gemfile, *.gemspec, config.ru, config/application.rb)
   - Add Ruby case in `AdapterRegistry::for_project()` and `for_project_with_config()`
   - Add `resolve_ruby_patterns()` function

```rust
// In detect_language():
if root.join("Gemfile").exists()
    || has_gemspec(root)
    || root.join("config.ru").exists()
    || root.join("config/application.rb").exists()
{
    return ProjectLanguage::Ruby;
}
```

2. **`crates/cli/src/config/mod.rs`**:
   - Add `mod ruby;`
   - Add `pub use ruby::{RubyConfig, RubyPolicyConfig, RubySuppressConfig};`
   - Add `ruby: RubyConfig` field to `Config` struct
   - Add Ruby cases in `cloc_check_level_for_language()`, `cloc_advice_for_language()`, `policy_check_level_for_language()`

**Verification:** `cargo test adapter::` passes including Ruby adapter.

---

### Phase 5: Behavioral Specs

Add spec tests for Ruby suppress functionality.

**Files to create:**

1. **`tests/specs/adapters/ruby.rs`** (stub for now, to be expanded):

```rust
//! Ruby adapter behavioral specs.

mod suppress {
    use crate::prelude::*;

    #[test]
    #[ignore = "TODO: Phase 486"]
    fn rubocop_disable_without_comment_fails() {
        // spec: # rubocop:disable without comment fails (when configured)
    }

    #[test]
    #[ignore = "TODO: Phase 486"]
    fn rubocop_disable_with_comment_passes() {
        // spec: # rubocop:disable with comment above passes
    }

    #[test]
    #[ignore = "TODO: Phase 486"]
    fn rubocop_todo_treated_as_disable() {
        // spec: # rubocop:todo detection
    }

    #[test]
    #[ignore = "TODO: Phase 486"]
    fn standard_disable_detected() {
        // spec: # standard:disable detection
    }

    #[test]
    #[ignore = "TODO: Phase 486"]
    fn multiple_cops_parsed() {
        // spec: # rubocop:disable Cop1, Cop2 detection
    }

    #[test]
    #[ignore = "TODO: Phase 486"]
    fn allow_list_bypasses_comment_requirement() {
        // spec: per-cop allow list
    }

    #[test]
    #[ignore = "TODO: Phase 486"]
    fn forbid_list_always_fails() {
        // spec: per-cop forbid list
    }

    #[test]
    #[ignore = "TODO: Phase 486"]
    fn test_files_allow_by_default() {
        // spec: separate source vs test suppress policies
    }
}
```

2. **`tests/fixtures/violations/ruby/`** - Add Ruby files with violations

**Verification:** All specs pass or are appropriately ignored.

---

### Phase 6: Landing the Plane

Final integration, cleanup, and verification.

**Tasks:**
1. Remove `#[ignore]` from passing specs
2. Update `CACHE_VERSION` in `crates/cli/src/cache.rs`
3. Run `make check` (fmt, clippy, test, build, audit, deny)
4. Verify spec output matches documentation in `docs/specs/langs/ruby.md`

**Verification:** `make check` passes completely.

## Key Implementation Details

### Directive Pattern Matching

RuboCop directives can appear in several forms:
```ruby
# rubocop:disable Style/StringLiterals           # single cop
# rubocop:disable Style/StringLiterals, Lint/X   # multiple cops
# rubocop:todo Metrics/MethodLength              # todo variant
# standard:disable Style/StringLiterals          # Standard Ruby
x = foo() # rubocop:disable Lint/UselessAssignment  # inline
```

The parser handles:
- Both `rubocop:` and `standard:` prefixes
- `disable`, `todo`, and `enable` (though `enable` is ignored)
- Comma-separated cop lists
- Comment prefix stripping and normalization

### Justification Comment Detection

Uses the shared `check_justification_comment()` from `suppress_common.rs` with Ruby-specific `CommentStyle`:
- Prefix: `#`
- Directive patterns: `["rubocop:", "standard:"]`

Comments above the directive line count as justification:
```ruby
# Legacy API returns inconsistent types
# rubocop:disable Lint/MixedRegexpCaptureTypes
```

### Per-Cop Guidance

The suppress_common module is extended with Ruby-specific guidance:
- `Metrics/MethodLength` → "Can this method be refactored into smaller pieces?"
- `Metrics/AbcSize` → "Can this method's complexity be reduced?"
- `Security/*` → "Is this security finding a false positive?"

### Source vs Test Policy

Default behavior mirrors other adapters:
- Source: `check = "comment"` (requires justification)
- Test: `check = "allow"` (suppressions allowed freely)

Test file detection uses patterns from `RubyConfig::default_tests()`:
- `spec/**/*_spec.rb` (RSpec)
- `test/**/*_test.rb` (Minitest)
- `features/**/*.rb` (Cucumber)

## Verification Plan

### Unit Tests (`suppress_tests.rs`)
- Parse single cop: `# rubocop:disable Cop/Name`
- Parse multiple cops: `# rubocop:disable Cop1, Cop2`
- Parse rubocop:todo
- Parse standard:disable
- Ignore rubocop:enable (closing tags)
- Handle inline comments: `code # rubocop:disable Cop`
- Detect justification comment above
- Require specific pattern when configured

### Integration Tests (escapes mod)
- Ruby file triggers Ruby suppress checking
- Violations emitted for missing comments
- Violations suppressed for test files by default
- Allow list bypasses comment requirement
- Forbid list always fails

### Behavioral Specs
- Match expected output format from `docs/specs/langs/ruby.md`
- Verify cop-specific guidance appears in advice

### Full Check
```bash
make check  # fmt, clippy, test, build, audit, deny
```
