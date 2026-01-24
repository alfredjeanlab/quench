# Checkpoint 15C: Init Command Refactor - Ratcheting

**Plan:** `checkpoint-15c-refactor`
**Root Feature:** `quench-init`
**Depends On:** checkpoint-15b-validate (dogfooding complete)

## Overview

Refactoring checkpoint to address behavioral gaps identified during validation and add ratcheting configuration support to the init command. The validation report (`reports/checkpoint-15-init.md`) identified two minor gaps and the ratcheting spec (`docs/specs/04-ratcheting.md`) defines config that should be generated.

**Current State**: Init command complete with 411 tests passing. Two minor gaps identified:
1. Missing `claude`/`cursor` profiles for `--with` flag
2. Cursor detection always sets `.cursorrules` as required (even for `.mdc` detection)

**End State**:
- Both behavioral gaps addressed
- Ratcheting config added to default template
- Profile code refactored for extensibility
- `make check` passes

## Project Structure

Files to create/modify:

```
crates/cli/src/
├── cli.rs              # Add claude/cursor profiles, add ratchet template
├── init.rs             # Improve cursor detection to track actual files found
└── init_tests.rs       # Add tests for new behavior

tests/specs/
└── cli_init.rs         # Add behavioral specs for profiles

docs/specs/templates/
└── init.default.toml   # Update to include [ratchet] section
```

## Dependencies

No new dependencies required.

## Implementation Phases

### Phase 15C.1: Add Agent Profiles for --with Flag

**Goal**: Support `quench init --with claude` and `--with cursor`.

**Current behavior**: `--with claude` produces warning "unknown profile 'claude', skipping"

**Solution**: Add agent profiles alongside language profiles in `cli.rs`.

**Add to `cli.rs`:**
```rust
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
    r#"[javascript]
source = ["**/*.js", "**/*.jsx", "**/*.ts", "**/*.tsx", "**/*.mjs", "**/*.mts"]
tests = ["**/*.test.*", "**/*.spec.*", "**/test/**", "**/tests/**", "**/__tests__/**"]

[javascript.suppress]
check = "comment"

[javascript.suppress.test]
check = "allow"

[javascript.policy]
lint_changes = "standalone"
lint_config = [".eslintrc", ".eslintrc.json", ".eslintrc.js", "eslint.config.js", ".prettierrc", ".prettierrc.json"]

[[check.escapes.patterns]]
name = "any_type"
pattern = ": any\\b"
action = "comment"
comment = "// ANY:"
advice = "Add a // ANY: comment explaining why any is needed."

[[check.escapes.patterns]]
name = "ts_ignore"
pattern = "@ts-ignore"
action = "forbid"
advice = "Use @ts-expect-error with an explanation instead."

[[check.escapes.patterns]]
name = "eslint_disable"
pattern = "eslint-disable"
action = "comment"
advice = "Add a comment explaining why this rule is disabled."
"#
    .to_string()
}
```

**Update profile lookup in `main.rs` or command handler:**
```rust
fn get_profile(name: &str) -> Option<String> {
    match name.to_lowercase().as_str() {
        "rust" => Some(rust_profile_defaults()),
        "shell" => Some(shell_profile_defaults()),
        "golang" | "go" => Some(golang_profile_defaults()),
        "javascript" | "js" | "typescript" | "ts" => Some(javascript_profile_defaults()),
        "claude" => Some(claude_profile_defaults().to_string()),
        "cursor" => Some(cursor_profile_defaults().to_string()),
        _ => None,
    }
}
```

**Milestone**: `quench init --with claude` generates config with `required = ["CLAUDE.md"]`

---

### Phase 15C.2: Fix Cursor Detection Required Value

**Goal**: Set `required` based on what was actually detected, not a hardcoded value.

**Current behavior**: Cursor detection returns `DetectedAgent::Cursor` but always maps to `.cursorrules`.

**Solution**: Enhance `DetectedAgent` to carry the actual file found.

**Update `init.rs`:**
```rust
/// Agents that can be detected in a project.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DetectedAgent {
    Claude,
    /// Cursor agent with the actual required file/pattern found.
    Cursor(CursorMarker),
}

/// Cursor marker type detected in the project.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CursorMarker {
    /// .cursorrules file exists
    Cursorrules,
    /// .cursor/rules/*.md or *.mdc files exist
    CursorRulesDir,
}

/// Detect all agents present in a project.
pub fn detect_agents(root: &Path) -> Vec<DetectedAgent> {
    let mut agents = Vec::new();

    // Claude: CLAUDE.md exists
    if root.join("CLAUDE.md").exists() {
        agents.push(DetectedAgent::Claude);
    }

    // Cursor: check for markers
    if root.join(".cursorrules").exists() {
        agents.push(DetectedAgent::Cursor(CursorMarker::Cursorrules));
    } else if has_cursor_rules_dir(root) {
        agents.push(DetectedAgent::Cursor(CursorMarker::CursorRulesDir));
    }

    agents
}

/// Check if project has .cursor/rules/*.md[c] files.
fn has_cursor_rules_dir(root: &Path) -> bool {
    let rules_dir = root.join(".cursor/rules");
    if rules_dir.is_dir()
        && let Ok(entries) = rules_dir.read_dir()
    {
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_file()
                && let Some(ext) = path.extension().and_then(|e| e.to_str())
                && (ext == "md" || ext == "mdc")
            {
                return true;
            }
        }
    }
    false
}
```

