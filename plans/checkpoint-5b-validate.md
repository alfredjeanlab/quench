# Checkpoint 5B: Shell Adapter Complete - Validation

**Root Feature:** `quench-a8e2`

## Overview

Validation checkpoint for the Shell adapter, confirming it produces useful output on shell projects and correctly detects shell-specific escape patterns. This checkpoint runs `quench check` on test fixtures and documents the results in a validation report.

## Project Structure

Key files involved:

```
quench/
├── tests/fixtures/
│   ├── shell-scripts/              # Clean shell project (should pass)
│   │   ├── quench.toml
│   │   ├── scripts/build.sh
│   │   ├── scripts/deploy.sh
│   │   └── tests/scripts.bats
│   └── violations/                 # Intentional violations (should fail)
│       ├── scripts/bad.sh          # Shell escapes: set +e, shellcheck disable
│       └── quench.toml
└── reports/
    └── checkpoint-5-shell-adapter.md  # Validation report (to create)
```

## Dependencies

No new dependencies. Uses existing:
- `quench` CLI binary
- Test fixtures already in place

## Implementation Phases

### Phase 1: Verify shell-scripts Fixture Produces Useful Output

**Goal:** Run `quench check` on the clean shell project and confirm it produces useful metrics.

**Commands:**
```bash
# Human-readable output
cargo run -- check tests/fixtures/shell-scripts

# JSON output for detailed verification
cargo run -- check tests/fixtures/shell-scripts -o json
```

**Expected Results:**
- Project auto-detected as shell project (no explicit `[shell]` config needed)
- `cloc` check reports source vs test LOC:
  - Source files: `scripts/build.sh`, `scripts/deploy.sh` (2 files)
  - Test files: `tests/scripts.bats` (1 file)
- `escapes` check passes (no violations in clean fixture)
- Human output shows `PASS: cloc, escapes`

**Milestone:** Shell-scripts fixture produces metrics including file counts, line counts, and passes all checks.

---

### Phase 2: Verify Shell Escapes Detected in Violations Fixture

**Goal:** Run `quench check` on violations fixture and confirm shell-specific escapes are detected.

**Commands:**
```bash
# Run escapes check on violations
cargo run -- check tests/fixtures/violations --escapes -o json

# Also test human-readable output
cargo run -- check tests/fixtures/violations --escapes
```

**Expected Violations in `scripts/bad.sh`:**

| Line | Pattern | Violation Type | Notes |
|------|---------|----------------|-------|
| 5-6 | `# shellcheck disable=SC2086` | forbidden/missing_comment | Shellcheck suppress without justification |
| 9 | `set +e` | missing_comment | No `# OK:` comment explaining why |
| 15-16 | `set +e` | (passes) | Has `# OK:` comment - should NOT be flagged |

**Milestone:** At least 2 shell-specific violations detected with appropriate advice messages.

---

### Phase 3: Document Results in Validation Report

**Goal:** Create the checkpoint validation report documenting test results.

**File:** `reports/checkpoint-5-shell-adapter.md`

**Report Structure:**
```markdown
# Checkpoint 5B: Shell Adapter Complete - Validation Report

Generated: YYYY-MM-DD

## Summary

| Criterion | Status | Notes |
|-----------|--------|-------|
| shell-scripts useful output | PASS/FAIL | Metrics description |
| Shell-specific escapes detected | PASS/FAIL | N violations detected |

**Overall Status: PASS/FAIL**

## Detailed Results

### 1. shell-scripts Output
[JSON output and verification checklist]

### 2. Shell Escape Detection in Violations
[Violations output and verification checklist]

## Test Suite Results
[Output from cargo test shell]

## Conclusion
[Summary of shell adapter readiness]
```

**Milestone:** Validation report complete with all test evidence.

---

### Phase 4: Run Full Quality Gates

**Goal:** Ensure the codebase passes all quality checks.

```bash
make check
```

This executes:
1. `cargo fmt --all -- --check`
2. `cargo clippy --all-targets --all-features -- -D warnings`
3. `cargo test --all`
4. `cargo build --all`
5. `./scripts/bootstrap`
6. `cargo audit`
7. `cargo deny check`

**Milestone:** All quality gates pass, confirming shell adapter is production-ready.

---

## Key Implementation Details

### Shell Escape Patterns

The shell adapter detects these escape patterns by default:

| Pattern | Regex | Comment Required | Advice |
|---------|-------|------------------|--------|
| `set +e` | `set \+e` | `# OK:` | Explain why error checking is disabled |
| `eval` | `\beval\s` | `# OK:` | Explain why eval is safe here |

### Shellcheck Suppress Detection

The adapter also detects `# shellcheck disable=SCXXXX` directives:
- Default policy: `forbid` (any suppress is a violation)
- Test files: `allow` (tests can suppress freely)
- Can be configured to require `# OK:` comment justification

### Violations Fixture Content

From `tests/fixtures/violations/scripts/bad.sh`:
```bash
#!/bin/bash
# Script with shell escape hatch violations

# VIOLATION: shellcheck disable without justification
# shellcheck disable=SC2086
echo $UNQUOTED_VAR

# VIOLATION: set +e without OK comment
set +e
risky_command_that_might_fail
set -e

# Proper set +e with comment (should pass)
# OK: We intentionally ignore errors here to collect all results
set +e
optional_command || true
set -e
```

### Shell-scripts Fixture Content

Clean fixture with no violations:
- `scripts/build.sh` - Uses proper `set -euo pipefail`
- `scripts/deploy.sh` - Uses proper `set -euo pipefail`
- `tests/scripts.bats` - Bats test file (classified as test code)

## Verification Plan

### Manual Verification Steps

```bash
# 1. Build quench
cargo build

# 2. Test shell-scripts fixture (should pass)
./target/debug/quench check tests/fixtures/shell-scripts
./target/debug/quench check tests/fixtures/shell-scripts -o json

# 3. Test violations fixture (should fail with shell escapes)
./target/debug/quench check tests/fixtures/violations --escapes
./target/debug/quench check tests/fixtures/violations --escapes -o json

# 4. Run shell-related tests
cargo test shell

# 5. Full quality gates
make check
```

### Verification Checklist

**Criterion 1: shell-scripts produces useful output**
- [ ] cloc metrics show source_files, source_lines
- [ ] cloc metrics show test_files, test_lines
- [ ] escapes check runs and passes
- [ ] Human output is readable

**Criterion 2: Shell-specific escapes detected in violations**
- [ ] `set +e` at line 9 flagged as missing_comment
- [ ] `# shellcheck disable` detected
- [ ] Advice mentions shell-specific guidance
- [ ] Proper `set +e` at line 15 NOT flagged (has `# OK:` comment)

### Expected Test Output

```
cargo test shell
    Running unittests ...
test adapter::shell::policy_tests::... ok
test adapter::shell::suppress_tests::... ok
...
test result: ok. N passed; 0 failed
```

## Summary

This checkpoint validates that the Shell adapter is complete by confirming:

1. **Useful Output**: `quench check` on shell projects produces meaningful metrics including file classification (source vs test) and line counts
2. **Escape Detection**: Shell-specific patterns (`set +e`, `eval`, shellcheck suppresses) are correctly detected and reported with actionable advice

The validation report (`reports/checkpoint-5-shell-adapter.md`) will document the test evidence for both criteria.
