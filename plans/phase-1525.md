# Phase 1525: Agent Auto-Detection

**Root Feature:** `quench-init`

## Overview

Implement agent auto-detection for `quench init`. When running `quench init` without `--with`, the command will detect agent configuration files (Claude, Cursor) present in the project and update the `[check.agents]` section with appropriate `required` paths. Detection is additive: projects with both `CLAUDE.md` and `.cursorrules` will include both in the required list.

## Project Structure

Files to modify:

```
crates/cli/src/
├── init.rs         # Add DetectedAgent enum and detect_agents()
├── init_tests.rs   # Add agent detection unit tests
└── main.rs         # Update run_init to use agent detection

tests/specs/cli/
└── init.rs         # Enable Phase 1525 specs
```

Reference files:

```
docs/specs/commands/quench-init.md#agent-detection
plans/phase-1520.md                # Pattern reference
```

## Dependencies

No new dependencies. Follows the same pattern as language detection from Phase 1520.

## Implementation Phases

### Phase 1: Add Agent Detection Types and Functions

Add to `crates/cli/src/init.rs`:

```rust
/// Agents that can be detected in a project.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DetectedAgent {
    Claude,
    Cursor,
}

/// Detect all agents present in a project.
///
/// Returns a list of detected agents. Detection is additive:
/// a project with CLAUDE.md and .cursorrules returns both Claude and Cursor.
pub fn detect_agents(root: &Path) -> Vec<DetectedAgent> {
    let mut agents = Vec::new();

    // Claude: CLAUDE.md exists
    if root.join("CLAUDE.md").exists() {
        agents.push(DetectedAgent::Claude);
    }

    // Cursor: .cursorrules or .cursor/rules/*.md[c] exists
    if has_cursor_markers(root) {
        agents.push(DetectedAgent::Cursor);
    }

    agents
}

/// Check if project has Cursor markers.
fn has_cursor_markers(root: &Path) -> bool {
    // Check for .cursorrules
    if root.join(".cursorrules").exists() {
        return true;
    }

    // Check for .cursor/rules/*.md or .cursor/rules/*.mdc
    let rules_dir = root.join(".cursor/rules");
    if rules_dir.is_dir() {
        if let Ok(entries) = rules_dir.read_dir() {
            for entry in entries.filter_map(|e| e.ok()) {
                let path = entry.path();
                if path.is_file() {
                    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                        if ext == "md" || ext == "mdc" {
                            return true;
                        }
                    }
                }
            }
        }
    }

    false
}
```

Key design decisions:

| Decision | Rationale |
|----------|-----------|
| Separate `DetectedAgent` enum | Mirrors `DetectedLanguage` pattern |
| Vec return type | Additive detection, consistent with languages |
| Check `.cursorrules` first | More common case, short-circuit evaluation |

### Phase 2: Add Agent Section Generator

Add to `crates/cli/src/cli.rs`:

