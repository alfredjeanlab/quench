# CLI Specification

Quench provides a minimal command-line interface optimized for AI agents.

## Commands

```
quench                    # Show help
quench help               # Show help
quench init               # Initialize quench.toml
quench check [FLAGS]      # Run quality checks
quench report [FLAGS]     # Generate reports
```

## quench check

Run quality checks on the codebase.

```bash
quench check              # All files, fast checks
quench check src/         # Check specific directory
quench check src/parser.rs src/lexer.rs  # Check specific files
```

### File Arguments

When file or directory arguments are provided, only those paths are checked:

```bash
quench check src/parser.rs        # Single file
quench check src/ tests/          # Multiple directories
quench check **/*.rs              # Shell glob (expanded by shell)
```

This is useful for quick iteration during development.

### Scope Flags

| Flag | Description |
|------|-------------|
| `--staged` | Check staged files only (pre-commit hook) |
| `--base <REF>` | Compare against git ref (branch, tag, commit); also determines baseline note for ratchet |
| `--ci` | CI mode: slow checks + auto-detect base |
| `--package <NAME>` | Target specific package |

```bash
quench check --staged         # Pre-commit: staged files only
quench check --base main      # Compare against main branch
quench check --base v1.0.0    # Compare against a tag
quench check --base HEAD~5    # Compare against recent commits
quench check --ci             # Full CI mode
```

### Check Toggles

Enable or disable specific checks:

| Flag | Check | Description |
|------|-------|-------------|
| `--[no-]cloc` | cloc | Lines of code, file size limits |
| `--[no-]escapes` | escapes | Escape hatch detection |
| `--[no-]agents` | agents | CLAUDE.md, .cursorrules validation |
| `--[no-]docs` | docs | File refs, specs, doc correlation (CI) |
| `--[no-]tests` | tests | Test correlation + coverage/time (CI) |
| `--[no-]git` | git | Commit message format validation |
| `--[no-]build` | build | Binary/bundle size + build time (CI only) |
| `--[no-]license` | license | License headers (CI only) |

```bash
quench check --no-docs        # Skip docs check
quench check --tests          # Only tests (implies --no-* for others)
quench check --no-cloc --no-escapes  # Skip multiple
```

### Output Flags

| Flag | Description |
|------|-------------|
| `-o, --output <FMT>` | Output format: `text` (default), `json` |
| `--[no-]color` | Color output (default: auto based on TTY) |
| `--[no-]limit [N]` | Violation limit (default: 15, --no-limit for all) |
| `--fix` | Auto-fix what can be fixed |
| `--dry-run` | Show what --fix would change without changing it |
| `--save <FILE>` | Save metrics to file (CI mode) |
| `--no-notes` | Disable git notes, use file-based baseline only |

**Violation Limit**: By default, quench shows at most **15 violations** to avoid overwhelming AI agent context windows. Use `--no-limit` to show all violations (e.g., for human review or CI logs). Use `--limit N` to set a custom limit.

**Baseline Storage**: By default, `--fix` saves metrics to git notes (`refs/notes/quench`). Use `--no-notes` to disable git notes and use only file-based baseline. Use `--save <FILE>` to explicitly save to a specific file.

```bash
quench check -o json          # JSON output
quench check --no-limit       # Show all violations
quench check --limit 50       # Show up to 50
quench check --fix            # Auto-fix (saves to git notes by default)
quench check --fix --dry-run  # Preview fixes without applying
quench check --fix --no-notes # Save to file only, no git notes
quench check --ci --save .quench/metrics.json  # Save metrics to specific file
```

### Development Flags

Flags for development and debugging:

| Flag | Description |
|------|-------------|
| `--no-cache` | Disable file cache (always re-check all files) |
| `--timing` | Show timing breakdown (file walking, pattern matching, etc.) |

```bash
quench check --no-cache       # Force fresh check, ignore cache
quench check --timing         # Show where time is spent
```

### Examples

```bash
# Fast check (default)
quench check

# Pre-commit hook
quench check --staged

# CI pipeline
quench check --ci

# CI with auto-fix on main
quench check --ci --fix

# JSON for tooling
quench check -o json

# Only escapes check
quench check --escapes

# Everything except docs
quench check --no-docs
```

## quench report

Generate reports from stored metrics.

