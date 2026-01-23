# Phase 510: Agents Check - Sync

**Root Feature:** `quench-23ce`

## Overview

Implement multi-file synchronization checking for the `agents` check. When `sync = true` and multiple agent files exist (e.g., CLAUDE.md and .cursorrules), this phase detects when their content differs and optionally syncs them via `--fix`.

Key capabilities:
- Section-level markdown parsing (extract `## Section` headers)
- Content comparison between agent files
- `out_of_sync` violation generation with section details
- `--fix` to sync target files from `sync_source`

## Project Structure

```
crates/cli/src/
├── checks/
│   ├── agents/
│   │   ├── mod.rs           # Add sync checking to run()
│   │   ├── config.rs        # Already has sync/sync_source fields
│   │   ├── detection.rs     # Already has file detection
│   │   ├── sync.rs          # NEW: Sync checking logic
│   │   └── sync_tests.rs    # NEW: Unit tests for sync
│   └── mod.rs
├── check.rs                 # Add other_file/section fields to Violation
tests/
├── fixtures/agents/
│   ├── out-of-sync/         # Existing fixture (update content)
│   └── out-of-sync-sections/# NEW: Multi-section sync test
└── specs/checks/agents.rs   # Enable sync specs
```

## Dependencies

No new external dependencies. Uses existing:
- `std::fs` for file reading
- `serde_json` for metrics output
- Existing `Violation` struct (with extensions)

## Implementation Phases

### Phase 1: Markdown Section Parser

Create a lightweight markdown section parser that extracts level-2 headings and their content.

**Tasks:**
1. Create `crates/cli/src/checks/agents/sync.rs` with section parsing
2. Add unit tests in `sync_tests.rs`

**Section parsing logic:**
```rust
/// A parsed markdown section.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Section {
    /// Section name (normalized: lowercase, trimmed).
    pub name: String,
    /// Original heading text (for display).
    pub heading: String,
    /// Content below the heading (until next section or EOF).
    pub content: String,
    /// Line number where section starts (1-indexed).
    pub line: u32,
}

/// Parse markdown content into sections.
///
/// Sections are delimited by `## ` headings.
/// Content before the first `## ` is captured as a preamble section.
pub fn parse_sections(content: &str) -> Vec<Section> {
    let mut sections = Vec::new();
    let mut current_name = String::new();
    let mut current_heading = String::new();
    let mut current_content = String::new();
    let mut current_line: u32 = 1;
    let mut in_preamble = true;

    for (line_num, line) in content.lines().enumerate() {
        let line_number = (line_num + 1) as u32;

        if let Some(heading) = line.strip_prefix("## ") {
            // Save previous section
            if !in_preamble || !current_content.trim().is_empty() {
                sections.push(Section {
                    name: normalize_name(&current_name),
                    heading: current_heading.clone(),
                    content: current_content.trim_end().to_string(),
                    line: current_line,
                });
            }

            // Start new section
            current_name = heading.trim().to_string();
            current_heading = heading.trim().to_string();
            current_content = String::new();
            current_line = line_number;
            in_preamble = false;
        } else {
            current_content.push_str(line);
            current_content.push('\n');
        }
    }

    // Save final section
    if !in_preamble || !current_content.trim().is_empty() {
        sections.push(Section {
            name: normalize_name(&current_name),
            heading: current_heading,
            content: current_content.trim_end().to_string(),
            line: current_line,
        });
    }

    sections
}

/// Normalize section name for comparison (lowercase, trimmed).
fn normalize_name(name: &str) -> String {
    name.trim().to_lowercase()
}
```

**Verification:**
```bash
cargo test checks::agents::sync::tests
```

### Phase 2: Section Comparison Logic

Implement comparison between two files' sections to detect differences.

**Tasks:**
1. Add comparison functions to `sync.rs`
2. Track which sections differ and how

**Comparison logic:**
```rust
/// Result of comparing two files.
#[derive(Debug)]
pub struct SyncComparison {
    /// True if files are in sync.
    pub in_sync: bool,
    /// Sections that differ between files.
    pub differences: Vec<SectionDiff>,
}