```rust
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

Note: For Cursor detection via `.cursor/rules/*.mdc`, we still output `.cursorrules` as the canonical required file. The detection proves Cursor is in use; the required path is the conventional location.

### Phase 3: Update run_init to Integrate Agent Detection

Modify `run_init` in `crates/cli/src/main.rs`:

```rust
use quench::init::{DetectedAgent, DetectedLanguage, detect_agents, detect_languages};

fn run_init(_cli: &Cli, args: &InitArgs) -> anyhow::Result<ExitCode> {
    // ... existing code ...

    let (config, message) = if !args.with_profiles.is_empty() {
        // --with specified: use full profiles, skip all detection
        // ... existing --with handling ...
    } else {
        // No --with: run auto-detection for both languages and agents
        let detected_langs = detect_languages(&cwd);
        let detected_agents = detect_agents(&cwd);

        let mut cfg = default_template().to_string();

        // Add language sections
        for lang in &detected_langs {
            cfg.push('\n');
            match lang {
                DetectedLanguage::Rust => cfg.push_str(rust_detected_section()),
                // ... etc
            }
        }

        // Add agent section if any detected
        if !detected_agents.is_empty() {
            cfg.push('\n');
            cfg.push_str(&agents_detected_section(&detected_agents));
        }

        // Build message listing detected items
        let mut detected_names = Vec::new();
        for lang in &detected_langs {
            detected_names.push(match lang {
                DetectedLanguage::Rust => "rust",
                // ... etc
            });
        }
        for agent in &detected_agents {
            detected_names.push(match agent {
                DetectedAgent::Claude => "claude",
                DetectedAgent::Cursor => "cursor",
            });
        }

        let msg = if detected_names.is_empty() {
            "Created quench.toml".to_string()
        } else {
            format!("Created quench.toml (detected: {})", detected_names.join(", "))
        };
        (cfg, msg)
    };

    // ... rest unchanged ...
}
```

### Phase 4: Unit Tests

Add tests to `crates/cli/src/init_tests.rs`:

```rust
#[test]
fn detect_claude_from_claude_md() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("CLAUDE.md"), "# Project").unwrap();

    let detected = detect_agents(temp.path());
    assert!(detected.contains(&DetectedAgent::Claude));
}

#[test]
fn detect_cursor_from_cursorrules() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join(".cursorrules"), "# Rules").unwrap();

    let detected = detect_agents(temp.path());
    assert!(detected.contains(&DetectedAgent::Cursor));
}

#[test]
fn detect_cursor_from_mdc_rules() {
    let temp = TempDir::new().unwrap();
    fs::create_dir_all(temp.path().join(".cursor/rules")).unwrap();
    fs::write(temp.path().join(".cursor/rules/project.mdc"), "# Rules").unwrap();

    let detected = detect_agents(temp.path());
    assert!(detected.contains(&DetectedAgent::Cursor));
}

#[test]
fn detect_cursor_from_md_rules() {
    let temp = TempDir::new().unwrap();
    fs::create_dir_all(temp.path().join(".cursor/rules")).unwrap();
    fs::write(temp.path().join(".cursor/rules/project.md"), "# Rules").unwrap();

    let detected = detect_agents(temp.path());
    assert!(detected.contains(&DetectedAgent::Cursor));
}

#[test]
fn agent_detection_is_additive() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("CLAUDE.md"), "# Project").unwrap();
    fs::write(temp.path().join(".cursorrules"), "# Rules").unwrap();

    let detected = detect_agents(temp.path());
    assert!(detected.contains(&DetectedAgent::Claude));
    assert!(detected.contains(&DetectedAgent::Cursor));
}

#[test]
fn no_agent_markers_returns_empty() {
    let temp = TempDir::new().unwrap();

    let detected = detect_agents(temp.path());
    assert!(detected.is_empty());
}
```

### Phase 5: Enable Behavioral Specs

Remove `#[ignore]` from these tests in `tests/specs/cli/init.rs`:

| Test | Verification |
|------|-------------|
| `init_detects_claude_from_claude_md` | CLAUDE.md adds `[check.agents]` with required |
| `init_detects_cursor_from_cursorrules` | .cursorrules adds `[check.agents]` with required |
| `init_detects_cursor_from_mdc_rules` | .cursor/rules/*.mdc adds `[check.agents]` |

## Key Implementation Details

### Detection Order

Agents are detected in this order: Claude, Cursor. This matches the enum definition order and ensures consistent output.

### Required File Output

When an agent is detected, the `required` array contains the canonical marker file:

| Agent | Detection Markers | Output in `required` |
|-------|-------------------|---------------------|
| Claude | `CLAUDE.md` | `"CLAUDE.md"` |
| Cursor | `.cursorrules`, `.cursor/rules/*.md[c]` | `".cursorrules"` |

This means Cursor detection from `.cursor/rules/*.mdc` still outputs `.cursorrules` as the required file, since that's the conventional location.

### Template Integration

The `agents_detected_section()` function replaces the default `[check.agents]` placeholder in the template. The default template already has a `[check.agents]` section with `check = "off"` - the detected output replaces this with `check = "error"` and the required files.

### Skip Detection with --with

When `--with` is specified, both language and agent detection are skipped. This maintains the existing behavior where `--with` gives full control to the user.

## Verification Plan

### 1. Unit Tests

```bash
cargo test init::tests::detect_claude
cargo test init::tests::detect_cursor
cargo test init::tests::agent_detection_is_additive
cargo test init::tests::no_agent_markers
```

Expected: All agent detection unit tests pass.

### 2. Behavioral Specs

```bash
cargo test --test specs init_detects_claude
cargo test --test specs init_detects_cursor
```

Expected: All 3 Phase 1525 specs pass.

### 3. Manual Verification

```bash
# Detection: Claude project
cd /tmp && mkdir claude-test && cd claude-test
echo '# Project' > CLAUDE.md
quench init
cat quench.toml  # Should have [check.agents] with required = ["CLAUDE.md"]

# Detection: Cursor project
cd /tmp && mkdir cursor-test && cd cursor-test
echo '# Rules' > .cursorrules
quench init
cat quench.toml  # Should have [check.agents] with required = [".cursorrules"]

# Detection: Multi-agent
cd /tmp && mkdir multi-agent-test && cd multi-agent-test
echo '# Project' > CLAUDE.md
echo '# Rules' > .cursorrules
quench init
cat quench.toml  # Should have required = ["CLAUDE.md", ".cursorrules"]

# --with skips agent detection
cd /tmp && mkdir skip-test && cd skip-test
echo '# Project' > CLAUDE.md
quench init --with shell
cat quench.toml  # Should NOT have CLAUDE.md in required
```

### 4. Full Check

```bash
make check
```

### 5. Spec Coverage

| Spec Requirement | Test Function | Status |
|-----------------|---------------|--------|
| CLAUDE.md → claude | `init_detects_claude_from_claude_md` | Enable |
| .cursorrules → cursor | `init_detects_cursor_from_cursorrules` | Enable |
| .cursor/rules/*.mdc → cursor | `init_detects_cursor_from_mdc_rules` | Enable |
| --with skips detection | `init_with_skips_auto_detection` | Already passing |
| Detection is additive | `init_detection_is_additive` | Already passing |
