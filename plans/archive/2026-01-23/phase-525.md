# Phase 525: Agents Check - Output

**Root Feature:** `quench-014b`

## Overview

Implement output formatting for the `agents` check. This phase ensures violations are displayed correctly in both text and JSON formats, with human-readable descriptions and actionable advice. It also implements the FIXED status output for `--fix` mode.

Key capabilities:
- Human-readable text output for all violation types
- Structured JSON output with complete metadata
- FIXED status output when `--fix` successfully repairs issues
- Behavioral specs verifying exact output format

## Project Structure

```
crates/cli/src/
├── checks/
│   ├── agents/
│   │   ├── mod.rs           # Add fix tracking, improved violation messages
│   │   ├── mod_tests.rs     # Unit tests for output formatting
│   │   └── ...              # Unchanged from Phase 520
│   └── mod.rs
├── output/
│   ├── text.rs              # Add agents-specific violation formatting
│   └── text_tests.rs        # Tests for text output format
├── check.rs                 # Add FixResult tracking (if needed)
tests/
├── fixtures/agents/
│   └── ...                  # Existing fixtures sufficient
└── specs/checks/agents.rs   # Add output format specs
```

## Dependencies

No new external dependencies. Uses existing:
- `termcolor` for colored terminal output
- `serde_json` for JSON serialization

## Implementation Phases

### Phase 1: Text Output Violation Descriptions

Update text formatter to produce human-readable violation descriptions per the spec.

**Target output format:**

```
agents: FAIL
  CLAUDE.md: missing required file
    Required agent file 'CLAUDE.md' not found at project root

  .cursorrules: out of sync with CLAUDE.md
    Section "Code Style" differs. Use --fix to sync from CLAUDE.md.

  CLAUDE.md: missing required section "Landing the Plane"
    Add a "## Landing the Plane" section: Checklist for AI agents before finishing work

  CLAUDE.md:45: forbidden table
    Tables are not token-efficient. Convert to a list or prose.

  CLAUDE.md: file too large (502 lines vs 500)
    File has 502 lines (max: 500). Split into smaller files or reduce content.
```

**Tasks:**
1. Update `format_violation_desc()` in `text.rs` to handle agents violation types
2. Add agents-specific formatting for `missing_file`, `out_of_sync`, `missing_section`, `forbidden_section`, `forbidden_table`, `forbidden_diagram`, `forbidden_mermaid`, `file_too_large`
3. Use section/other_file fields for context-rich descriptions

**Implementation in `text.rs`:**
```rust
fn format_violation_desc(&self, v: &Violation) -> String {
    match v.violation_type.as_str() {
        // Agents check - human-readable descriptions
        "missing_file" => "missing required file".to_string(),
        "forbidden_file" => "forbidden file exists".to_string(),
        "out_of_sync" => {
            if let Some(ref other) = v.other_file {
                format!("out of sync with {}", other.display())
            } else {
                "out of sync".to_string()
            }
        }
        "missing_section" => {
            // Extract section name from advice if available
            "missing required section".to_string()
        }
        "forbidden_section" => "forbidden section found".to_string(),
        "forbidden_table" => "forbidden table".to_string(),
        "forbidden_diagram" => "forbidden box diagram".to_string(),
        "forbidden_mermaid" => "forbidden mermaid block".to_string(),
        "file_too_large" => {
            // Use value/threshold for labeled format
            if let (Some(val), Some(thresh)) = (v.value, v.threshold) {
                format!("file too large ({} vs {})", val, thresh)
            } else {
                "file too large".to_string()
            }
        }
        // Other checks - existing behavior
        _ => self.format_default_desc(v),
    }
}

fn format_default_desc(&self, v: &Violation) -> String {
    let base = match (v.value, v.threshold) {
        (Some(val), Some(thresh)) => {
            let label = match v.violation_type.as_str() {
                "file_too_large" => "lines: ",
                "file_too_large_nonblank" => "nonblank: ",
                _ => "",
            };
            format!("{} ({}{} vs {})", v.violation_type, label, val, thresh)
        }
        _ => v.violation_type.clone(),
    };

    if let Some(ref pattern) = v.pattern {
        format!("{}: {}", base, pattern)
    } else {
        base
    }
}
```

