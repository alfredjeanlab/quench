# Phase 210: Escapes Check - Pattern Matching

## Overview

Implement the core pattern matching infrastructure for the `escapes` check. This phase focuses on:
- Parsing `[[check.escapes.patterns]]` configuration
- Compiling patterns into optimized matchers (literal, multi-literal, regex)
- Finding matches in file contents with line number tracking
- Integration with the check infrastructure

This phase does NOT implement:
- Action logic (count/comment/forbid) - Phase 215
- Source/test metrics - Phase 220

## Project Structure

```
quench/
├── crates/cli/src/
│   ├── checks/
│   │   ├── mod.rs           # Register escapes check
│   │   ├── escapes.rs       # NEW: Escapes check implementation
│   │   └── escapes_tests.rs # NEW: Unit tests
│   ├── config.rs            # Add EscapesConfig parsing
│   └── pattern/             # NEW: Pattern matching module
│       ├── mod.rs           # Pattern types and API
│       ├── mod_tests.rs     # Unit tests
│       ├── matcher.rs       # CompiledPattern enum
│       └── matcher_tests.rs # Unit tests
├── tests/
│   ├── specs/
│   │   └── escapes_*.rs     # Behavioral specs (from Phase 205)
│   └── fixtures/
│       └── escapes/         # Test fixtures (from Phase 205)
└── plans/
    └── phase-210.md
```

## Dependencies

Add to `crates/cli/Cargo.toml`:

```toml
[dependencies]
memchr = "2.7"      # Single literal matching (SIMD-optimized)
aho-corasick = "1"  # Multi-literal matching
regex = "1"         # Complex patterns
```

These crates are battle-tested (all from ripgrep author) and provide the pattern matching hierarchy from `docs/specs/20-performance.md`.

## Implementation Phases

### Phase 1: Pattern Configuration Types

Add configuration parsing for escape patterns.

**Update `crates/cli/src/config.rs`:**

```rust
/// Escapes check configuration.
#[derive(Debug, Default, Deserialize)]
pub struct EscapesConfig {
    /// Check level: error, warn, or off.
    #[serde(default)]
    pub check: CheckLevel,

    /// Patterns to detect (overrides defaults).
    #[serde(default)]
    pub patterns: Vec<EscapePattern>,
}

/// A single escape hatch pattern definition.
#[derive(Debug, Clone, Deserialize)]
pub struct EscapePattern {
    /// Unique name for this pattern (e.g., "unwrap", "unsafe").
    pub name: String,

    /// Regex pattern to match.
    pub pattern: String,

    /// Action to take: count, comment, or forbid.
    #[serde(default)]
    pub action: EscapeAction,

    /// Required comment pattern for action = "comment".
    #[serde(default)]
    pub comment: Option<String>,

    /// Count threshold for action = "count" (default: 0).
    #[serde(default)]
    pub threshold: usize,

    /// Custom advice message for violations.
    #[serde(default)]
    pub advice: Option<String>,
}

/// Action to take when pattern is matched.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EscapeAction {
    #[default]
    Forbid,
    Comment,
    Count,
}
```

Add `EscapesConfig` to `CheckConfig` struct and wire up parsing in `parse_with_warnings`.

**Milestone:** Configuration parses without errors.

**Verification:**
```bash
cargo test config -- escapes
```

---

### Phase 2: Pattern Matcher Types

Create the pattern matching module with the hierarchical matcher enum.

**New file `crates/cli/src/pattern/mod.rs`:**

```rust
//! Pattern matching for escape hatch detection.
//!
//! Implements the pattern matching hierarchy from docs/specs/20-performance.md:
//! - Single literal: memchr::memmem
//! - Multiple literals: aho-corasick
//! - Complex regex: regex crate

pub mod matcher;

pub use matcher::CompiledPattern;
```

**New file `crates/cli/src/pattern/matcher.rs`:**

