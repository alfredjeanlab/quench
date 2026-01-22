# Checkpoint 1B: CLI Runs - Validation

**Root Feature:** `quench-9489`

## Overview

Validate that the `quench check` CLI functions correctly across various scenarios: minimal fixtures, help output, JSON output, and self-checking the quench codebase. This checkpoint documents actual CLI behavior and produces a validation report.

**Current State**: Checkpoint 1A complete. All `make check` passes, CLI is built and functional.

**End State**: All checkpoint criteria validated, report generated with captured output, CLI proven stable for real-world use.

## Project Structure

No code changes. This checkpoint creates validation artifacts:

```
quench/
├── reports/
│   └── checkpoint-1-cli-runs.md    # NEW: Validation report
└── tests/fixtures/
    └── minimal/
        └── quench.toml              # NEW: Minimal valid config
```

## Dependencies

External tools used for validation:
- `jq` - JSON validation and formatting
- `ajv-cli` (optional) - JSON Schema validation

## Implementation Phases

### Phase 1B.1: Setup Minimal Fixture

**Goal**: Ensure `fixtures/minimal` is a valid test target.

**Context**: Currently `fixtures/minimal` contains only `.gitkeep`. The cloc check requires files to scan.

**Tasks**:
1. Create a minimal `quench.toml` in the fixture
2. Optionally add a single source file for cloc to count

**Files**:

```toml
# tests/fixtures/minimal/quench.toml
# Minimal valid configuration for testing
```

```rust
# tests/fixtures/minimal/src/lib.rs (optional)
// Empty file for minimal scanning
```

**Verification**:
```bash
ls tests/fixtures/minimal/
# Should show at least quench.toml
```

---

### Phase 1B.2: Verify No-Panic Execution

**Goal**: Confirm `quench check` runs without panicking on minimal fixture.

**Commands**:
```bash
# Run check on minimal fixture
cargo run --release -- check tests/fixtures/minimal

# Capture exit code
echo "Exit code: $?"
```

**Expected**:
- No panic or crash
- Clean output (may be empty if no violations)
- Exit code 0 (all stubs pass)

**Validation criteria**: Process completes without SIGABRT, SIGSEGV, or panic backtrace.

---

### Phase 1B.3: Verify Help Output

**Goal**: Confirm `--help` displays all expected flags.

**Commands**:
```bash
cargo run -- check --help
```

**Expected flags** (16 check toggles + general options):

| Flag | Description |
|------|-------------|
| `--cloc` | Run only the cloc check |
| `--escapes` | Run only the escapes check |
| `--agents` | Run only the agents check |
| `--docs` | Run only the docs check |
| `--tests` | Run only the tests check |
| `--git` | Run only the git check |
| `--build` | Run only the build check |
| `--license` | Run only the license check |
| `--no-cloc` | Skip the cloc check |
| `--no-escapes` | Skip the escapes check |
| `--no-agents` | Skip the agents check |
| `--no-docs` | Skip the docs check |
| `--no-tests` | Skip the tests check |
| `--no-git` | Skip the git check |
| `--no-build` | Skip the build check |
| `--no-license` | Skip the license check |
| `-o, --output` | Output format (text/json) |
| `--color` | Color output mode |
| `--no-color` | Disable color output |
| `--limit` | Maximum violations to display |
| `--no-limit` | Show all violations |
| `--config-only` | Validate config and exit |
| `--max-depth` | Maximum directory depth |
| `-v, --verbose` | Enable verbose output |
| `-C, --config` | Use specific config file |

**Verification**:
```bash
# Check for presence of key flags
cargo run -- check --help 2>&1 | grep -E '\-\-cloc|\-\-no-cloc'
cargo run -- check --help 2>&1 | grep -E '\-o.*--output'
cargo run -- check --help 2>&1 | grep -E '\-\-limit|\-\-no-limit'
```

---

### Phase 1B.4: Verify JSON Output

**Goal**: Confirm `-o json` produces valid, schema-compliant JSON.

**Commands**:
```bash
# Generate JSON output
cargo run --release -- check tests/fixtures/minimal -o json > /tmp/quench-output.json

# Validate JSON syntax
jq . /tmp/quench-output.json

# Check required fields
jq 'has("passed") and has("checks")' /tmp/quench-output.json
```

**Expected JSON structure** (per `docs/specs/output.schema.json`):
```json
{
  "timestamp": "2026-01-22T...",
  "passed": true,
  "checks": [
    {
      "name": "cloc",
      "passed": true
    },
    {
      "name": "escapes",
      "passed": true
    },
    ...
  ]
}
```

**Validation checklist**:
- [ ] Valid JSON (parses with jq)
- [ ] Contains `passed` boolean
- [ ] Contains `checks` array
- [ ] Each check has `name` and `passed` fields
- [ ] No unexpected `null` values
- [ ] Optional: Validate against `output.schema.json`

**Schema validation** (if ajv-cli installed):
```bash
ajv validate -s docs/specs/output.schema.json -d /tmp/quench-output.json
```

---

### Phase 1B.5: Verify Exit Code 0 When No Checks Enabled

**Goal**: Confirm exit code behavior when explicitly disabling all checks.

