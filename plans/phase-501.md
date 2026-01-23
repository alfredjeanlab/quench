# Phase 501: Agents Check - Behavioral Specs

## Overview

Write behavioral specifications (black-box tests) for the `agents` check, which validates AI agent context files (CLAUDE.md, .cursorrules). These specs verify CLI behavior from `docs/specs/checks/agents.md` without testing internal implementation.

## Project Structure

```
tests/
├── specs/
│   └── checks/
│       └── agents.rs          # New spec file (12 specs)
└── fixtures/
    └── agents/                # New fixture subdirectory
        ├── basic/             # Synced files, all sections, passes
        ├── missing-file/      # Missing required CLAUDE.md
        ├── out-of-sync/       # CLAUDE.md differs from .cursorrules
        ├── missing-section/   # Missing "Landing the Plane" section
        ├── forbidden-section/ # Has forbidden "Secrets" section
        ├── with-table/        # Contains markdown table
        ├── oversized-lines/   # Over max_lines limit
        ├── oversized-tokens/  # Over max_tokens limit
        └── metrics/           # For JSON metrics testing
```

## Dependencies

No new external dependencies. Uses existing test infrastructure:
- `assert_cmd` for CLI invocation
- `predicates` for assertions
- `serde_json` for JSON output parsing
- `tempfile` for temporary directories

## Implementation Phases

### Phase 1: Spec File Scaffold and Basic Detection (3 specs)

Create the spec file structure and implement basic file detection specs.

**Tasks:**
1. Create `tests/specs/checks/agents.rs` with module documentation
2. Create `tests/fixtures/agents/basic/` fixture with synced files
3. Implement specs:
   - `agents_detects_claude_md_at_project_root`
   - `agents_detects_cursorrules_at_project_root`
   - `agents_passes_on_valid_project`

**Fixture: `agents/basic/`**
```
agents/basic/
├── quench.toml
├── CLAUDE.md
└── .cursorrules
```

**quench.toml:**
```toml
version = 1
[check.agents]
files = ["CLAUDE.md", ".cursorrules"]
required = ["CLAUDE.md"]
sync = true
sync_source = "CLAUDE.md"
```

**Spec pattern:**
```rust
/// Spec: docs/specs/checks/agents.md#agent-files
#[test]
fn agents_detects_claude_md_at_project_root() {
    let agents = check("agents").on("agents/basic").json().passes();
    let metrics = agents.require("metrics");
    let files_found = metrics.get("files_found").unwrap().as_array().unwrap();
    assert!(files_found.iter().any(|f| f.as_str() == Some("CLAUDE.md")));
}
```

### Phase 2: Violation Detection Specs (4 specs)

Implement specs for violation scenarios: missing file, out of sync, missing section, forbidden section.

**Tasks:**
1. Create `tests/fixtures/agents/missing-file/` (no CLAUDE.md but required)
2. Create `tests/fixtures/agents/out-of-sync/` (differing files)
3. Create `tests/fixtures/agents/missing-section/` (no "Landing the Plane")
4. Create `tests/fixtures/agents/forbidden-section/` (has "Secrets" section)
5. Implement specs:
   - `agents_missing_required_file_generates_violation`
   - `agents_out_of_sync_generates_violation`
   - `agents_missing_section_generates_violation_with_advice`
   - `agents_forbidden_section_generates_violation`

**Fixture: `agents/missing-file/`**
```toml
version = 1
[check.agents]
required = ["CLAUDE.md"]
```
(No CLAUDE.md file)

**Fixture: `agents/out-of-sync/`**
```
agents/out-of-sync/
├── quench.toml
├── CLAUDE.md          # Has "## Sync Test\nContent A"
└── .cursorrules       # Has "## Sync Test\nContent B"
```

**Fixture: `agents/missing-section/`**
```toml
version = 1
[[check.agents.sections.required]]
name = "Landing the Plane"
advice = "Checklist before completing work"
```
(CLAUDE.md exists but lacks "Landing the Plane")

**Fixture: `agents/forbidden-section/`**
```toml
version = 1
[check.agents]
sections.forbid = ["Secrets", "API Keys"]
```
(CLAUDE.md has "## Secrets" heading)

### Phase 3: Content Rules Specs (3 specs)

Implement specs for content validation: tables, max_lines, max_tokens.

**Tasks:**
1. Create `tests/fixtures/agents/with-table/` (has markdown table)
2. Create `tests/fixtures/agents/oversized-lines/` (over 50 lines with max_lines=50)
3. Create `tests/fixtures/agents/oversized-tokens/` (over 200 tokens with max_tokens=200)
4. Implement specs:
   - `agents_markdown_table_generates_violation`
   - `agents_file_over_max_lines_generates_violation`
   - `agents_file_over_max_tokens_generates_violation`

**Fixture: `agents/with-table/`**
```toml
version = 1
[check.agents]
tables = "forbid"
```
(CLAUDE.md contains `| Header | Header |` table)

**Fixture: `agents/oversized-lines/`**
```toml
version = 1
[check.agents]
max_lines = 50
```
(CLAUDE.md has 60+ lines)