```rust
//! Compiled pattern matchers with automatic optimization.

use aho_corasick::AhoCorasick;
use memchr::memmem::Finder;
use regex::Regex;

/// A compiled pattern optimized for its structure.
pub enum CompiledPattern {
    /// Single literal string (fastest).
    Literal(LiteralMatcher),
    /// Multiple literal strings (Aho-Corasick).
    MultiLiteral(MultiLiteralMatcher),
    /// Full regex (most flexible).
    Regex(RegexMatcher),
}

pub struct LiteralMatcher {
    pattern: String,
    finder: Finder<'static>,
}

pub struct MultiLiteralMatcher {
    patterns: Vec<String>,
    automaton: AhoCorasick,
}

pub struct RegexMatcher {
    pattern: String,
    regex: Regex,
}

/// A match found in content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatternMatch {
    /// Byte offset where match starts.
    pub start: usize,
    /// Byte offset where match ends.
    pub end: usize,
}

impl CompiledPattern {
    /// Compile a pattern string into an optimized matcher.
    pub fn compile(pattern: &str) -> Result<Self, PatternError> {
        // Analysis logic to choose optimal matcher
        todo!()
    }

    /// Find all matches in content.
    pub fn find_all(&self, content: &str) -> Vec<PatternMatch> {
        todo!()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PatternError {
    #[error("invalid regex pattern: {0}")]
    InvalidRegex(#[from] regex::Error),
}
```

**Milestone:** Pattern types compile, methods are stubs.

**Verification:**
```bash
cargo build
```

---

### Phase 3: Pattern Analysis and Compilation

Implement pattern analysis to choose the optimal matcher.

**Pattern Classification Logic:**

```rust
impl CompiledPattern {
    pub fn compile(pattern: &str) -> Result<Self, PatternError> {
        if is_literal(pattern) {
            Ok(CompiledPattern::Literal(LiteralMatcher::new(pattern)))
        } else if let Some(literals) = extract_alternation_literals(pattern) {
            Ok(CompiledPattern::MultiLiteral(MultiLiteralMatcher::new(&literals)?))
        } else {
            Ok(CompiledPattern::Regex(RegexMatcher::new(pattern)?))
        }
    }
}

/// Check if pattern is a plain literal (no regex metacharacters).
fn is_literal(pattern: &str) -> bool {
    !pattern.chars().any(|c| matches!(c,
        '\\' | '.' | '*' | '+' | '?' | '(' | ')' |
        '[' | ']' | '{' | '}' | '^' | '$' | '|'
    ))
}

/// Extract literals from patterns like "foo|bar|baz".
fn extract_alternation_literals(pattern: &str) -> Option<Vec<String>> {
    // Pattern must be pure alternation: "lit1|lit2|lit3"
    // Each alternative must be a literal
    let parts: Vec<&str> = pattern.split('|').collect();
    if parts.len() < 2 {
        return None;
    }

    for part in &parts {
        if !is_literal(part) {
            return None;
        }
    }

    Some(parts.into_iter().map(String::from).collect())
}
```

**Individual Matcher Implementations:**

