# Phase 515: Agents Check - Sections

**Root Feature:** `quench-014b`

## Overview

Implement section-level validation for the `agents` check. This phase adds:
- Required section validation (ensure specific `## Heading` sections exist)
- Forbidden section validation (ensure specific sections don't exist)
- Glob pattern matching for forbidden sections (e.g., `Test*` matches `Testing`, `Test Plan`)
- Extended configuration form with advice for missing sections
- Profile defaults for Claude and Cursor with standard section requirements

Key capabilities:
- Case-insensitive section name matching
- Actionable advice output when sections are missing
- Glob pattern support for forbidden sections
- Profile-specific default configurations

## Project Structure

```
crates/cli/src/
├── checks/
│   ├── agents/
│   │   ├── mod.rs           # Add section validation to run()
│   │   ├── config.rs        # Add SectionsConfig, RequiredSection
│   │   ├── config_tests.rs  # Tests for section config parsing
│   │   ├── sections.rs      # NEW: Section validation logic
│   │   ├── sections_tests.rs # NEW: Unit tests for section validation
│   │   ├── sync.rs          # Expose section parsing (already exists)
│   │   └── detection.rs     # Unchanged
│   └── mod.rs
├── config/
│   ├── mod.rs               # Add profile defaults
│   └── parse.rs             # Parse sections config from TOML
tests/
├── fixtures/agents/
│   ├── missing-section/     # Update with advice config
│   ├── forbidden-section/   # Update with glob patterns
│   └── profile-defaults/    # NEW: Test profile configurations
└── specs/checks/agents.rs   # Enable section specs
```

## Dependencies

No new external dependencies. Uses existing:
- `glob` crate (already in workspace for file patterns) - for section glob matching
- `std::fs` for file reading
- Existing `sync::parse_sections()` for markdown parsing

## Implementation Phases

### Phase 1: Section Configuration Schema

Add configuration structures for section validation.

**Tasks:**
1. Extend `config.rs` with `SectionsConfig` and `RequiredSection` structs
2. Add parsing support in `config/parse.rs`
3. Add unit tests for config parsing

**Configuration structures in `config.rs`:**
```rust
/// Section validation configuration.
#[derive(Debug, Default, Clone, Deserialize)]
pub struct SectionsConfig {
    /// Required sections (simple form: names only, or extended form with advice).
    #[serde(default)]
    pub required: Vec<RequiredSection>,

    /// Forbidden sections (supports globs like "Test*").
    #[serde(default)]
    pub forbid: Vec<String>,
}

/// A required section with optional advice.
#[derive(Debug, Clone)]
pub struct RequiredSection {
    /// Section name (case-insensitive matching).
    pub name: String,
    /// Advice shown when section is missing.
    pub advice: Option<String>,
}

// Custom deserializer to handle both simple and extended forms:
// Simple: sections.required = ["Directory Structure", "Landing the Plane"]
// Extended: [[check.agents.sections.required]]
//           name = "Directory Structure"
//           advice = "Overview of project layout"
```

**Custom deserialization for RequiredSection:**
```rust
impl<'de> Deserialize<'de> for RequiredSection {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum RequiredSectionRepr {
            Simple(String),
            Extended { name: String, advice: Option<String> },
        }

        match RequiredSectionRepr::deserialize(deserializer)? {
            RequiredSectionRepr::Simple(name) => Ok(RequiredSection { name, advice: None }),
            RequiredSectionRepr::Extended { name, advice } => {
                Ok(RequiredSection { name, advice })
            }
        }
    }
}
```

**Update AgentsConfig:**
```rust
pub struct AgentsConfig {
    // ... existing fields ...

    /// Section validation configuration.
    #[serde(default)]
    pub sections: SectionsConfig,
}
```

**Verification:**
```bash
cargo test checks::agents::config::tests
```

### Phase 2: Section Validation Logic

Create the core section validation functions.

**Tasks:**
1. Create `crates/cli/src/checks/agents/sections.rs`
2. Implement required section validation
3. Implement forbidden section validation with glob support
4. Add unit tests in `sections_tests.rs`

**Section validation in `sections.rs`:**
```rust
use crate::checks::agents::config::{RequiredSection, SectionsConfig};
use crate::checks::agents::sync::{Section, parse_sections};

/// Result of section validation.
#[derive(Debug)]
pub struct SectionValidation {
    /// Missing required sections.
    pub missing: Vec<MissingSection>,
    /// Present forbidden sections.
    pub forbidden: Vec<ForbiddenSection>,
}

#[derive(Debug)]
pub struct MissingSection {
    /// Required section name.
    pub name: String,
    /// Advice for adding the section.
    pub advice: Option<String>,
}

#[derive(Debug)]
pub struct ForbiddenSection {
    /// Matched section heading (original case).
    pub heading: String,
    /// Line number where section starts.
    pub line: u32,
    /// Pattern that matched (for advice).
    pub matched_pattern: String,
}

/// Validate sections in content against configuration.
pub fn validate_sections(content: &str, config: &SectionsConfig) -> SectionValidation {
    let sections = parse_sections(content);

    let missing = check_required(&sections, &config.required);
    let forbidden = check_forbidden(&sections, &config.forbid);

    SectionValidation { missing, forbidden }
}

/// Check for missing required sections.
fn check_required(sections: &[Section], required: &[RequiredSection]) -> Vec<MissingSection> {
    let section_names: Vec<String> = sections
        .iter()
        .map(|s| s.name.clone()) // Already normalized (lowercase)
        .collect();

    required
        .iter()
        .filter(|req| {
            let normalized = req.name.trim().to_lowercase();
            !section_names.contains(&normalized)
        })
        .map(|req| MissingSection {
            name: req.name.clone(),
            advice: req.advice.clone(),
        })
        .collect()
}

/// Check for forbidden sections (supports glob patterns).
fn check_forbidden(sections: &[Section], forbid: &[String]) -> Vec<ForbiddenSection> {
    let mut forbidden = Vec::new();

    for section in sections {
        for pattern in forbid {
            if matches_section_pattern(&section.name, pattern) {
                forbidden.push(ForbiddenSection {
                    heading: section.heading.clone(),
                    line: section.line,
                    matched_pattern: pattern.clone(),
                });
                break; // One match per section is enough
            }
        }
    }

    forbidden
}

/// Check if a section name matches a pattern (case-insensitive, glob support).
fn matches_section_pattern(section_name: &str, pattern: &str) -> bool {
    let normalized_pattern = pattern.trim().to_lowercase();

    // Check for glob characters
    if normalized_pattern.contains('*') || normalized_pattern.contains('?') {
        // Use glob matching
        glob_match(&normalized_pattern, section_name)
    } else {
        // Exact match (case-insensitive, already normalized)
        section_name == normalized_pattern
    }
}

/// Simple glob matching for section names.
/// Supports * (any chars) and ? (single char).
fn glob_match(pattern: &str, text: &str) -> bool {
    let mut pattern_chars = pattern.chars().peekable();
    let mut text_chars = text.chars().peekable();

    while let Some(p) = pattern_chars.next() {
        match p {
            '*' => {
                // Match zero or more characters
                if pattern_chars.peek().is_none() {
                    return true; // Trailing * matches everything
                }
                // Try matching rest of pattern at each position
                let remaining_pattern: String = pattern_chars.collect();
                loop {
                    let remaining_text: String = text_chars.clone().collect();
                    if glob_match(&remaining_pattern, &remaining_text) {
                        return true;
                    }
                    if text_chars.next().is_none() {
                        return false;
                    }
                }
            }
            '?' => {
                // Match exactly one character
                if text_chars.next().is_none() {
                    return false;
                }
            }
            c => {
                // Literal match
                if text_chars.next() != Some(c) {
                    return false;
                }
            }
        }
    }

    text_chars.peek().is_none()
}
```

**Verification:**
```bash
cargo test checks::agents::sections::tests
```

### Phase 3: Integration with Agents Check

Integrate section validation into `AgentsCheck::run()` and generate violations.

**Tasks:**
1. Add `check_sections()` function in `mod.rs`
2. Generate `missing_section` violations with advice
3. Generate `forbidden_section` violations
4. Update metrics output

**Integration in `mod.rs`:**
```rust
mod sections;

use sections::validate_sections;

fn run(&self, ctx: &CheckContext) -> CheckResult {
    // ... existing detection and file validation ...

    // Check sections in each detected file
    check_sections(ctx, config, &detected, &mut violations);

    // ... rest of run() ...
}

/// Check section requirements in agent files.
fn check_sections(
    ctx: &CheckContext,
    config: &AgentsConfig,
    detected: &[DetectedFile],
    violations: &mut Vec<Violation>,
) {
    // Skip if no section requirements configured
    if config.sections.required.is_empty() && config.sections.forbid.is_empty() {
        return;
    }

    // Only check files at root scope for now
    let root_files: Vec<_> = detected.iter().filter(|f| f.scope == Scope::Root).collect();

    for file in root_files {
        let Ok(content) = std::fs::read_to_string(&file.path) else {
            continue;
        };

        let validation = validate_sections(&content, &config.sections);
        let filename = file.path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        // Generate violations for missing required sections
        for missing in validation.missing {
            let advice = if let Some(ref section_advice) = missing.advice {
                format!(
                    "Add a \"## {}\" section: {}",
                    missing.name, section_advice
                )
            } else {
                format!("Add a \"## {}\" section", missing.name)
            };

            violations.push(Violation::file_only(
                &filename,
                "missing_section",
                advice,
            ));
        }

        // Generate violations for forbidden sections
        for forbidden in validation.forbidden {
            let advice = format!(
                "Remove or rename the \"{}\" section (matches forbidden pattern \"{}\")",
                forbidden.heading, forbidden.matched_pattern
            );

            violations.push(Violation::file_line(
                &filename,
                forbidden.line,
                "forbidden_section",
                advice,
            ));
        }
    }
}
```

**Verification:**
```bash
cargo test checks::agents
```

### Phase 4: Profile Defaults

Implement Claude and Cursor profile defaults with standard section requirements.

**Tasks:**
1. Add profile configuration constants
2. Implement profile defaults in config loading
3. Add Landing the Plane template structure

**Profile defaults in `config/mod.rs`:**
```rust
/// Claude profile default configuration.
pub fn claude_profile_agents() -> AgentsConfig {
    AgentsConfig {
        check: CheckLevel::Error,
        files: vec!["CLAUDE.md".to_string()],
        required: vec!["CLAUDE.md".to_string()],
        optional: Vec::new(),
        forbid: Vec::new(),
        sync: true,
        sync_source: Some("CLAUDE.md".to_string()),
        sections: SectionsConfig {
            required: vec![
                RequiredSection {
                    name: "Directory Structure".to_string(),
                    advice: Some("Overview of project layout and key directories".to_string()),
                },
                RequiredSection {
                    name: "Landing the Plane".to_string(),
                    advice: Some("Checklist for AI agents before completing work".to_string()),
                },
            ],
            forbid: Vec::new(),
        },
        root: None,
        package: None,
        module: None,
    }
}

/// Cursor profile default configuration.
pub fn cursor_profile_agents() -> AgentsConfig {
    AgentsConfig {
        check: CheckLevel::Error,
        files: vec![".cursorrules".to_string()],
        required: vec![".cursorrules".to_string()],
        optional: Vec::new(),
        forbid: Vec::new(),
        sync: true,
        sync_source: Some(".cursorrules".to_string()),
        sections: SectionsConfig {
            required: vec![
                RequiredSection {
                    name: "Directory Structure".to_string(),
                    advice: Some("Overview of project layout and key directories".to_string()),
                },
                RequiredSection {
                    name: "Landing the Plane".to_string(),
                    advice: Some("Checklist for AI agents before completing work".to_string()),
                },
            ],
            forbid: Vec::new(),
        },
        root: None,
        package: None,
        module: None,
    }
}
```

**Landing the Plane template structure:**
```rust
/// Base Landing the Plane checklist (always included).
pub const LANDING_THE_PLANE_BASE: &str = r#"## Landing the Plane

Before completing work:

- [ ] Run `quench check`
"#;

/// Rust-specific items for Landing the Plane.
pub const LANDING_THE_PLANE_RUST: &str = r#"- [ ] `cargo fmt --check`
- [ ] `cargo clippy -- -D warnings`
- [ ] `cargo test`
- [ ] `cargo build`
"#;

/// Shell-specific items for Landing the Plane.
pub const LANDING_THE_PLANE_SHELL: &str = r#"- [ ] `shellcheck scripts/*.sh`
- [ ] `bats tests/` (if present)
"#;

/// Generate Landing the Plane section for given language profiles.
pub fn generate_landing_the_plane(profiles: &[&str]) -> String {
    let mut content = LANDING_THE_PLANE_BASE.to_string();

    for profile in profiles {
        match *profile {
            "rust" => content.push_str(LANDING_THE_PLANE_RUST),
            "shell" => content.push_str(LANDING_THE_PLANE_SHELL),
            _ => {}
        }
    }

    content
}
```

**Verification:**
```bash
cargo test config::profiles
```

### Phase 5: Test Fixtures and Behavioral Specs

Update fixtures and enable behavioral specs.

**Tasks:**
1. Update `tests/fixtures/agents/missing-section/` with advice config
2. Update `tests/fixtures/agents/forbidden-section/` with glob patterns
3. Remove `#[ignore]` from section specs
4. Add new specs for profile defaults

**Update missing-section fixture:**

`tests/fixtures/agents/missing-section/quench.toml`:
```toml
[check.agents]
required = ["CLAUDE.md"]

[[check.agents.sections.required]]
name = "Landing the Plane"
advice = "Checklist for AI agents before finishing work"
```

`tests/fixtures/agents/missing-section/CLAUDE.md`:
```markdown
# Project

A test project.

## Directory Structure

```
src/
└── main.rs
```
```

**Update forbidden-section fixture:**

`tests/fixtures/agents/forbidden-section/quench.toml`:
```toml
[check.agents]
required = ["CLAUDE.md"]
sections.forbid = ["API Keys", "Secrets", "Test*"]
```

`tests/fixtures/agents/forbidden-section/CLAUDE.md`:
```markdown
# Project

A test project.

## Directory Structure

```
src/
└── main.rs
```

## Testing Plan

This section should be flagged (matches Test*).
```

**Enable specs in `tests/specs/checks/agents.rs`:**
```rust
/// Spec: docs/specs/checks/agents.md#required-sections
///
/// > Missing required section generates violation with advice.
#[test]
fn agents_missing_section_generates_violation_with_advice() {
    let agents = check("agents").on("agents/missing-section").json().fails();
    let violations = agents.require("violations").as_array().unwrap();

    let missing = violations.iter().find(|v| {
        v.get("type").and_then(|t| t.as_str()) == Some("missing_section")
    });

    assert!(missing.is_some(), "should have missing_section violation");

    let advice = missing.unwrap().get("advice").and_then(|a| a.as_str()).unwrap();
    assert!(
        advice.contains("Landing the Plane") && advice.contains("Checklist"),
        "advice should include section name and configured advice"
    );
}

/// Spec: docs/specs/checks/agents.md#forbid-sections
///
/// > Forbidden section generates violation.
#[test]
fn agents_forbidden_section_generates_violation() {
    let agents = check("agents").on("agents/forbidden-section").json().fails();
    let violations = agents.require("violations").as_array().unwrap();

    let forbidden = violations.iter().find(|v| {
        v.get("type").and_then(|t| t.as_str()) == Some("forbidden_section")
    });

    assert!(forbidden.is_some(), "should have forbidden_section violation");

    let line = forbidden.unwrap().get("line").and_then(|l| l.as_u64());
    assert!(line.is_some(), "forbidden_section violation should have line number");
}

/// Spec: docs/specs/checks/agents.md#glob-patterns
///
/// > Glob patterns match multiple section names.
#[test]
fn agents_forbidden_section_glob_matches() {
    let agents = check("agents").on("agents/forbidden-section").json().fails();
    let violations = agents.require("violations").as_array().unwrap();

    let matches_test = violations.iter().any(|v| {
        v.get("type").and_then(|t| t.as_str()) == Some("forbidden_section")
            && v.get("advice")
                .and_then(|a| a.as_str())
                .map(|a| a.contains("Test*"))
                .unwrap_or(false)
    });

    assert!(matches_test, "should match Test* glob pattern");
}
```

**Verification:**
```bash
cargo test --test specs agents
```

## Key Implementation Details

### Case-Insensitive Section Matching

Section matching is always case-insensitive:
- Config: `name = "Directory Structure"`
- File heading: `## directory structure`
- Result: Match

The `sync.rs` parser already normalizes section names to lowercase.

### Glob Pattern Syntax

Forbidden sections support glob patterns:
- `*` matches zero or more characters
- `?` matches exactly one character
- Patterns are case-insensitive

Examples:
- `Test*` matches `Testing`, `Test Plan`, `Tests`
- `API?Key*` matches `API Key`, `API-Keys`, `API_Key_Storage`

### Advice Output Format

When a required section is missing:

Without advice configured:
```
agents: FAIL
  CLAUDE.md missing required section
    Add a "## Landing the Plane" section
```

With advice configured:
```
agents: FAIL
  CLAUDE.md missing required section
    Add a "## Landing the Plane" section: Checklist for AI agents before finishing work
```

### Profile Selection

Profiles apply during `quench init` and set default configurations:

```bash
quench init --profile claude
# Sets: required = ["CLAUDE.md"], sections.required = [...]

quench init --profile cursor
# Sets: required = [".cursorrules"], sections.required = [...]

quench init --profile claude --profile cursor
# Sets: required = ["CLAUDE.md"], optional = [".cursorrules"], sync_source = "CLAUDE.md"
```

### Landing the Plane Auto-Population

During `quench init`, if an agent file exists but lacks "Landing the Plane":
1. Check if section already exists (case-insensitive)
2. If missing, append the generated section
3. Base checklist always includes `quench check`
4. Language-specific items added based on detected/selected profiles

The section is only added during `init`, never during `check`.

## Verification Plan

### Unit Tests

```bash
# Configuration parsing
cargo test checks::agents::config::tests

# Section validation logic
cargo test checks::agents::sections::tests

# Glob matching
cargo test checks::agents::sections::tests::glob

# Full agents check
cargo test checks::agents
```

### Behavioral Specs

```bash
# Run agents specs (should pass after implementation)
cargo test --test specs agents

# Show remaining ignored specs
cargo test --test specs agents -- --ignored
```

### Full Validation

```bash
make check
```

### Acceptance Criteria

1. Required sections validated (case-insensitive)
2. Missing section violations include configured advice
3. Forbidden sections validated with glob support
4. Line numbers included in forbidden section violations
5. Claude profile sets correct defaults
6. Cursor profile sets correct defaults
7. All Phase 515 behavioral specs pass
8. `make check` passes

## Spec Status (After Implementation)

| Spec | Phase 515 Status |
|------|------------------|
| agents_detects_claude_md_at_project_root | ✅ Pass (505) |
| agents_detects_cursorrules_at_project_root | ✅ Pass (505) |
| agents_passes_on_valid_project | ✅ Pass (505) |
| agents_missing_required_file_generates_violation | ✅ Pass (505) |
| agents_forbidden_file_generates_violation | ✅ Pass (505) |
| agents_out_of_sync_generates_violation | ✅ Pass (510) |
| agents_fix_syncs_files_from_sync_source | ✅ Pass (510) |
| agents_missing_section_generates_violation_with_advice | ✅ Pass |
| agents_forbidden_section_generates_violation | ✅ Pass |
| agents_forbidden_section_glob_matches | ✅ Pass |
| agents_markdown_table_generates_violation | ⏳ Phase 520 |
| agents_file_over_max_lines_generates_violation | ⏳ Phase 520 |
| agents_file_over_max_tokens_generates_violation | ⏳ Phase 520 |
| agents_json_includes_files_found_and_in_sync_metrics | ✅ Pass (505) |
| agents_violation_type_is_valid | ✅ Pass (510) |