/// A difference between sections in two files.
#[derive(Debug)]
pub struct SectionDiff {
    /// Section name (normalized).
    pub section: String,
    /// Original heading in source file.
    pub source_heading: Option<String>,
    /// Original heading in target file.
    pub target_heading: Option<String>,
    /// Type of difference.
    pub diff_type: DiffType,
    /// Line in target file where section starts (for violation).
    pub target_line: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffType {
    /// Section exists in source but not in target.
    MissingInTarget,
    /// Section exists in target but not in source.
    ExtraInTarget,
    /// Section exists in both but content differs.
    ContentDiffers,
}

/// Compare two files for sync.
pub fn compare_files(
    source_content: &str,
    target_content: &str,
) -> SyncComparison {
    let source_sections = parse_sections(source_content);
    let target_sections = parse_sections(target_content);

    let mut differences = Vec::new();

    // Build lookup map for target sections
    let target_map: HashMap<String, &Section> = target_sections
        .iter()
        .map(|s| (s.name.clone(), s))
        .collect();

    // Check each source section
    for source in &source_sections {
        match target_map.get(&source.name) {
            None => {
                // Missing in target
                differences.push(SectionDiff {
                    section: source.name.clone(),
                    source_heading: Some(source.heading.clone()),
                    target_heading: None,
                    diff_type: DiffType::MissingInTarget,
                    target_line: None,
                });
            }
            Some(target) => {
                // Compare content (normalize whitespace)
                if normalize_content(&source.content) != normalize_content(&target.content) {
                    differences.push(SectionDiff {
                        section: source.name.clone(),
                        source_heading: Some(source.heading.clone()),
                        target_heading: Some(target.heading.clone()),
                        diff_type: DiffType::ContentDiffers,
                        target_line: Some(target.line),
                    });
                }
            }
        }
    }

    // Check for sections only in target
    let source_names: HashSet<_> = source_sections.iter().map(|s| &s.name).collect();
    for target in &target_sections {
        if !source_names.contains(&target.name) {
            differences.push(SectionDiff {
                section: target.name.clone(),
                source_heading: None,
                target_heading: Some(target.heading.clone()),
                diff_type: DiffType::ExtraInTarget,
                target_line: Some(target.line),
            });
        }
    }

    SyncComparison {
        in_sync: differences.is_empty(),
        differences,
    }
}

/// Normalize content for comparison (collapse whitespace).
fn normalize_content(content: &str) -> String {
    content
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}
```

**Verification:**
```bash
cargo test checks::agents::sync::tests::compare
```

### Phase 3: Sync Violation Generation

Integrate sync checking into the `AgentsCheck::run()` method and generate violations.

**Tasks:**
1. Extend `Violation` struct with `other_file` and `section` fields
2. Add `check_sync()` function in `mod.rs`
3. Update metrics with `in_sync` status
4. Generate `out_of_sync` violations

**Extend Violation in `crates/cli/src/check.rs`:**
```rust
pub struct Violation {
    // ... existing fields ...

    /// Other file involved in sync comparison.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub other_file: Option<PathBuf>,

