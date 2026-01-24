# Phase 1535: Agent Config Output

**Root Feature:** `quench-init`

## Overview

Generate agent-specific configuration when agents are detected during `quench init`. When CLAUDE.md is detected, the `[check.agents]` section includes `required = ["CLAUDE.md"]`. When .cursorrules is detected, it includes `required = [".cursorrules"]`. When both are detected, the required list merges both files.

**Prerequisite:** Phase 1530 (Language Section Output) completed.

## Project Structure

Files involved:

```
crates/cli/src/
├── cli.rs           # agents_detected_section() function (already implemented)
├── cli_tests.rs     # Add unit tests for agents_detected_section()
├── init.rs          # Detection functions (Phase 1525)
└── main.rs          # run_init() wires sections to output (already implemented)

tests/specs/cli/
└── init.rs          # Behavioral specs for agent detection (already enabled)
```

Reference files:

```
docs/specs/commands/quench-init.md#agent-detection
docs/specs/checks/agents.md#profile-defaults
```

## Dependencies

No new dependencies. Uses existing infrastructure from Phase 1525 (agent detection).

## Implementation Phases

### Phase 1: Verify Current Implementation

The `agents_detected_section()` function exists in `crates/cli/src/cli.rs`:

```rust
pub fn agents_detected_section(agents: &[DetectedAgent]) -> String {
    if agents.is_empty() {
        return String::new();
    }

    let required: Vec<&str> = agents
        .iter()
        .map(|a| match a {
            DetectedAgent::Claude => "CLAUDE.md",
            DetectedAgent::Cursor => ".cursorrules",
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
```

**Issue:** The default template already has `[check.agents]\ncheck = "error"`. When agents are detected, we add another `[check.agents]` section, creating duplicate headers. TOML allows this (later sections extend earlier), but it's not clean.

### Phase 2: Fix Duplicate Section Issue

Modify `default_template()` in `cli.rs` to use a placeholder approach:

**Before (current):**
```rust
pub fn default_template() -> &'static str {
    r#"# Quench configuration
...
[check.agents]
check = "error"
...
"#
}
```

**After:**
```rust
/// Base template without [check.agents] section.
/// The agents section is generated separately to support required field.
pub fn default_template_base() -> &'static str {
    r#"# Quench configuration
# https://github.com/alfredjeanlab/quench
version = 1

[check.cloc]
check = "error"

[check.escapes]
check = "error"

"#
}

/// Portion of template after agents section.
pub fn default_template_suffix() -> &'static str {
    r#"
[check.docs]
check = "error"

[check.tests]
check = "off"  # stub in quench v0.3.0

[check.license]
check = "off"  # stub in quench v0.3.0

[git.commit]
check = "off"  # stub in quench v0.3.0

# Supported Languages:
# [rust], [golang], [javascript], [shell]
"#
}

/// Generate [check.agents] section with optional required field.
pub fn agents_section(agents: &[DetectedAgent]) -> String {
    if agents.is_empty() {
        return "[check.agents]\ncheck = \"error\"\n".to_string();
    }

    let required: Vec<&str> = agents
        .iter()
        .map(|a| match a {
            DetectedAgent::Claude => "CLAUDE.md",
            DetectedAgent::Cursor => ".cursorrules",
        })
        .collect();

    format!(
        "[check.agents]\ncheck = \"error\"\nrequired = {:?}\n",
        required
    )
}

/// Full default template for backward compatibility.
pub fn default_template() -> String {
    format!(
        "{}{}{}",
        default_template_base(),
        agents_section(&[]),
        default_template_suffix()
    )
}
```

### Phase 3: Update run_init() to Use New Template Structure

Modify `run_init()` in `main.rs`:

```rust
fn run_init(_cli: &Cli, args: &InitArgs) -> anyhow::Result<ExitCode> {
    use quench::cli::{
        agents_section, default_template_base, default_template_suffix,
        // ... language sections ...
    };

    // ...

    let (config, message) = if !args.with_profiles.is_empty() {
        // --with specified: use full profiles
        // (existing profile logic unchanged)
    } else {
        // No --with: run auto-detection
        let detected_langs = detect_languages(&cwd);
        let detected_agents = detect_agents(&cwd);

        // Build config with proper agents section placement
        let mut cfg = default_template_base().to_string();
        cfg.push_str(&agents_section(&detected_agents));
        cfg.push_str(default_template_suffix());

        // Add language sections (after # Supported Languages:)
        for lang in &detected_langs {
            cfg.push('\n');
            match lang {
                DetectedLanguage::Rust => cfg.push_str(rust_detected_section()),
                // ... other languages ...
            }
        }

        // Build message
        // (existing message logic unchanged)
    };

    // ...
}
```

