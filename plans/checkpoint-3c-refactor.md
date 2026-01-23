# Checkpoint 3C: Post-Checkpoint Refactor - Escapes Works

**Root Feature:** `quench-0dc9`

## Overview

This refactor checkpoint addresses behavioral gaps identified in `reports/checkpoint-3-escapes-works.md`. Two issues were discovered during validation:

1. **Duplicate violations** - The same escape pattern on a single line can generate multiple violations when the pattern appears both in code and in comments (e.g., `.unwrap()` in code AND `.unwrap()` mentioned in a comment)

2. **Comment search false positives** - The `has_justification_comment()` function matches comment patterns anywhere in a line, including embedded in other text (e.g., `// VIOLATION: unsafe without // SAFETY: comment` incorrectly passes because it contains `// SAFETY:`)

## Project Structure

Key files to modify:

```
quench/
├── crates/cli/src/
│   ├── checks/
│   │   ├── escapes.rs          # Main fix: deduplication + comment search
│   │   └── escapes_tests.rs    # Unit tests for fixes
│   └── pattern/
│       ├── matcher.rs          # May need line-aware deduplication
│       └── matcher_tests.rs    # Tests for matcher changes
├── tests/
│   ├── specs/checks/escapes.rs # Behavioral specs for edge cases
│   └── fixtures/violations/    # Fix fixture to properly test issues
└── reports/
    └── checkpoint-3-escapes-works.md  # Reference for issues
```

## Dependencies

No new dependencies required.

## Implementation Phases

### Phase 1: Analyze and Document Root Causes

**Goal:** Confirm the exact source of each bug with test cases.

**Issue 1: Duplicate Violations**

The violations fixture `src/escapes.rs` line 5:
```rust
input.parse().unwrap()  // VIOLATION: .unwrap() forbidden
```

The regex `\.unwrap\(\)` finds two matches:
- Byte offset ~13: The actual `.unwrap()` call
- Byte offset ~45: The `.unwrap()` in the comment

Both matches resolve to line 5, creating duplicate violations.

**Issue 2: Comment Search Edge Case**

The `has_justification_comment()` function at `escapes.rs:400-427`:
```rust
if lines[line_idx].contains(comment_pattern) {
    return true;
}
```

This matches `// SAFETY:` anywhere in the line, even when embedded in other text like:
```rust
unsafe { *ptr }  // VIOLATION: unsafe without // SAFETY: comment
```

**Milestone:** Document both issues with exact byte offsets and test cases.

**Status:** [ ] Pending

---

### Phase 2: Fix Duplicate Violations via Line-Based Deduplication

**Goal:** Ensure at most one violation per (file, line, pattern) tuple.

**Approach:** Deduplicate matches by line number before creating violations.

**Changes to `escapes.rs`:**

```rust
// In the file processing loop (around line 202)
for pattern in &patterns {
    let matches = pattern.matcher.find_all_with_lines(&content);

    // Deduplicate matches by line - keep only first match per line
    let mut seen_lines = std::collections::HashSet::new();
    let unique_matches: Vec<_> = matches
        .into_iter()
        .filter(|m| seen_lines.insert(m.line))
        .collect();

    for m in unique_matches {
        // ... existing logic
    }
}
```

**Alternative:** Deduplication in `CompiledPattern::find_all_with_lines()` (in `matcher.rs`). However, keeping it in `escapes.rs` is simpler since other checks may want all matches.

**Unit Test:**

```rust
// escapes_tests.rs
#[test]
fn deduplicates_multiple_matches_on_same_line() {
    // Create content with pattern appearing twice on one line
    let content = "code.unwrap()  // .unwrap() mentioned";
    // Pattern should only produce one match for the line
}
```

**Milestone:** No duplicate violations for same (file, line, pattern).

**Status:** [ ] Pending

---

### Phase 3: Fix Comment Search Edge Cases

**Goal:** Comment patterns should only match at the start of a comment, not embedded in text.

**Current Behavior:**
```rust
// This passes incorrectly:
unsafe { *ptr }  // VIOLATION: missing // SAFETY: comment
```

The function finds `// SAFETY:` embedded within `// VIOLATION: ... // SAFETY: ...`.

**Fix Strategy:**

The comment pattern (e.g., `// SAFETY:`) should match only when it appears as a distinct comment marker. Two approaches:

**Option A: Match at comment boundary**

Check that the pattern starts at position 0 of a trimmed comment line, or immediately after a comment marker (`//`, `#`, etc.):

```rust
fn has_justification_comment(content: &str, match_line: u32, comment_pattern: &str) -> bool {
    let lines: Vec<&str> = content.lines().collect();
    let line_idx = (match_line - 1) as usize;

    // Check same line - look for pattern after a comment marker
    if line_idx < lines.len() {
        let line = lines[line_idx];
        if let Some(comment_start) = find_comment_start(line) {
            let comment = &line[comment_start..];
            // Pattern must be at start of comment (after marker + whitespace)
            if comment_starts_with_pattern(comment, comment_pattern) {
                return true;
            }
        }
    }

    // Search upward through preceding comment lines
    if line_idx > 0 {
        for i in (0..line_idx).rev() {
            let line = lines[i].trim();

            if is_comment_line(line) && comment_starts_with_pattern(line, comment_pattern) {
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

/// Check if comment content starts with the pattern (after comment marker).
fn comment_starts_with_pattern(comment: &str, pattern: &str) -> bool {
    let trimmed = comment.trim_start_matches(|c| c == '/' || c == '#' || c == '*' || c == '-' || c == ';');
    trimmed.trim_start().starts_with(pattern.trim_start_matches(|c| c == '/' || c == '#' || c == ' '))
}

/// Find the start of a comment in a line (returns byte offset of `//` or `#`).
fn find_comment_start(line: &str) -> Option<usize> {
    // Simple heuristic: find first // or # not in a string
    // For robustness, just find the markers
    if let Some(pos) = line.find("//") {
        return Some(pos);
    }
    if let Some(pos) = line.find('#') {
        // Avoid matching # in strings or shebangs
        if pos == 0 || line.chars().nth(pos.saturating_sub(1)) == Some(' ') {
            return Some(pos);
        }
    }
    None
}
```

**Option B: Strip comment marker prefix from pattern**

If the pattern is `// SAFETY:`, require it to appear as the start of a comment:

```rust
fn has_justification_comment(content: &str, match_line: u32, comment_pattern: &str) -> bool {
    // Normalize pattern - extract the key text after comment marker
    let normalized = normalize_comment_pattern(comment_pattern);

    // ... then match normalized pattern at start of trimmed comment content
}
```

**Recommended: Option A** - More explicit and handles edge cases better.

**Unit Tests:**

```rust
// escapes_tests.rs

#[test]
fn comment_search_ignores_embedded_patterns() {
    // Pattern appears embedded in another comment
    let content = "code  // VIOLATION: missing // SAFETY: comment\nmore code";
    assert!(!has_justification_comment(content, 1, "// SAFETY:"));
}

#[test]
fn comment_search_finds_standalone_pattern() {
    // Pattern is the actual comment
    let content = "// SAFETY: this is safe\nunsafe { *ptr }";
    assert!(has_justification_comment(content, 2, "// SAFETY:"));
}

#[test]
fn comment_search_finds_pattern_on_same_line() {
    let content = "unsafe { *ptr }  // SAFETY: this is safe";
    assert!(has_justification_comment(content, 1, "// SAFETY:"));
}
```

**Milestone:** Comment patterns only match at comment boundaries, not embedded in text.

**Status:** [ ] Pending

---

### Phase 4: Update violations Fixture

**Goal:** Fix the fixtures so they properly test the intended behaviors.

**Current violations/src/escapes.rs:**
```rust
input.parse().unwrap()  // VIOLATION: .unwrap() forbidden
...
unsafe { *ptr }  // VIOLATION: unsafe without // SAFETY: comment
```

**Fixed version - remove escape patterns from violation description comments:**
```rust
input.parse().unwrap()  // VIOLATION: unwrap forbidden in source
...
unsafe { *ptr }  // VIOLATION: unsafe block requires SAFETY comment
```

This ensures:
- Line 5 has only one `.unwrap()` match (in code, not comment)
- Line 15 doesn't accidentally contain `// SAFETY:`

**Milestone:** Fixture produces expected single violations per issue.

**Status:** [ ] Pending

---

### Phase 5: Add Behavioral Specs for Edge Cases

**Goal:** Prevent regression with explicit specs.