    /// Section name for section-level violations.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub section: Option<String>,
}

impl Violation {
    /// Add sync context to the violation.
    pub fn with_sync(mut self, other_file: impl Into<PathBuf>, section: impl Into<String>) -> Self {
        self.other_file = Some(other_file.into());
        self.section = Some(section.into());
        self
    }
}
```

**Add sync checking to `mod.rs`:**
```rust
use sync::{compare_files, DiffType};

fn run(&self, ctx: &CheckContext) -> CheckResult {
    // ... existing detection code ...

    // Check sync if enabled
    let in_sync = if config.sync {
        check_sync(ctx, config, &detected, &mut violations)
    } else {
        true
    };

    let metrics = json!({
        "files_found": files_found,
        "files_missing": files_missing,
        "in_sync": in_sync,
    });
    // ...
}

/// Check synchronization between agent files.
fn check_sync(
    ctx: &CheckContext,
    config: &AgentsConfig,
    detected: &[DetectedFile],
    violations: &mut Vec<Violation>,
) -> bool {
    // Get root-scope files only
    let root_files: Vec<_> = detected
        .iter()
        .filter(|f| f.scope == Scope::Root)
        .collect();

    if root_files.len() < 2 {
        // Nothing to sync
        return true;
    }

    // Determine sync source (first in files list, or explicit sync_source)
    let source_name = config.sync_source.as_ref()
        .or_else(|| config.files.first())
        .map(|s| s.as_str());

    let Some(source_name) = source_name else {
        return true;
    };

    // Find source file in detected
    let source_file = root_files.iter()
        .find(|f| f.path.file_name()
            .map(|n| n.to_string_lossy() == source_name)
            .unwrap_or(false));

    let Some(source_file) = source_file else {
        return true; // Source not present, nothing to sync
    };

    // Read source content
    let Ok(source_content) = std::fs::read_to_string(&source_file.path) else {
        return true; // Can't read source
    };

    let mut all_in_sync = true;

    // Compare against each other root file
    for target_file in &root_files {
        if target_file.path == source_file.path {
            continue;
        }

        let Ok(target_content) = std::fs::read_to_string(&target_file.path) else {
            continue;
        };

        let comparison = compare_files(&source_content, &target_content);

        if !comparison.in_sync {
            all_in_sync = false;

            let target_name = target_file.path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| target_file.path.display().to_string());

            for diff in comparison.differences {
                let advice = match diff.diff_type {
                    DiffType::ContentDiffers => format!(
                        "Section \"{}\" differs. Use --fix to sync from {}, or reconcile manually.",
                        diff.section, source_name
                    ),
                    DiffType::MissingInTarget => format!(
                        "Section \"{}\" missing in {}. Use --fix to sync from {}.",
                        diff.section, target_name, source_name
                    ),
                    DiffType::ExtraInTarget => format!(
                        "Section \"{}\" exists in {} but not in {}. Remove or add to source.",
                        diff.section, target_name, source_name
                    ),
                };

                let violation = Violation::file_only(&target_name, "out_of_sync", advice)
                    .with_sync(source_name, &diff.section);

                violations.push(violation);
            }
        }
    }

    all_in_sync
}
```

**Verification:**
```bash
cargo test --test specs agents_out_of_sync
```

### Phase 4: --fix Support for Sync

Implement the fix capability to sync target files from the source.

**Tasks:**
1. Add `fix()` method to `AgentsCheck`
2. Implement section-level syncing (preserve target-only sections if configured)
3. Add fix tests

**Fix implementation:**
```rust
impl AgentsCheck {
    /// Apply fixes for sync violations.
    pub fn fix(&self, ctx: &CheckContext) -> FixResult {
        let config = &ctx.config.check.agents;

        if !config.sync {
            return FixResult::nothing_to_fix();
        }

        // Detection and source resolution (same as check_sync)
        let detected = detect_agent_files(ctx.root, &ctx.config.workspace.packages, &config.files);
        let root_files: Vec<_> = detected.iter().filter(|f| f.scope == Scope::Root).collect();

        if root_files.len() < 2 {
            return FixResult::nothing_to_fix();
        }

        let source_name = config.sync_source.as_ref()
            .or_else(|| config.files.first())
            .map(|s| s.as_str());

        let Some(source_name) = source_name else {
            return FixResult::nothing_to_fix();
        };

        let source_file = root_files.iter()
            .find(|f| f.path.file_name()
                .map(|n| n.to_string_lossy() == source_name)
                .unwrap_or(false));

        let Some(source_file) = source_file else {
            return FixResult::nothing_to_fix();
        };

        let Ok(source_content) = std::fs::read_to_string(&source_file.path) else {
            return FixResult::error("Could not read sync source file");
        };

        let mut fixed_count = 0;

        for target_file in &root_files {
            if target_file.path == source_file.path {
                continue;
            }

            let Ok(target_content) = std::fs::read_to_string(&target_file.path) else {
                continue;
            };

            let comparison = compare_files(&source_content, &target_content);

            if !comparison.in_sync {
                // Full replacement for simplicity (preserve nothing from target)
                if std::fs::write(&target_file.path, &source_content).is_ok() {
                    fixed_count += 1;
                }
            }
        }

        if fixed_count > 0 {
            FixResult::fixed(format!("Synced {} file(s) from {}", fixed_count, source_name))
        } else {
            FixResult::nothing_to_fix()
        }
    }
}
```

**Verification:**
```bash
cargo test --test specs agents_fix_syncs
```

### Phase 5: Update Test Fixtures and Specs

Update fixtures and enable behavioral specs for sync checking.

**Tasks:**
1. Update `tests/fixtures/agents/out-of-sync/` with section-level content
2. Create `tests/fixtures/agents/out-of-sync-sections/` for multi-section test
3. Remove `#[ignore]` from sync-related specs
4. Add additional spec for section-level diff output