**Verification:**
```bash
cargo test output::text::tests
```

### Phase 2: FIXED Status Output

Implement FIXED status when `--fix` successfully repairs violations.

**Target output format:**
```
agents: FIXED
  Synced .cursorrules from CLAUDE.md (3 sections updated)
```

**Tasks:**
1. Track fix actions during check execution
2. Return fix summary in CheckResult
3. Display FIXED status instead of FAIL when fixes were applied
4. Show what was fixed in output

**Fix tracking in `mod.rs`:**
```rust
/// Track fixes applied during the check.
struct FixSummary {
    files_synced: Vec<(String, usize)>, // (filename, sections_updated)
}

impl FixSummary {
    fn new() -> Self {
        Self {
            files_synced: Vec::new(),
        }
    }

    fn add_sync(&mut self, filename: String, sections: usize) {
        self.files_synced.push((filename, sections));
    }

    fn is_empty(&self) -> bool {
        self.files_synced.is_empty()
    }
}

/// Check synchronization between agent files.
fn check_sync(
    ctx: &CheckContext,
    config: &AgentsConfig,
    detected: &[DetectedFile],
    violations: &mut Vec<Violation>,
    fixes: &mut FixSummary,
) -> bool {
    // ... existing logic ...

    // If fix mode is enabled, sync the target file from source
    if ctx.fix && std::fs::write(&target_file.path, &source_content).is_ok() {
        // Track the fix
        let section_count = comparison.differences.len();
        fixes.add_sync(target_name.clone(), section_count);
        continue;
    }

    // ... rest of function ...
}
```

**CheckResult extension for fixes:**
```rust
/// Add fix information to a check result.
#[derive(Debug, Clone, Serialize)]
pub struct CheckResult {
    // ... existing fields ...

    /// True if fixes were applied.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub fixed: bool,

    /// Summary of fixes applied.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fix_summary: Option<JsonValue>,
}

impl CheckResult {
    /// Create a fixed check result.
    pub fn fixed(name: impl Into<String>, summary: JsonValue) -> Self {
        Self {
            name: name.into(),
            passed: true,
            skipped: false,
            stub: false,
            fixed: true,
            error: None,
            violations: Vec::new(),
            metrics: None,
            by_package: None,
            fix_summary: Some(summary),
        }
    }
}
```

**Text formatter FIXED status:**
```rust
pub fn write_check(&mut self, result: &CheckResult) -> std::io::Result<bool> {
    if result.passed && !result.fixed {
        return Ok(false); // Silent on pass per spec
    }

    // Check name: bold
    self.stdout.set_color(&scheme::check_name())?;
    write!(self.stdout, "{}", result.name)?;
    self.stdout.reset()?;

    if result.fixed {
        // ": FIXED" in green
        write!(self.stdout, ": ")?;
        self.stdout.set_color(&scheme::fixed())?;
        write!(self.stdout, "FIXED")?;
        self.stdout.reset()?;
        writeln!(self.stdout)?;

        // Show fix summary
        if let Some(ref summary) = result.fix_summary {
            self.write_fix_summary(summary)?;
        }

        return Ok(false);
    }

    // ... existing FAIL handling ...
}

fn write_fix_summary(&mut self, summary: &JsonValue) -> std::io::Result<()> {
    if let Some(synced) = summary.get("files_synced").and_then(|s| s.as_array()) {
        for entry in synced {
            let file = entry.get("file").and_then(|f| f.as_str()).unwrap_or("?");
            let source = entry.get("source").and_then(|s| s.as_str()).unwrap_or("?");
            let sections = entry.get("sections").and_then(|n| n.as_i64()).unwrap_or(0);
            writeln!(
                self.stdout,
                "  Synced {} from {} ({} sections updated)",
                file, source, sections
            )?;
        }
    }
    Ok(())
}
```

