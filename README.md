# Quench

A fast linting tool for AI agents that measures quality signals.

## Installation

```bash
# macOS
brew install alfredjeanlab/tap/quench

# Linux / macOS (manual)
curl -fsSL https://github.com/alfredjeanlab/quench/releases/latest/download/install.sh | bash
```

## Supported Languages

- **Rust** - `.rs` files, `#[allow]` suppression tracking
- **Go** - `.go` files, `//nolint` suppression tracking
- **Shell** - `.sh`/`.bash` files, shellcheck directive tracking
- **Other** - Basic cloc checks work on any text file

## Quick Start

```bash
quench init    # Create quench.toml
quench check   # Run quality checks
```

## Configuration

```toml
version = 1

[check.cloc]
exclude = ["tests/fixtures/**"]
advice_test = "Use table-driven tests with t.Run()."  # customize advice for project

[check.agents]
required = ["CLAUDE.md"]  # default: ["*"] any agent file
tables = "forbid"         # default: "allow"
# mermaid = "forbid"
# box_diagrams = "forbid"

# Per-scope config:
#   .root (project, monorepo or workspace)
#   .package (crates/, packages/, etc)
#   .module (package subdirs)
[check.agents.root]
max_lines = 750  # default: 500

[check.agents.package]
required = ["CLAUDE.md"]  # require agent file in each package
sections.required = ["API"]  # with an "API" section
```

## Checks

| Check | What it does |
|-------|--------------|
| `cloc` | Enforces file size limits (lines, tokens) |
| `escapes` | Flags escape hatches (`.unwrap()`, `unsafe`, `set +e`) |
| `suppress` | Requires justification for lint suppressions (`#[allow]`, `//nolint`) |
| `agents` | Validates AI context files (CLAUDE.md, .cursorrules) |

## License

MIT - Copyright (c) 2026 Alfred Jean LLC
