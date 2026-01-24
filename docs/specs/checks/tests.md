# Tests Check Specification

The `tests` check validates test practices and collects test metrics.

## Purpose

- **Fast mode**: Verify source changes have corresponding test changes
- **CI mode**: Run tests and collect coverage/timing metrics

## Commit Checking (Fast Mode)

Ensures code changes are accompanied by test changes. Configured via `[check.tests.commit]`.

### Scope

Commit checking can operate at different scopes:

### Branch Scope (Default)

Checks all changes on the branch together:
- All source and test changes across all commits count
- Order doesn't matter (tests first or code first both work)
- Ideal for PR checks

```bash
quench check --base main
```

### Commit Scope

Checks individual commits with **asymmetric rules**:
- Tests without code = **OK** (TDD recognized)
- Code without tests = **FAIL**
- Supports "tests first" workflows naturally

```bash
quench check --staged          # Pre-commit
quench check --base HEAD~5     # Recent commits
```

```toml
[check.tests.commit]
scope = "branch"  # or "commit"
```

### Check Levels

#### Error (Default)

Source changes require corresponding test changes:
- New source files → require new test file (or test additions)
- Modified source files → require test changes
- Deletions → no test requirement

#### Warn

Report but don't fail:
- Report when tests appear missing
- Exit code 0 regardless
- Useful during adoption

#### Off

Disable commit checking entirely.

## Change Detection

### Git Integration

```bash
# Staged changes (pre-commit)
quench check --staged

# Compare to branch (PR/CI)
quench check --base main

# Compare to tag or commits
quench check --base v1.0.0
quench check --base HEAD~5
```

### What Counts as "Changed"

- Added files: new file in diff
- Modified files: existing file with changes
- Deleted files: not checked (no test required)

### Inline Test Changes (Rust)

For Rust, changes to `#[cfg(test)]` blocks in the same file **satisfy the test requirement**:
- Adding `#[test]` functions counts as test changes
- Modifying existing test code counts
- No separate test file required if inline tests updated

## Test File Matching

### Default Patterns

| Pattern | Description |
|---------|-------------|
| `tests/**/*` | Tests directory |
| `test/**/*` | Test directory (singular) |
| `spec/**/*` | Spec directory (RSpec/Ruby convention) |
| `**/__tests__/**` | Jest convention |
| `**/*_test.*` | Underscore suffix |
| `**/*_tests.*` | Underscore suffix (plural) |
| `**/*.test.*` | Dot suffix (Jest/Vitest) |
| `**/*.spec.*` | Spec suffix |
| `**/test_*.*` | Test prefix |

### Pattern-Based (Language Agnostic)

For source file `src/parser.rs`, look for tests in:
1. `tests/parser.rs`
2. `tests/parser_test.rs`
3. `tests/parser_tests.rs`
4. `src/parser_test.rs`
5. `src/parser_tests.rs`
6. `test/parser.rs`
7. Any file matching test patterns containing "parser"

For source file `src/parser.ts`, look for tests in:
1. `parser.test.ts`
2. `__tests__/parser.test.ts`
3. `parser.spec.ts`
4. `tests/parser.test.ts`

## Placeholder Tests

Placeholder tests indicate planned test implementation in the target project. Quench recognizes these patterns and treats them as valid test correlation:

**Rust:**
```rust
#[test]
#[ignore = "TODO: implement parser"]
fn test_parser() { todo!() }
```

**JavaScript/TypeScript:**
```javascript
test.todo('parser should handle edge case');
it.todo('validates input correctly');
describe.todo('edge cases');
test.skip('temporarily disabled', () => { /* ... */ });
it.skip('needs implementation', () => { /* ... */ });
```

When placeholder tests exist for a source file, correlation is satisfied even without implementation—the test intent is recorded.

```toml
[check.tests]
# Recognize placeholder patterns as valid correlation
placeholders = "allow"  # default: true
```

## Output

### Pass (silent)

No output when correlation is satisfied.

### Fail (require mode)

```
tests: FAIL
  src/parser.rs: modified, no corresponding test changes
    Add tests in tests/parser_tests.rs or update inline #[cfg(test)] block
  src/lexer.rs: new file, no test file found
    Create tests/lexer_tests.rs with tests for the lexer module
```

### Advisory (warn mode)

```
tests: WARN
  src/parser.rs: changes without test updates
    Consider adding tests for the modified functionality.
```

### JSON Output