**Verification:**
```bash
cargo test checks::agents::mod_tests::fix
cargo test output::text::tests::fixed
```

### Phase 3: JSON Output Completeness

Ensure JSON output includes all required fields per spec.

**Target JSON structure:**
```json
{
  "name": "agents",
  "passed": false,
  "violations": [
    {
      "file": "CLAUDE.md",
      "line": null,
      "type": "missing_file",
      "advice": "Required agent file 'CLAUDE.md' not found at project root"
    },
    {
      "file": ".cursorrules",
      "line": null,
      "type": "out_of_sync",
      "other_file": "CLAUDE.md",
      "section": "Code Style",
      "advice": "Section \"Code Style\" differs. Use --fix to sync from CLAUDE.md."
    },
    {
      "file": "CLAUDE.md",
      "line": null,
      "type": "missing_section",
      "advice": "Add a \"## Landing the Plane\" section: Checklist for AI agents"
    },
    {
      "file": "CLAUDE.md",
      "line": 45,
      "type": "forbidden_table",
      "advice": "Tables are not token-efficient. Convert to a list or prose."
    },
    {
      "file": "CLAUDE.md",
      "line": null,
      "type": "file_too_large",
      "value": 502,
      "threshold": 500,
      "advice": "File has 502 lines (max: 500)..."
    }
  ],
  "metrics": {
    "files_found": ["CLAUDE.md", ".cursorrules"],
    "files_missing": [],
    "in_sync": false
  }
}
```

**Fixed JSON structure:**
```json
{
  "name": "agents",
  "passed": true,
  "fixed": true,
  "fix_summary": {
    "files_synced": [
      {"file": ".cursorrules", "source": "CLAUDE.md", "sections": 3}
    ]
  },
  "metrics": {
    "files_found": ["CLAUDE.md", ".cursorrules"],
    "files_missing": [],
    "in_sync": true
  }
}
```

**Tasks:**
1. Verify all violation fields are serialized correctly
2. Add `fixed` and `fix_summary` fields to CheckResult
3. Ensure metrics include all required fields
4. Add unit tests for JSON serialization

**Verification:**
```bash
cargo test checks::agents::mod_tests::json
cargo test output::json_tests
```

### Phase 4: Behavioral Specs for Output

Add specs that verify the exact output format.

**Tasks:**
1. Add text output format specs
2. Add JSON output field specs
3. Add FIXED status specs
4. Remove any remaining `#[ignore]` attributes