```rust
impl LiteralMatcher {
    pub fn new(pattern: &str) -> Self {
        // Leak to get 'static lifetime for Finder
        let pattern_owned = pattern.to_string();
        let pattern_static: &'static str = Box::leak(pattern_owned.clone().into_boxed_str());
        Self {
            pattern: pattern_owned,
            finder: Finder::new(pattern_static),
        }
    }

    pub fn find_all(&self, content: &str) -> Vec<PatternMatch> {
        self.finder
            .find_iter(content.as_bytes())
            .map(|pos| PatternMatch {
                start: pos,
                end: pos + self.pattern.len(),
            })
            .collect()
    }
}

impl MultiLiteralMatcher {
    pub fn new(patterns: &[String]) -> Result<Self, PatternError> {
        let automaton = AhoCorasick::new(patterns)
            .map_err(|e| PatternError::InvalidRegex(
                regex::Error::Syntax(e.to_string())
            ))?;
        Ok(Self {
            patterns: patterns.to_vec(),
            automaton,
        })
    }

    pub fn find_all(&self, content: &str) -> Vec<PatternMatch> {
        self.automaton
            .find_iter(content)
            .map(|m| PatternMatch {
                start: m.start(),
                end: m.end(),
            })
            .collect()
    }
}

impl RegexMatcher {
    pub fn new(pattern: &str) -> Result<Self, PatternError> {
        let regex = Regex::new(pattern)?;
        Ok(Self {
            pattern: pattern.to_string(),
            regex,
        })
    }

    pub fn find_all(&self, content: &str) -> Vec<PatternMatch> {
        self.regex
            .find_iter(content)
            .map(|m| PatternMatch {
                start: m.start(),
                end: m.end(),
            })
            .collect()
    }
}
```

**Milestone:** Pattern compilation works for all three matcher types.

**Verification:**
```bash
cargo test pattern::matcher
```

---

### Phase 4: Line Number Extraction

Add utility to convert byte offsets to line numbers.

**Add to `crates/cli/src/pattern/matcher.rs`:**

```rust
/// A match with resolved line number.
#[derive(Debug, Clone)]
pub struct LineMatch {
    /// 1-based line number.
    pub line: u32,
    /// The matched text.
    pub text: String,
    /// Byte offset in file.
    pub offset: usize,
}

/// Convert byte offset to 1-based line number.
pub fn byte_offset_to_line(content: &str, offset: usize) -> u32 {
    // Count newlines before offset
    content[..offset]
        .bytes()
        .filter(|&b| b == b'\n')
        .count() as u32
        + 1
}

impl CompiledPattern {
    /// Find all matches with line numbers.
    pub fn find_all_with_lines(&self, content: &str) -> Vec<LineMatch> {
        self.find_all(content)
            .into_iter()
            .map(|m| {
                let line = byte_offset_to_line(content, m.start);
                let text = content[m.start..m.end].to_string();
                LineMatch {
                    line,
                    text,
                    offset: m.start,
                }
            })
            .collect()
    }
}
```

**Milestone:** Line number extraction works correctly.

**Verification:**
```bash
cargo test pattern::matcher -- line
```

---

### Phase 5: Escapes Check Implementation

Implement the escapes check using the pattern infrastructure.

**New file `crates/cli/src/checks/escapes.rs`:**

