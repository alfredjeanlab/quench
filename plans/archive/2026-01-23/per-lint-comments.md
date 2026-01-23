# Per-Lint Comment Patterns

**Root Feature:** `quench-f164`

## Overview

Implement per-lint-code comment patterns for suppress configuration. Currently, `[rust.suppress.source.<lint-code>]` and `[shell.suppress.source.<lint-code>]` sections accept a `comment` field, but it is silently ignored. All suppressions use only the global `comment` pattern.

This plan also addresses the architectural concern of duplicated suppress-checking logic between Rust and Shell implementations (~130 lines each, 95% identical). We'll refactor into a shared abstraction first, then implement per-lint patterns once for all adapters.

## Project Structure

```
crates/cli/src/
├── config/
│   ├── mod.rs           # SuppressScopeConfig gains `patterns` field
│   ├── parse.rs         # parse_suppress_scope_config() handles nested tables
│   └── shell.rs         # ShellSuppressConfig unchanged (reuses SuppressScopeConfig)
├── checks/
│   ├── escapes/
│   │   ├── mod.rs       # check_suppress_violations() delegates to shared
│   │   ├── shell_suppress.rs  # check_shell_suppress_violations() delegates to shared
│   │   └── suppress_common.rs # NEW: shared suppress checking logic
└── adapter/
    └── common/
        └── suppress.rs  # check_justification_comment() unchanged
tests/specs/
└── suppress_per_lint_pattern_spec.rs  # NEW: behavioral tests
```

## Dependencies

No new external dependencies. Uses existing:
- `toml` - Already used for config parsing
- `std::collections::HashMap` - Already in scope

## Implementation Phases

### Phase 1: Add `patterns` field to SuppressScopeConfig

**Goal:** Update the data structure to hold per-lint-code comment patterns.

**Files:**
- `crates/cli/src/config/mod.rs`

**Changes:**

```rust
// crates/cli/src/config/mod.rs:207-230
#[derive(Debug, Clone, Default, Deserialize)]
pub struct SuppressScopeConfig {
    #[serde(default)]
    pub check: Option<SuppressLevel>,

    #[serde(default)]
    pub allow: Vec<String>,

    #[serde(default)]
    pub forbid: Vec<String>,

    /// Per-lint-code comment patterns. Maps lint code to required comment prefix.
    /// Example: {"dead_code" => "// NOTE(compat):"}
    #[serde(default)]
    pub patterns: HashMap<String, String>,
}
```

**Verification:**
- `cargo build` compiles
- Existing tests pass

---

### Phase 2: Parse per-lint-code sections

**Goal:** Extract `comment` fields from nested `[rust.suppress.source.<lint-code>]` tables.

**Files:**
- `crates/cli/src/config/parse.rs`

**Changes:**

Update `parse_suppress_scope_config()` to iterate over table entries and identify lint-code sections:

```rust
fn parse_suppress_scope_config(value: Option<&toml::Value>, is_test: bool) -> SuppressScopeConfig {
    let Some(toml::Value::Table(t)) = value else {
        return if is_test {
            SuppressScopeConfig::default_for_test()
        } else {
            SuppressScopeConfig::default()
        };
    };

    // ... existing check, allow, forbid parsing ...

    // NEW: Parse per-lint-code comment patterns
    // Any key that maps to a table with a "comment" string is a lint-code section
    let mut patterns = HashMap::new();
    for (key, val) in t.iter() {
        // Skip known fields
        if matches!(key.as_str(), "check" | "allow" | "forbid") {
            continue;
        }
        // If value is a table with "comment" field, it's a per-lint pattern
        if let toml::Value::Table(lint_table) = val {
            if let Some(toml::Value::String(pattern)) = lint_table.get("comment") {
                patterns.insert(key.clone(), pattern.clone());
            }
        }
    }

    SuppressScopeConfig { check, allow, forbid, patterns }
}
```

**Verification:**
- Unit test: parse config with per-lint patterns, verify `patterns` HashMap populated
- Existing config tests still pass

---

### Phase 3: Extract shared suppress-checking logic

**Goal:** Refactor duplicated Rust/Shell checking into a shared module.

**Files:**
- `crates/cli/src/checks/escapes/suppress_common.rs` (NEW)
- `crates/cli/src/checks/escapes/mod.rs`
- `crates/cli/src/checks/escapes/shell_suppress.rs`

**Approach:**

Create a trait-based or generic approach. Both checkers need:
1. Access to scope config (allow, forbid, patterns)
2. Access to global comment pattern
3. List of suppress attributes to check
4. Way to create violations with language-specific error codes

**New shared module:**

