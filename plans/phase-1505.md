# Phase 1505: Init Command - Behavioral Specs

**Root Feature:** `quench-5065`

## Overview

Write behavioral specifications for the `quench init` command per the spec in `docs/specs/commands/quench-init.md`. These specs define the expected behavior for:

- `--with` flag for explicit profile selection
- Language auto-detection from project markers
- Agent file auto-detection
- Output format matching the template

All specs will use `#[ignore = "TODO: Phase 15xx"]` since the implementation comes in later phases.

## Project Structure

```
tests/
├── specs/
│   └── cli/
│       └── init.rs    # Add new specs here (existing file)
└── fixtures/
    └── init/          # NEW: Fixtures for init detection tests
        ├── rust-project/
        │   └── Cargo.toml
        ├── go-project/
        │   └── go.mod
        ├── js-project/
        │   └── package.json
        ├── shell-project/
        │   └── scripts/
        │       └── build.sh
        ├── multi-lang/
        │   ├── Cargo.toml
        │   └── scripts/
        │       └── deploy.sh
        ├── claude-agent/
        │   └── CLAUDE.md
        ├── cursor-rules/
        │   └── .cursorrules
        ├── cursor-mdc/
        │   └── .cursor/
        │       └── rules/
        │           └── project.mdc
        └── full-project/
            ├── Cargo.toml
            └── CLAUDE.md
```

## Dependencies

No new dependencies required. Uses existing test helpers:

- `crate::prelude::*` - Test DSL (`quench_cmd()`, `Project::empty()`)
- `predicates` - String matching
- `tempfile` - Temp directories (via `Project`)

## Implementation Phases

### Phase 1: Create Test Fixtures (5 fixtures)

Create minimal fixture directories for detection tests.

**Files to create:**

```
tests/fixtures/init/rust-project/Cargo.toml
tests/fixtures/init/go-project/go.mod
tests/fixtures/init/js-project/package.json
tests/fixtures/init/shell-project/scripts/build.sh
tests/fixtures/init/multi-lang/Cargo.toml
tests/fixtures/init/multi-lang/scripts/deploy.sh
```

**Content:** Minimal valid files (e.g., `[package]\nname = "test"` for Cargo.toml).

### Phase 2: --with Flag Specs (3 tests)

Specs for explicit profile selection behavior.

```rust
/// Spec: docs/specs/commands/quench-init.md#--with-flag
///
/// > --with flag accepts comma-separated profiles
#[test]
#[ignore = "TODO: Phase 1510 - Rename --profile to --with"]
fn init_with_accepts_comma_separated_profiles() {
    let temp = Project::empty();
    quench_cmd()
        .args(["init", "--with", "rust,shell"])
        .current_dir(temp.path())
        .assert()
        .success();

    let config = std::fs::read_to_string(temp.path().join("quench.toml")).unwrap();
    assert!(config.contains("[rust]"));
    assert!(config.contains("[shell]"));
}

/// Spec: docs/specs/commands/quench-init.md#--with-flag
///
/// > --with skips auto-detection
#[test]
#[ignore = "TODO: Phase 1520 - Language Auto-Detection"]
fn init_with_skips_auto_detection() {
    let temp = Project::empty();
    temp.file("Cargo.toml", "[package]\nname = \"test\"\n");
    temp.file("go.mod", "module test\n");

    quench_cmd()
        .args(["init", "--with", "shell"])
        .current_dir(temp.path())
        .assert()
        .success();

    let config = std::fs::read_to_string(temp.path().join("quench.toml")).unwrap();
    assert!(config.contains("[shell]"));
    assert!(!config.contains("[rust]"), "--with should skip rust detection");
    assert!(!config.contains("[golang]"), "--with should skip go detection");
}

/// Spec: docs/specs/commands/quench-init.md#language-detection
///
/// > no --with triggers auto-detection
#[test]
#[ignore = "TODO: Phase 1520 - Language Auto-Detection"]
fn init_without_with_triggers_auto_detection() {
    let temp = Project::empty();
    temp.file("Cargo.toml", "[package]\nname = \"test\"\n");

    quench_cmd()
        .args(["init"])
        .current_dir(temp.path())
        .assert()
        .success();

    let config = std::fs::read_to_string(temp.path().join("quench.toml")).unwrap();
    assert!(config.contains("[rust]"), "should auto-detect rust");
}
```

### Phase 3: Language Detection Specs (5 tests)

Specs for each language marker detection.

