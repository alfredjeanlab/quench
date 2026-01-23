# Phase 215: Escapes Check - Actions

**Root Feature:** `quench-6c9b`

## Overview

Implement action logic for the `escapes` check. Phase 210 established pattern matching infrastructure; this phase adds:
- **Count action**: Track occurrences, fail if threshold exceeded
- **Comment action**: Allow pattern if preceded by justification comment
- **Forbid action**: Always report violation in source code
- **Source/test separation**: Test code is counted but never fails

## Project Structure

```
quench/
├── crates/cli/src/
│   ├── checks/
│   │   ├── escapes.rs       # Action logic, comment detection
│   │   └── escapes_tests.rs # Unit tests for comment search
│   └── pattern/
│       ├── matcher.rs       # Add line extraction utilities
│       └── matcher_tests.rs # Tests for line utilities
├── tests/
│   ├── specs/
│   │   └── checks/escapes.rs # Behavioral specs
│   └── fixtures/
│       └── escapes/         # Test fixtures
└── plans/
    └── phase-215.md
```

## Dependencies

No new dependencies. Uses existing:
- `globset` for test pattern matching (from cloc check)
- `crates/cli/src/adapter` for file classification

## Implementation Phases

### Phase 1: Comment Detection Infrastructure

Add utilities to extract line content and search for comments.

**Update `crates/cli/src/pattern/matcher.rs`:**

```rust
/// Get the full line containing a byte offset.
pub fn get_line_at_offset(content: &str, offset: usize) -> &str {
    let start = content[..offset].rfind('\n').map(|i| i + 1).unwrap_or(0);
    let end = content[offset..].find('\n').map(|i| offset + i).unwrap_or(content.len());
    &content[start..end]
}

/// Get the line number and content for all lines in content.
/// Returns iterator of (1-based line number, line content).
pub fn lines_with_numbers(content: &str) -> impl Iterator<Item = (u32, &str)> {
    content.lines().enumerate().map(|(i, line)| (i as u32 + 1, line))
}
```

**Update `LineMatch` to include line content:**

```rust
#[derive(Debug, Clone)]
pub struct LineMatch {
    pub line: u32,
    pub text: String,
    pub offset: usize,
    /// The full line containing the match.
    pub line_content: String,
}
```

**Milestone:** Line content is available in matches.

**Verification:**
```bash
cargo test pattern::matcher -- line_content
```

---

### Phase 2: Upward Comment Search Algorithm

Implement comment detection per spec: search same line, then preceding lines.

**Add to `crates/cli/src/checks/escapes.rs`:**

```rust
/// Search upward from a line for a required comment pattern.
///
/// Searches:
/// 1. Same line as the match
/// 2. Preceding lines, stopping at non-blank/non-comment lines
///
/// Returns true if comment pattern is found.
fn has_justification_comment(
    content: &str,
    match_line: u32,
    comment_pattern: &str,
) -> bool {
    let lines: Vec<&str> = content.lines().collect();
    let line_idx = (match_line - 1) as usize;

    // Check same line first
    if line_idx < lines.len() && lines[line_idx].contains(comment_pattern) {
        return true;
    }

    // Search upward through comments and blank lines
    if line_idx > 0 {
        for i in (0..line_idx).rev() {
            let line = lines[i].trim();

            // Check for comment pattern
            if line.contains(comment_pattern) {
                return true;
            }

            // Stop at non-blank, non-comment lines
            if !line.is_empty() && !is_comment_line(line) {
                break;
            }
        }
    }

    false
}

/// Check if a line is a comment line (language-agnostic heuristics).
fn is_comment_line(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.starts_with("//")      // C-style
        || trimmed.starts_with("#")   // Shell/Python/Ruby
        || trimmed.starts_with("/*")  // C block comment start
        || trimmed.starts_with("*")   // C block comment continuation
        || trimmed.starts_with("--")  // SQL/Lua
        || trimmed.starts_with(";;")  // Lisp
}
```

**Milestone:** Comment search works for same-line and preceding lines.

**Verification:**
```bash
cargo test checks::escapes -- has_justification
```

---

### Phase 3: Source/Test File Classification

Integrate with adapter system for source/test separation.

**Update `crates/cli/src/checks/escapes.rs`:**

```rust
use crate::adapter::{GenericAdapter, FileKind};

/// Classify file as source or test.
fn classify_file(
    path: &Path,
    root: &Path,
    test_patterns: &[String],
) -> FileKind {
    let adapter = GenericAdapter::new(&[], test_patterns);
    let relative = path.strip_prefix(root).unwrap_or(path);
    adapter.classify(relative)
}
```