```rust
// crates/cli/src/checks/escapes/suppress_common.rs

use crate::config::{SuppressLevel, SuppressScopeConfig};
use crate::violation::Violation;

pub struct SuppressCheckParams<'a> {
    pub scope_config: &'a SuppressScopeConfig,
    pub scope_check: SuppressLevel,
    pub global_comment: Option<&'a str>,
}

pub struct SuppressAttrInfo<'a> {
    pub line: usize,
    pub codes: &'a [String],
    pub has_comment: bool,
    pub comment_text: Option<&'a str>,
}

pub enum SuppressViolationKind {
    Forbidden { code: String },
    MissingComment { codes: Vec<String>, required_pattern: Option<String> },
}

/// Check a suppress attribute against scope config.
/// Returns None if no violation, Some(kind) if violation detected.
pub fn check_suppress_attr(
    params: &SuppressCheckParams,
    attr: &SuppressAttrInfo,
) -> Option<SuppressViolationKind> {
    // 1. Check forbid list
    for code in attr.codes {
        if params.scope_config.forbid.iter().any(|f| code_matches(code, f)) {
            return Some(SuppressViolationKind::Forbidden { code: code.clone() });
        }
    }

    // 2. Check allow list (skip remaining checks)
    for code in attr.codes {
        if params.scope_config.allow.iter().any(|a| code_matches(code, a)) {
            return None;
        }
    }

    // 3. Check comment requirement
    if params.scope_check == SuppressLevel::Comment {
        // Check per-lint patterns first, then fall back to global
        let required_pattern = find_required_pattern(params, attr);
        if !has_valid_comment(attr, required_pattern.as_deref()) {
            return Some(SuppressViolationKind::MissingComment {
                codes: attr.codes.to_vec(),
                required_pattern,
            });
        }
    }

    None
}

fn find_required_pattern(params: &SuppressCheckParams, attr: &SuppressAttrInfo) -> Option<String> {
    // Check per-lint patterns first
    for code in attr.codes {
        if let Some(pattern) = params.scope_config.patterns.get(code) {
            return Some(pattern.clone());
        }
    }
    // Fall back to global pattern
    params.global_comment.map(String::from)
}

fn has_valid_comment(attr: &SuppressAttrInfo, required_pattern: Option<&str>) -> bool {
    if !attr.has_comment {
        return false;
    }
    match (required_pattern, &attr.comment_text) {
        (Some(pattern), Some(text)) => text.starts_with(pattern),
        (Some(_), None) => false,
        (None, _) => attr.has_comment,
    }
}

fn code_matches(code: &str, pattern: &str) -> bool {
    code == pattern || code.starts_with(&format!("{}::", pattern))
}
```

**Update Rust checker:**

```rust
// crates/cli/src/checks/escapes/mod.rs
// Replace inline checking logic with calls to suppress_common

use super::suppress_common::{check_suppress_attr, SuppressCheckParams, SuppressAttrInfo, SuppressViolationKind};

// In check_suppress_violations():
let params = SuppressCheckParams {
    scope_config: &scope_config,
    scope_check,
    global_comment: config.comment.as_deref(),
};

for attr in &suppresses {
    let attr_info = SuppressAttrInfo {
        line: attr.line,
        codes: &attr.codes,
        has_comment: attr.has_comment,
        comment_text: attr.comment_text.as_deref(),
    };

    if let Some(kind) = check_suppress_attr(&params, &attr_info) {
        let violation = match kind {
            SuppressViolationKind::Forbidden { code } => {
                // Create Rust-specific violation with "suppress_forbidden" code
            }
            SuppressViolationKind::MissingComment { codes, required_pattern } => {
                // Create Rust-specific violation with "suppress_missing_comment" code
            }
        };
        violations.push(violation);
    }
}
```

**Update Shell checker:**
- Same pattern, using `"shellcheck_forbidden"` and `"shellcheck_missing_comment"` codes

**Verification:**
- All existing tests pass (no behavioral change yet)
- Both Rust and Shell use shared logic

---

### Phase 4: Implement per-lint pattern checking

**Goal:** Use per-lint patterns during comment validation with fallback to global.

**Files:**
- `crates/cli/src/checks/escapes/suppress_common.rs`
- `crates/cli/src/adapter/rust/suppress.rs`
- `crates/cli/src/adapter/shell/suppress.rs`

**Key change:** The shared `find_required_pattern()` function already handles the lookup. The checking logic needs to verify the comment matches the specific pattern.

**Update suppress attribute parsing:**

Currently, `parse_suppress_attrs()` takes a single `comment_pattern: Option<&str>` and stores `has_comment: bool`. For per-lint patterns, we need to store the actual comment text so we can check it against specific patterns later.

The `SuppressAttr` struct already has `comment_text: Option<String>`, but we need to ensure it's always populated when a comment exists.

**Update `check_justification_comment` call site:**

```rust
// In parse_suppress_attrs, always capture comment text:
let (has_comment, comment_text) = check_justification_comment(
    &lines,
    line_idx,
    None,  // Don't filter by pattern during parsing
    &CommentStyle::RUST,
);
```

Then in the checker, verify pattern match:

```rust
fn has_valid_comment(attr: &SuppressAttrInfo, required_pattern: Option<&str>) -> bool {
    match (required_pattern, &attr.comment_text) {
        (Some(pattern), Some(text)) => {
            // Strip comment prefix and check pattern
            let trimmed = text.trim_start_matches("//").trim_start_matches('#').trim();
            trimmed.starts_with(pattern.trim_start_matches("//").trim_start_matches('#').trim())
        }
        (None, _) => attr.has_comment,
        (Some(_), None) => false,
    }
}
```

**Verification:**
- Unit test: per-lint pattern respected
- Unit test: fallback to global pattern works
- Unit test: no pattern means any comment acceptable

---

### Phase 5: Update error messages

**Goal:** Violation messages reference the specific required pattern.

**Files:**
- `crates/cli/src/checks/escapes/mod.rs`
- `crates/cli/src/checks/escapes/shell_suppress.rs`

**Changes:**

When creating `MissingComment` violations, include the required pattern in the advice:

```rust
SuppressViolationKind::MissingComment { codes, required_pattern } => {
    let advice = match required_pattern {
        Some(pat) => format!("Lint suppression requires justification. Add a {} comment.", pat),
        None => "Lint suppression requires justification. Add a comment explaining why.".to_string(),
    };
    // Create violation with advice
}
```

**Verification:**
- Error messages show per-lint pattern when configured
- Error messages show global pattern as fallback
- Generic message when no pattern configured

---

### Phase 6: Add behavioral tests

**Goal:** Black-box tests verifying the feature end-to-end.

**Files:**
- `tests/specs/suppress_per_lint_pattern_spec.rs` (NEW)
- `tests/fixtures/per-lint-patterns/` (NEW)

**Test cases:**

1. **Per-lint pattern respected:**
   ```toml
   [rust.suppress.source.dead_code]
   comment = "// NOTE(compat):"
   ```
   - `#[allow(dead_code)] // NOTE(compat): legacy API` → PASS
   - `#[allow(dead_code)] // Some other comment` → FAIL

2. **Fallback to global pattern:**
   ```toml
   [rust.suppress]
   comment = "// KEEP:"

   [rust.suppress.source.dead_code]
   comment = "// NOTE:"
   ```
   - `#[allow(unused)] // KEEP: needed for tests` → PASS (uses global)
   - `#[allow(dead_code)] // KEEP: reason` → FAIL (needs `// NOTE:`)

3. **Multiple codes with different patterns:**
   - `#[allow(dead_code, unused_variables)]` with both patterns configured
   - Should accept if comment matches ANY of the per-lint patterns

4. **Shell equivalent tests:**
   - Same scenarios with `# shellcheck disable=SC2034`

**Verification:**
- `cargo test --all` passes
- `make check` passes

## Key Implementation Details

### Pattern Lookup Order

When checking if a suppress attribute has valid justification:

1. Check each lint code in `scope_config.patterns` (first match wins)
2. If no per-lint pattern found, fall back to `config.comment` (global)
3. If no global pattern, any comment is acceptable

### Multiple Codes Handling

For `#[allow(dead_code, unused)]`:
- If `dead_code` has pattern `// NOTE:` and `unused` has no pattern
- Check succeeds if comment matches `// NOTE:` OR global pattern
- Rationale: User is addressing the more specific lint

### Comment Text Normalization

Comment patterns in config may or may not include the comment prefix:
- `comment = "// NOTE:"` or `comment = "NOTE:"`

Both should work. Normalize by stripping prefix before comparison:
```rust
fn normalize_pattern(pattern: &str) -> &str {
    pattern
        .trim_start_matches("//")
        .trim_start_matches('#')
        .trim()
}
```

### Backward Compatibility

- Configs without per-lint patterns continue to work unchanged
- Global `comment` field remains the primary pattern
- Per-lint patterns are additive/override behavior

## Verification Plan

### Unit Tests

| Test | File | Description |
|------|------|-------------|
| `patterns_field_populated` | `config/parse_tests.rs` | Parsing extracts per-lint patterns |
| `patterns_field_empty_when_no_sections` | `config/parse_tests.rs` | No patterns when not configured |
| `check_suppress_attr_uses_per_lint_pattern` | `checks/escapes/suppress_common_tests.rs` | Per-lint pattern matched |
| `check_suppress_attr_falls_back_to_global` | `checks/escapes/suppress_common_tests.rs` | Fallback when no per-lint |
| `check_suppress_attr_any_comment_when_no_pattern` | `checks/escapes/suppress_common_tests.rs` | Any comment accepted |

### Behavioral Tests (specs)

| Spec | Description |
|------|-------------|
| `per_lint_pattern_rust` | Rust suppress with per-lint pattern |
| `per_lint_pattern_shell` | Shell suppress with per-lint pattern |
| `per_lint_pattern_fallback` | Global pattern as fallback |
| `per_lint_pattern_multiple_codes` | Multiple codes with mixed patterns |

### Integration Checklist

- [ ] `make check` passes
- [ ] No regressions in existing suppress tests
- [ ] Error messages reference specific patterns
- [ ] Shell and Rust use shared logic (no duplication)
