# Cursor Rule Reconciliation

Extension to the [agents check](agents.md) for reconciling `.cursor/rules/*.mdc` files with CLAUDE.md / AGENTS.md agent files.

## Overview

Cursor rules (`.mdc` files) encode project standards with YAML frontmatter that describes their scope. This extension verifies that cursor rules and agent files stay in sync.

## .mdc Frontmatter

Cursor rule files use YAML frontmatter between `---` delimiters:

```markdown
---
description: "Standards for API endpoints"
globs: "src/api/**"
alwaysApply: false
---

## API Conventions

Use RESTful patterns...
```

### Fields

| Field | Type | Description |
|-------|------|-------------|
| `description` | string | Rule description (for "apply intelligently" mode) |
| `globs` | string or array | File/directory glob patterns |
| `alwaysApply` | boolean | Whether this rule always applies |

## Rule Scope Classification

| Scope | Criteria | Reconciliation Target |
|-------|----------|----------------------|
| AlwaysApply | `alwaysApply: true` | Root CLAUDE.md |
| SingleDirectory | Single glob like `src/api/**` | Directory CLAUDE.md |
| FilePattern | Multiple globs or file-level patterns | None (not reconciled) |
| OnDemand | No globs, `alwaysApply: false` | None (not reconciled) |

### Single-Directory Detection

A glob maps to a single directory when:
- It has exactly one pattern
- The pattern ends with `/**`, `/**/*`, or `/*`
- The prefix before the suffix contains no wildcards

Examples:
- `src/api/**` → SingleDirectory(`src/api`)
- `src/components/ui/**` → SingleDirectory(`src/components/ui`)
- `src/**/*.tsx` → FilePattern
- `["src/**", "lib/**"]` → FilePattern

## Reconciliation Behavior

### AlwaysApply Rules → Root CLAUDE.md

All `alwaysApply: true` rules are collectively reconciled against the root agent file:

1. Parse each `.mdc` body into sections (using `## ` delimiters)
2. Build a union of all alwaysApply sections
3. Compare against root CLAUDE.md sections

**Forward check (cursor → claude):** Each section from alwaysApply rules must exist in CLAUDE.md with matching content.

**Reverse check (claude → cursor):** Each CLAUDE.md section must exist in at least one alwaysApply rule.

### Directory-Scoped Rules → Directory Agent Files

For each SingleDirectory rule:

1. Check if `{dir}/CLAUDE.md` (or configured agent file) exists
2. If exists, compare sections bidirectionally
3. If missing, generate `cursor_no_agent_file` violation

### Header Stripping

The leading `# Header` line in `.mdc` bodies is stripped before comparison, since it typically serves as a title that doesn't correspond to agent file content.

## Violation Types

| Type | Meaning |
|------|---------|
| `cursor_missing_in_claude` | `.mdc` section not found in CLAUDE.md |
| `claude_missing_in_cursor` | CLAUDE.md section not in any alwaysApply `.mdc` |
| `cursor_no_agent_file` | Directory-scoped `.mdc` but no agent file in target dir |
| `cursor_dir_missing_in_agent` | `.mdc` section not in directory agent file |
| `agent_dir_missing_in_cursor` | Directory agent file section not in `.mdc` |
| `cursor_parse_error` | Malformed `.mdc` frontmatter |

## Configuration

```toml
[check.agents]
# Enable cursor reconciliation (default: true)
reconcile_cursor = true

# Direction: "bidirectional" (default), "cursor_to_claude", "claude_to_cursor"
reconcile_direction = "bidirectional"
```

### Disabling Reconciliation

Projects that intentionally maintain different content in cursor rules and agent files can disable reconciliation:

```toml
[check.agents]
reconcile_cursor = false
```

### One-Way Reconciliation

For projects where CLAUDE.md has Claude-specific content not relevant to cursor:

```toml
[check.agents]
reconcile_direction = "cursor_to_claude"
```

## Fix Mode

| Scenario | Fix Action |
|----------|-----------|
| `cursor_missing_in_claude` | Append section to CLAUDE.md |
| `cursor_no_agent_file` | Create CLAUDE.md in target directory from `.mdc` body |
| Content differs | Update target based on reconcile direction |

## Content Comparison

- Section names match case-insensitively
- Content matches after whitespace normalization (trim lines, collapse blanks)
- Leading `# Header` lines are stripped before comparison
- Reconciliation is aggregate (many `.mdc` → one CLAUDE.md), not pairwise