**Commands**:
```bash
# Disable all default-enabled checks
cargo run --release -- check tests/fixtures/minimal \
  --no-cloc --no-escapes --no-agents --no-docs --no-tests

# Check exit code
echo "Exit code: $?"
```

**Expected**:
- Exit code: 0 (no checks ran, nothing failed)
- Output: Empty or minimal header

**Alternative test** (explicit empty enable list):
```bash
# When no checks are enabled via positive flags, defaults run
# This tests the negative path: disable everything
cargo run --release -- check tests/fixtures/minimal -o json \
  --no-cloc --no-escapes --no-agents --no-docs --no-tests \
  | jq '.checks | length'
# Expected: 0 (no checks ran)
```

---

### Phase 1B.6: Dogfooding - Run on Quench Codebase

**Goal**: Run `quench check` on the quench repository itself.

**Commands**:
```bash
# Build release binary
cargo build --release

# Run on quench codebase
./target/release/quench check .

# Also test JSON output
./target/release/quench check . -o json | jq '.passed, .checks | length'
```

**Expected behaviors**:
- Cloc check: May report metrics on quench source files
- Stub checks: Pass (return empty results)
- No panics or crashes
- Graceful handling of deep directory structures

**Document any findings**:
- Unexpected violations discovered
- Performance characteristics (if notable)
- Edge cases encountered

---

### Phase 1B.7: Generate Validation Report

**Goal**: Create comprehensive report documenting all validation results.

**Output file**: `reports/checkpoint-1-cli-runs.md`

**Report structure**:
```markdown
# Checkpoint 1: CLI Runs - Validation Report

Generated: YYYY-MM-DD

## Summary

| Criterion | Status | Notes |
|-----------|--------|-------|
| No-panic on minimal | ✓/✗ | ... |
| Help shows all flags | ✓/✗ | ... |
| JSON output valid | ✓/✗ | ... |
| Exit code 0 (no checks) | ✓/✗ | ... |

## Detailed Results

### 1. No-Panic Execution
[Captured output]

### 2. Help Output
[Captured help text]

### 3. JSON Output Validation
[JSON sample and validation results]

### 4. Exit Code Verification
[Command output and exit codes]

### 5. Dogfooding Results
[Output from running on quench itself]

## Unexpected Behaviors
[Any anomalies discovered]

## Performance Observations
[Timing notes if relevant]
```

**Commands to capture output**:
```bash
# Capture all output for report
mkdir -p reports

# No-panic test
cargo run --release -- check tests/fixtures/minimal 2>&1 | tee /tmp/cp1-minimal.txt

# Help output
cargo run -- check --help 2>&1 | tee /tmp/cp1-help.txt

# JSON output
cargo run --release -- check tests/fixtures/minimal -o json 2>&1 | tee /tmp/cp1-json.txt

# Exit code test
cargo run --release -- check tests/fixtures/minimal \
  --no-cloc --no-escapes --no-agents --no-docs --no-tests 2>&1
echo "Exit: $?" | tee /tmp/cp1-exit.txt

# Dogfooding
./target/release/quench check . 2>&1 | tee /tmp/cp1-dogfood.txt
```

## Key Implementation Details

### Exit Code Semantics

| Code | Meaning |
|------|---------|
| 0 | All checks passed (or no checks ran) |
| 1 | One or more checks failed |
| 2 | Configuration or CLI error |

### Stub Check Behavior

Six checks currently use stubs (`escapes`, `agents`, `docs`, `tests`, `build`, `license`):
- Return empty `CheckResult` with `passed: true`
- No violations, no metrics
- Allows CLI testing before full implementation

### JSON Serialization Rules

From `output/json.rs`:
- `#[serde(skip_serializing_if = "...")]` omits empty/false fields
- No `skipped` field if false
- No `violations` array if empty
- No `error` field if absent

## Verification Plan

### Automated Verification

Run all validation commands and check outputs:

```bash
# Build
cargo build --release

# Phase 1B.2: No-panic
./target/release/quench check tests/fixtures/minimal
test $? -eq 0 && echo "PASS: no-panic" || echo "FAIL: no-panic"

# Phase 1B.3: Help flags
./target/release/quench check --help | grep -q '\-\-cloc' && echo "PASS: help" || echo "FAIL: help"

# Phase 1B.4: JSON valid
./target/release/quench check tests/fixtures/minimal -o json | jq -e 'has("passed")' > /dev/null && echo "PASS: json" || echo "FAIL: json"

# Phase 1B.5: Exit code 0
./target/release/quench check tests/fixtures/minimal \
  --no-cloc --no-escapes --no-agents --no-docs --no-tests
test $? -eq 0 && echo "PASS: exit-code" || echo "FAIL: exit-code"

# Phase 1B.6: Dogfooding
./target/release/quench check . -o json | jq -e '.passed != null' > /dev/null && echo "PASS: dogfood" || echo "FAIL: dogfood"
```

### Success Criteria

- [ ] `quench check` on fixtures/minimal runs without panic
- [ ] `quench check --help` shows all 16 check toggle flags
- [ ] `quench check -o json` produces valid JSON structure
- [ ] Exit code 0 when no checks enabled
- [ ] `reports/checkpoint-1-cli-runs.md` created with all outputs
- [ ] Dogfooding on quench codebase completes successfully