```rust
/// Spec: docs/specs/commands/quench-init.md#language-detection
///
/// > Cargo.toml → rust
#[test]
#[ignore = "TODO: Phase 1520 - Language Auto-Detection"]
fn init_detects_rust_from_cargo_toml() {
    let temp = Project::empty();
    temp.file("Cargo.toml", "[package]\nname = \"test\"\n");

    quench_cmd()
        .args(["init"])
        .current_dir(temp.path())
        .assert()
        .success();

    let config = std::fs::read_to_string(temp.path().join("quench.toml")).unwrap();
    assert!(config.contains("[rust]"));
    assert!(config.contains("rust.cloc.check"));
}

/// Spec: docs/specs/commands/quench-init.md#language-detection
///
/// > go.mod → golang
#[test]
#[ignore = "TODO: Phase 1520 - Language Auto-Detection"]
fn init_detects_golang_from_go_mod() {
    let temp = Project::empty();
    temp.file("go.mod", "module test\n");

    quench_cmd()
        .args(["init"])
        .current_dir(temp.path())
        .assert()
        .success();

    let config = std::fs::read_to_string(temp.path().join("quench.toml")).unwrap();
    assert!(config.contains("[golang]"));
}

/// Spec: docs/specs/commands/quench-init.md#language-detection
///
/// > package.json → javascript
#[test]
#[ignore = "TODO: Phase 1520 - Language Auto-Detection"]
fn init_detects_javascript_from_package_json() {
    let temp = Project::empty();
    temp.file("package.json", "{\"name\": \"test\"}\n");

    quench_cmd()
        .args(["init"])
        .current_dir(temp.path())
        .assert()
        .success();

    let config = std::fs::read_to_string(temp.path().join("quench.toml")).unwrap();
    assert!(config.contains("[javascript]"));
}

/// Spec: docs/specs/commands/quench-init.md#language-detection
///
/// > *.sh in root/bin/scripts → shell
#[test]
#[ignore = "TODO: Phase 1520 - Language Auto-Detection"]
fn init_detects_shell_from_scripts_dir() {
    let temp = Project::empty();
    temp.file("scripts/build.sh", "#!/bin/bash\necho hello\n");

    quench_cmd()
        .args(["init"])
        .current_dir(temp.path())
        .assert()
        .success();

    let config = std::fs::read_to_string(temp.path().join("quench.toml")).unwrap();
    assert!(config.contains("[shell]"));
}

/// Spec: docs/specs/commands/quench-init.md#language-detection
///
/// > Detection is additive (multiple languages/agents)
#[test]
#[ignore = "TODO: Phase 1520 - Language Auto-Detection"]
fn init_detection_is_additive() {
    let temp = Project::empty();
    temp.file("Cargo.toml", "[package]\nname = \"test\"\n");
    temp.file("scripts/deploy.sh", "#!/bin/bash\n");

    quench_cmd()
        .args(["init"])
        .current_dir(temp.path())
        .assert()
        .success();

    let config = std::fs::read_to_string(temp.path().join("quench.toml")).unwrap();
    assert!(config.contains("[rust]"), "should detect rust");
    assert!(config.contains("[shell]"), "should detect shell");
}
```

### Phase 4: Agent Detection Specs (3 tests)

Specs for agent file detection.

```rust
/// Spec: docs/specs/commands/quench-init.md#agent-detection
///
/// > CLAUDE.md → claude
#[test]
#[ignore = "TODO: Phase 1525 - Agent Auto-Detection"]
fn init_detects_claude_from_claude_md() {
    let temp = Project::empty();
    temp.file("CLAUDE.md", "# Project\n");

    quench_cmd()
        .args(["init"])
        .current_dir(temp.path())
        .assert()
        .success();

    let config = std::fs::read_to_string(temp.path().join("quench.toml")).unwrap();
    assert!(config.contains("[check.agents]"));
    assert!(config.contains("required") && config.contains("CLAUDE.md"));
}

/// Spec: docs/specs/commands/quench-init.md#agent-detection
///
/// > .cursorrules → cursor
#[test]
#[ignore = "TODO: Phase 1525 - Agent Auto-Detection"]
fn init_detects_cursor_from_cursorrules() {
    let temp = Project::empty();
    temp.file(".cursorrules", "# Cursor rules\n");

    quench_cmd()
        .args(["init"])
        .current_dir(temp.path())
        .assert()
        .success();

    let config = std::fs::read_to_string(temp.path().join("quench.toml")).unwrap();
    assert!(config.contains("[check.agents]"));
    assert!(config.contains("required") && config.contains(".cursorrules"));
}

/// Spec: docs/specs/commands/quench-init.md#agent-detection
///
/// > .cursor/rules/*.md[c] → cursor
#[test]
#[ignore = "TODO: Phase 1525 - Agent Auto-Detection"]
fn init_detects_cursor_from_mdc_rules() {
    let temp = Project::empty();
    temp.file(".cursor/rules/project.mdc", "# Cursor rules\n");

    quench_cmd()
        .args(["init"])
        .current_dir(temp.path())
        .assert()
        .success();

    let config = std::fs::read_to_string(temp.path().join("quench.toml")).unwrap();
    assert!(config.contains("[check.agents]"));
    // Should detect cursor agent presence
}
```

