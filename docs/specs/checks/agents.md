# Agents Check Specification

The `agents` check validates AI agent context files (CLAUDE.md, .cursorrules, etc.).

## Purpose

Agent markdown files provide context to AI coding agents. This check ensures:
- Configured files exist where expected
- Content follows token-efficient conventions
- Files stay in sync with each other (default behavior)

## Zero-Config Defaults

With no configuration, the agents check applies these defaults:

| Setting | Default | Rationale |
|---------|---------|-----------|
| `files` | CLAUDE.md, AGENTS.md, .cursorrules, .cursorignore, .cursor/rules/*.md[c] | All recognized agent files |
| `required` | `["*"]` | At least one agent file must exist |
| `sync` | `true` | Keep multiple agent files consistent |
| `tables` | `forbid` | Tables waste tokens |
| `box_diagrams` | `allow` | ASCII diagrams are often useful |
| `mermaid` | `allow` | Mermaid blocks are often useful |
| `max_lines` | `500` | Encourage concise context |
| `max_tokens` | `20000` | Token-aware limit for LLM context |
| `sections.required` | "Directory Structure", "Landing the Plane" | Essential sections for AI agents |

**The `"*"` wildcard** means "at least one of the detected agent files must exist". This ensures projects have some agent context without mandating a specific file.

**Default required sections** ensure agent files contain the minimum context AI agents need:
- **Directory Structure**: Overview of project layout (supports both box-drawing and indentation formats)
- **Landing the Plane**: Checklist before completing work

**Disable size limits** if needed:
```toml
[check.agents]
max_lines = false
max_tokens = false
```

## Agent Files

Quench recognizes these agent context files:

| File | Description |
|------|-------------|
| `CLAUDE.md` | Claude Code / Anthropic agents |
| `AGENTS.md` | Generic agent instructions |
| `.cursorrules` | Cursor IDE |
| `.cursor/rules/*.md[c]` | Cursor IDE |
| `.cursorignore` | Cursor IDE ignore patterns |

Configure which files are recognized:

```toml
[check.agents]
# Files to check (default: all recognized)
files = ["CLAUDE.md", ".cursorrules"]
```

## Profile Defaults

When using [`quench init --profile claude`](../01-cli.md#profile-selection-recommended) or `--profile cursor`, opinionated defaults are configured for agent file validation.

### Claude Profile

```toml
[check.agents]
files = ["CLAUDE.md"]
required = ["CLAUDE.md"]
sync = true
sync_source = "CLAUDE.md"
tables = "forbid"
max_lines = 500
max_tokens = 20000

[[check.agents.sections.required]]
name = "Directory Structure"
advice = "Overview of project layout and key directories"

[[check.agents.sections.required]]
name = "Landing the Plane"
advice = "Checklist for AI agents before completing work"
```

### Cursor Profile

```toml
[check.agents]
files = [".cursorrules"]
required = [".cursorrules"]
sync = true
sync_source = ".cursorrules"
tables = "forbid"
max_lines = 500
max_tokens = 20000

[[check.agents.sections.required]]
name = "Directory Structure"
advice = "Overview of project layout and key directories"

[[check.agents.sections.required]]
name = "Landing the Plane"
advice = "Checklist for AI agents before completing work"
```

### Combined Profiles

When both `claude` and `cursor` profiles are used together:

```toml
[check.agents]
files = ["CLAUDE.md", ".cursorrules"]
required = ["CLAUDE.md"]
optional = [".cursorrules"]
sync = true
sync_source = "CLAUDE.md"
```

### Landing the Plane Auto-Population

During `quench init`, if agent files exist but lack a "Landing the Plane" section, quench auto-populates it with a default checklist:

**Base checklist** (always included):
```markdown
## Landing the Plane

Before completing work:

- [ ] Run `quench check`
```

**Language-specific items** are appended based on detected or selected profiles:

| Profile | Added Items |
|---------|-------------|
| `rust` | `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`, `cargo build` |
| `shell` | `shellcheck scripts/*.sh`, `bats tests/` (if present) |

**Behavior**:
- If "Landing the Plane" section exists, it is not modified
- If missing, the section is appended to the agent file
- `quench check` is always the first item in the checklist
- Language items follow in detection order

## Sync Behavior

**Sync is enabled by default** (`sync = true`). When multiple agent files exist at the same scope, they are compared section-by-section.

Sync behavior:
- If CLAUDE.md and .cursorrules both exist → check they're in sync
- If only one exists → no sync check needed
- If neither exists but one is required → fail on missing file

```toml
[check.agents]
# Sync is ON by default - disable if files should differ
sync = true

# Which file is the source of truth for --fix (default: first in `files` list)
# sync_source = "CLAUDE.md"
```

**Disable sync** if agent files should have different content:
```toml
[check.agents]
sync = false
```

**Auto-fix behavior**:
- `sync_source` defaults to first file in the `files` list (e.g., if `files = ["CLAUDE.md", ".cursorrules"]`, then CLAUDE.md is the source)
- `--fix` syncs other files from the source
- `--fix` is only unavailable if `files` list is empty

## Existence Requirements

Configure which files must exist at each scope:

```toml
[check.agents]
# Wildcard: at least one agent file must exist (default)
required = ["*"]

# Or require specific files
required = ["CLAUDE.md"]

# Or require nothing
required = []
```

Per-scope configuration:

```toml
[check.agents.root]
# At project root
required = ["CLAUDE.md"]              # Must exist
optional = [".cursorrules"]           # Checked if present, not required
forbid = []                           # Must not exist

[check.agents.package]
# At each package
required = []                         # None required by default
optional = ["CLAUDE.md"]

[check.agents.module]
# At subdirectories
required = []
optional = ["CLAUDE.md"]
```

Simpler flat config (root only):

```toml
[check.agents]
required = ["CLAUDE.md"]
optional = [".cursorrules"]
```

## Scope Levels

### Root (Project Root)

The main agent files at project root. Typically:
- Project overview and architecture
- Development setup and conventions
- "Landing the Plane" checklist

### Package (Subproject/Crate)

Agent files in a package directory (e.g., `crates/cli/CLAUDE.md`). Typically:
- Package-specific context
- Local conventions differing from root
- Shorter, focused content

### Module (Subdirectory)

Agent files in a subdirectory (e.g., `src/parser/CLAUDE.md`). Typically:
- Module-specific implementation notes
- Very brief, focused context
- Usually optional

## Section Validation

### Required Sections

Sections that must be present. Matching is **case-insensitive**.

Simple form (no advice):
```toml
[check.agents]
sections.required = ["Project Structure", "Development"]
```

Extended form (with advice for agents):
```toml
[[check.agents.sections.required]]
name = "Project Structure"
advice = "Overview of directory layout and key files"

[[check.agents.sections.required]]
name = "Development"
advice = "How to build, test, and run the project"

[[check.agents.sections.required]]
name = "Landing the Plane"
advice = "Checklist for AI agents before finishing work"
```

When a section is missing, the advice becomes actionable output:
```
agents: FAIL
  CLAUDE.md missing required section: "Landing the Plane"
    Add a "## Landing the Plane" section: Checklist for AI agents before finishing work
```

### Forbid Sections

Sections that should not be present (case-insensitive, supports globs):

```toml
[check.agents]
sections.forbid = [
  "API Keys",           # Exact match (case-insensitive)
  "Secrets",
  "Test*",              # Glob: matches "Testing", "Test Plan", etc.
]
```

## Content Rules

### Markdown Tables

Tables are verbose and not token-efficient. Default: forbid.

```toml
tables = "forbid"           # allow | forbid (default: forbid)
```

Advice: Use lists or prose instead.

### Diagrams

Control which diagram types are allowed (both default to true):

```toml
box_diagrams = "allow"   # ┌─┐ style ASCII diagrams
mermaid = "allow"        # ```mermaid blocks
```

### Size Limits

Keep agent files concise. Configure per-scope:

```toml
[check.agents.root]
max_lines = 500
max_tokens = 20000          # use false to disable

[check.agents.package]
max_lines = 200
max_tokens = 800

[check.agents.module]
max_lines = 100
max_tokens = 400
```

Token estimation uses `chars / 4` for speed (no external tokenizer dependency).

## Output

### Fail (missing required file)

```
agents: FAIL
  CLAUDE.md not found at project root
    Create a CLAUDE.md with project context for AI agents.
```

### Fail (files out of sync, sync_source configured)

```
agents: FAIL
  CLAUDE.md and .cursorrules are out of sync
    Section "Code Style" differs between files.
    Use --fix to sync from CLAUDE.md.
```

### Fail (files out of sync)

```
agents: FAIL
  CLAUDE.md and .cursorrules are out of sync
    Section "Code Style" differs between files.
    Run --fix to sync from CLAUDE.md, or reconcile manually.
```

### Fail (missing section)

```
agents: FAIL
  CLAUDE.md missing required section: "Landing the Plane"
    Add a "## Landing the Plane" section: Checklist for AI agents before finishing work
```

### Fail (forbid content)

```
agents: FAIL
  CLAUDE.md:45: Markdown table detected
    Tables are not token-efficient. Convert to a list or prose.
```

### Fixed (with --fix)

```
agents: FIXED
  Synced .cursorrules from CLAUDE.md (3 sections updated)
```

### JSON Output

```json
{
  "name": "agents",
  "passed": false,
  "violations": [
    {
      "file": "CLAUDE.md",
      "line": null,
      "type": "out_of_sync",
      "other_file": ".cursorrules",
      "section": "Code Style",
      "advice": "Run --fix to sync from CLAUDE.md, or reconcile manually."
    },
    {
      "file": "CLAUDE.md",
      "line": 45,
      "type": "forbidden_table",
      "advice": "Tables are not token-efficient. Convert to a list."
    }
  ],
  "metrics": {
    "files_found": ["CLAUDE.md", ".cursorrules"],
    "files_missing": [],
    "in_sync": false
  }
}
```

**Violation types**: `missing_file`, `out_of_sync`, `missing_section`, `forbidden_section`, `forbidden_table`, `file_too_large`

## Configuration

```toml
[check.agents]
check = "error"

# Which agent files to check
files = ["CLAUDE.md", ".cursorrules"]

# Sync behavior (default: true if multiple files exist)
sync = true

# Source of truth for --fix (default: first file in `files` list)
sync_source = "CLAUDE.md"

# Existence requirements (root scope)
required = ["CLAUDE.md"]
optional = [".cursorrules"]

# Section validation (simple form)
sections.required = ["Project Structure", "Development"]
sections.forbid = ["Secrets", "API Keys"]

# Section validation (extended form with advice)
# [[check.agents.sections.required]]
# name = "Landing the Plane"
# advice = "Checklist for AI agents before finishing work"

# Content rules
tables = "forbid"
box_diagrams = "allow"
mermaid = "allow"

# Size limits (use false to disable)
max_lines = 500
max_tokens = 20000

# Per-scope overrides
[check.agents.root]
required = ["CLAUDE.md"]
max_tokens = 20000

[check.agents.package]
required = []
optional = ["CLAUDE.md"]
max_tokens = 800

[check.agents.module]
required = []
max_tokens = 400
```
