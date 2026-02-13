# CLOC Configuration Guide

Configuration reference for the `cloc` check.

## File Size Limits

```toml
[check.cloc]
check = "error"
max_lines = 800        # Source files
max_lines_test = 1000  # Test files
max_tokens = 20000     # Or false to disable
```

## Metric Selection

Controls which metric is checked against `max_lines`:

- `"lines"` — total lines, matches `wc -l` (default)
- `"nonblank"` — non-blank lines only

```toml
[check.cloc]
check = "error"
metric = "lines"
max_lines = 800
```

## Custom Advice

Override the advice shown when source or test files exceed their limits.

```toml
[check.cloc]
check = "error"
max_lines = 800
advice = """
Can the code be made more concise?

If not, split large source files into sibling modules or submodules in a folder;
consider refactoring to be more unit testable.

Avoid picking and removing individual lines to satisfy the linter,
prefer properly refactoring out testable code blocks."""

advice_test = "Can tests be parameterized or use shared fixtures to be more concise? If not, split large test files into a folder."
```

## Exclude Patterns

Skip files matching these patterns from size checks.

```toml
[check.cloc]
check = "error"
max_lines = 800
exclude = ["**/generated/**", "**/migrations/**", "**/vendor/**"]
```

## Per-Package Limits

```toml
[check.cloc]
check = "error"
max_lines = 800  # Default for all packages

[check.cloc.package.cli]
max_lines = 500  # Stricter for CLI package

[check.cloc.package.core]
max_lines = 800  # More lenient for core

[check.cloc.package.generated]
check = "off"    # Skip entirely
```

## Disable Token Limits

```toml
[check.cloc]
check = "error"
max_lines = 800
max_tokens = false  # No token limit
```

## Complete Example

```toml
[check.cloc]
check = "error"
metric = "lines"
max_lines = 800
max_lines_test = 1000
max_tokens = 20000
exclude = ["**/generated/**", "**/migrations/**"]

advice = "Can the code be made more concise? If not, split into modules."
advice_test = "Can tests be parameterized? If not, split into multiple files."

[check.cloc.package.cli]
max_lines = 500
advice = "CLI code should be especially concise."

[check.cloc.package.generated]
check = "off"
```
