# Checkpoint 4G: Bug Fixes - Rust Adapter

**Root Feature:** `quench-4g8b`

## Overview

Bug fix checkpoint for the Rust adapter's `#[cfg(test)]` block detection. The current brace-counting parser has documented limitations that can cause incorrect test/source line classification in files containing:

1. **Raw string literals** (`r"..."` and `r#"..."#`) - braces inside raw strings are incorrectly counted
2. **Character literals** (`'{'` and `'}'`) - braces in char literals are incorrectly counted

These bugs cause the parser to lose track of brace depth, potentially classifying source code as test code or vice versa.

**Bugs identified:**

| Bug | File | Impact | Priority |
|-----|------|--------|----------|
| Raw string handling | `cfg_test.rs:44-67` | Misclassifies lines in files with raw strings containing braces | HIGH |
| Character literal handling | `cfg_test.rs:44-67` | Misclassifies lines in files with char literals `'{'` or `'}'` | HIGH |
| Limited glob patterns | `workspace.rs:83` | Only handles `path/*`, not `path/**` or other patterns | LOW |

## Project Structure

```
crates/cli/src/adapter/rust/
├── cfg_test.rs           # FIX: Raw strings and char literals
├── cfg_test_tests.rs     # ADD: Test cases for edge cases
├── workspace.rs          # FIX: Better glob pattern handling
└── workspace_tests.rs    # ADD: Test cases for glob patterns
```

## Dependencies

No new dependencies required.

## Implementation Phases

### Phase 1: Add Failing Tests for Raw Strings and Char Literals

Before fixing the bugs, add test cases that demonstrate the current broken behavior.

**Add to `cfg_test_tests.rs`:**

```rust
#[test]
fn raw_string_with_braces() {
    // Raw strings containing braces should not affect brace counting
    let content = r#"
fn source() {}

#[cfg(test)]
mod tests {
    #[test]
    fn test_raw_string() {
        let s = r"{ not a real brace }";
        assert!(true);
    }
}
"#;
    let info = CfgTestInfo::parse(content);

    assert_eq!(info.test_ranges.len(), 1);
    assert!(info.is_test_line(3));  // #[cfg(test)]
    assert!(info.is_test_line(10)); // closing brace of mod tests
}

#[test]
fn raw_string_with_hashes() {
    // Raw strings with hash delimiters
    let content = r###"
fn source() {}

#[cfg(test)]
mod tests {
    #[test]
    fn test() {
        let s = r#"{ braces } and "quotes""#;
        let t = r##"more { braces }"##;
        assert!(true);
    }
}
"###;
    let info = CfgTestInfo::parse(content);

    assert_eq!(info.test_ranges.len(), 1);
    assert!(info.is_test_line(3));  // #[cfg(test)]
    assert!(info.is_test_line(11)); // closing brace
}

#[test]
fn char_literal_with_brace() {
    // Character literals containing braces
    let content = r#"
fn source() {}

#[cfg(test)]
mod tests {
    #[test]
    fn test_char() {
        let open = '{';
        let close = '}';
        assert_eq!(open, '{');
    }
}
"#;
    let info = CfgTestInfo::parse(content);

    assert_eq!(info.test_ranges.len(), 1);
    assert!(info.is_test_line(3));  // #[cfg(test)]
    assert!(info.is_test_line(11)); // closing brace
}

#[test]
fn char_literal_with_escaped_quote() {
    // Escaped quote in char literal shouldn't confuse parser
    let content = r#"
fn source() {}

#[cfg(test)]
mod tests {
    #[test]
    fn test() {
        let quote = '\'';
        let brace = '{';
        assert!(true);
    }
}
"#;
    let info = CfgTestInfo::parse(content);

    assert_eq!(info.test_ranges.len(), 1);
    assert!(info.is_test_line(10)); // closing brace
}
```

**Milestone:** Tests demonstrate the bugs exist.

**Verification:**
```bash
cargo test -p quench -- cfg_test  # Some tests should fail
```

**Status:** [ ] Pending

---

### Phase 2: Implement Lexer State Machine

