# Git Check Specification

The `git` check validates commit message format.

## Purpose

Ensure commit messages follow a consistent format:
- Conventional commit structure (`type(scope): description`)
- Configurable type and scope restrictions
- Documentation in agent files so AI agents know the format

## Configuration

```toml
[git.commit]
check = "error"                    # error | warn | off
# format = "conventional"          # conventional | none (default: conventional)
# skip_merge = true                # Skip merge commits (default: true)

# Optional: restrict to specific types (default: common conventional types)
# types = ["feat", "fix", "chore", "docs", "test", "refactor", "perf", "ci", "build", "style"]

# Optional: restrict to specific scopes (default: any)
# scopes = ["api", "cli", "core"]

# Check that commit format is documented in agent files (CLAUDE.md, etc.)
agents = true                      # default: true

# Create .gitmessage template with --fix
template = true                    # default: true
```

## Commit Format Validation

### Conventional Format

When `format = "conventional"`, commits must match:

```
<type>(<scope>): <description>
```

Or without scope:

```
<type>: <description>
```

Examples:
- `feat(api): add export endpoint`
- `fix: handle empty input`
- `chore(deps): update dependencies`

### Types

| Setting | Behavior |
|---------|----------|
| omitted | Accept common conventional types (built-in default) |
| `[]` | Accept any type (just check structure) |
| `["feat", "fix"]` | Only these types allowed |

Built-in default types:
`feat`, `fix`, `chore`, `docs`, `test`, `refactor`, `perf`, `ci`, `build`, `style`

### Scopes

| Setting | Behavior |
|---------|----------|
| omitted | Any scope allowed (or none) |
| `["api", "cli"]` | Only these scopes allowed |

### Merge Commits

By default, merge commits are skipped:
- `Merge branch 'feature' into main`
- `Merge pull request #123 from user/branch`
- `Merge remote-tracking branch 'origin/main'`

| Setting | Behavior |
|---------|----------|
| `skip_merge = true` (default) | Skip merge commits silently |
| `skip_merge = false` | Validate merge commits against format |

This avoids false positives from git-generated commit messages.

## Agent Documentation Check

When `agents = true` (default), quench verifies that commit format is documented in agent-readable files (CLAUDE.md, AGENTS.md, .cursorrules).

### Detection

Searches for any of:
- Type prefixes followed by `:` or `(` (e.g., `feat:`, `fix(`)
- The phrase "conventional commits" (case-insensitive)

### Output

```
git: FAIL
  Commit format not documented in CLAUDE.md
    Add a "Commits" section describing the format, e.g.:

    ## Commits

    Use conventional commit format: `type(scope): description`
    Types: feat, fix, chore, docs, test, refactor
```

### Disable

```toml
[git.commit]
agents = false
```

## Template Creation

When `template = true` (default), `--fix` creates a `.gitmessage` file and configures git to use it.

### Generated Template

```
# <type>(<scope>): <description>
#
# Types: feat, fix, chore, docs, test, refactor
# Scope: optional (api, cli, core)
#
# Examples:
#   feat(api): add export endpoint
#   fix: handle empty input
#   chore: update dependencies
```

Content is derived from `[git.commit]` config (types, scopes).

### Git Config

`--fix` also runs:

```bash
git config commit.template .gitmessage
```

This is per-checkout. The `.gitmessage` file should be committed to the repo so all clones have it, but each checkout needs the git config set (via `quench init` or `quench check --fix`).

### Behavior

| Scenario | Action |
|----------|--------|
| `.gitmessage` missing | Create it |
| `.gitmessage` exists | Leave it alone |
| `commit.template` not set | Set it |
| `commit.template` already set | Leave it alone |

### Disable

```toml
[git.commit]
template = false
```

## Output

### Fail (bad commit message)

```
git: FAIL
  abc123: "update stuff" - missing type prefix
    Expected: <type>(<scope>): <description>
  def456: "feat(unknown): add thing" - invalid scope "unknown"
    Allowed scopes: api, cli, core
```

### Fail (missing documentation)

```
git: FAIL
  Commit format not documented in CLAUDE.md
    Add a "Commits" section describing the format, e.g.:

    ## Commits

    Use conventional commit format: `type(scope): description`
    Types: feat, fix, chore, docs, test, refactor
```

### Fixed

```
git: FIXED
  Created .gitmessage (commit template)
  Configured git commit.template
```

### JSON Output

```json
{
  "name": "git",
  "passed": false,
  "violations": [
    {
      "file": null,
      "line": null,
      "type": "invalid_format",
      "commit": "abc123",
      "message": "update stuff",
      "advice": "Expected: <type>(<scope>): <description>"
    },
    {
      "file": null,
      "line": null,
      "type": "invalid_scope",
      "commit": "def456",
      "message": "feat(unknown): add thing",
      "scope": "unknown",
      "advice": "Allowed scopes: api, cli, core"
    },
    {
      "file": "CLAUDE.md",
      "line": null,
      "type": "missing_docs",
      "advice": "Add a Commits section describing the format."
    }
  ],
  "metrics": {
    "commits_checked": 5,
    "commits_valid": 3,
    "commits_skipped": 1
  }
}
```

**Violation types**: `invalid_format`, `invalid_type`, `invalid_scope`, `missing_docs`

**Note**: Commit-related violations have `file: null` with `commit` field instead.

**Metrics** (when commits are checked):
- `commits_checked`: Number of commits validated
- `commits_valid`: Commits that passed validation
- `commits_skipped`: Merge commits that were skipped (when `skip_merge = true`)

## Scope

### What Gets Checked

- `--base <ref>`: Validates all commits on branch since base
- `--ci`: Validates all commits on branch
- `--staged`: No commit message exists yet; git check is skipped

### Interaction with Other Checks

The `[check.docs.commit]` check uses commit types to decide which commits require documentation:

```toml
[check.docs.commit]
check = "error"
# types = ["feat", "feature", "story"]   # default
```

Override to require docs for other commit types:

```toml
[check.docs.commit]
check = "error"
types = ["feat", "fix"]   # Also require docs for fixes
```

## Examples

### Minimal

```toml
[git.commit]
check = "error"
```

Uses conventional format, default types, any scope, checks agent docs, creates template.

### Strict

```toml
[git.commit]
check = "error"
types = ["feat", "fix", "chore"]
scopes = ["api", "cli", "core"]
```

### Permissive (structure only)

```toml
[git.commit]
check = "error"
types = []              # Any type (just check structure)
```

### No automation

```toml
[git.commit]
check = "error"
agents = false          # Don't check CLAUDE.md
template = false        # Don't create .gitmessage
```

### Disabled

```toml
[git.commit]
check = "off"
```