**Update out-of-sync fixture:**

`tests/fixtures/agents/out-of-sync/CLAUDE.md`:
```markdown
# Project

Overview of the project.

## Code Style

Use 4 spaces for indentation.
Follow Rust conventions.

## Testing

Run tests with `cargo test`.
```

`tests/fixtures/agents/out-of-sync/.cursorrules`:
```markdown
# Project

Overview of the project.

## Code Style

Use 2 spaces for indentation.
Follow custom conventions.

## Testing

Run tests with `cargo test`.
```

**Enable specs in `tests/specs/checks/agents.rs`:**
```rust
/// Spec: docs/specs/checks/agents.md#sync-behavior
///
/// > Files out of sync with sync_source generate a violation.
#[test]
fn agents_out_of_sync_generates_violation() {
    let agents = check("agents").on("agents/out-of-sync").json().fails();
    let violations = agents.require("violations").as_array().unwrap();

    assert!(
        violations
            .iter()
            .any(|v| v.get("type").and_then(|t| t.as_str()) == Some("out_of_sync")),
        "should have out_of_sync violation"
    );
}
```

**Verification:**
```bash
cargo test --test specs agents
```

## Key Implementation Details

### Section Matching Strategy

Sections are matched by **normalized name** (lowercase, trimmed) rather than exact heading text. This allows:
- `## Code Style` matches `## code style`
- `## Code Style Guide` does NOT match `## Code Style`

This is intentional: sections should have the same semantic meaning in both files.

### Preamble Handling

Content before the first `## ` heading is treated as a "preamble" section with an empty name. Preambles are compared like any other section, ensuring the file header/intro stays in sync.

### Whitespace Normalization

Content comparison normalizes whitespace:
- Trim each line
- Remove empty lines
- Join with single newline

This prevents false positives from trailing whitespace or blank line differences.

### Fix Strategy

The `--fix` implementation uses **full replacement** rather than section-level merging:
- Simple and predictable
- Avoids complex merge logic
- Users who want to preserve target-only sections should not use `--fix`

Future enhancement: add `sync_preserve_sections = ["Section Name"]` config to preserve specific target sections.

### Violation Output Format

Text output for out-of-sync violations:
```
agents: FAIL
  .cursorrules out of sync with CLAUDE.md
    Section "Code Style" content differs
    Use --fix to sync from CLAUDE.md, or reconcile manually.
```

JSON output includes `other_file` and `section` for tooling:
```json
{
  "file": ".cursorrules",
  "type": "out_of_sync",
  "other_file": "CLAUDE.md",
  "section": "code style",
  "advice": "Section \"Code Style\" content differs. Use --fix to sync from CLAUDE.md, or reconcile manually."
}
```

## Verification Plan

### Unit Tests

```bash
# Section parsing
cargo test checks::agents::sync::tests::parse

# Section comparison
cargo test checks::agents::sync::tests::compare

# Full sync check
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

1. `sync = true` enables sync checking
2. Multi-file comparison detects section differences
3. `out_of_sync` violations include section name and advice
4. `--fix` syncs target files from `sync_source`
5. Metrics include `in_sync: true/false`
6. All Phase 510 behavioral specs pass
7. `make check` passes

## Spec Status (After Implementation)

| Spec | Phase 510 Status |
|------|------------------|
| agents_detects_claude_md_at_project_root | ✅ Pass (505) |
| agents_detects_cursorrules_at_project_root | ✅ Pass (505) |
| agents_passes_on_valid_project | ✅ Pass (505) |
| agents_missing_required_file_generates_violation | ✅ Pass (505) |
| agents_forbidden_file_generates_violation | ✅ Pass (505) |
| agents_out_of_sync_generates_violation | ✅ Pass |
| agents_missing_section_generates_violation_with_advice | ⏳ Phase 515 |
| agents_forbidden_section_generates_violation | ⏳ Phase 515 |
| agents_markdown_table_generates_violation | ⏳ Phase 520 |
| agents_file_over_max_lines_generates_violation | ⏳ Phase 520 |
| agents_file_over_max_tokens_generates_violation | ⏳ Phase 520 |
| agents_json_includes_files_found_and_in_sync_metrics | ✅ Pass (505) |
| agents_violation_type_is_valid | ✅ Pass |
| agents_fix_syncs_files_from_sync_source | ✅ Pass |
