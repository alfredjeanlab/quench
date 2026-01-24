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
max_lines = 750       # Source file limit (default: 750)
max_lines_test = 1100 # Test file limit (default: 1100)
exclude = ["generated/**", "vendor/**"]

[check.escapes]
# Detects escape hatches like .unwrap(), unsafe, todo!()
# Language-specific defaults apply automatically

[check.agents]
required = ["CLAUDE.md"]  # Require agent context files
tables = "forbid"         # No markdown tables in agent files
```

## Checks

| Check | What it does |
|-------|--------------|
| `cloc` | Enforces file size limits (lines, tokens) |
| `escapes` | Flags unsafe patterns (`.unwrap()`, `unsafe`, `todo!()`) |
| `suppress` | Requires justification for lint suppressions (`#[allow]`, `//nolint`) |
| `agents` | Validates AI context files (CLAUDE.md, .cursorrules) |

## License

MIT - Copyright (c) 2026 Alfred Jean LLC
