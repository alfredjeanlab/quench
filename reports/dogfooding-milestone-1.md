# Dogfooding Milestone 1 Report

Date: 2026-01-23

## Summary

First dogfooding milestone - running quench on the quench project itself. This validates that:
1. All enabled checks (cloc, escapes, agents) pass on the quench codebase
2. The agents check properly validates CLAUDE.md
3. Exact output tests capture output format for regression testing

## quench check Output

### Initial Run

```
PASS: cloc, escapes, agents
```

All checks passed on the first run after adding the agents configuration.

### JSON Metrics

```json
{
  "timestamp": "2026-01-24T02:16:25Z",
  "passed": true,
  "checks": [
    {
      "name": "cloc",
      "passed": true,
      "metrics": {
        "ratio": 1.49,
        "source_files": 66,
        "source_lines": 10339,
        "source_tokens": 92170,
        "test_files": 187,
        "test_lines": 15440,
        "test_tokens": 131646
      },
      "by_package": {
        "quench": {
          "ratio": 0.57,
          "source_files": 61,
          "source_lines": 10130,
          "source_tokens": 90469,
          "test_files": 46,
          "test_lines": 5819,
          "test_tokens": 49409
        }
      }
    },
    {
      "name": "escapes",
      "passed": true,
      "metrics": {
        "source": { "transmute": 1, "unsafe": 0 },
        "test": { "transmute": 1, "unsafe": 14 }
      }
    },
    {
      "name": "agents",
      "passed": true,
      "metrics": {
        "files_found": ["CLAUDE.md"],
        "files_missing": [],
        "in_sync": true
      }
    }
  ]
}
```

## Violations Found and Fixed

No violations were found. The quench project already had:
- Properly structured CLAUDE.md with required sections
- No oversized files exceeding cloc limits
- No unsafe code or forbidden escapes

## quench.toml Configuration

```toml
version = 1

[check.cloc]
advice = """
Can the code be made more concise?

If not, split large source files into sibling modules or submodules in a folder;
consider refactoring to be more unit testable.

Avoid picking and removing individual lines to satistfy the linter,
prefer properly refactoring out testable code blocks.
"""
advice_test = """
Can tests be parameterized using `yare` to be more concise?
If not, split large test files into a {module}_tests/ folder.
""""
exclude = [
    # ONLY for intentionally large fixture data
    # NEVER for tests
    # NEVER for source files
    "tests/fixtures/cloc/**"
]

[check.agents]
check = "error"
files = ["CLAUDE.md"]
required = ["CLAUDE.md"]
sync = false                    # Only one agent file in quench
tables = "forbid"
max_lines = 500
max_tokens = 20000

[[check.agents.sections.required]]
name = "Directory Structure"
advice = "Document the project layout"

[[check.agents.sections.required]]
name = "Landing the Plane"
advice = "Checklist for AI agents before committing"
```

## Tests Added

### Exact output tests

Added 6 Exact output tests in `tests/specs/checks/agents.rs`:

| Test | Description |
|------|-------------|
| `snapshot_missing_file_text` | Text output for missing required file |
| `snapshot_out_of_sync_text` | Text output for out-of-sync files |
| `snapshot_forbidden_table_text` | Text output for forbidden markdown table |
| `snapshot_missing_section_text` | Text output for missing required section |
| `snapshot_oversized_lines_text` | Text output for file exceeding max_lines |
| `snapshot_agents_project_json` | JSON output for multi-scope project |

### Dependencies Added

Added `insta` crate to dev-dependencies for snapshot testing:
```toml
insta = { version = "1", features = ["json"] }
```

## Unexpected Behaviors

None encountered. The dogfooding process was smooth:
- Configuration was intuitive
- Output was clear and actionable
- All existing tests continued to pass

## Recommendations

1. ~~**cloc metrics show 0 files**~~ - **RESOLVED**: The cloc check now correctly reports workspace member files (66 source files, 187 test files). The fix was automatic workspace detection from `Cargo.toml` which reads the `[workspace]` members configuration.

2. **Consider adding timestamp redaction** - For Exact output tests with JSON output, timestamps need to be manually redacted for determinism. Consider adding a built-in option or helper for this.

## Exit Criteria Status

- [x] `quench check` passes on quench project itself
- [x] `quench.toml` includes agents configuration
- [x] Exact output tests exist for all violation types
- [x] Fix functionality tested and working (via `agents_fix_syncs_files_from_sync_source`)
- [x] `reports/dogfooding-milestone-1.md` documents the experience
- [x] All tests pass: `make check`