Replace the simplified string detection with a proper lexer state machine that handles:
- Regular strings (`"..."`)
- Raw strings (`r"..."`, `r#"..."#`, `r##"..."##`, etc.)
- Character literals (`'x'`, `'\''`, `'{'`)
- Escaped characters in strings

**New implementation in `cfg_test.rs`:**

```rust
/// Lexer state for tracking what context we're in.
#[derive(Debug, Clone, Copy, PartialEq)]
enum LexerState {
    /// Normal code - braces count
    Code,
    /// Inside a regular string "..."
    String,
    /// Inside a raw string r"..." or r#"..."#
    /// The usize is the number of # delimiters
    RawString(usize),
    /// Inside a character literal '...'
    Char,
}

/// Count brace depth changes in a line, accounting for string/char literals.
fn count_braces(line: &str) -> i32 {
    let mut depth_change: i32 = 0;
    let mut state = LexerState::Code;
    let mut chars = line.chars().peekable();
    let mut prev_char = '\0';

    while let Some(ch) = chars.next() {
        match state {
            LexerState::Code => {
                match ch {
                    '"' => {
                        state = LexerState::String;
                    }
                    'r' => {
                        // Check for raw string: r"..." or r#"..."#
                        if let Some(&next) = chars.peek() {
                            if next == '"' {
                                chars.next(); // consume "
                                state = LexerState::RawString(0);
                            } else if next == '#' {
                                // Count consecutive #s
                                let mut hash_count = 0;
                                while chars.peek() == Some(&'#') {
                                    chars.next();
                                    hash_count += 1;
                                }
                                // Must be followed by "
                                if chars.peek() == Some(&'"') {
                                    chars.next();
                                    state = LexerState::RawString(hash_count);
                                }
                            }
                        }
                    }
                    '\'' => {
                        // Character literal - but be careful about lifetimes
                        // Lifetime syntax: 'a, 'static, etc.
                        // Char literal: 'x', '\n', '\''
                        // Peek ahead to determine which
                        if let Some(&next) = chars.peek() {
                            // Check if this looks like a char literal
                            // Char literals are 'x' (single char) or '\x' (escaped)
                            let mut temp_chars = chars.clone();
                            if next == '\\' {
                                // Escape sequence: '\n', '\'', etc.
                                temp_chars.next(); // skip backslash
                                temp_chars.next(); // skip escaped char
                                if temp_chars.peek() == Some(&'\'') {
                                    state = LexerState::Char;
                                }
                            } else if temp_chars.next().is_some() {
                                // Single character 'x'
                                if temp_chars.peek() == Some(&'\'') {
                                    state = LexerState::Char;
                                }
                            }
                        }
                    }
                    '{' => depth_change += 1,
                    '}' => depth_change -= 1,
                    _ => {}
                }
            }
            LexerState::String => {
                if ch == '"' && prev_char != '\\' {
                    state = LexerState::Code;
                }
            }
            LexerState::RawString(hash_count) => {
                // Raw string ends with "### where # count matches
                if ch == '"' {
                    let mut matched = 0;
                    while matched < hash_count && chars.peek() == Some(&'#') {
                        chars.next();
                        matched += 1;
                    }
                    if matched == hash_count {
                        state = LexerState::Code;
                    }
                }
            }
            LexerState::Char => {
                // Char literal ends at closing '
                if ch == '\'' && prev_char != '\\' {
                    state = LexerState::Code;
                }
            }
        }
        prev_char = ch;
    }

    depth_change
}
```

**Update `CfgTestInfo::parse()` to use the new function:**

```rust
pub fn parse(content: &str) -> Self {
    let mut info = Self::default();
    let mut in_cfg_test = false;
    let mut brace_depth: i32 = 0;
    let mut block_start = 0;

    for (line_idx, line) in content.lines().enumerate() {
        let trimmed = line.trim();

        // Check for #[cfg(test)] attribute
        if !in_cfg_test && is_cfg_test_attr(trimmed) {
            in_cfg_test = true;
            block_start = line_idx;
            brace_depth = 0;
            continue;
        }

        if in_cfg_test {
            let delta = count_braces(trimmed);
            brace_depth += delta;

            if brace_depth == 0 && delta < 0 {
                // Block ended (we saw a closing brace that brought us to 0)
                info.test_ranges.push(block_start..line_idx + 1);
                in_cfg_test = false;
            }
        }
    }

    info
}
```