**Update `cli.rs` to handle the marker type:**
```rust
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
```

**Milestone**: `quench init` with only `.cursor/rules/*.mdc` sets `required = [".cursor/rules/*.mdc"]`

---

### Phase 15C.3: Add Ratcheting Config to Default Template

**Goal**: Include `[ratchet]` section in generated config per spec.

**Per `docs/specs/04-ratcheting.md`**, the default template should include:

```toml
[git]
baseline = ".quench/baseline.json"

[ratchet]
check = "error"
coverage = true
escapes = true
```

**Update `cli.rs` default template functions:**
```rust
/// Portion of template after agents section.
pub fn default_template_suffix() -> &'static str {
    r#"
[check.docs]
check = "error"

[check.tests]
check = "off"  # stub in quench v0.3.0

[check.license]
check = "off"  # stub in quench v0.3.0

[git]
baseline = ".quench/baseline.json"

[git.commit]
check = "off"  # stub in quench v0.3.0

[ratchet]
check = "error"       # error | warn | off
coverage = true       # Coverage can't drop
escapes = true        # Escape counts can't increase
# binary_size = false # Opt-in: binaries can't grow
# build_time_cold = false
# build_time_hot = false
# test_time_total = false

# Supported Languages:
# [rust], [golang], [javascript], [shell]
"#
}
```

**Update `docs/specs/templates/init.default.toml`** to match.

**Milestone**: `quench init` generates config with `[ratchet]` section.

---

### Phase 15C.4: Add Config Struct for Ratcheting

**Goal**: Add `RatchetConfig` to `config/mod.rs` for parsing.

**Create `config/ratchet.rs`:**
```rust
//! Ratcheting configuration.

use serde::Deserialize;
use crate::config::CheckLevel;

/// Ratcheting configuration.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RatchetConfig {
    /// Check level: "error" | "warn" | "off"
    #[serde(default)]
    pub check: CheckLevel,

    /// Ratchet coverage (default: true).
    #[serde(default = "default_true")]
    pub coverage: bool,

    /// Ratchet escape hatch counts (default: true).
    #[serde(default = "default_true")]
    pub escapes: bool,

    /// Ratchet binary size (default: false).
    #[serde(default)]
    pub binary_size: bool,

    /// Ratchet cold build time (default: false).
    #[serde(default)]
    pub build_time_cold: bool,

    /// Ratchet hot build time (default: false).
    #[serde(default)]
    pub build_time_hot: bool,

    /// Ratchet total test time (default: false).
    #[serde(default)]
    pub test_time_total: bool,

    /// Ratchet average test time (default: false).
    #[serde(default)]
    pub test_time_avg: bool,

    /// Ratchet max single test time (default: false).
    #[serde(default)]
    pub test_time_max: bool,

    /// Coverage tolerance (percentage points allowed to drop).
    #[serde(default)]
    pub coverage_tolerance: Option<f64>,

    /// Binary size tolerance (e.g., "100KB").
    #[serde(default)]
    pub binary_size_tolerance: Option<String>,

    /// Build time tolerance (e.g., "5s").
    #[serde(default)]
    pub build_time_tolerance: Option<String>,
}

fn default_true() -> bool {
    true
}
```

**Update `config/mod.rs`:**
```rust
mod ratchet;
pub use ratchet::RatchetConfig;

// In Config struct:
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    // ... existing fields ...

    /// Git configuration.
    #[serde(default)]
    pub git: GitConfig,

    /// Ratcheting configuration.
    #[serde(default)]
    pub ratchet: RatchetConfig,
}

/// Git configuration.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GitConfig {
    /// Baseline file path for ratcheting.
    #[serde(default = "GitConfig::default_baseline")]
    pub baseline: String,

    /// Commit message validation settings.
    #[serde(default)]
    pub commit: GitCommitConfig,
}

impl GitConfig {
    fn default_baseline() -> String {
        ".quench/baseline.json".to_string()
    }
}
```

**Milestone**: Config parsing accepts `[ratchet]` and `[git].baseline` fields.

---

### Phase 15C.5: Refactor Profile Registry

**Goal**: Centralize profile lookup for maintainability.