```bash
quench report                 # Markdown to stdout
quench report -o json         # JSON output
quench report -o html         # HTML dashboard
quench report -o report.html  # Write to file
```

### Output Formats

| Format | Description |
|--------|-------------|
| `text` | Agent and context friendly text output (default) |
| `json` | Machine-readable metrics |
| `html` | Static dashboard page |

Reports read from `.quench/baseline.json` or git notes.

### Check Toggles

Same as `quench check`:

```bash
quench report --docs          # Only docs metrics
quench report --no-license    # Exclude license metrics
```

## quench init

Initialize quench configuration.

```bash
quench init                   # Auto-detect and create quench.toml
quench init --force           # Overwrite existing
```

### Explicit Profiles

Use `--with` to initialize with opinionated defaults for specific languages and agents:

```bash
quench init --with rust              # Rust project defaults
quench init --with golang            # Go project defaults
quench init --with shell             # Shell project defaults
quench init --with rust,shell        # Multi-language project
quench init --with golang,rust       # Multi-language project
quench init --with claude            # Claude Code agent defaults
quench init --with cursor            # Cursor IDE agent defaults
quench init --with golang,claude     # Combined language + agent
```

| Profile | Description |
|---------|-------------|
| `rust` | Cargo workspace, clippy escapes, unsafe/unwrap detection |
| `shell` | Shellcheck integration, set +e/eval escapes |
| `golang` | Go modules, nolint escapes, unsafe.Pointer detection |
| `claude` | CLAUDE.md with required sections, sync setup |
| `cursor` | .cursorrules with required sections, sync setup |

**Auto-detection**: When no `--with` is specified, quench detects:
- Languages from project root (Cargo.toml → rust, go.mod → golang, *.sh → shell)
- Agent files from existing files (CLAUDE.md, .cursorrules)

See language-specific defaults:
- [Rust defaults](langs/rust.md#profile-defaults)
- [Go defaults](langs/golang.md#profile-defaults)
- [Shell defaults](langs/shell.md#profile-defaults)
- [Agent file defaults](checks/agents.md#profile-defaults)

### Landing the Plane Auto-Population

When initializing with agent profiles (`claude`, `cursor`), quench will:
1. Check if a "Landing the Plane" section exists in agent files
2. If missing, auto-populate with a default checklist
3. Include language-specific items based on detected/selected profiles
4. Always include `quench check` as a checklist item

Example auto-generated section:
```markdown
## Landing the Plane

Before completing work:

- [ ] Run `quench check`
- [ ] Run `cargo test` (rust profile)
- [ ] Run `cargo clippy` (rust profile)
- [ ] Run `shellcheck` on changed scripts (shell profile)
```

## Global Flags

Available on all commands:

| Flag | Description |
|------|-------------|
| `-h, --help` | Show help |
| `-v, --version` | Show version |

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | All checks passed |
| 1 | One or more checks failed |
| 2 | Configuration or argument error |
| 3 | Internal error |

## Checks Summary

| Check | Fast | CI | Fixable | Description |
|-------|------|-----|---------|-------------|
| `cloc` | ✓ | ✓ | | Lines of code, file size limits |
| `escapes` | ✓ | ✓ | | Escape hatch detection |
| `agents` | ✓ | ✓ | ✓ | Agent file validation and sync |
| `docs` | ✓ | ✓ | | File refs, specs, correlation (CI) |
| `tests` | ✓ | ✓ | | Test correlation + coverage/time (CI) |
| `git` | ✓ | ✓ | ✓ | Commit message format (disabled by default) |
| `build` | | ✓ | | Binary/bundle size + build time |
| `license` | | ✓ | ✓ | License header validation |

**Fast mode**: Runs by default, quick checks only.
**CI mode**: `--ci` flag, enables slow checks (build, license, test execution).

## CI Integration

### GitHub Actions

```yaml
- name: Quality checks
  run: quench check --ci

- name: Auto-fix on main
  if: github.ref == 'refs/heads/main'
  run: |
    quench check --ci --fix
    git add -A
    git commit -m "chore: quality fixes" || true
    git push
```

### Pre-commit Hook

```bash
#!/bin/bash
quench check --staged
```

## Color Detection

Color is enabled when:
- `--color` flag is set, OR
- stdout is a TTY AND no agent environment detected

Agent detection checks `CLAUDE_CODE`, `CODEX`, `CURSOR` environment variables.

Override with `--color` or `--no-color`.