**Milestone:** Raw strings and char literals are correctly handled.

**Verification:**
```bash
cargo test -p quench -- cfg_test  # All tests should pass
```

**Status:** [ ] Pending

---

### Phase 3: Add Tests for Edge Cases

Add comprehensive tests for additional edge cases discovered during implementation.

**Add to `cfg_test_tests.rs`:**

```rust
#[test]
fn mixed_string_types() {
    // Mix of regular strings, raw strings, and char literals
    let content = r###"
fn source() {}

#[cfg(test)]
mod tests {
    #[test]
    fn test() {
        let a = "{ regular }";
        let b = r"{ raw }";
        let c = r#"{ raw # }"#;
        let d = '{';
        let e = '}';
        assert!(true);
    }
}
"###;
    let info = CfgTestInfo::parse(content);

    assert_eq!(info.test_ranges.len(), 1);
    assert!(info.is_test_line(14)); // closing brace
}

#[test]
fn lifetime_not_confused_with_char() {
    // Lifetimes should not be confused with char literals
    let content = r#"
fn source<'a>(x: &'a str) -> &'a str { x }

#[cfg(test)]
mod tests {
    fn helper<'a>(x: &'a str) -> &'a str {
        x
    }
}
"#;
    let info = CfgTestInfo::parse(content);

    assert_eq!(info.test_ranges.len(), 1);
    assert!(!info.is_test_line(1)); // source function
    assert!(info.is_test_line(3));  // #[cfg(test)]
}

#[test]
fn nested_raw_strings() {
    // Raw strings can contain quote characters
    let content = r####"
fn source() {}

#[cfg(test)]
mod tests {
    #[test]
    fn test() {
        let s = r#"contains "quotes" and { braces }"#;
        assert!(true);
    }
}
"####;
    let info = CfgTestInfo::parse(content);

    assert_eq!(info.test_ranges.len(), 1);
}

#[test]
fn empty_string_and_char() {
    // Edge case: empty-ish strings
    let content = r#"
fn source() {}

#[cfg(test)]
mod tests {
    #[test]
    fn test() {
        let s = "";
        let c = ' ';
        let brace_str = "{}";
        assert!(true);
    }
}
"#;
    let info = CfgTestInfo::parse(content);

    assert_eq!(info.test_ranges.len(), 1);
}
```

**Milestone:** Edge cases are covered by tests.

**Verification:**
```bash
cargo test -p quench -- cfg_test
```

**Status:** [ ] Pending

---

### Phase 4: Improve Workspace Glob Pattern Support

The current `expand_workspace_members` only handles `path/*` patterns. Improve to handle:
- `path/**` (recursive glob)
- Other common workspace patterns

**Update `workspace.rs`:**

```rust
/// Expand workspace member patterns to package names.
fn expand_workspace_members(patterns: &[String], root: &Path) -> Vec<String> {
    let mut packages = Vec::new();

    for pattern in patterns {
        if pattern.contains('*') {
            // Handle glob patterns
            if let Some(base) = pattern.strip_suffix("/*") {
                // Single-level glob: crates/*
                expand_single_level(root, base, &mut packages);
            } else if let Some(base) = pattern.strip_suffix("/**") {
                // Recursive glob: crates/** (treat as single level for now)
                // Most workspaces use ** as a future-proofing pattern
                expand_single_level(root, base, &mut packages);
            } else if pattern.ends_with("/*") || pattern.contains("/*") {
                // Pattern like path/to/* - extract base
                if let Some(pos) = pattern.rfind("/*") {
                    let base = &pattern[..pos];
                    expand_single_level(root, base, &mut packages);
                }
            }
            // Other glob patterns are not supported - skip silently
        } else {
            // Direct path to package
            let pkg_dir = root.join(pattern);
            if let Some(name) = read_package_name(&pkg_dir) {
                packages.push(name);
            }
        }
    }

    packages.sort();
    packages
}

/// Expand a single-level directory to find packages.
fn expand_single_level(root: &Path, base: &str, packages: &mut Vec<String>) {
    let dir = root.join(base);
    if let Ok(entries) = fs::read_dir(&dir) {
        for entry in entries.flatten() {
            if entry.path().is_dir() {
                if let Some(name) = read_package_name(&entry.path()) {
                    packages.push(name);
                }
            }
        }
    }
}
```

