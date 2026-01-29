# Git Configuration Guide

Configuration reference for git integration.

## Basic Conventional Commits

```toml
[git.commit]
check = "error"
format = "conventional"
skip_merge = true
agents = true
template = true
```

## Restrict Commit Types

Only allow these types (default: common conventional types).

```toml
[git.commit]
check = "error"
types = ["feat", "fix", "chore", "docs", "test", "refactor"]
```

## Restrict Commit Scopes

Only allow these scopes (default: any scope allowed).

```toml
[git.commit]
check = "error"
scopes = ["api", "cli", "core"]
```

## Allow Any Type (Structure Only)

An empty array accepts any type but still checks the structure.

```toml
[git.commit]
check = "error"
types = []
```

## Disable Features

```toml
[git.commit]
check = "error"
agents = false    # Don't check agent file documentation
template = false  # Don't create .gitmessage
```

## Allow Merge Commits

By default, merge commits are skipped. Set `skip_merge = false` to validate
merge commits against the format.

```toml
[git.commit]
check = "error"
skip_merge = false
```

## Git Configuration

Baseline storage modes:

- `"notes"` — use git notes (`refs/notes/quench`)
- `"<path>"` — use file at path (e.g., `".quench/baseline.json"`)

```toml
[git]
base = "main"
baseline = "notes"
```

## Complete Example

```toml
[git]
base = "main"
baseline = "notes"

[git.commit]
check = "error"
format = "conventional"
types = ["feat", "fix", "chore", "docs", "test", "refactor", "perf", "ci", "build", "style"]
scopes = ["api", "cli", "core"]
skip_merge = true
agents = true
template = true
```