```rust
//! Escapes (escape hatches) check.
//!
//! Detects patterns that bypass type safety or error handling.
//! See docs/specs/checks/escape-hatches.md.

use std::sync::atomic::Ordering;

use crate::check::{Check, CheckContext, CheckResult, Violation};
use crate::config::{CheckLevel, EscapeAction};
use crate::pattern::CompiledPattern;

/// Compiled escape pattern ready for matching.
struct CompiledEscapePattern {
    name: String,
    matcher: CompiledPattern,
    action: EscapeAction,
    advice: String,
}

/// The escapes check detects escape hatch patterns.
pub struct EscapesCheck;

impl Check for EscapesCheck {
    fn name(&self) -> &'static str {
        "escapes"
    }

    fn description(&self) -> &'static str {
        "Escape hatch detection"
    }

    fn run(&self, ctx: &CheckContext) -> CheckResult {
        let config = &ctx.config.check.escapes;

        if config.check == CheckLevel::Off {
            return CheckResult::passed(self.name());
        }

        // Compile patterns once
        let patterns = match compile_patterns(config) {
            Ok(p) => p,
            Err(e) => return CheckResult::skipped(self.name(), e.to_string()),
        };

        let mut violations = Vec::new();

        for file in ctx.files {
            // Read file content
            let content = match std::fs::read_to_string(&file.path) {
                Ok(c) => c,
                Err(_) => continue,
            };

            let relative = file.path.strip_prefix(ctx.root).unwrap_or(&file.path);

            // Find matches for each pattern
            for pattern in &patterns {
                let matches = pattern.matcher.find_all_with_lines(&content);

                for m in matches {
                    // For Phase 210, all matches become violations (action logic in Phase 215)
                    if let Some(v) = try_create_violation(ctx, relative, m.line, pattern) {
                        violations.push(v);
                    } else {
                        // Limit reached
                        return CheckResult::failed(self.name(), violations);
                    }
                }
            }
        }

        if violations.is_empty() {
            CheckResult::passed(self.name())
        } else {
            CheckResult::failed(self.name(), violations)
        }
    }

    fn default_enabled(&self) -> bool {
        true
    }
}

fn compile_patterns(config: &crate::config::EscapesConfig) -> Result<Vec<CompiledEscapePattern>, crate::pattern::PatternError> {
    config
        .patterns
        .iter()
        .map(|p| {
            let matcher = CompiledPattern::compile(&p.pattern)?;
            let advice = p.advice.clone().unwrap_or_else(|| default_advice(&p.action));
            Ok(CompiledEscapePattern {
                name: p.name.clone(),
                matcher,
                action: p.action,
                advice,
            })
        })
        .collect()
}

fn default_advice(action: &EscapeAction) -> String {
    match action {
        EscapeAction::Forbid => "Remove this escape hatch from production code.".to_string(),
        EscapeAction::Comment => "Add a justification comment.".to_string(),
        EscapeAction::Count => "Reduce escape hatch usage.".to_string(),
    }
}

fn try_create_violation(
    ctx: &CheckContext,
    path: &std::path::Path,
    line: u32,
    pattern: &CompiledEscapePattern,
) -> Option<Violation> {
    let current = ctx.violation_count.fetch_add(1, Ordering::SeqCst);
    if let Some(limit) = ctx.limit {
        if current >= limit {
            return None;
        }
    }

    Some(
        Violation::file(path, line, "forbidden", &pattern.advice)
            .with_pattern(&pattern.name),
    )
}

#[cfg(test)]
#[path = "escapes_tests.rs"]
mod tests;
```

**Update `crates/cli/src/checks/mod.rs`:**

```rust
pub mod escapes;  // Add this

// In all_checks():
Arc::new(escapes::EscapesCheck),  // Replace the stub
```

**Milestone:** Escapes check finds pattern matches and reports violations.

**Verification:**
```bash
cargo test --test specs escapes_detects_pattern
cargo test --test specs escapes_reports_line_number
```

---

### Phase 6: Unit Tests and Polish

Add comprehensive unit tests for pattern matching.

**New file `crates/cli/src/pattern/mod_tests.rs`:**

```rust
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;

#[test]
fn literal_pattern_matches_exact_string() {
    let p = CompiledPattern::compile("TODO").unwrap();
    let matches = p.find_all("line1\n// TODO: fix this\nline3");
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].start, 10);
}

#[test]
fn literal_pattern_no_match() {
    let p = CompiledPattern::compile("FIXME").unwrap();
    let matches = p.find_all("line1\n// TODO: fix this\nline3");
    assert!(matches.is_empty());
}

#[test]
fn alternation_uses_multi_literal() {
    let p = CompiledPattern::compile("TODO|FIXME|XXX").unwrap();
    assert!(matches!(p, CompiledPattern::MultiLiteral(_)));
}

#[test]
fn multi_literal_finds_all_variants() {
    let p = CompiledPattern::compile("TODO|FIXME").unwrap();
    let matches = p.find_all("TODO here\nFIXME there");
    assert_eq!(matches.len(), 2);
}

#[test]
fn regex_pattern_with_metacharacters() {
    let p = CompiledPattern::compile(r"\.unwrap\(\)").unwrap();
    assert!(matches!(p, CompiledPattern::Regex(_)));
    let matches = p.find_all("x.unwrap() and y.unwrap()");
    assert_eq!(matches.len(), 2);
}

#[test]
fn line_number_first_line() {
    let content = "match here";
    assert_eq!(byte_offset_to_line(content, 0), 1);
}

#[test]
fn line_number_second_line() {
    let content = "line1\nmatch here";
    assert_eq!(byte_offset_to_line(content, 6), 2);
}

#[test]
fn line_number_third_line() {
    let content = "line1\nline2\nmatch here";
    assert_eq!(byte_offset_to_line(content, 12), 3);
}

#[test]
fn find_with_lines_returns_correct_data() {
    let p = CompiledPattern::compile("unwrap").unwrap();
    let content = "line1\nx.unwrap()\nline3";
    let matches = p.find_all_with_lines(content);
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].line, 2);
    assert_eq!(matches[0].text, "unwrap");
}
```