**Fixture: `agents/oversized-tokens/`**
```toml
version = 1
[check.agents]
max_tokens = 200
```
(CLAUDE.md has ~1000 chars = ~250 tokens)

### Phase 4: JSON Output and Fix Specs (2 specs)

Implement specs for JSON output structure and --fix behavior.

**Tasks:**
1. Create `tests/fixtures/agents/metrics/` fixture
2. Implement specs:
   - `agents_json_includes_files_found_and_in_sync_metrics`
   - `agents_violation_type_is_valid`
3. Implement --fix spec using temp_project():
   - `agents_fix_syncs_files_from_sync_source`

**JSON metrics spec:**
```rust
/// Spec: docs/specs/checks/agents.md#json-output
#[test]
fn agents_json_includes_files_found_and_in_sync_metrics() {
    let agents = check("agents").on("agents/metrics").json().passes();
    let metrics = agents.require("metrics");

    assert!(metrics.get("files_found").is_some());
    assert!(metrics.get("in_sync").is_some());
}
```

**Violation types spec:**
```rust
/// Spec: docs/specs/checks/agents.md#json-output
#[test]
fn agents_violation_type_is_valid() {
    let agents = check("agents").on("agents/missing-file").json().fails();
    let violations = agents.require("violations").as_array().unwrap();

    let valid_types = [
        "missing_file",
        "out_of_sync",
        "missing_section",
        "forbidden_section",
        "forbidden_table",
        "file_too_large",
    ];

    for v in violations {
        let vtype = v.get("type").and_then(|t| t.as_str()).unwrap();
        assert!(valid_types.contains(&vtype), "unexpected type: {}", vtype);
    }
}
```

**Fix spec (using temp_project):**
```rust
/// Spec: docs/specs/checks/agents.md#sync-behavior
#[test]
#[ignore = "TODO: Phase 501 - Agents Check Implementation"]
fn agents_fix_syncs_files_from_sync_source() {
    let dir = temp_project();
    std::fs::write(dir.path().join("quench.toml"), r#"
version = 1
[check.agents]
files = ["CLAUDE.md", ".cursorrules"]
sync = true
sync_source = "CLAUDE.md"
"#).unwrap();
    std::fs::write(dir.path().join("CLAUDE.md"), "# Source\nContent A").unwrap();
    std::fs::write(dir.path().join(".cursorrules"), "# Source\nContent B").unwrap();

    // Run with --fix
    check("agents").pwd(dir.path()).args(&["--fix"]).passes();

    // Verify files are now synced
    let cursorrules = std::fs::read_to_string(dir.path().join(".cursorrules")).unwrap();
    assert_eq!(cursorrules, "# Source\nContent A");
}
```

## Key Implementation Details

### Spec Naming Convention

Follow existing pattern from `escapes.rs`:
- `agents_<behavior>` for happy path
- `agents_<violation>_generates_violation` for failure cases
- `agents_<feature>_<specific_aspect>` for specific behaviors

### Fixture Design Principles

1. **Minimal fixtures**: Only include what's needed to test the specific behavior
2. **Explicit config**: Use `quench.toml` with explicit settings, don't rely on defaults
3. **Self-documenting**: File contents should make the test purpose obvious

### Ignored Specs Pattern

All specs start with `#[ignore = "TODO: Phase 501 - Agents Check Implementation"]` since the check isn't implemented yet. This follows the spec-first development approach from `CLAUDE.md`.

### Temp Project vs Fixture

Use `temp_project()` for:
- Tests requiring file modification (--fix)
- Tests requiring dynamic content generation

Use fixtures for:
- Static test cases
- Tests that are run frequently (faster)

## Verification Plan

### During Spec Development

```bash
# Verify specs compile
cargo test --test specs --no-run

# Run only agents specs (will all be ignored)
cargo test --test specs agents

# Show ignored count
cargo test --test specs -- --ignored
```

### After Check Implementation

```bash
# Run agents specs
cargo test --test specs agents

# Run full check suite
cargo test --test specs

# Run full validation
make check
```

### Acceptance Criteria

1. All 12 specs compile and are properly ignored
2. Fixtures exist and are minimal
3. Spec doc comments reference `docs/specs/checks/agents.md`
4. Spec naming follows existing convention
5. `cargo test --test specs --no-run` passes

## Spec Checklist

| # | Spec | Fixture | Status |
|---|------|---------|--------|
| 1 | detects CLAUDE.md at project root | agents/basic | TODO |
| 2 | detects .cursorrules at project root | agents/basic | TODO |
| 3 | missing required file generates violation | agents/missing-file | TODO |
| 4 | files out of sync generates violation | agents/out-of-sync | TODO |
| 5 | --fix syncs files from sync_source | temp_project | TODO |
| 6 | missing required section generates violation with advice | agents/missing-section | TODO |
| 7 | forbidden section generates violation | agents/forbidden-section | TODO |
| 8 | markdown table generates violation (default forbid) | agents/with-table | TODO |
| 9 | file over max_lines generates violation | agents/oversized-lines | TODO |
| 10 | file over max_tokens generates violation | agents/oversized-tokens | TODO |
| 11 | JSON includes files_found, in_sync metrics | agents/metrics | TODO |
| 12 | agents violation.type is one of expected values | agents/missing-file | TODO |