**New specs in `tests/specs/checks/agents.rs`:**
```rust
// =============================================================================
// TEXT OUTPUT FORMAT SPECS (Phase 525)
// =============================================================================

/// Spec: docs/specs/checks/agents.md#output
///
/// > Missing file shows human-readable description.
#[test]
fn agents_missing_file_text_output() {
    check("agents")
        .on("agents/missing-file")
        .fails()
        .stdout_has("missing required file");
}

/// Spec: docs/specs/checks/agents.md#output
///
/// > Out of sync shows other file name.
#[test]
fn agents_out_of_sync_text_output() {
    check("agents")
        .on("agents/out-of-sync")
        .fails()
        .stdout_has("out of sync with");
}

/// Spec: docs/specs/checks/agents.md#output
///
/// > Missing section includes section name and advice.
#[test]
fn agents_missing_section_text_output() {
    check("agents")
        .on("agents/missing-section")
        .fails()
        .stdout_has("Landing the Plane")
        .stdout_has("Checklist");
}

/// Spec: docs/specs/checks/agents.md#output
///
/// > Forbidden table shows line number.
#[test]
fn agents_forbidden_table_text_output() {
    let output = check("agents").on("agents/with-table").text().fails();
    // Verify line number is present (format: CLAUDE.md:N: forbidden table)
    assert!(
        output.stdout().contains(":") && output.stdout().contains("forbidden table"),
        "should show file:line: forbidden table"
    );
}

/// Spec: docs/specs/checks/agents.md#output
///
/// > File too large shows value vs threshold.
#[test]
fn agents_file_too_large_text_output() {
    check("agents")
        .on("agents/oversized-lines")
        .fails()
        .stdout_has("vs");
}

// =============================================================================
// FIXED STATUS SPECS (Phase 525)
// =============================================================================

/// Spec: docs/specs/checks/agents.md#fixed
///
/// > Running with --fix shows FIXED status when files are synced.
#[test]
fn agents_fix_shows_fixed_status() {
    let dir = temp_project();
    std::fs::write(
        dir.path().join("quench.toml"),
        r#"version = 1
[check.agents]
files = ["CLAUDE.md", ".cursorrules"]
sync = true
sync_source = "CLAUDE.md"
"#,
    )
    .unwrap();
    std::fs::write(dir.path().join("CLAUDE.md"), "# Source\nContent A").unwrap();
    std::fs::write(dir.path().join(".cursorrules"), "# Source\nContent B").unwrap();

    check("agents")
        .pwd(dir.path())
        .args(&["--fix"])
        .passes()
        .stdout_has("FIXED")
        .stdout_has("Synced");
}

/// Spec: docs/specs/checks/agents.md#json-output
///
/// > JSON includes fixed:true when --fix applies changes.
#[test]
fn agents_fix_json_includes_fixed_field() {
    let dir = temp_project();
    std::fs::write(
        dir.path().join("quench.toml"),
        r#"version = 1
[check.agents]
files = ["CLAUDE.md", ".cursorrules"]
sync = true
sync_source = "CLAUDE.md"
"#,
    )
    .unwrap();
    std::fs::write(dir.path().join("CLAUDE.md"), "# Source\nContent A").unwrap();
    std::fs::write(dir.path().join(".cursorrules"), "# Source\nContent B").unwrap();

    let result = check("agents")
        .pwd(dir.path())
        .args(&["--fix"])
        .json()
        .passes();

    assert_eq!(
        result.require("fixed").as_bool(),
        Some(true),
        "should have fixed: true"
    );
}
```

**Verification:**
```bash
cargo test --test specs agents
```

### Phase 5: Color Scheme for FIXED Status

Add color for FIXED status in the terminal output.

**Tasks:**
1. Add `fixed()` color to `color/scheme.rs`
2. Use green for FIXED (same as PASS or distinct shade)

**Implementation in `color/scheme.rs`:**
```rust
/// Color for FIXED status (green, like PASS).
pub fn fixed() -> ColorSpec {
    let mut spec = ColorSpec::new();
    spec.set_fg(Some(Color::Green));
    spec.set_bold(true);
    spec
}
```

**Verification:**
```bash
cargo test color::scheme
```

## Key Implementation Details

### Text Output Format

The text output follows the pattern:
```
<check>: <STATUS>
  <file>[:<line>]: <description>
    <advice>
```

Where:
- `<STATUS>` is `FAIL`, `FIXED`, or `SKIP`
- `<description>` is human-readable, not just the violation_type
- `<advice>` is indented 4 spaces

### JSON Output Fields

| Field | Required | Description |
|-------|----------|-------------|
| `file` | Yes | File path (relative to root) |
| `line` | No | Line number (1-indexed) |
| `type` | Yes | Violation type (`missing_file`, etc.) |
| `advice` | Yes | Actionable guidance |
| `value` | No | Current value (for thresholds) |
| `threshold` | No | Limit exceeded |
| `other_file` | No | Other file (for sync violations) |
| `section` | No | Section name (for section violations) |

### FIXED Status Logic

A check shows FIXED when:
1. `ctx.fix` is true
2. At least one fix was applied
3. No remaining violations

If fixes were applied but violations remain, status is FAIL (not FIXED).

### Violation Type Descriptions