**New file `crates/cli/src/checks/escapes_tests.rs`:**

```rust
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;

// Unit tests for escapes check internals
// Behavioral tests are in tests/specs/
```

**Milestone:** All unit tests pass.

**Verification:**
```bash
cargo test pattern
cargo test escapes
make check
```

---

## Key Implementation Details

### Pattern Matching Hierarchy

From `docs/specs/20-performance.md`, patterns are matched using the fastest applicable method:

| Pattern Type | Matcher | Example |
|--------------|---------|---------|
| Single literal | `memchr::memmem` | `"TODO"` |
| Multiple literals | `aho-corasick` | `"TODO\|FIXME\|XXX"` |
| Complex regex | `regex` | `unsafe\s*\{` |

The `is_literal` function checks for regex metacharacters. Pure alternation of literals (`foo|bar`) uses Aho-Corasick for efficient multi-pattern matching.

### Memory Considerations

- `memchr::memmem::Finder` requires `'static` lifetime, so we leak the pattern string. This is acceptable since patterns are compiled once at startup and live for the program duration.
- File contents are read per-file, not held in memory across files.
- Pattern compilation happens once, matchers are shared via `Arc` in the check context.

### Line Number Calculation

Byte offset to line number conversion counts newlines before the offset:
- Offset 0 -> Line 1
- First newline at offset N means offset N+1 is line 2

This is O(offset) per match, but escape hatches are rare enough that this is acceptable.

### Phase 215 Integration Points

This phase creates violations with `type: "forbidden"` for all matches. Phase 215 will:
1. Implement action-specific violation types (`missing_comment`, `threshold_exceeded`)
2. Add comment detection for `action = "comment"`
3. Add threshold checking for `action = "count"`
4. Add source/test classification

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
# Remove #[ignore] from Phase 210 specs and run
cargo test --test specs escapes_detects_pattern_matches_in_source
cargo test --test specs escapes_reports_line_number_of_match

# Full quality gates
make check
```

### Expected Test Results

| Test | Expected |
|------|----------|
| `escapes_detects_pattern_matches_in_source` | PASS (after removing ignore) |
| `escapes_reports_line_number_of_match` | PASS (after removing ignore) |
| Phase 215 specs | Still ignored |
| Phase 220 specs | Still ignored |

## Summary

| Phase | Task | Key Files | Status |
|-------|------|-----------|--------|
| 1 | Pattern configuration types | `config.rs` | [ ] Pending |
| 2 | Pattern matcher types | `pattern/mod.rs`, `pattern/matcher.rs` | [ ] Pending |
| 3 | Pattern analysis and compilation | `pattern/matcher.rs` | [ ] Pending |
| 4 | Line number extraction | `pattern/matcher.rs` | [ ] Pending |
| 5 | Escapes check implementation | `checks/escapes.rs`, `checks/mod.rs` | [ ] Pending |
| 6 | Unit tests and polish | `*_tests.rs` | [ ] Pending |

## Future Phases

- **Phase 215**: Implement actions (count/comment/forbid logic, comment detection)
- **Phase 220**: Implement metrics (source/test separation, JSON metrics output)