```rust
// tests/specs/checks/escapes.rs

/// Spec: Edge case - pattern in both code and comment
///
/// > When escape pattern appears in code AND in a comment on the same line,
/// > only one violation should be reported for that line.
#[test]
fn escapes_single_violation_per_line_even_with_pattern_in_comment() {
    // Need a fixture that specifically tests this
    let result = check("escapes")
        .on("escapes/forbid-source")  // Or create new fixture
        .json()
        .fails();

    let violations = result.require("violations").as_array().unwrap();
    // Count violations per line - should be max 1 per line
    let mut line_counts = std::collections::HashMap::new();
    for v in violations {
        let line = v.get("line").and_then(|l| l.as_i64()).unwrap_or(0);
        *line_counts.entry(line).or_insert(0) += 1;
    }
    for (line, count) in line_counts {
        assert!(count <= 1, "Line {} has {} violations, expected max 1", line, count);
    }
}

/// Spec: Edge case - embedded comment pattern
///
/// > Comment pattern embedded in other text should NOT satisfy the requirement
#[test]
fn escapes_comment_embedded_in_text_does_not_satisfy() {
    // Create a fixture or inline test that has embedded pattern
    // Pattern "// SAFETY:" embedded in "// VIOLATION: ... // SAFETY: ..."
    // should NOT pass
}
```

**Milestone:** Edge case specs prevent regression.

**Status:** [ ] Pending

---

### Phase 6: Run Full Test Suite

Execute `make check` to ensure all changes pass quality gates.

```bash
make check
```

**Checklist:**
- [ ] `cargo fmt --all -- --check` - no formatting issues
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` - no warnings
- [ ] `cargo test --all` - all tests pass
- [ ] `cargo test escapes` - all escapes specs pass
- [ ] `cargo build --all` - builds successfully
- [ ] `./scripts/bootstrap` - conventions pass
- [ ] `cargo audit` - no vulnerabilities
- [ ] `cargo deny check` - licenses/bans OK

**Milestone:** All quality gates pass.

**Status:** [ ] Pending

## Key Implementation Details

### Line-Based Deduplication

The deduplication should happen after pattern matching but before violation creation:

```rust
// Pseudocode for deduplication
let matches = pattern.matcher.find_all_with_lines(&content);

// Deduplicate by line number (keep first match per line)
let mut seen_lines = HashSet::new();
let matches: Vec<_> = matches
    .into_iter()
    .filter(|m| seen_lines.insert(m.line))
    .collect();
```

**Why keep first match?** The first match is typically the one in actual code (before comments), which is the more relevant location.

### Comment Boundary Detection

The key insight is that `// SAFETY:` should be at the *start* of a comment's content, not embedded:

| Line | Has `// SAFETY:`? | Should Match? |
|------|-------------------|---------------|
| `// SAFETY: reason` | At start | Yes |
| `code // SAFETY: reason` | At start of inline comment | Yes |
| `// VIOLATION: missing // SAFETY:` | Embedded | No |
| `// SAFETY: reason // more` | At start | Yes |

### Pattern Normalization

Comment patterns like `// SAFETY:` should match regardless of leading marker variation:

```rust
// All should match "// SAFETY:" pattern:
// SAFETY: reason
/// SAFETY: reason
//! SAFETY: reason
# SAFETY: reason (in shell/Python)
```

The fix should normalize by stripping the comment marker and matching the remainder.

## Verification Plan

1. **Unit tests for deduplication:**
   ```bash
   cargo test --package quench -- deduplicates
   ```

2. **Unit tests for comment search:**
   ```bash
   cargo test --package quench -- comment_search
   ```

3. **Behavioral specs:**
   ```bash
   cargo test --test specs escapes
   ```

4. **Manual verification:**
   ```bash
   # Verify single violation per line
   ./target/release/quench check tests/fixtures/violations --escapes -o json | jq '.checks[0].violations | group_by(.line) | map(length) | max'
   # Should output 1
   ```

5. **Full suite:**
   ```bash
   make check
   ```

## Summary

| Phase | Task | Status |
|-------|------|--------|
| 1 | Analyze root causes | [ ] Pending |
| 2 | Fix duplicate violations | [ ] Pending |
| 3 | Fix comment search edge cases | [ ] Pending |
| 4 | Update violations fixture | [ ] Pending |
| 5 | Add behavioral specs | [ ] Pending |
| 6 | Run full test suite | [ ] Pending |