```json
{
  "name": "tests",
  "passed": false,
  "violations": [
    {
      "file": "src/parser.rs",
      "line": null,
      "type": "missing_tests",
      "change_type": "modified",
      "lines_changed": 79,
      "advice": "Add tests in tests/parser_tests.rs or update inline #[cfg(test)] block"
    },
    {
      "file": "src/lexer.rs",
      "line": null,
      "type": "missing_tests",
      "change_type": "added",
      "lines_changed": 234,
      "advice": "Create tests/lexer_tests.rs with tests for the lexer module."
    }
  ],
  "metrics": {
    "source_files_changed": 5,
    "with_test_changes": 3,
    "without_test_changes": 2,
    "scope": "branch"
  }
}
```

**Violation types**: `missing_tests`

## Configuration

```toml
[check.tests]
check = "error"

# Commit checking (source changes need test changes)
[check.tests.commit]
check = "error"                # error | warn | off
# types = ["feat", "feature", "story", "breaking"]   # default; only these commits require tests

# Scope: branch | commit
# branch = all changes on branch count together (order doesn't matter)
# commit = per-commit checking with asymmetric rules (tests-first OK)
scope = "branch"

# Placeholder tests
placeholders = "allow"      # #[ignore], test.todo(), test.fixme() count

# Test file patterns (extend defaults)
test_patterns = [
  "tests/**/*",          # Everything in tests/ directory
  "test/**/*",           # Everything in test/ directory
  "**/*_test.rs",        # Suffix pattern (outside test dirs)
  "**/*_tests.rs",
  "**/*.spec.rs",
  "**/*.spec.ts",
]

# Source patterns to check
source_patterns = ["src/**/*.rs"]

# Exclude patterns (never require tests)
exclude = [
  "**/mod.rs",           # Module declarations
  "**/lib.rs",           # Library roots
  "**/main.rs",          # Binary entry points
  "**/generated/**",     # Generated code
]

# Test suites (time thresholds per-suite)
[[check.tests.suite]]
runner = "cargo"
max_total = "30s"
max_test = "1s"

[[check.tests.suite]]
runner = "bats"
path = "tests/cli/"
setup = "cargo build"
targets = ["myapp"]
max_total = "10s"
max_test = "500ms"

[[check.tests.suite]]
runner = "pytest"
path = "tests/integration/"
ci = true                              # only run in CI mode (slow)
targets = ["myserver"]
max_total = "60s"

# Coverage settings
[check.tests.coverage]
check = "error"
min = 75

[check.tests.coverage.package.core]
min = 90

# Test time check level (thresholds are per-suite)
[check.tests.time]
check = "warn"
```

## CI Mode: Test Execution

In CI mode (`--ci`), the tests check also **runs test suites** and collects metrics:

- **Test time**: Total, average, and slowest test
- **Coverage**: Line coverage percentage

Test suites are configured via `[[check.tests.suite]]`. See [11-test-runners.md](../11-test-runners.md) for runner details.

```toml
[[check.tests.suite]]
runner = "cargo"
# Implicit: targets Rust code via llvm-cov

[[check.tests.suite]]
runner = "bats"
path = "tests/cli/"
setup = "cargo build"
targets = ["myapp"]                     # Instrument Rust binary
```

### Coverage

Coverage is collected based on what each suite exercises. Runners that test their own language provide implicit coverage:

| Runner | Implicit Coverage | Tool |
|--------|-------------------|------|
| `cargo` | Rust | llvm-cov |
| `go` | Go | built-in |
| `pytest` | Python | coverage.py |
| `jest`/`vitest`/`bun` | JS/TS | built-in |

For integration tests of compiled binaries or shell scripts, use the `targets` field:

```toml
[[check.tests.suite]]
runner = "bats"
path = "tests/cli/"
targets = ["myapp", "scripts/*.sh"]     # Rust binary + shell scripts via kcov
```

Output:

```
tests: coverage 78.4%
  rust: 82.3% (cargo + bats)
  python: 71.2% (pytest)
```

Configure thresholds via `[check.tests.coverage]`:

```toml
[check.tests.coverage]
check = "error"
min = 75

[check.tests.coverage.package.core]
min = 90
```

### Test Time

```
tests: time
  total: 12.4s
  avg: 45ms
  max: 2.1s (tests::integration::large_file_parse)
```

Time thresholds are configured per-suite:

```toml
[[check.tests.suite]]
runner = "cargo"
max_total = "30s"
max_test = "1s"

[[check.tests.suite]]
runner = "bats"
path = "tests/cli/"
ci = true                              # Only run in CI mode
max_total = "60s"
```

Configure check level via `[check.tests.time]`:

```toml
[check.tests.time]
check = "warn"                         # error | warn | off
```

### Ratcheting

Coverage and test time can be ratcheted to prevent regressions:

```toml
[ratchet]
coverage = true          # Coverage can't drop
test_time_max = false    # Don't ratchet slowest test (too noisy)
```