| Type | Text Description |
|------|------------------|
| `missing_file` | missing required file |
| `forbidden_file` | forbidden file exists |
| `out_of_sync` | out of sync with `<other>` |
| `missing_section` | missing required section |
| `forbidden_section` | forbidden section found |
| `forbidden_table` | forbidden table |
| `forbidden_diagram` | forbidden box diagram |
| `forbidden_mermaid` | forbidden mermaid block |
| `file_too_large` | file too large (`N` vs `M`) |

## Verification Plan

### Unit Tests

```bash
# Text output formatting
cargo test output::text::tests

# JSON output serialization
cargo test output::json_tests

# Agents check output
cargo test checks::agents::mod_tests

# Color scheme
cargo test color::scheme
```

### Behavioral Specs

```bash
# Run agents specs (should pass after implementation)
cargo test --test specs agents

# Show all specs status
cargo test --test specs agents -- --show-output
```

### Manual Verification

```bash
# Test text output for each violation type
quench check --agents tests/fixtures/agents/missing-file
quench check --agents tests/fixtures/agents/out-of-sync
quench check --agents tests/fixtures/agents/missing-section
quench check --agents tests/fixtures/agents/with-table
quench check --agents tests/fixtures/agents/oversized-lines

# Test JSON output
quench check --agents -o json tests/fixtures/agents/missing-file

# Test FIXED status
cd /tmp && mkdir test-fix && cd test-fix
echo 'version = 1' > quench.toml
echo '[check.agents]' >> quench.toml
echo 'sync = true' >> quench.toml
echo '# Source' > CLAUDE.md
echo '# Different' > .cursorrules
quench check --agents --fix
```

### Full Validation

```bash
make check
```

### Acceptance Criteria

1. Missing file violations show "missing required file" in text output
2. Out of sync violations show the other file name
3. Missing section violations include section name and advice
4. Forbidden content violations show line numbers
5. File too large violations show "N vs M" format
6. FIXED status appears when --fix successfully syncs files
7. JSON output includes all required fields
8. JSON includes `fixed: true` when fixes applied
9. All Phase 525 behavioral specs pass
10. `make check` passes

## Spec Status (After Implementation)

| Spec | Status |
|------|--------|
| agents_detects_claude_md_at_project_root | ✅ Pass (505) |
| agents_detects_cursorrules_at_project_root | ✅ Pass (505) |
| agents_passes_on_valid_project | ✅ Pass (505) |
| agents_missing_required_file_generates_violation | ✅ Pass (505) |
| agents_forbidden_file_generates_violation | ✅ Pass (505) |
| agents_out_of_sync_generates_violation | ✅ Pass (510) |
| agents_fix_syncs_files_from_sync_source | ✅ Pass (510) |
| agents_missing_section_generates_violation_with_advice | ✅ Pass (515) |
| agents_forbidden_section_generates_violation | ✅ Pass (515) |
| agents_forbidden_section_glob_matches | ✅ Pass (515) |
| agents_markdown_table_generates_violation | ✅ Pass (520) |
| agents_file_over_max_lines_generates_violation | ✅ Pass (520) |
| agents_file_over_max_tokens_generates_violation | ✅ Pass (520) |
| agents_box_diagram_generates_violation | ✅ Pass (520) |
| agents_mermaid_block_generates_violation | ✅ Pass (520) |
| agents_size_violation_includes_threshold | ✅ Pass (520) |
| agents_json_includes_files_found_and_in_sync_metrics | ✅ Pass (505) |
| agents_violation_type_is_valid | ✅ Pass (510) |
| agents_missing_file_text_output | ✅ Pass |
| agents_out_of_sync_text_output | ✅ Pass |
| agents_missing_section_text_output | ✅ Pass |
| agents_forbidden_table_text_output | ✅ Pass |
| agents_file_too_large_text_output | ✅ Pass |
| agents_fix_shows_fixed_status | ✅ Pass |
| agents_fix_json_includes_fixed_field | ✅ Pass |
