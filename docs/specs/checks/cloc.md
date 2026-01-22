# CLOC (Code Lines of Code) Specification

The `cloc` check counts lines of code, separating source from test code.

## Purpose

- Track codebase size and growth
- Report source-to-test ratio for CI metrics
- Per-package breakdown for focused metrics

**Note**: This check is reporting-only. It does not fail based on ratio thresholds.
Ratio enforcement is left to CI configuration or future ratcheting features.

## Counting Rules

### What Counts as a Line

A line is counted if it contains at least one non-whitespace character.

**Counted**:
```rust
fn main() {           // counted
    println!("hi");   // counted
}                     // counted
```

**Not counted**:
```rust
                      // blank - not counted

fn main() {
                      // blank - not counted
}
```

### Comments

Comments are counted as lines (they're part of the code).

Future consideration: separate comment LOC metric.

## Source vs Test Separation

### Pattern-Based (Language Agnostic)

Default test patterns:
- `**/tests/**` - test directories
- `**/test/**` - test directories
- `**/*_test.*` - test file suffix
- `**/*_tests.*` - test file suffix
- `**/*.test.*` - test file infix
- `**/*.spec.*` - spec file infix
- `**/test_*.*` - test file prefix

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

When `check.rust.split_cfg_test = true` (default), lines inside `#[cfg(test)]`
blocks are counted as test LOC even in source files.

## Output

### Text Output

LOC metrics are not shown in text output (they're available in JSON for tooling).
Text output only appears when file size limits are exceeded:

```
cloc: FAIL
  src/parser.rs: 923 lines (max: 750)
    Split into smaller modules.

5 checks passed, 1 failed
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
      "advice": "Split into smaller modules."
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
# Max lines per file (default: 750 source, 1100 test)
max_lines = 750
max_lines_test = 1100

# Max tokens per file (default: 20000, use false to disable)
max_tokens = 20000
```

When limits are set, violations are reported:

```
cloc: FAIL
  src/parser.rs: 923 lines (max: 900)
    Split into smaller modules.
```

Average lines per file is **reported** in metrics but not enforced.

## Configuration

```toml
[check.cloc]
check = "error"

# Override default patterns
source_patterns = ["src/**/*.rs", "lib/**/*.rs"]
test_patterns = ["tests/**/*.rs", "**/*_tests.rs"]

# File size limits (defaults shown)
max_lines = 750
max_lines_test = 1100
max_tokens = 20000               # use false to disable

# Exclude from size limits
exclude = ["**/generated/**", "**/migrations/**"]

# Rust-specific: parse #[cfg(test)] blocks
# check.rust.split_cfg_test = true  # default
```

**Note**: Ratio is reporting-only. File size limits are enforced if configured.

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
