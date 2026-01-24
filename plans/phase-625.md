# Phase 625: Docs Check - Specs Content

## Overview

Add content validation for spec files in `docs/specs/`. This includes required/forbidden section validation, content rules (tables, diagrams), and size limits. Reuses patterns from the `agents` check while respecting the different defaults for specification documents (tables/diagrams allowed by default).

## Project Structure

```
crates/cli/src/
├── checks/docs/
│   ├── mod.rs              # Add content validation call
│   ├── specs.rs            # Extend with content validation
│   └── content.rs          # NEW: Content validation for specs
├── config/
│   └── checks.rs           # Extend SpecsConfig
tests/
├── specs/checks/docs/
│   └── content.rs          # NEW: Behavioral tests
└── fixtures/docs-content/  # NEW: Test fixtures
```

## Dependencies

No new external dependencies. Reuses existing patterns from:
- `crates/cli/src/checks/agents/content.rs` - Content detection (tables, diagrams, mermaid)
- `crates/cli/src/checks/agents/sections.rs` - Section validation logic
- `crates/cli/src/checks/agents/sync.rs` - `parse_sections()` function

## Implementation Phases

### Phase 1: Configuration Schema

Extend `SpecsConfig` in `crates/cli/src/config/checks.rs` with content validation options.

**Key changes:**

```rust
// In crates/cli/src/config/checks.rs

/// Configuration for specs directory validation.
#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SpecsConfig {
    // ... existing fields ...

    /// Section validation configuration.
    #[serde(default)]
    pub sections: SpecsSectionsConfig,

    /// Markdown table enforcement (default: allow).
    #[serde(default = "ContentRule::allow")]
    pub tables: ContentRule,

    /// Box diagram enforcement (default: allow).
    #[serde(default = "ContentRule::allow")]
    pub box_diagrams: ContentRule,

    /// Mermaid block enforcement (default: allow).
    #[serde(default = "ContentRule::allow")]
    pub mermaid: ContentRule,

    /// Maximum lines per spec file (default: 1000, None to disable).
    #[serde(
        default = "SpecsConfig::default_max_lines",
        deserialize_with = "deserialize_optional_usize"
    )]
    pub max_lines: Option<usize>,

    /// Maximum tokens per spec file (default: 20000, None to disable).
    #[serde(
        default = "SpecsConfig::default_max_tokens",
        deserialize_with = "deserialize_optional_usize"
    )]
    pub max_tokens: Option<usize>,
}

/// Section validation for specs (separate from agents to allow different defaults).
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpecsSectionsConfig {
    /// Required sections (simple form: names only, or extended form with advice).
    #[serde(default)]
    pub required: Vec<RequiredSection>,

    /// Forbidden sections (supports globs like "Draft*").
    #[serde(default)]
    pub forbid: Vec<String>,
}

impl Default for SpecsSectionsConfig {
    fn default() -> Self {
        Self {
            required: Vec::new(),  // No required sections by default
            forbid: Vec::new(),
        }
    }
}
```

**Import `ContentRule` and `RequiredSection`** from agents config (move to shared location or re-export).

**Verification:**
- Unit tests in `crates/cli/src/config/checks_tests.rs` for TOML parsing
- Test both simple `["Purpose"]` and extended `[{ name = "Purpose", advice = "..." }]` formats

---

### Phase 2: Content Detection Module

Create `crates/cli/src/checks/docs/content.rs` for spec content validation.

**Approach:** Reuse detection functions from `agents/content.rs` rather than duplicating. Two options:

1. **Option A (Preferred):** Move shared detection functions to a common location
2. **Option B:** Import and re-export from agents module

For this phase, use Option B to minimize churn. The agents module's content detection is general-purpose.