### Phase 5: Output Format Specs (2 tests)

Specs for output format matching template.

```rust
/// Spec: docs/specs/commands/quench-init.md#default-output
///
/// > Output matches templates/init.default.toml format
#[test]
#[ignore = "TODO: Phase 1515 - Init Output Template"]
fn init_output_matches_template_format() {
    let temp = Project::empty();

    quench_cmd()
        .args(["init"])
        .current_dir(temp.path())
        .assert()
        .success();

    let config = std::fs::read_to_string(temp.path().join("quench.toml")).unwrap();

    // Base template fields
    assert!(config.contains("version = 1"));
    assert!(config.contains("[check.cloc]"));
    assert!(config.contains("[check.escapes]"));
    assert!(config.contains("[check.agents]"));
    assert!(config.contains("[check.docs]"));
    assert!(config.contains("# Supported Languages:"));
}

/// Spec: docs/specs/commands/quench-init.md#language-detection
///
/// > Detected language appends [lang] section with dotted keys
#[test]
#[ignore = "TODO: Phase 1530 - Language Section Output"]
fn init_detected_language_uses_dotted_keys() {
    let temp = Project::empty();
    temp.file("Cargo.toml", "[package]\nname = \"test\"\n");

    quench_cmd()
        .args(["init"])
        .current_dir(temp.path())
        .assert()
        .success();

    let config = std::fs::read_to_string(temp.path().join("quench.toml")).unwrap();

    assert!(config.contains("[rust]"));
    assert!(config.contains("rust.cloc.check"));
    assert!(config.contains("rust.policy.check"));
    assert!(config.contains("rust.suppress.check"));
}
```

## Key Implementation Details

### Test Helper Usage

All specs use the existing test DSL from `tests/specs/prelude.rs`:

```rust
use crate::prelude::*;

// Empty project for init tests
let temp = Project::empty();
temp.file("path/to/file", "content");

// Run quench init
quench_cmd()
    .args(["init", "--with", "rust"])
    .current_dir(temp.path())
    .assert()
    .success();

// Read generated config
let config = std::fs::read_to_string(temp.path().join("quench.toml")).unwrap();
```

### Ignore Annotations

Each spec targets a specific implementation phase:

| Ignore Tag | Target Phase |
|------------|--------------|
| `Phase 1510` | Rename --profile to --with |
| `Phase 1515` | Init Output Template |
| `Phase 1520` | Language Auto-Detection |
| `Phase 1525` | Agent Auto-Detection |
| `Phase 1530` | Language Section Output |
| `Phase 1535` | Agent Config Output |

### Spec Documentation Pattern

Each test includes a doc comment referencing the specification:

```rust
/// Spec: docs/specs/commands/quench-init.md#section-name
///
/// > Quoted text from the spec
#[test]
#[ignore = "TODO: Phase NNNN - Description"]
fn descriptive_test_name() { ... }
```

## Verification Plan

### 1. Compile Check

All specs must compile without errors:

```bash
cargo test --test specs -- --ignored 2>&1 | head -20
```

### 2. Ignored Count

Verify the expected number of ignored tests:

```bash
cargo test --test specs init -- --ignored 2>&1 | grep -c "ignored"
# Expected: 14 new ignored tests (13 specs + existing tests updated)
```

### 3. Fixture Verification

Verify fixture files exist and are valid:

```bash
ls -la tests/fixtures/init/*/
```

### 4. Spec Coverage Matrix

| Spec Requirement | Test Function |
|-----------------|---------------|
| --with accepts comma-separated | `init_with_accepts_comma_separated_profiles` |
| --with skips auto-detection | `init_with_skips_auto_detection` |
| no --with triggers detection | `init_without_with_triggers_auto_detection` |
| Cargo.toml → rust | `init_detects_rust_from_cargo_toml` |
| go.mod → golang | `init_detects_golang_from_go_mod` |
| package.json → javascript | `init_detects_javascript_from_package_json` |
| *.sh → shell | `init_detects_shell_from_scripts_dir` |
| CLAUDE.md → claude | `init_detects_claude_from_claude_md` |
| .cursorrules → cursor | `init_detects_cursor_from_cursorrules` |
| .cursor/rules/*.mdc → cursor | `init_detects_cursor_from_mdc_rules` |
| Detection is additive | `init_detection_is_additive` |
| Output matches template | `init_output_matches_template_format` |
| Language uses dotted keys | `init_detected_language_uses_dotted_keys` |
| Agent updates required | `init_detects_claude_from_claude_md` (verifies required field) |

### 5. Final Check

Run `make check` to ensure no regressions:

```bash
make check
```
