# Tests Configuration Guide

Configuration reference for the `tests` check.

## Commit Checking

Scope of checking:

- `"branch"` — all changes on branch together (default)
- `"commit"` — per-commit with asymmetric rules (tests-first OK)

Placeholder tests (`#[ignore]`, `test.todo()`) can be allowed or forbidden.

```toml
[check.tests.commit]
check = "error"
scope = "branch"
placeholders = "allow"
```

## Commit Types

Only these commit types require test changes (default shown).

```toml
[check.tests.commit]
check = "error"
types = ["feat", "feature", "story", "breaking"]
```

## Exclude from Commit Checking

Never require tests for these files.

```toml
[check.tests.commit]
check = "error"
scope = "branch"
exclude = ["**/mod.rs", "**/main.rs", "**/generated/**"]
```

## Custom Test Patterns

Patterns to identify test and source files.

```toml
[check.tests]
check = "error"
test_patterns = [
  "tests/**/*",
  "test/**/*",
  "**/*_test.rs",
  "**/*_tests.rs",
  "**/*.spec.ts",
]
source_patterns = ["src/**/*.rs"]
```

## Single Test Suite

```toml
[[check.tests.suite]]
runner = "cargo"
max_total = "30s"
max_test = "1s"
```

## Multiple Test Suites

Unit tests:

```toml
[[check.tests.suite]]
runner = "cargo"
max_total = "30s"
max_test = "1s"
```

CLI integration tests:

```toml
[[check.tests.suite]]
runner = "bats"
path = "tests/cli/"
setup = "cargo build"
targets = ["myapp"]
max_total = "10s"
max_test = "500ms"
```

## CI-Only Suites

Fast unit tests run in all modes:

```toml
[[check.tests.suite]]
runner = "cargo"
max_total = "30s"
```

Slow integration tests run only in `--ci` mode:

```toml
[[check.tests.suite]]
runner = "pytest"
path = "tests/integration/"
ci = true
targets = ["myserver"]
max_total = "60s"
```

## Shell Script Coverage

Instrument shell scripts via `kcov` by specifying targets.

```toml
[[check.tests.suite]]
runner = "bats"
path = "tests/"
targets = ["scripts/*.sh", "bin/*"]
```

## Coverage Thresholds

```toml
[check.tests.coverage]
check = "error"
min = 75
```

## Per-Package Coverage

```toml
[check.tests.coverage]
check = "error"
min = 75

[check.tests.coverage.package.core]
min = 90  # Stricter for core

[check.tests.coverage.package.cli]
min = 60                   # More lenient for CLI
exclude = ["src/main.rs"]  # Skip entry points
```

## Test Time Check

Controls how test time violations are handled:

- `"error"` — fail if thresholds exceeded
- `"warn"` — report but don't fail (default)
- `"off"` — don't check

```toml
[check.tests.time]
check = "warn"
```

## Complete Example

```toml
[check.tests]
check = "error"

[check.tests.commit]
check = "error"
types = ["feat", "feature", "story", "breaking"]
scope = "branch"
placeholders = "allow"
exclude = ["**/mod.rs", "**/main.rs"]

test_patterns = ["tests/**/*", "**/*_test.rs"]
source_patterns = ["src/**/*.rs"]

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
ci = true
targets = ["myserver"]
max_total = "60s"

[check.tests.coverage]
check = "error"
min = 75

[check.tests.coverage.package.core]
min = 90

[check.tests.time]
check = "warn"
```
