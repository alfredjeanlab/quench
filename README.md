# Quench

A fast linting tool for AI agents that measures quality signals.

## Installation

### Homebrew (macOS)

```bash
brew install alfredjeanlab/tap/quench
```

### Linux / Manual

```bash
curl -fsSL https://github.com/alfredjeanlab/quench/releases/latest/download/install.sh | bash
```

## Quick Start

Initialize configuration in your project:

```bash
quench init
```

Run quality checks:

```bash
quench check
```

## Configuration

Quench is configured via `quench.toml`. Example with cloc limits:

```toml
version = 1

[check.cloc]
exclude = ["tests/fixtures/**", "vendor/**"]
advice = "Split large files into smaller modules."
advice_test = "Consider parameterized tests with yare."
```

## License

MIT - Copyright (c) 2026 Alfred Jean LLC
