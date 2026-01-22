# Output Specification

Quench output is designed for AI agent consumption with progressive disclosure.

## Core Principles

### Progressive Disclosure

Only surface actionable information:
- **Passing checks**: Silent (no output)
- **Failing checks**: Show violation + advice
- **Fixed items** (with `--fix`): Brief confirmation

Rationale: Agents operate under token constraints. Verbose output about passing
checks wastes context and obscures actionable items.

### Actionable Advice

Every failure includes:
1. **Location**: File path and line number (when applicable)
2. **Violation**: What rule was violated and current value
3. **Advice**: Concise guidance on how to fix (not instructions to agents)

Advice is:
- **Specific**: References the actual violation
- **Concise**: One or two sentences
- **Technical**: Assumes reader understands the codebase
- **Not prescriptive**: Says _what_ to fix, not _how_ to approach it

Good: `Add a // SAFETY: comment explaining the invariants.`
Bad: `You should consider adding a safety comment to explain why this unsafe block is sound.`

Good: `Split into smaller modules. Consider extracting TokenStream logic.`
Bad: `This file is too long. Please refactor it into smaller pieces.`

## Output Formats

### Text Format (default)

```
<check-name>: FAIL
  <file>:<line>: <brief violation description>
    <advice>

<check-name>: FAIL
  <file>: <violation> (<value> vs <threshold>)
    <advice>
```

Example:
```
escapes: FAIL
  src/parser.rs:47: unsafe block without // SAFETY: comment
    Add a // SAFETY: comment explaining the invariants.
  src/parser.rs:112: .unwrap() in production code
    Handle the error case or add // OK: comment if infallible.

file-size: FAIL
  src/lexer.rs: 923 lines (max: 900)
    Split into smaller modules. The tokenize() function could be extracted.
```

### JSON Format (`-o json`)

#### Top-Level Schema

```json
{
  "timestamp": "2026-01-21T10:30:00Z",
  "passed": false,
  "checks": [
    { /* check object */ }
  ]
}
```

#### Check Object Schema

Every check follows this normalized structure:

```json
{
  "name": "check_name",
  "passed": false,
  "violations": [
    {
      "file": "path/to/file",
      "line": 47,
      "type": "violation_category",
      "advice": "Actionable guidance."
    }
  ],
  "metrics": {
    "total_field": 123,
    "other_field": 456
  },
  "by_package": {
    "package_name": {
      "total_field": 45,
      "other_field": 67
    }
  }
}
```

| Field | Type | Description |
|-------|------|-------------|
| `name` | string | Check identifier (e.g., `"escapes"`, `"cloc"`) |
| `passed` | boolean | Whether check passed |
| `violations` | array | List of violations (omit if empty) |
| `metrics` | object | Aggregated counts and measurements (omit if none) |
| `by_package` | object | Per-package metrics breakdown (omit if no packages) |

#### Violation Object Schema

```json
{
  "file": "src/parser.rs",
  "line": 47,
  "type": "missing_comment",
  "advice": "Add a // SAFETY: comment explaining the invariants."
}
```

| Field | Type | Description |
|-------|------|-------------|
| `file` | string\|null | File path (null for non-file violations like commit messages) |
| `line` | number\|null | Line number (null if not applicable) |
| `type` | string | Violation category (check-specific) |
| `advice` | string | Actionable guidance |

Checks may add context-specific fields alongside these (e.g., `pattern`, `threshold`, `commit`).

#### Example

```json
{
  "timestamp": "2026-01-21T10:30:00Z",
  "passed": false,
  "checks": [
    {
      "name": "escapes",
      "passed": false,
      "violations": [
        {
          "file": "src/parser.rs",
          "line": 47,
          "type": "missing_comment",
          "pattern": "unsafe",
          "advice": "Add a // SAFETY: comment explaining the invariants."
        }
      ],
      "metrics": {
        "source": { "unsafe": 3, "unwrap": 0 },
        "test": { "unsafe": 0, "unwrap": 47 }
      },
      "by_package": {
        "core": {
          "source": { "unsafe": 2, "unwrap": 0 },
          "test": { "unsafe": 0, "unwrap": 24 }
        }
      }
    },
    {
      "name": "cloc",
      "passed": false,
      "violations": [
        {
          "file": "src/lexer.rs",
          "line": null,
          "type": "file_too_large",
          "value": 923,
          "threshold": 750,
          "advice": "Split into smaller modules."
        }
      ],
      "metrics": {
        "source_lines": 12453,
        "test_lines": 8921,
        "ratio": 0.72
      }
    }
  ]
}
```

