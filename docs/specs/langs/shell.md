# Shell Language Support

Shell-specific behavior for quench checks.

## Detection

Detected when `*.sh` files exist in project root, `bin/`, or `scripts/`.

## Profile Defaults

When using [`quench init --profile shell`](../01-cli.md#profile-selection-recommended), the following opinionated defaults are configured:

```toml
[shell]
source = ["**/*.sh", "**/*.bash", "bin/*", "scripts/*"]
tests = ["tests/**/*.bats", "test/**/*.bats", "*_test.sh"]

[shell.suppress]
check = "forbid"

[shell.suppress.test]
check = "allow"

[shell.policy]
lint_changes = "standalone"
lint_config = [".shellcheckrc"]

[[check.escapes.patterns]]
pattern = "set +e"
action = "comment"
comment = "# OK:"
advice = "Add a # OK: comment explaining why error checking is disabled."

[[check.escapes.patterns]]
pattern = "eval "
action = "comment"
comment = "# OK:"
advice = "Add a # OK: comment explaining why eval is safe here."

[[check.escapes.patterns]]
pattern = "# shellcheck disable="
action = "forbid"
in_tests = "allow"
advice = "Fix the shellcheck warning instead of disabling it."
```

**Landing the Plane items** (added to agent files when combined with `claude` or `cursor` profile):
- `shellcheck scripts/*.sh`
- `bats tests/` (if bats tests exist)

## Default Patterns

```toml
[shell]
source = ["**/*.sh", "**/*.bash"]
tests = ["tests/**/*.bats", "test/**/*.bats", "*_test.sh"]
```

## Test Code Detection

**Test files** (entire file is test code):
- `*.bats` files (BATS test framework)
- `*_test.sh` files
- Files in `tests/` or `test/` directories

No inline test code convention for shell.

## Default Escape Patterns

| Pattern | Action | Comment Required |
|---------|--------|------------------|
| `set +e` | comment | `# OK:` |
| `eval ` | comment | `# OK:` |

## Suppress

Controls `# shellcheck disable=` comments.

| Setting | Behavior |
|---------|----------|
| `"forbid"` | Never allowed (default) |
| `"comment"` | Requires justification comment |
| `"allow"` | Always allowed |

```toml
[shell.suppress]
check = "forbid"               # forbid | comment | allow
# comment = "# OK:"            # optional: require specific pattern (default: any)

[shell.suppress.source]
allow = ["SC2034"]             # unused variable OK

[shell.suppress.test]
check = "allow"                # tests can suppress freely
```

## Policy

```toml
[shell.policy]
lint_changes = "standalone"
lint_config = [".shellcheckrc"]
```

## Coverage

Shell coverage uses `kcov`. To enable coverage for shell scripts, specify them as targets in test suites:

```toml
[[check.tests.suite]]
runner = "bats"
path = "tests/"
targets = ["scripts/*.sh", "bin/*"]    # Shell scripts via kcov
```

Coverage targets resolve against `[shell].source` patterns.

## Configuration

```toml
[shell]
# Source/test patterns (defaults shown)
# source = ["**/*.sh", "**/*.bash"]
# tests = ["tests/**/*.bats", "test/**/*.bats", "*_test.sh"]

[shell.suppress]
check = "forbid"

[shell.suppress.test]
check = "allow"

[shell.policy]
lint_changes = "standalone"
lint_config = [".shellcheckrc"]
```

Test suites and coverage thresholds are configured in `[check.tests]`.
