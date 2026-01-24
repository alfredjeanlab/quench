# Quench Specs

A fast, configurable quality linting CLI for software projects.

## Personas

**AI Coding Agents** (primary audience)
- Claude Code, Codex, Cursor, etc.
- Operate under token constraints
- Need actionable, parseable output
- "Landing the plane" - final checks before completing work

**Human Developers** (secondary)
- Benefit from same concise output
- Use `--no-limit` to see all violations
- Configure project-specific rules

**CI Pipelines**
- Collect and store metrics
- Enforce ratcheting (no regressions)
- Generate reports and dashboards

## Principles

**Agent-First Output**
- Progressive disclosure: only surface failures, never passing checks
- Token efficiency: concise output, no verbose explanations
- Actionable advice: each failure includes specific, fixable guidance
- Advice tells _what_ to fix, not _how_ to approach fixing

**Fast by Design**
- Target: sub-second for fast checks on typical projects
- Acceptable: 1-5s for full CI checks
- Unacceptable: anything over 30 seconds
- Design decisions prioritize speed: parallel scanning, early termination, mmap

**Convention Over Configuration**
- Zero config should work for most projects
- Smart defaults for common patterns
- Language detection enables language-specific defaults
- Configuration only for project-specific conventions

**Ratcheting Over Thresholds**
- Metrics can improve, never regress
- Baseline auto-updates when metrics improve
- Gradual quality improvement without manual threshold maintenance

## Output Rules

- Passing checks: silent
- Failing checks: location + violation + advice
- Advice is specific, concise, technical, not prescriptive
- Default 15 violation limit (agent context protection)
- Full counts only in `--ci` mode

Good advice: `Add a // SAFETY: comment explaining the invariants.`
Bad advice: `You should consider adding a safety comment to explain why this unsafe block is sound.`

## Modes

- **Fast** (default): quick checks, early termination, limited output
- **CI** (`--ci`): full checks, metrics collection, no limits
- **Fix** (`--fix`): auto-fix what can be fixed, report what remains

## Naming Conventions

**Spec filenames** use complete, unambiguous names for documentation clarity:
- `escape-hatches.md`, `license-headers.md`, `language-adapters.md`

**CLI flags and config keys** use short, single-word names for brevity:
- `--escapes`, `--license`, `[check.escapes]`, `[check.license]`

This allows documentation to be self-descriptive while keeping commands terse.

## CLI Conventions

- Commands: `quench check`, `quench report`, `quench init`
- Check toggles: `--[no-]<check>` (e.g., `--no-docs`, `--escapes`)
- Comparison: `--base <REF>` (branch, tag, or commit)
- Output: `-o <FMT>` or `--output <FMT>` (`text`, `json`, `html`)
- Limits: `--[no-]limit [N]` (default: 15 violations)

## Config Conventions

**Check levels** (`check` field):
- `"error"` - fail on violations (default)
- `"warn"` - report but don't fail
- `"off"` - disable

**Binary options** (allow/forbid pattern):
- `tables = "allow"` or `"forbid"`
- `placeholders = "allow"` or `"forbid"`

**Escape pattern actions** (`action` field):
- `"count"` - count occurrences, optional threshold
- `"comment"` - require justification comment
- `"forbid"` - never allowed in source

**Lint suppression** (`[check.<lang>.suppress]`):
- `check = "forbid" | "comment" | "allow"`
- `comment = "// PATTERN:"` - optional specific pattern (default: any comment)
- `allow = [...]` - codes that don't need comment
- `forbid = [...]` - codes never allowed
- `.source` / `.test` subscopes for different policies

**Policy** (`[check.<lang>.policy]`):
- `lint_changes = "standalone"` - lint config must be separate PR
- `lint_config = [...]` - files that trigger standalone requirement

**Size limits**:
- `max_lines`, `max_lines_test`
- `max_tokens` (default: 20000, use `false` to disable)

## Spec Writing

**Prefer exact output comparison** in behavioral tests to catch format regressions:

```rust
// GOOD: Exact comparison catches unexpected changes
cli().on("fixture").exits(1).stdout_eq(
    "cloc: FAIL
  src/file.rs: file_too_large (lines: 100 vs 50)
    Split into smaller modules.

FAIL: cloc
"
);

// ACCEPTABLE: Pattern matching when output varies
cli().on("fixture").fails().stdout_has("file_too_large");
```

Use `stdout_eq()` for output format specs. Use `stdout_has()` only when exact comparison isn't practical (dynamic counts, timestamps, etc.). See `tests/specs/prelude.rs` for helpers.

## File Structure

```
docs/specs/
├── 00-overview.md      # Problem, philosophy, capabilities
├── 01-cli.md           # Commands and flags
├── 02-config.md        # quench.toml schema
├── 03-output.md        # Output formats and rules
├── 04-ratcheting.md    # Regression prevention
├── 10-language-adapters.md
├── 11-test-runners.md
├── 20-performance.md   # Performance strategy
├── 99-todo.md          # Future work
├── output.schema.json  # JSON output schema for tooling
├── checks/             # Per-check specifications
│   ├── agents.md
│   ├── build.md        # Binary/bundle size, build time
│   ├── cloc.md
│   ├── docs.md
│   ├── escape-hatches.md
│   ├── git.md          # Commit message format
│   ├── license-headers.md
│   └── tests.md        # Includes coverage, test time
└── langs/              # Language-specific details
    ├── golang.md       # Go: build metrics, coverage
    ├── javascript.md   # JS/TS: packages, tests
    ├── rust.md         # Rust: build metrics, coverage
    └── shell.md        # Shell: bats, kcov coverage
```
