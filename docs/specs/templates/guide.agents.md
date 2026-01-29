# Agent Files Configuration Guide

Configuration reference for the `agents` check.

## Basic Setup

```toml
[check.agents]
check = "error"
files = ["CLAUDE.md", ".cursorrules"]
sync = true
sync_from = "CLAUDE.md"
```

## Content Rules

Control token-inefficient content in agent files.

```toml
[check.agents]
check = "error"
tables = "forbid"
box_diagrams = "allow"  # ASCII box diagrams
mermaid = "allow"       # Mermaid code blocks
max_lines = 500         # Or false to disable
max_tokens = 20000      # Or false to disable
```

## Required Sections (Simple)

Section names are matched case-insensitively.

```toml
[check.agents]
check = "error"
sections.required = ["Directory Structure", "Landing the Plane"]
```

## Required Sections (With Advice)

```toml
[check.agents]
check = "error"

[[check.agents.sections.required]]
name = "Directory Structure"
advice = "Overview of project layout and key directories"

[[check.agents.sections.required]]
name = "Landing the Plane"
advice = "Checklist for AI agents before completing work"
```

## Forbid Sections

Section names are matched case-insensitively and support globs.

```toml
[check.agents]
check = "error"
sections.forbid = ["API Keys", "Secrets", "Test*"]
```

## Scope-Based Configuration

Configure agent files differently at the project root, package, and module levels.

Project root:

```toml
[check.agents]
check = "error"

[check.agents.root]
required = ["CLAUDE.md"]
optional = [".cursorrules"]
forbid = []
max_lines = 500
max_tokens = 20000
sections.required = ["Directory Structure", "Landing the Plane"]
```

Each package directory:

```toml
[check.agents.package]
required = []
optional = ["CLAUDE.md"]
max_lines = 200
max_tokens = 800
```

Subdirectories:

```toml
[check.agents.module]
required = []
max_lines = 100
max_tokens = 400
```

## Sync Behavior

Keep agent files in sync. When `--fix` is used, content is synced from the
source of truth (default: first in `files` list).

```toml
[check.agents]
check = "error"
files = ["CLAUDE.md", ".cursorrules"]
sync = true
sync_from = "CLAUDE.md"
```

## Disable Sync

Allow agent files to have different content.

```toml
[check.agents]
check = "error"
files = ["CLAUDE.md", ".cursorrules"]
sync = false
```

## Claude Profile

```toml
[check.agents]
check = "error"
files = ["CLAUDE.md"]
required = ["CLAUDE.md"]
sync = true
sync_from = "CLAUDE.md"
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

## Combined Claude and Cursor

`CLAUDE.md` is required; `.cursorrules` is optional. Content syncs from `CLAUDE.md`.

```toml
[check.agents]
check = "error"
files = ["CLAUDE.md", ".cursorrules"]
required = ["CLAUDE.md"]       # CLAUDE.md is required
optional = [".cursorrules"]    # .cursorrules is optional
sync = true
sync_from = "CLAUDE.md"        # Sync from CLAUDE.md if both exist
tables = "forbid"
max_lines = 500
max_tokens = 20000
```
