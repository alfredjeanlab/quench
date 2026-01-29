# Plan: Reconcile Cursor Rules

## Overview

Add first-class support for `.cursor/rules/*.mdc` files by parsing their YAML frontmatter metadata (`alwaysApply`, `globs`, `description`) and using it to drive reconciliation between Cursor rules and Claude/AGENTS agent files. The key insight: Cursor's `.mdc` frontmatter encodes _where_ a rule applies, and that scope information maps directly to the agent file hierarchy quench already validates.

**Core behaviors:**

1. **`alwaysApply: true` rules** reconcile against the root `CLAUDE.md` (the body of each such `.mdc` is expected to appear as content in the root agent file)
2. **Single-directory glob rules** (e.g., `globs: "src/api/**"`) reconcile against a directory-level `CLAUDE.md` or `AGENTS.md` in the target directory
3. **Multi-glob / file-pattern rules** are validated for format but not reconciled (they don't map cleanly to a single agent file)
4. **Fix mode** can generate/update CLAUDE.md files from cursor rules (and vice versa)

## Project Structure

```
crates/cli/src/checks/agents/
├── mod.rs              # Check orchestration (modified)
├── config.rs           # Configuration (modified - new reconcile options)
├── detection.rs        # File detection (modified - .mdc awareness)
├── sync.rs             # Section-level sync (existing, reused)
├── mdc.rs              # NEW: .mdc frontmatter parser
├── mdc_tests.rs        # NEW: .mdc parser tests
├── reconcile.rs        # NEW: cursor<->claude reconciliation logic
├── reconcile_tests.rs  # NEW: reconciliation tests
├── sections.rs         # Section validation (unchanged)
├── content.rs          # Content rules (unchanged)
└── ...

docs/specs/checks/
├── agents.md           # Existing spec (unchanged)
└── agents.cursor.md    # NEW: Cursor reconciliation spec

tests/specs/checks/agents/
├── mod.rs              # (modified - add cursor module)
└── cursor.rs           # NEW: Cursor reconciliation behavioral specs

tests/fixtures/agents/
├── cursor-always-apply/    # NEW: .mdc with alwaysApply: true + CLAUDE.md
├── cursor-dir-scope/       # NEW: .mdc with single-dir glob + dir CLAUDE.md
├── cursor-mixed-rules/     # NEW: mix of alwaysApply + dir-scoped + file-scoped
├── cursor-out-of-sync/     # NEW: .mdc content diverged from CLAUDE.md
├── cursor-no-claude/       # NEW: .mdc exists but no corresponding CLAUDE.md
└── cursor-mdc-invalid/     # NEW: malformed .mdc frontmatter
```

## Dependencies

- **No new crate dependencies.** The `.mdc` frontmatter is simple YAML (3 fields); we can parse it with a minimal hand-rolled parser or reuse `serde_yaml` which is already available transitively. If `serde_yaml` is not in the dependency tree, a line-by-line frontmatter parser (delimited by `---`) is trivial and avoids a new dep.
- Existing: `globset` (already used for glob matching in detection.rs)
- Existing: `serde`, `serde_json` (already used throughout)

## Implementation Phases

### Phase 1: Write the Cursor Reconciliation Spec

**Goal:** Define the "what" before the "how."

**Deliverables:**
- `docs/specs/checks/agents.cursor.md` - Full specification of cursor reconciliation behavior
- `tests/specs/checks/agents/cursor.rs` with all specs as `#[ignore = "TODO: Phase N"]`

**Spec content covers:**
- `.mdc` frontmatter parsing rules (fields: `alwaysApply`, `globs`, `description`)
- `alwaysApply: true` → root CLAUDE.md reconciliation semantics
- Single-directory glob → directory CLAUDE.md reconciliation semantics
- Content comparison algorithm (section-level, ignoring leading `# Header`)
- Violation types: `cursor_missing_in_claude`, `claude_missing_in_cursor`, `cursor_no_agent_file`
- Configuration: `[check.agents] reconcile_cursor = true` (default: true when cursor files are in `files`)
- Output format for reconciliation violations
- Fix mode behavior

**Milestone:** Spec document reviewed; ignored test stubs compile.

---

### Phase 2: Parse `.mdc` Frontmatter

**Goal:** Extract structured metadata from `.mdc` files.

**Deliverables:**
- `crates/cli/src/checks/agents/mdc.rs` - Parser module
- `crates/cli/src/checks/agents/mdc_tests.rs` - Unit tests

**Key implementation:**

```rust
/// Parsed .mdc rule file.
#[derive(Debug)]
pub struct MdcRule {
    /// Rule description (for "apply intelligently" mode).
    pub description: Option<String>,
    /// Glob patterns for file-scoped rules.
    pub globs: Option<Vec<String>>,
    /// Whether this rule always applies.
    pub always_apply: bool,
    /// Markdown body (content after frontmatter).
    pub body: String,
    /// Original file path.
    pub path: PathBuf,
}

/// Classification of how a rule applies.
#[derive(Debug, PartialEq)]
pub enum RuleScope {
    /// alwaysApply: true → reconciles with root CLAUDE.md
    AlwaysApply,
    /// Single directory glob (e.g. "src/api/**") → reconciles with dir agent file
    SingleDirectory(PathBuf),
    /// File-pattern globs (e.g. "**/*.tsx") → no reconciliation target
    FilePattern,
    /// Manual or intelligent application → no reconciliation target
    OnDemand,
}
```

**Parsing strategy:** Split on `---` delimiters, parse key-value pairs between them. The `globs` field may be a string or YAML list. Handle both:
```yaml
globs: "src/api/**"         # single string
globs: ["src/**", "lib/**"] # array
```

**Classifying single-directory globs:** A glob maps to a single directory when:
- It has exactly one pattern
- The pattern ends with `/**` or `/**/*`
- The prefix before `/**` contains no wildcards
- Example: `src/api/**` → `SingleDirectory("src/api")`
- Counter-example: `src/**/*.tsx` → `FilePattern` (file-level, not directory-level)
- Counter-example: `src/api/**, lib/**` → `FilePattern` (multiple directories)

**Milestone:** `MdcRule::parse(content, path)` works; `RuleScope::classify(rule)` categorizes correctly.

---

### Phase 3: Reconciliation Logic (alwaysApply → Root CLAUDE.md)

**Goal:** Verify that all `alwaysApply: true` cursor rules have their content present in the root CLAUDE.md (and vice versa).

**Deliverables:**
- `crates/cli/src/checks/agents/reconcile.rs` - Reconciliation engine
- `crates/cli/src/checks/agents/reconcile_tests.rs` - Unit tests
- Wire into `mod.rs` check orchestration
- Test fixtures: `cursor-always-apply/`, `cursor-out-of-sync/`

**Algorithm:**

1. Collect all `.mdc` files with `alwaysApply: true`
2. Parse each into sections (reuse `sync::parse_sections` on the `.body`)
3. Read root CLAUDE.md and parse into sections
4. **Forward check (cursor → claude):** For each section in each `alwaysApply` `.mdc` body, verify a matching section exists in CLAUDE.md with equivalent content (using `sync::normalize_content` for comparison)
5. **Reverse check (claude → cursor):** For each section in CLAUDE.md, verify it appears in at least one `alwaysApply` `.mdc` file
6. **Header tolerance:** Strip leading `# Header` line from `.mdc` body before comparison. The `# Header` in an `.mdc` file is often a title that doesn't appear in CLAUDE.md (since CLAUDE.md has its own `# Header`). Compare preamble content excluding the first `# ` heading line.

**Content comparison rules:**
- Section names match case-insensitively (existing `normalize_name`)
- Content matches after whitespace normalization (existing `normalize_content`)
- A single CLAUDE.md section may correspond to content from multiple `.mdc` files (aggregate model)
- Preamble content (before first `## `) is compared with leading `# Header` stripped

**Violation types:**

| Type | Meaning |
|------|---------|
| `cursor_missing_in_claude` | `.mdc` section not found in CLAUDE.md |
| `claude_missing_in_cursor` | CLAUDE.md section not found in any `alwaysApply` `.mdc` |

**Configuration:**

```toml
[check.agents]
# Enable cursor reconciliation (default: true when .cursor/rules are in files list)
reconcile_cursor = true

# Direction: "bidirectional" (default), "cursor_to_claude", "claude_to_cursor"
reconcile_direction = "bidirectional"
```

**Milestone:** Reconciliation detects missing/divergent sections between `alwaysApply` `.mdc` files and root CLAUDE.md.

---

### Phase 4: Directory-Scoped Rule Reconciliation

**Goal:** When a cursor rule targets a single specific directory, verify a corresponding CLAUDE.md or AGENTS.md exists in that directory.

**Deliverables:**
- Extend `reconcile.rs` with directory-scope logic
- Test fixtures: `cursor-dir-scope/`, `cursor-no-claude/`
- Remove `#[ignore]` from directory-scope specs

**Behavior:**

1. For each `.mdc` file classified as `RuleScope::SingleDirectory(dir)`:
   - Check if `{dir}/CLAUDE.md` or `{dir}/AGENTS.md` exists (based on `[check.agents]` file configuration)
   - If the directory agent file exists, compare content (same section-level comparison as Phase 3)
   - If no agent file exists, generate `cursor_no_agent_file` violation

2. Which file to expect depends on configuration:
   - If `files` contains `"CLAUDE.md"` → expect `CLAUDE.md` in the directory
   - If `files` contains `"AGENTS.md"` → expect `AGENTS.md`
   - If both → expect whichever is `sync_source` (or first in list)

**Violation types:**

| Type | Meaning |
|------|---------|
| `cursor_no_agent_file` | Directory-scoped `.mdc` but no agent file in target directory |
| `cursor_dir_missing_in_agent` | `.mdc` section not found in directory agent file |
| `agent_dir_missing_in_cursor` | Directory agent file section not in corresponding `.mdc` |

**Milestone:** Directory-scoped `.mdc` rules are reconciled against per-directory agent files.

---

### Phase 5: Fix Mode and Output

**Goal:** Support `--fix` for cursor reconciliation and produce clear output.

**Deliverables:**
- Fix mode implementation in `reconcile.rs`
- Text and JSON output formatting
- Remove remaining `#[ignore]` from specs

**Fix behaviors:**

| Scenario | Fix Action |
|----------|-----------|
| `cursor_missing_in_claude` | Append section to CLAUDE.md |
| `claude_missing_in_cursor` | Create/update `.mdc` file |
| `cursor_no_agent_file` | Create CLAUDE.md in target directory from `.mdc` body |
| Content differs | Update target from sync_source direction |

**sync_source interaction:**
- If `sync_source = "CLAUDE.md"` → CLAUDE.md is authoritative, update `.mdc` files
- If `sync_source` is not set or is a `.mdc` file → `.mdc` files are authoritative

**Output examples:**

```
agents: FAIL
  .cursor/rules/api-guidelines.mdc: cursor rule not reconciled with CLAUDE.md
    Section "API Conventions" exists in api-guidelines.mdc (alwaysApply) but not in CLAUDE.md.
    Use --fix to add missing sections.
```

```
agents: FAIL
  .cursor/rules/frontend.mdc: no agent file for target directory
    Rule scoped to src/components/ but no CLAUDE.md found there.
    Use --fix to create src/components/CLAUDE.md from rule content.
```

**JSON additions:**

```json
{
  "type": "cursor_missing_in_claude",
  "file": ".cursor/rules/api-guidelines.mdc",
  "section": "API Conventions",
  "target": "CLAUDE.md",
  "advice": "..."
}
```

**Milestone:** Full fix mode works; all behavioral specs pass; `make check` green.

---

### Phase 6: Edge Cases and Polish

**Goal:** Handle real-world `.mdc` files gracefully.

**Deliverables:**
- Edge case handling and tests
- Test fixture: `cursor-mdc-invalid/`, `cursor-mixed-rules/`
- Documentation updates to `docs/specs/checks/agents.md` (cross-reference)

**Edge cases:**
- `.mdc` file with no frontmatter (treat as plain markdown, `alwaysApply: false`)
- `.mdc` file with malformed frontmatter (warn, skip reconciliation for that file)
- `.mdc` file with empty body (no sections to reconcile)
- `.mdc` file with `globs` that match multiple directories (classify as `FilePattern`, skip reconciliation)
- CLAUDE.md has sections from both `alwaysApply` and non-cursor sources (reverse check should not require all CLAUDE.md sections to appear in cursor rules)
- Deeply nested directory in glob (e.g., `src/components/ui/**`) → still maps to single directory
- `.md` files in `.cursor/rules/` (no frontmatter, treated as `alwaysApply: false` / on-demand)

**Configuration for partial reconciliation:**

```toml
[check.agents]
# Only check cursor→claude direction (don't require every CLAUDE.md section in cursor)
reconcile_direction = "cursor_to_claude"
```

This is important because many projects have CLAUDE.md content that isn't cursor-relevant (e.g., "Landing the Plane" checklist is Claude-specific).

**Milestone:** All edge cases handled; full test coverage; `make check` green.

## Key Implementation Details

### .mdc Frontmatter Parsing

The `.mdc` format uses YAML frontmatter between `---` delimiters:

```markdown
---
description: "Standards for API endpoints"
globs: "src/api/**"
alwaysApply: false
---

## API Conventions

Use RESTful patterns...
```

**Parser approach:** Line-by-line scan, no YAML library needed:

```rust
pub fn parse_mdc(content: &str, path: PathBuf) -> Result<MdcRule, MdcParseError> {
    let mut lines = content.lines().peekable();

    // Check for frontmatter delimiter
    if lines.peek() != Some(&"---") {
        // No frontmatter - treat as plain markdown
        return Ok(MdcRule {
            description: None,
            globs: None,
            always_apply: false,
            body: content.to_string(),
            path,
        });
    }
    lines.next(); // skip opening ---

    let mut description = None;
    let mut globs = None;
    let mut always_apply = false;

    for line in lines.by_ref() {
        if line == "---" { break; }
        if let Some(value) = line.strip_prefix("description:") {
            description = Some(unquote(value.trim()));
        } else if let Some(value) = line.strip_prefix("globs:") {
            globs = Some(parse_globs(value.trim()));
        } else if let Some(value) = line.strip_prefix("alwaysApply:") {
            always_apply = value.trim() == "true";
        }
    }

    let body: String = lines.collect::<Vec<_>>().join("\n");

    Ok(MdcRule { description, globs, always_apply, body, path })
}
```

### Single-Directory Glob Classification

```rust
pub fn classify_scope(rule: &MdcRule) -> RuleScope {
    if rule.always_apply {
        return RuleScope::AlwaysApply;
    }

    let Some(ref globs) = rule.globs else {
        return RuleScope::OnDemand;
    };

    if globs.len() != 1 {
        return RuleScope::FilePattern;
    }

    let glob = &globs[0];

    // Check if glob is "{dir}/**" or "{dir}/**/*"
    let dir = glob
        .strip_suffix("/**/*")
        .or_else(|| glob.strip_suffix("/**"))
        .or_else(|| glob.strip_suffix("/*"));

    match dir {
        Some(d) if !d.contains('*') && !d.contains('?') => {
            RuleScope::SingleDirectory(PathBuf::from(d))
        }
        _ => RuleScope::FilePattern,
    }
}
```

### Header Stripping for Comparison

When comparing `.mdc` body to CLAUDE.md content, strip the leading `# Header` from both:

```rust
/// Strip the leading `# Header` line from markdown content.
/// Returns the content starting from the line after the first `# ` heading,
/// or the original content if no leading heading is found.
fn strip_leading_header(content: &str) -> &str {
    if let Some(rest) = content.strip_prefix("# ") {
        // Find end of header line
        rest.find('\n').map(|i| &rest[i+1..]).unwrap_or("")
    } else {
        content
    }
}
```

### Reconciliation is Aggregate, Not Pairwise

Unlike the existing sync check (which compares two files 1:1), cursor reconciliation is **many-to-one**:

- Multiple `alwaysApply: true` `.mdc` files collectively should cover the root CLAUDE.md
- Each `.mdc` section needs to exist _somewhere_ in CLAUDE.md
- Each CLAUDE.md section needs to exist in _some_ `alwaysApply` `.mdc` file

This means we build a **union** of all `alwaysApply` `.mdc` sections and compare that union against CLAUDE.md sections, rather than comparing file pairs.

### Configuration Defaults

When `.cursor/rules/*.mdc` is in the `files` list (which it is by default), `reconcile_cursor` defaults to `true`. This means zero-config projects with both CLAUDE.md and `.cursor/rules/` get reconciliation automatically.

Users who intentionally maintain different content can disable:

```toml
[check.agents]
reconcile_cursor = false
```

## Verification Plan

### Unit Tests (sibling _tests.rs files)

- `mdc_tests.rs`: Frontmatter parsing (valid, missing, malformed, no-frontmatter `.md`)
- `mdc_tests.rs`: Glob classification (single-dir, multi-dir, file-pattern, no-globs)
- `reconcile_tests.rs`: Section matching (header stripping, case normalization)
- `reconcile_tests.rs`: Forward check (cursor→claude missing sections)
- `reconcile_tests.rs`: Reverse check (claude→cursor missing sections)
- `reconcile_tests.rs`: Aggregate union (multiple `.mdc` files covering one CLAUDE.md)
- `reconcile_tests.rs`: Directory-scoped reconciliation

### Behavioral Specs (tests/specs/checks/agents/cursor.rs)

- `.mdc` files detected and reported in metrics
- `alwaysApply` rule reconciled with CLAUDE.md
- Directory-scoped rule expects agent file in target directory
- Out-of-sync cursor rule generates violation with advice
- Fix mode creates/updates CLAUDE.md from cursor rules
- Malformed `.mdc` produces warning, doesn't crash
- `reconcile_cursor = false` disables reconciliation
- Exact output format tests for each violation type

### Test Fixtures

| Fixture | Contents | Tests |
|---------|----------|-------|
| `cursor-always-apply/` | CLAUDE.md + `.cursor/rules/general.mdc` (alwaysApply, synced) | Pass case |
| `cursor-out-of-sync/` | CLAUDE.md + `.cursor/rules/general.mdc` (alwaysApply, diverged) | Violation |
| `cursor-dir-scope/` | CLAUDE.md + `.cursor/rules/api.mdc` (globs: "src/api/**") + `src/api/CLAUDE.md` | Pass case |
| `cursor-no-claude/` | `.cursor/rules/api.mdc` (globs: "src/api/**") but no `src/api/CLAUDE.md` | Violation |
| `cursor-mixed-rules/` | Mix of alwaysApply + dir-scoped + file-pattern rules | Selective reconciliation |
| `cursor-mdc-invalid/` | Malformed frontmatter | Graceful degradation |

### Integration

- `make check` passes (fmt, clippy, test, build, audit, deny)
- Bump `CACHE_VERSION` in `crates/cli/src/cache.rs` since check logic changes
- Existing agents check specs continue to pass (no regressions)