**Update run() to skip test file violations:**

```rust
fn run(&self, ctx: &CheckContext) -> CheckResult {
    // ... existing setup ...

    // Use project test patterns or defaults
    let test_patterns = if ctx.config.project.tests.is_empty() {
        &default_test_patterns()
    } else {
        &ctx.config.project.tests
    };

    for file in ctx.files {
        let is_test = classify_file(&file.path, ctx.root, test_patterns) == FileKind::Test;

        // ... find matches ...

        for m in matches {
            // Test code: count but don't report violations
            if is_test {
                // Metrics only (Phase 220)
                continue;
            }

            // Source code: apply action logic
            // ...
        }
    }
}
```

**Milestone:** Test files are identified and skipped for violations.

**Verification:**
```bash
cargo test --test specs escapes_allows_test_code
```

---

### Phase 4: Count Action Implementation

Implement count action with threshold checking.

**Update `crates/cli/src/checks/escapes.rs`:**

```rust
use std::collections::HashMap;

/// Track counts per pattern.
struct PatternCounts {
    counts: HashMap<String, usize>,
}

impl PatternCounts {
    fn new() -> Self {
        Self { counts: HashMap::new() }
    }

    fn increment(&mut self, pattern_name: &str) -> usize {
        let count = self.counts.entry(pattern_name.to_string()).or_insert(0);
        *count += 1;
        *count
    }

    fn get(&self, pattern_name: &str) -> usize {
        self.counts.get(pattern_name).copied().unwrap_or(0)
    }
}
```

**Update match handling in run():**

```rust
// In the match loop:
match pattern.action {
    EscapeAction::Count => {
        let count = source_counts.increment(&pattern.name);
        // Only report violation after scanning all files (threshold check)
    }
    // ...
}

// After file loop, check thresholds:
for pattern in &patterns {
    if pattern.action == EscapeAction::Count {
        let count = source_counts.get(&pattern.name);
        if count > pattern.threshold {
            violations.push(create_threshold_violation(
                &pattern.name,
                count,
                pattern.threshold,
                &pattern.advice,
            ));
        }
    }
}
```

**Milestone:** Count action tracks occurrences and checks threshold.

**Verification:**
```bash
cargo test --test specs escapes_count_threshold
```

---

### Phase 5: Comment Action Implementation

Implement comment action with justification search.

**Update match handling:**

```rust
match pattern.action {
    EscapeAction::Comment => {
        let comment_pattern = pattern.comment.as_deref()
            .unwrap_or("// JUSTIFIED:");

        if !has_justification_comment(&content, m.line, comment_pattern) {
            if let Some(v) = try_create_violation(
                ctx,
                relative,
                m.line,
                "missing_comment",
                &format_comment_advice(&pattern.advice, comment_pattern),
            ) {
                violations.push(v);
            }
        }
    }
    // ...
}

fn format_comment_advice(custom_advice: &Option<String>, comment_pattern: &str) -> String {
    custom_advice.clone().unwrap_or_else(|| {
        format!("Add a {} comment explaining why this is necessary.", comment_pattern)
    })
}
```

**Milestone:** Comment action checks for justification comments.

**Verification:**
```bash
cargo test --test specs escapes_comment_same_line
cargo test --test specs escapes_comment_preceding_line
cargo test --test specs escapes_comment_through_blank_lines
```

---

### Phase 6: Forbid Action and Unit Tests

Finalize forbid action (mostly existing) and add comprehensive unit tests.

**Forbid action (already implemented, add test awareness):**

```rust
match pattern.action {
    EscapeAction::Forbid => {
        // Test code is allowed - handled by outer is_test check
        if let Some(v) = try_create_violation(
            ctx,
            relative,
            m.line,
            "forbidden",
            &pattern.advice,
        ) {
            violations.push(v);
        }
    }
    // ...
}
```

**Add unit tests in `crates/cli/src/checks/escapes_tests.rs`:**

