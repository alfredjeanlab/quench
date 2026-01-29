# Python Configuration Guide

Configuration reference for Python language support.

## File Patterns

```toml
[python]
source = ["**/*.py"]
tests = ["tests/**/*.py", "test_*.py", "*_test.py", "conftest.py"]
ignore = [".venv/", "__pycache__/", ".mypy_cache/", ".pytest_cache/", "dist/", "build/", "*.egg-info/"]
```

## CLOC Advice

```toml
[python.cloc]
check = "error"
advice = "Custom advice for oversized Python files."
```

## Suppress Directives

Controls how `# noqa`, `# type: ignore`, and `# pylint: disable` comments are handled:

- `"forbid"` — never allowed
- `"comment"` — requires justification comment
- `"allow"` — always allowed (default)

```toml
[python.suppress]
check = "comment"

[python.suppress.test]
check = "allow"
```

## Suppress with Allowlist/Denylist

```toml
[python.suppress]
check = "comment"

[python.suppress.source]
allow = ["noqa: E501"]      # Line length OK without comment
forbid = ["type: ignore"]   # Never allow untyped ignore

[python.suppress.test]
check = "allow"
```

## Lint Config Policy

Require ruff/flake8/pylint config changes in standalone PRs.

```toml
[python.policy]
check = "error"
lint_changes = "standalone"
lint_config = ["pyproject.toml", "ruff.toml", ".flake8", ".pylintrc"]
```

## Escape Patterns

Python-specific escape hatches:

```toml
[[check.escapes.patterns]]
pattern = "breakpoint\\(\\)"
action = "forbid"
in_tests = "forbid"  # Forbidden even in tests (breaks CI)
advice = "Remove breakpoint() before committing."

[[check.escapes.patterns]]
pattern = "pdb\\.set_trace\\(\\)"
action = "forbid"
in_tests = "forbid"
advice = "Remove pdb.set_trace() before committing."

[[check.escapes.patterns]]
pattern = "eval\\("
action = "comment"
comment = "# EVAL:"
advice = "Add a # EVAL: comment explaining the use case."

[[check.escapes.patterns]]
pattern = "exec\\("
action = "comment"
comment = "# EXEC:"
advice = "Add a # EXEC: comment explaining the use case."

[[check.escapes.patterns]]
pattern = "__import__\\("
action = "comment"
comment = "# DYNAMIC:"
advice = "Add a # DYNAMIC: comment explaining why dynamic import is needed."
```

## Coverage

pytest provides built-in coverage via coverage.py:

```toml
[[check.tests.suite]]
runner = "pytest"
```

## Complete Example

```toml
[python]
source = ["**/*.py"]
tests = ["tests/**/*.py", "test_*.py", "*_test.py", "conftest.py"]
ignore = [".venv/", "__pycache__/", "dist/"]

[python.cloc]
check = "error"
advice = "Custom advice for Python files."

[python.suppress]
check = "comment"

[python.suppress.source]
allow = ["noqa: E501"]
forbid = ["type: ignore"]

[python.suppress.test]
check = "allow"

[python.policy]
check = "error"
lint_changes = "standalone"
lint_config = ["pyproject.toml", "ruff.toml", ".flake8"]

[[check.escapes.patterns]]
pattern = "breakpoint\\(\\)"
action = "forbid"
in_tests = "forbid"

[[check.escapes.patterns]]
pattern = "eval\\("
action = "comment"
comment = "# EVAL:"

[[check.tests.suite]]
runner = "pytest"
```