```rust
// crates/cli/src/checks/docs/content.rs

//! Content validation for spec files.
//!
//! Validates sections, content rules, and size limits.

use std::path::Path;

use crate::check::Violation;
use crate::checks::agents::content::{
    detect_tables, detect_box_diagrams, detect_mermaid_blocks,
    check_line_count, check_token_count, SizeLimitType,
};
use crate::checks::agents::sections::{validate_sections, SectionValidation};
use crate::config::checks::{ContentRule, SpecsConfig, SpecsSectionsConfig};

/// Validate content of a single spec file.
pub fn validate_spec_content(
    path: &Path,
    content: &str,
    config: &SpecsConfig,
) -> Vec<Violation> {
    let mut violations = Vec::new();

    // Section validation
    validate_spec_sections(path, content, &config.sections, &mut violations);

    // Content rules
    validate_content_rules(path, content, config, &mut violations);

    // Size limits
    validate_size_limits(path, content, config, &mut violations);

    violations
}

fn validate_spec_sections(
    path: &Path,
    content: &str,
    config: &SpecsSectionsConfig,
    violations: &mut Vec<Violation>,
) {
    // Convert to agents SectionsConfig for reuse
    let agent_sections_config = crate::checks::agents::config::SectionsConfig {
        required: config.required.clone(),
        forbid: config.forbid.clone(),
    };

    let result = validate_sections(content, &agent_sections_config);

    for missing in result.missing {
        let advice = missing.advice.map_or_else(
            || format!("Add a \"## {}\" section.", missing.name),
            |a| format!("Add a \"## {}\" section: {}", missing.name, a),
        );
        violations.push(Violation::file_only(path, "missing_section", &advice)
            .with_section(&missing.name));
    }

    for forbidden in result.forbidden {
        violations.push(
            Violation::file(path, forbidden.line, "forbidden_section",
                &format!("Section \"{}\" is forbidden (matched pattern: {}).",
                    forbidden.heading, forbidden.matched_pattern))
                .with_section(&forbidden.heading),
        );
    }
}

fn validate_content_rules(
    path: &Path,
    content: &str,
    config: &SpecsConfig,
    violations: &mut Vec<Violation>,
) {
    // Tables
    if config.tables == ContentRule::Forbid {
        for issue in detect_tables(content) {
            violations.push(Violation::file(
                path,
                issue.line,
                issue.content_type.violation_type(),
                issue.content_type.advice(),
            ));
        }
    }

    // Box diagrams
    if config.box_diagrams == ContentRule::Forbid {
        for issue in detect_box_diagrams(content) {
            violations.push(Violation::file(
                path,
                issue.line,
                issue.content_type.violation_type(),
                issue.content_type.advice(),
            ));
        }
    }

    // Mermaid
    if config.mermaid == ContentRule::Forbid {
        for issue in detect_mermaid_blocks(content) {
            violations.push(Violation::file(
                path,
                issue.line,
                issue.content_type.violation_type(),
                issue.content_type.advice(),
            ));
        }
    }
}

fn validate_size_limits(
    path: &Path,
    content: &str,
    config: &SpecsConfig,
    violations: &mut Vec<Violation>,
) {
    // Line limit
    if let Some(max_lines) = config.max_lines {
        if let Some(violation) = check_line_count(content, max_lines) {
            violations.push(
                Violation::file_only(path, "spec_too_large",
                    &violation.limit_type.advice(violation.value, violation.threshold))
                    .with_threshold(violation.value as i64, violation.threshold as i64),
            );
        }
    }

    // Token limit
    if let Some(max_tokens) = config.max_tokens {
        if let Some(violation) = check_token_count(content, max_tokens) {
            violations.push(
                Violation::file_only(path, "spec_too_large",
                    &violation.limit_type.advice(violation.value, violation.threshold))
                    .with_threshold(violation.value as i64, violation.threshold as i64),
            );
        }
    }
}
```

**Verification:**
- Unit tests in `crates/cli/src/checks/docs/content_tests.rs`
- Test each validation type independently

---

### Phase 3: Integration into Specs Validation

Integrate content validation into `crates/cli/src/checks/docs/specs.rs`.

**Key changes:**

```rust
// In validate_specs(), after index validation:

// Content validation for each spec file
if !all_specs.is_empty() {
    validate_specs_content(ctx, &all_specs, violations);
}

/// Validate content of all spec files.
fn validate_specs_content(
    ctx: &CheckContext,
    specs: &HashSet<PathBuf>,
    violations: &mut Vec<Violation>,
) {
    let config = &ctx.config.check.docs.specs;
    let canonical_root = match ctx.root.canonicalize() {
        Ok(r) => r,
        Err(_) => return,
    };

    for spec_path in specs {
        if ctx.limit.is_some_and(|l| violations.len() >= l) {
            break;
        }

        let content = match std::fs::read_to_string(spec_path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let rel_path = spec_path.strip_prefix(&canonical_root).unwrap_or(spec_path);
        let file_violations = content::validate_spec_content(rel_path, &content, config);

        for v in file_violations {
            if ctx.limit.is_some_and(|l| violations.len() >= l) {
                break;
            }
            violations.push(v);
        }
    }
}
```

**Update mod.rs** to declare the content module:

```rust
mod content;
```

**Verification:**
- Behavioral tests confirm end-to-end flow
- Test with fixtures containing various violations

---

### Phase 4: Behavioral Tests

Create behavioral tests in `tests/specs/checks/docs/content.rs`.

**Test cases:**