### Phase 4: Add Unit Tests for agents_section()

Add to `crates/cli/src/cli_tests.rs`:

```rust
#[test]
fn agents_section_empty_has_no_required() {
    let section = agents_section(&[]);
    assert!(section.contains("[check.agents]"));
    assert!(section.contains("check = \"error\""));
    assert!(!section.contains("required"));
}

#[test]
fn agents_section_claude_requires_claude_md() {
    let section = agents_section(&[DetectedAgent::Claude]);
    assert!(section.contains("[check.agents]"));
    assert!(section.contains("check = \"error\""));
    assert!(section.contains("required"));
    assert!(section.contains("CLAUDE.md"));
}

#[test]
fn agents_section_cursor_requires_cursorrules() {
    let section = agents_section(&[DetectedAgent::Cursor]);
    assert!(section.contains("[check.agents]"));
    assert!(section.contains("required"));
    assert!(section.contains(".cursorrules"));
}

#[test]
fn agents_section_both_merges_required() {
    let section = agents_section(&[DetectedAgent::Claude, DetectedAgent::Cursor]);
    assert!(section.contains("required"));
    assert!(section.contains("CLAUDE.md"));
    assert!(section.contains(".cursorrules"));
    // Should be a single array with both
    assert!(section.matches("required").count() == 1);
}

#[test]
fn default_template_no_duplicate_agents_section() {
    let template = default_template();
    let count = template.matches("[check.agents]").count();
    assert_eq!(count, 1, "should have exactly one [check.agents] section");
}
```

### Phase 5: Run Full Check

```bash
make check
```

This verifies:
- `cargo fmt --all -- --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --all`
- `cargo build --all`
- `cargo audit`
- `cargo deny check`

## Key Implementation Details

### Output Format

When `quench init` detects Claude (via `CLAUDE.md`), the generated `quench.toml` includes:

```toml
[check.agents]
check = "error"
required = ["CLAUDE.md"]
```

When both Claude and Cursor are detected:

```toml
[check.agents]
check = "error"
required = ["CLAUDE.md", ".cursorrules"]
```

### Required List Format

The `required` field uses Rust's `{:?}` formatting for Vec, which produces:
- `["CLAUDE.md"]` for Claude only
- `[".cursorrules"]` for Cursor only
- `["CLAUDE.md", ".cursorrules"]` for both

This is valid TOML array syntax.

### Template Structure

The template is split into three parts:
1. **Base**: version, cloc, escapes sections
2. **Agents**: dynamically generated with optional required field
3. **Suffix**: docs, tests, license, git, language comment

This ensures exactly one `[check.agents]` section in the output.

## Verification Plan

### 1. Unit Tests

```bash
cargo test cli::tests::agents_section
```

Expected: All new tests pass.

### 2. Behavioral Specs

The existing specs in `tests/specs/cli/init.rs` should pass:

```bash
cargo test --test specs init_detects_claude_from_claude_md
cargo test --test specs init_detects_cursor_from_cursorrules
cargo test --test specs init_detects_cursor_from_mdc_rules
```

### 3. Manual Verification

```bash
# Create temp directory
cd /tmp && mkdir agent-test && cd agent-test

# Test Claude detection
echo '# Project' > CLAUDE.md
quench init
cat quench.toml | grep -A3 '\[check.agents\]'
# Expected:
# [check.agents]
# check = "error"
# required = ["CLAUDE.md"]

rm quench.toml

# Test both agents
echo '# Rules' > .cursorrules
quench init --force
cat quench.toml | grep -A3 '\[check.agents\]'
# Expected:
# [check.agents]
# check = "error"
# required = ["CLAUDE.md", ".cursorrules"]

# Cleanup
cd .. && rm -rf agent-test
```

### 4. No Duplicate Sections

```bash
cd /tmp && mkdir dup-test && cd dup-test
echo '# Project' > CLAUDE.md
quench init
grep -c '\[check.agents\]' quench.toml
# Expected: 1

cd .. && rm -rf dup-test
```

### 5. Full Check

```bash
make check
```

### 6. Spec Coverage

| Roadmap Item | Status | Verification |
|-------------|--------|--------------|
| Update `[check.agents]` when agents detected | Implemented | `agents_section()` generates complete section |
| Add `required = ["CLAUDE.md"]` for claude | Implemented | Unit test + behavioral spec |
| Add `required = [".cursorrules"]` for cursor | Implemented | Unit test + behavioral spec |
| Merge required lists when both detected | Implemented | `agents_section_both_merges_required` test |
| No duplicate section headers | **TODO** | Fix template structure |