JSON is pipe-friendly: `quench check -o json | jq '.checks[] | select(.passed == false)'`

## Colorization

### Detection Logic

```
if --color=always:
    use color
elif --color=never:
    no color
else (--color=auto, default):
    if not stdout.is_tty():
        no color
    elif env.CLAUDE_CODE or env.CODEX or env.CI:
        no color
    else:
        use color
```

### Color Scheme

- **Check name**: Bold
- **FAIL**: Red
- **File path**: Cyan
- **Line number**: Yellow
- **Advice**: Default (no color)

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | All checks passed |
| 1 | One or more checks failed |
| 2 | Configuration error or invalid arguments |
| 3 | Internal error (bug in quench) |

## Verbosity

### Default (failures + summary)

Failing checks produce output, followed by a one-line summary:

```
escapes: FAIL
  src/parser.rs:47: unsafe block without // SAFETY: comment
    Add a // SAFETY: comment explaining the invariants.

4 checks passed, 1 failed
```

When all checks pass, only the summary is shown:

```
5 checks passed
```

## Fix Mode Output (`--fix`)

When auto-fixing:
```
agent: FIXED
  Synced .cursorrules from CLAUDE.md (3 sections updated)

escapes: FAIL (not auto-fixable)
  src/parser.rs:47: unsafe block without // SAFETY: comment
    Add a // SAFETY: comment explaining the invariants.

1 check fixed, 1 failed
```

## Violation Limits (Agent-First)

To avoid overwhelming agent context with violations, quench limits output by default:

- **Default limit**: 15 violations shown
- **Behavior**: Once limit reached, stop and report
- **Override**: Use `--limit N` for custom limit, `--no-limit` for all

```
escapes: FAIL
  src/parser.rs:47: unsafe block without // SAFETY: comment
    Add a // SAFETY: comment explaining the invariants.
  src/parser.rs:112: .unwrap() in production code
    Handle the error case.

Stopped after 15 violations. Use --no-limit to see all.
```

### Show All (`--no-limit`)

To see all violations (e.g., for human review):

```bash
quench check --no-limit       # Show all violations
quench check -o json --no-limit  # JSON with all violations
```

Full counts are always available in `--ci` mode for metrics storage.

## Streaming vs Buffered

- **Text format**: Stream output as checks complete (better for slow checks)
- **JSON format**: Buffer and output complete JSON at end

## Error Recovery

Quench runs all checks regardless of individual failures:

| Scenario | Behavior |
|----------|----------|
| Check fails (violations found) | Continue to next check, exit 1 at end |
| Check errors (config invalid) | Skip check, report error, continue, exit 2 at end |
| Check crashes (internal error) | Log error, continue to next check, exit 3 at end |
| File unreadable | Skip file with warning, continue check |
| Pattern timeout (5s per file) | Skip file with warning, continue check |

**Rationale**: Agents benefit from seeing all violations at once rather than fixing one check, re-running, and discovering more failures. A single run should surface everything actionable.

### Exit Code Priority

When multiple error types occur, the most severe exit code wins:

```
3 (internal error) > 2 (config error) > 1 (check failed) > 0 (passed)
```

### Partial Results

JSON output always includes all checks that ran, even if some errored:

```json
{
  "passed": false,
  "checks": [
    { "name": "cloc", "passed": true },
    { "name": "escapes", "passed": false, "violations": [...] },
    { "name": "build", "error": "cargo not found", "skipped": true }
  ]
}
```

Checks with `"skipped": true` encountered an error and did not complete.

## JSON Schema

The JSON output format is formally documented in [output.schema.json](output.schema.json) for tooling integration.