```rust
// tests/specs/checks/docs/content.rs

use crate::prelude::*;

// === Required Sections ===

#[test]
fn spec_missing_required_section() {
    let project = default_project()
        .file("quench.toml", r#"
[check.docs.specs]
sections.required = ["Purpose"]
"#)
        .file("docs/specs/CLAUDE.md", "# Overview")
        .file("docs/specs/feature.md", "# Feature\n\nSome content.");

    cli().on(project).fails().stdout_has("missing_section");
}

#[test]
fn spec_has_required_section() {
    let project = default_project()
        .file("quench.toml", r#"
[check.docs.specs]
sections.required = ["Purpose"]
"#)
        .file("docs/specs/CLAUDE.md", "# Overview")
        .file("docs/specs/feature.md", "# Feature\n\n## Purpose\n\nExplains the feature.");

    cli().on(project).passes();
}

// === Forbidden Sections ===

#[test]
fn spec_has_forbidden_section() {
    let project = default_project()
        .file("quench.toml", r#"
[check.docs.specs]
sections.forbid = ["TODO", "Draft*"]
"#)
        .file("docs/specs/CLAUDE.md", "# Overview")
        .file("docs/specs/feature.md", "# Feature\n\n## Draft Notes\n\nWork in progress.");

    cli().on(project).fails().stdout_has("forbidden_section");
}

// === Content Rules ===

#[test]
fn spec_tables_allowed_by_default() {
    let project = default_project()
        .file("docs/specs/CLAUDE.md", "# Overview")
        .file("docs/specs/feature.md", "# Feature\n\n| A | B |\n|---|---|\n| 1 | 2 |");

    cli().on(project).passes();
}

#[test]
fn spec_tables_forbidden_when_configured() {
    let project = default_project()
        .file("quench.toml", r#"
[check.docs.specs]
tables = "forbid"
"#)
        .file("docs/specs/CLAUDE.md", "# Overview")
        .file("docs/specs/feature.md", "# Feature\n\n| A | B |\n|---|---|\n| 1 | 2 |");

    cli().on(project).fails().stdout_has("forbidden_table");
}

// === Size Limits ===

#[test]
fn spec_exceeds_line_limit() {
    let project = default_project()
        .file("quench.toml", r#"
[check.docs.specs]
max_lines = 10
"#)
        .file("docs/specs/CLAUDE.md", "# Overview")
        .file("docs/specs/feature.md", &"line\n".repeat(20));

    cli().on(project).fails().stdout_has("spec_too_large");
}

#[test]
fn spec_size_limit_disabled() {
    let project = default_project()
        .file("quench.toml", r#"
[check.docs.specs]
max_lines = false
max_tokens = false
"#)
        .file("docs/specs/CLAUDE.md", "# Overview")
        .file("docs/specs/feature.md", &"line\n".repeat(2000));

    cli().on(project).passes();
}
```

**Add to test module** in `tests/specs/checks/docs/mod.rs`:

```rust
mod content;
```

**Verification:**
- `cargo test --test specs` passes
- Coverage of all violation types

---

### Phase 5: Documentation & Polish

1. **Update spec documentation** in `docs/specs/checks/docs.md`:
   - Section validation examples are already present
   - Verify configuration examples match implementation

2. **Update JSON output schema** if needed (violation types already documented)

3. **Run full test suite**:
   ```bash
   make check
   ```

4. **Bump `CACHE_VERSION`** in `crates/cli/src/cache.rs` if check logic changed

## Key Implementation Details

### Reusing Agents Content Detection

The `agents/content.rs` module provides:
- `detect_tables()` - Finds markdown tables
- `detect_box_diagrams()` - Finds Unicode box-drawing diagrams
- `detect_mermaid_blocks()` - Finds mermaid fenced blocks
- `check_line_count()` / `check_token_count()` - Size validation

These are general-purpose and can be reused directly.

### Reusing Agents Section Validation

The `agents/sections.rs` module provides:
- `validate_sections()` - Validates required/forbidden sections
- `parse_sections()` (in sync.rs) - Parses markdown headings

The `SectionsConfig` and `RequiredSection` types in `agents/config.rs` can be reused.

### Default Differences from Agents Check

| Setting | Agents Default | Specs Default |
|---------|---------------|---------------|
| `tables` | `forbid` | `allow` |
| `box_diagrams` | `allow` | `allow` |
| `mermaid` | `allow` | `allow` |
| `max_lines` | `500` | `1000` |
| `max_tokens` | `20000` | `20000` |
| `sections.required` | `["Directory Structure", "Landing the Plane"]` | `[]` |

### Violation Types

- `missing_section` - Required section not found
- `forbidden_section` - Forbidden section present (with line number)
- `forbidden_table` - Markdown table when tables = "forbid"
- `forbidden_diagram` - Box diagram when box_diagrams = "forbid"
- `forbidden_mermaid` - Mermaid block when mermaid = "forbid"
- `spec_too_large` - Exceeds max_lines or max_tokens

## Verification Plan

1. **Unit Tests**
   - `crates/cli/src/config/checks_tests.rs` - Config parsing
   - `crates/cli/src/checks/docs/content_tests.rs` - Content validation

2. **Behavioral Tests**
   - `tests/specs/checks/docs/content.rs` - End-to-end scenarios

3. **Integration**
   - `make check` passes
   - Existing docs tests still pass
   - New violation types appear in output

4. **Manual Testing**
   ```bash
   # Test on a project with spec files
   cargo run -- check --docs

   # Test specific configuration
   cargo run -- check --docs --config test-quench.toml
   ```
