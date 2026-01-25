# Shell Language Support

Shell-specific behavior for quench checks.

## Detection

Detected when `*.sh` files exist in project root, `bin/`, or `scripts/`.

## Profile Defaults

When using [`quench init --with shell`](../01-cli.md#explicit-profiles),
the following opinionated defaults are configured:

```toml
[shell]
source = ["**/*.sh", "**/*.bash", "bin/*", "scripts/*"]
tests = ["**/tests/**/*.bats", "**/test/**/*.bats", "**/*_test.sh"]

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
tests = ["**/tests/**/*.bats", "**/test/**/*.bats", "**/*_test.sh"]
```

When `[shell].tests` is not configured, patterns fall back to `[project].tests`, then to these defaults. See [Pattern Resolution](../02-config.md#pattern-resolution).

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

# Per-lint patterns (optional)
[shell.suppress.source.SC2086]
comment = "# INTENTIONAL:"     # require specific pattern for word splitting
```

### Violation Messages

When a shellcheck suppression is missing a required comment (when `check = "comment"`), the error message provides:
1. A general statement that justification is required
2. Lint-specific guidance tailored to common ShellCheck codes
3. The list of acceptable comment patterns (when configured)

**Example (no specific pattern):**

```
scripts/deploy.sh:23: shellcheck_missing_comment: # shellcheck disable=SC2086
  Lint suppression requires justification.
  Is unquoted expansion intentional here?
  Add a comment above the directive.
```

**Example (with configured patterns):**

See the configuration example above showing `[shell.suppress.source.SC2086]` with `comment = "# INTENTIONAL:"`. When patterns are configured, the violation message will list them.

**Default per-lint guidance** (for common ShellCheck codes):

| Code | Guidance Question |
|------|-------------------|
| SC2086 | Is unquoted expansion intentional here? |
| SC2154 | Is this variable defined externally? |
| SC2034 | Is this unused variable needed? |
| SC2155 | Should declaration and assignment be split? |

Other codes use: "Is this ShellCheck finding a false positive?"

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
# Source/test patterns (defaults shown; falls back to [project].tests if not set)
# source = ["**/*.sh", "**/*.bash"]
# tests = ["**/tests/**/*.bats", "**/test/**/*.bats", "**/*_test.sh"]

[shell.cloc]
check = "error"                  # error | warn | off
# advice = "..."                 # Custom advice for oversized shell scripts

[shell.suppress]
check = "forbid"

[shell.suppress.test]
check = "allow"

[shell.policy]
lint_changes = "standalone"
lint_config = [".shellcheckrc"]
```

Test suites and coverage thresholds are configured in `[check.tests]`.