**Current state**: Profile strings scattered across `cli.rs` with no unified registry.

**Solution**: Create `ProfileRegistry` struct:

```rust
/// Profile registry for init command.
pub struct ProfileRegistry;

impl ProfileRegistry {
    /// Get all available profile names.
    pub fn available() -> &'static [&'static str] {
        &["rust", "golang", "javascript", "shell", "claude", "cursor"]
    }

    /// Get profile content by name.
    pub fn get(name: &str) -> Option<String> {
        match name.to_lowercase().as_str() {
            "rust" => Some(rust_profile_defaults()),
            "shell" => Some(shell_profile_defaults()),
            "golang" | "go" => Some(golang_profile_defaults()),
            "javascript" | "js" | "typescript" | "ts" => Some(javascript_profile_defaults()),
            "claude" => Some(claude_profile_defaults().to_string()),
            "cursor" => Some(cursor_profile_defaults().to_string()),
            _ => None,
        }
    }

    /// Check if a profile name is valid.
    pub fn is_valid(name: &str) -> bool {
        Self::get(name).is_some()
    }

    /// Suggest similar profile names for typos.
    pub fn suggest(name: &str) -> Option<&'static str> {
        let lower = name.to_lowercase();
        Self::available()
            .iter()
            .find(|&&p| strsim::jaro_winkler(&lower, p) > 0.8)
            .copied()
    }
}
```

**Milestone**: Profile lookup unified, extensible for future profiles.

---

### Phase 15C.6: Run Tests and Quality Gates

**Goal**: Verify all changes preserve behavior and pass quality checks.

**Commands:**
```bash
# Run all tests
cargo test --all

# Run init-specific tests
cargo test init
cargo test --test specs cli_init

# Full quality gates
make check
```

**Expected**: All 411+ tests pass, no warnings, no formatting issues.

**Milestone**: `make check` passes.

---

## Key Implementation Details

### Agent Profile Behavior

When `--with claude` is specified:
- Full agent profile is used (not just detection section)
- `[check.agents]` is set with `required = ["CLAUDE.md"]`
- No language sections are added (auto-detection skipped)

When `--with rust,claude` is specified:
- Both rust language profile and claude agent profile are merged
- Escape patterns from rust profile included
- Agent required file set correctly

### Cursor Detection Rules

| Marker Found | `required` Value |
|--------------|------------------|
| `.cursorrules` | `[".cursorrules"]` |
| `.cursor/rules/*.mdc` only | `[".cursor/rules/*.mdc"]` |
| Both | `[".cursorrules"]` (canonical) |

### Ratcheting Defaults

Per spec, ratcheting should:
- Be on by default for `coverage` and `escapes`
- Be off by default for timing/size metrics (noisy)
- Use `.quench/baseline.json` as default baseline path

### Backward Compatibility

- Existing configs without `[ratchet]` section work (defaults apply)
- Existing configs without `[git].baseline` work (default path used)
- Profile registry returns same content as before for language profiles

## Verification Plan

### Automated Verification

```bash
# Full CI check
make check

# Specific test suites
cargo test init
cargo test --test specs cli_init
cargo test config
```

### Manual Verification

```bash
# Test --with claude
TEMP=$(mktemp -d) && cd "$TEMP"
quench init --with claude
grep -q 'required = \["CLAUDE.md"\]' quench.toml && echo "PASS: claude profile"
cd - && rm -rf "$TEMP"

# Test --with cursor
TEMP=$(mktemp -d) && cd "$TEMP"
quench init --with cursor
grep -q 'required = \[".cursorrules"\]' quench.toml && echo "PASS: cursor profile"
cd - && rm -rf "$TEMP"

# Test cursor directory detection
TEMP=$(mktemp -d) && cd "$TEMP"
mkdir -p .cursor/rules && echo '# Rules' > .cursor/rules/project.mdc
quench init
grep -q '.cursor/rules' quench.toml && echo "PASS: cursor dir detection"
cd - && rm -rf "$TEMP"

# Test ratchet section in default
TEMP=$(mktemp -d) && cd "$TEMP"
quench init
grep -q '\[ratchet\]' quench.toml && echo "PASS: ratchet section"
grep -q 'coverage = true' quench.toml && echo "PASS: coverage default"
cd - && rm -rf "$TEMP"
```

### Success Criteria

- [ ] `quench init --with claude` generates valid config with CLAUDE.md required
- [ ] `quench init --with cursor` generates valid config with .cursorrules required
- [ ] Cursor `.mdc` detection sets appropriate required value
- [ ] Default template includes `[ratchet]` section
- [ ] Default template includes `[git].baseline`
- [ ] `RatchetConfig` struct parses all spec fields
- [ ] Profile registry centralizes lookup
- [ ] All 411+ tests pass
- [ ] `make check` passes
- [ ] No clippy warnings