```rust
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;

mod comment_detection {
    use super::*;

    #[test]
    fn finds_comment_on_same_line() {
        let content = "unsafe { code } // SAFETY: reason";
        assert!(has_justification_comment(content, 1, "// SAFETY:"));
    }

    #[test]
    fn finds_comment_on_preceding_line() {
        let content = "// SAFETY: reason\nunsafe { code }";
        assert!(has_justification_comment(content, 2, "// SAFETY:"));
    }

    #[test]
    fn finds_comment_through_blank_lines() {
        let content = "// SAFETY: reason\n\nunsafe { code }";
        assert!(has_justification_comment(content, 3, "// SAFETY:"));
    }

    #[test]
    fn finds_comment_through_other_comments() {
        let content = "// SAFETY: reason\n// more context\nunsafe { code }";
        assert!(has_justification_comment(content, 3, "// SAFETY:"));
    }

    #[test]
    fn stops_at_code_line() {
        let content = "// SAFETY: old\nfn other() {}\nunsafe { code }";
        assert!(!has_justification_comment(content, 3, "// SAFETY:"));
    }

    #[test]
    fn no_comment_returns_false() {
        let content = "unsafe { code }";
        assert!(!has_justification_comment(content, 1, "// SAFETY:"));
    }
}

mod is_comment_line {
    use super::*;

    #[test]
    fn c_style_single() {
        assert!(is_comment_line("// comment"));
        assert!(is_comment_line("  // indented"));
    }

    #[test]
    fn c_style_block() {
        assert!(is_comment_line("/* block */"));
        assert!(is_comment_line(" * continuation"));
    }

    #[test]
    fn shell_style() {
        assert!(is_comment_line("# comment"));
        assert!(is_comment_line("  # indented"));
    }

    #[test]
    fn code_is_not_comment() {
        assert!(!is_comment_line("fn main() {}"));
        assert!(!is_comment_line("let x = 1;"));
    }
}
```

**Milestone:** All action types work correctly, unit tests pass.

**Verification:**
```bash
cargo test checks::escapes
cargo test --test specs escapes
make check
```

---

## Key Implementation Details

### Comment Search Algorithm

The spec requires searching upward from the pattern match:

```
Line N-2: // SAFETY: The pointer is valid
Line N-1: // Additional context
Line N:   unsafe { *ptr = value }  <- match found here
```

Search order:
1. Check line N for comment pattern (same line)
2. Check line N-1, N-2, etc. while lines are blank or comments
3. Stop when hitting a non-blank, non-comment line

This allows comments to be separated from the pattern by blank lines or additional comment lines.

### is_comment_line Heuristics

Language-agnostic comment detection uses common prefixes:
- `//` - C, C++, Rust, Java, Go, JavaScript
- `#` - Shell, Python, Ruby, Perl
- `/*`, `*` - C block comments
- `--` - SQL, Lua
- `;;` - Lisp

This covers most common languages. False positives (e.g., `#include`) don't harm correctness - they just extend the search window.

### Source vs Test Separation

Test files are identified using the same patterns as the `cloc` check:
- `**/tests/**` - test directories
- `**/*_test.*`, `**/*_tests.*` - test file suffixes
- `**/*.test.*`, `**/*.spec.*` - test/spec file patterns

Test code is counted for metrics (Phase 220) but never generates violations.

### Violation Types

| Action | Violation Type | When |
|--------|---------------|------|
| `forbid` | `forbidden` | Any match in source code |
| `comment` | `missing_comment` | Match without justification comment |
| `count` | `threshold_exceeded` | Count exceeds configured threshold |

### Default Threshold

The default threshold for `count` action is 0, meaning any occurrence will fail by default. This makes the behavior explicit: you must configure a threshold if you want to allow any occurrences.

## Verification Plan

### After Each Phase

```bash
# Compile check
cargo build

# Run relevant unit tests
cargo test <module>

# Check lints
cargo clippy --all-targets --all-features -- -D warnings
```

### End-to-End Verification

```bash
# Run Phase 215 specs (remove #[ignore])
cargo test --test specs escapes_forbid_reports_violation
cargo test --test specs escapes_comment_allows_with_justification
cargo test --test specs escapes_count_fails_threshold

# Full quality gates
make check
```

### Test Matrix

| Test Case | Action | Expected |
|-----------|--------|----------|
| Forbid in source | forbid | FAIL |
| Forbid in test | forbid | PASS |
| Comment with justification | comment | PASS |
| Comment without justification | comment | FAIL |
| Count under threshold | count | PASS |
| Count over threshold | count | FAIL |
| Count exactly at threshold | count | PASS |

## Summary

| Phase | Task | Key Files | Status |
|-------|------|-----------|--------|
| 1 | Comment detection infrastructure | `pattern/matcher.rs` | [ ] Pending |
| 2 | Upward comment search algorithm | `checks/escapes.rs` | [ ] Pending |
| 3 | Source/test file classification | `checks/escapes.rs` | [ ] Pending |
| 4 | Count action implementation | `checks/escapes.rs` | [ ] Pending |
| 5 | Comment action implementation | `checks/escapes.rs` | [ ] Pending |
| 6 | Forbid action and unit tests | `checks/escapes_tests.rs` | [ ] Pending |

## Future Phases

- **Phase 220**: Implement metrics (source/test counts, JSON metrics output, per-package breakdown)
