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
# max_lines = 750           # default
# max_lines_test = 1100     # default
exclude = ["generated/**", "vendor/**"]
advice_test = "Use table-driven tests with t.Run()."

[check.escapes]
# Language-specific defaults apply (unsafe, .unwrap(), set +e, etc.)

[check.agents]
# required = ["*"]          # default: at least one agent file
tables = "forbid"           # no markdown tables

[check.agents.sections]
required = ["Landing the Plane"]
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