**Add tests to `workspace_tests.rs`:**

```rust
#[test]
fn expands_double_star_pattern() {
    // Test that ** patterns are handled (treated as single level)
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();

    // Create workspace with ** pattern
    let cargo_toml = r#"
[workspace]
members = ["crates/**"]
"#;
    std::fs::write(root.join("Cargo.toml"), cargo_toml).unwrap();

    // Create a crate
    std::fs::create_dir_all(root.join("crates/foo")).unwrap();
    std::fs::write(
        root.join("crates/foo/Cargo.toml"),
        r#"[package]
name = "foo"
version = "0.1.0"
"#,
    )
    .unwrap();

    let ws = CargoWorkspace::from_root(root);
    assert!(ws.is_workspace);
    assert!(ws.packages.contains(&"foo".to_string()));
}
```

**Milestone:** More glob patterns supported.

**Verification:**
```bash
cargo test -p quench -- workspace
```

**Status:** [ ] Pending

---

### Phase 5: Final Verification

Run the full verification suite.

**Verification checklist:**
- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --all-targets --all-features -- -D warnings`
- [ ] `cargo test --all`
- [ ] `cargo build --all`
- [ ] `./scripts/bootstrap`
- [ ] `cargo audit`
- [ ] `cargo deny check`

**Run:**
```bash
make check
```

**Milestone:** All quality gates pass.

**Status:** [ ] Pending

## Key Implementation Details

### Why a State Machine for Lexing?

The original implementation used a simple `in_string` boolean with backslash escape detection. This fails for:

1. **Raw strings:** `r"..."` has no escape sequences, so `r"\n"` contains a literal backslash
2. **Raw strings with hashes:** `r#"..."#` allows embedded quotes without escaping
3. **Character literals:** `'{'` is a single character, not a block start

A proper state machine tracks:
- Whether we're in code, a string, a raw string, or a char literal
- For raw strings, how many `#` delimiters to expect at the end

### Lifetime vs Character Literal Disambiguation

Rust uses `'` for both lifetimes (`'a`, `'static`) and character literals (`'x'`).

The difference:
- Lifetimes: `'identifier` (letter/underscore followed by alphanumerics)
- Char literals: `'x'` (exactly one character or escape sequence, followed by `'`)

The parser can distinguish by looking ahead:
- If the next char after `'` is followed by `'` (possibly with an escape), it's a char literal
- Otherwise, it's likely a lifetime and we stay in code mode

### No Breaking Changes

The bug fixes should be purely additive:
- Files that parsed correctly before will still parse correctly
- Files with raw strings or char literals containing braces will now parse correctly
- No API changes

## Verification Plan

1. **Before changes** - capture baseline behavior:
   ```bash
   cargo build
   cargo test -p quench -- cfg_test 2>&1 | tee /tmp/before.txt
   ```

2. **After Phase 1** - verify tests fail (demonstrating bugs):
   ```bash
   cargo test -p quench -- cfg_test 2>&1 | grep -E "(FAILED|passed)"
   # Should show some failures
   ```

3. **After Phase 2** - verify tests pass:
   ```bash
   cargo test -p quench -- cfg_test
   # All tests should pass
   ```

4. **After all changes** - full verification:
   ```bash
   make check
   ```

## Summary

| Phase | Task | Files Changed | Status |
|-------|------|---------------|--------|
| 1 | Add failing tests | `cfg_test_tests.rs` | [ ] Pending |
| 2 | Implement lexer state machine | `cfg_test.rs` | [ ] Pending |
| 3 | Add edge case tests | `cfg_test_tests.rs` | [ ] Pending |
| 4 | Improve workspace globs | `workspace.rs`, `workspace_tests.rs` | [ ] Pending |
| 5 | Final verification | - | [ ] Pending |

## Notes

- The raw string and char literal bugs have been documented limitations since v1
- This fix resolves the most impactful parsing issues
- Multi-line attributes remain unsupported (rare in practice)
- `mod tests;` external module declarations remain unsupported (handled by path patterns)
