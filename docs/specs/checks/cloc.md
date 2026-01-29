# CLOC (Code Lines of Code) Specification

The `cloc` check counts lines of code, separating source from test code.

## Purpose

- Track codebase size and growth
- Report source-to-test ratio for CI metrics
- Per-package breakdown for focused metrics

**Note**: This check is reporting-only. It does not fail based on ratio thresholds.
Ratio enforcement is left to CI configuration or future ratcheting features.

## Counting Rules

### Metrics Output

The `source_lines` and `test_lines` metrics count **non-blank lines** (lines with at least one non-whitespace character). This provides a meaningful measure of code size.

```json
{"metrics": {"source_lines": 786, "test_lines": 142, "ratio": 5.53}}
```

### File Size Limits

File size violations report both total and non-blank line counts:

```
file.rs: file_too_large (lines: 899 vs 750)
```

JSON violations include both for convenience:
```json
{"file": "file.rs", "type": "file_too_large", "value": 899, "threshold": 750, "lines": 899, "nonblank": 786}
```

### Metric Configuration

Configure which metric to check against `max_lines` via `metric` (default: `lines`):

```toml
[check.cloc]
metric = "lines"     # Total lines (default, matches wc -l)
# metric = "nonblank"  # Non-blank lines only
```

Using `lines` (total) makes violations easy to verify with `wc -l`.

### Comments

Comments are counted in both metrics (they're part of the code).

## Source vs Test Separation

### Pattern Resolution Hierarchy

Test patterns are resolved per-language using this hierarchy:

1. **`[<language>].tests`** - Language-specific override (most specific)
2. **`[project].tests`** - Project-wide patterns
3. **Adapter defaults** - Built-in convention (zero-config)

See [Pattern Resolution](../02-config.md#pattern-resolution) for details and examples.

### Default Test Patterns

Each language adapter has built-in defaults. See individual language specs:
- [Rust](../langs/rust.md#default-patterns) - `**/tests/**`, `*_test.rs`, etc.
- [Shell](../langs/shell.md#default-patterns) - `**/tests/**/*.bats`, `*_test.sh`
- [Go](../langs/golang.md#default-patterns) - `**/*_test.go`
- [JavaScript](../langs/javascript.md#default-patterns) - `**/*.test.*`, `**/*.spec.*`, etc.

Files matching any test pattern are counted as test code.
All other files matching source patterns are counted as source code.

### Language-Specific (Rust)

Rust adapter can parse `#[cfg(test)]` blocks:

```rust
// Source file with inline tests
pub fn add(a: i32, b: i32) -> i32 {  // source LOC: 3
    a + b
}

#[cfg(test)]                          // test LOC: 6
mod tests {
    use super::*;

    #[test]
    fn test_add() {
        assert_eq!(add(1, 2), 3);
    }
}
```

When `rust.cfg_test_split = "count"` (default), lines inside `#[cfg(test)]`
blocks are counted as test LOC even in source files. See [langs/rust.md#cfg-test-split-modes](../langs/rust.md#cfg-test-split-modes) for other modes.

## Output

### Text Output

LOC metrics are not shown in text output (they're available in JSON for tooling).
Text output only appears when file size limits are exceeded:

```
cloc: FAIL
  src/parser.rs: 923 lines (max: 750)
    First, look for repetitive patterns that could be extracted into helper
    functions, or refactor to be more unit testable and concise.

    Then split into sibling modules or submodules in a folder by semantic concern
    (target 150–250 lines each).

    Avoid removing individual lines to satisfy the linter;
    prefer extracting testable code blocks.

PASS: escapes, agents, docs, tests
FAIL: cloc
```

When all files are within limits, cloc passes silently and contributes to the summary.

### Ratio Direction

Ratio is **test LOC / source LOC**:
- `0.72x` = test code is 72% the size of source code
- `1.0x` = equal amounts of test and source
- `2.0x` = twice as much test code as source

Typical healthy ranges: `0.5x` to `2.0x` (project-dependent).

### JSON Output

```json
{
  "name": "cloc",
  "passed": false,
  "violations": [
    {
      "file": "src/parser.rs",
      "line": null,
      "type": "file_too_large",
      "value": 923,
      "threshold": 750,
      "advice": "First, look for repetitive patterns that could be extracted into helper functions, or refactor to be more unit testable and concise.\n\nThen split into sibling modules or submodules in a folder by semantic concern (target 150–250 lines each).\n\nAvoid removing individual lines to satisfy the linter; prefer extracting testable code blocks."
    }
  ],
  "metrics": {
    "source_lines": 12453,
    "source_files": 47,
    "test_lines": 8921,
    "test_files": 32,
    "ratio": 0.72
  },
  "by_package": {
    "cli": {
      "source_lines": 3421,
      "source_files": 15,
      "test_lines": 2890,
      "test_files": 12,
      "ratio": 0.84
    },
    "core": {
      "source_lines": 9032,
      "source_files": 32,
      "test_lines": 6031,
      "test_files": 20,
      "ratio": 0.67
    }
  }
}
```

**Notes**:
- `violations` only present when file size limits exceeded
- `by_package` omitted if no packages configured
- `metrics` always present (LOC is reporting-only)

## File Size Limits

Per-file limits (enabled by default):

```toml
[check.cloc]
# Max lines per file (default: 750 source, 1000 test)
max_lines = 750
max_lines_test = 1000

# Max tokens per file (default: 20000, use false to disable)
max_tokens = 20000
```

When limits are set, violations are reported:

```
cloc: FAIL
  src/parser.rs: 923 lines (max: 900)
    First, look for repetitive patterns that could be extracted into helper
    functions, or refactor to be more unit testable and concise.

    Then split into sibling modules or submodules in a folder by semantic concern
    (target 180–300 lines each).

    Avoid removing individual lines to satisfy the linter;
    prefer extracting testable code blocks.
```

Average lines per file is **reported** in metrics but not enforced.

## Configuration

```toml
[check.cloc]
check = "error"

# File size limits (defaults shown)
max_lines = 750
max_lines_test = 1000
max_tokens = 20000               # use false to disable

# Exclude from size limits
exclude = ["**/generated/**", "**/migrations/**"]

# Custom advice for violations (overrides computed defaults)
# advice = "Your custom advice here..."
# advice_test = "Your custom test advice here..."

# Per-language advice (overrides generic advice for that language)
# [rust]
# cloc_advice = "Custom advice for Rust source files..."
# [golang]
# cloc_advice = "Custom advice for Go source files..."

# Rust-specific: parse #[cfg(test)] blocks
# rust.cfg_test_split = "count"  # count | require | off (default: "count")
```

**Note**: Source and test patterns are configured in `[project]` or language-specific sections like `[shell]` and `[rust]`. See [Pattern Resolution](../02-config.md#pattern-resolution).

File size limits are enforced if configured. Ratio is reporting-only.

## Performance

LOC counting must be fast (target: <100ms for 50k LOC).

Implementation:
1. Parallel file walking with `ignore` crate
2. Memory-mapped file reading
3. Simple newline counting (no parsing)
4. Early termination not applicable (must count all)

For Rust `#[cfg(test)]` parsing:
- Use tree-sitter or simple regex matching
- Cache parse results if same file needed by multiple checks
