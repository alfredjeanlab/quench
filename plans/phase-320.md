# Phase 320: Rust Adapter - Suppress

**Root Feature:** `quench-a0ea`

## Overview

Implement lint suppression attribute detection for the Rust adapter. This controls how `#[allow(...)]` and `#[expect(...)]` attributes are handled based on configuration.

The key features are:

| Feature | Description |
|---------|-------------|
| Attribute detection | Parse `#[allow(...)]` and `#[expect(...)]` attributes |
| Check levels | `forbid`, `comment`, `allow` control whether suppressions need justification |
| Custom comment pattern | Optionally require a specific pattern like `// JUSTIFIED:` |
| Per-code allow list | Codes that never need comments (e.g., `dead_code`) |
| Per-code forbid list | Codes that are never allowed to be suppressed (e.g., `unsafe_code`) |
| Source/test policies | Separate suppression rules for source vs test code |

Default behavior:
- **Source code**: `check = "comment"` (suppressions need justification comments)
- **Test code**: `check = "allow"` (suppressions allowed freely)

Reference docs:
- `docs/specs/langs/rust.md` (Suppress section)
- `docs/specs/02-config.md` (`[rust.suppress]` schema)

## Project Structure

```
quench/
├── crates/cli/src/
│   ├── adapter/
│   │   ├── rust.rs            # UPDATE: add suppress attribute parsing
│   │   └── rust_tests.rs      # UPDATE: add suppress detection tests
│   ├── config.rs              # UPDATE: add [rust.suppress] section
│   └── checks/
│       ├── escapes.rs         # UPDATE: integrate suppress violations
│       └── escapes_tests.rs   # UPDATE: add suppress tests
├── tests/
│   ├── specs/
│   │   └── adapters/rust.rs   # UPDATE: remove #[ignore] from suppress specs
│   └── fixtures/
│       └── rust/
│           ├── suppress-fail/       # NEW: suppress without comment
│           ├── suppress-ok/         # NEW: suppress with comment
│           └── suppress-forbid/     # NEW: forbidden suppression
└── plans/
    └── phase-320.md
```

## Dependencies

No new external dependencies. Uses existing:
- `regex` crate for attribute pattern matching
- Phase 310's `CfgTestInfo` for test code detection
- Phase 315's escape pattern infrastructure

## Implementation Phases

### Phase 1: Config Schema for Suppress

Add the `[rust.suppress]` configuration section.

**Update `crates/cli/src/config.rs`:**

```rust
/// Rust language-specific configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct RustConfig {
    /// Split #[cfg(test)] blocks from source LOC (default: true).
    #[serde(default = "RustConfig::default_split_cfg_test")]
    pub split_cfg_test: bool,

    /// Lint suppression settings.
    #[serde(default)]
    pub suppress: SuppressConfig,
}

/// Lint suppression configuration for #[allow(...)] and #[expect(...)].
#[derive(Debug, Clone, Deserialize)]
pub struct SuppressConfig {
    /// Check level: forbid, comment, or allow (default: "comment").
    #[serde(default = "SuppressConfig::default_check")]
    pub check: SuppressLevel,

    /// Optional comment pattern required (default: any comment).
    /// Example: "// JUSTIFIED:" or "// REASON:"
    #[serde(default)]
    pub comment: Option<String>,

    /// Source-specific settings.
    #[serde(default)]
    pub source: SuppressScopeConfig,

    /// Test-specific settings (overrides base settings for test code).
    #[serde(default)]
    pub test: SuppressScopeConfig,
}

impl Default for SuppressConfig {
    fn default() -> Self {
        Self {
            check: Self::default_check(),
            comment: None,
            source: SuppressScopeConfig::default(),
            test: SuppressScopeConfig::default_for_test(),
        }
    }
}

impl SuppressConfig {
    fn default_check() -> SuppressLevel {
        SuppressLevel::Comment
    }
}

/// Scope-specific suppress configuration.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct SuppressScopeConfig {
    /// Override check level for this scope.
    #[serde(default)]
    pub check: Option<SuppressLevel>,

    /// Lint codes that don't require comments (per-code allow list).
    #[serde(default)]
    pub allow: Vec<String>,

    /// Lint codes that are never allowed to be suppressed (per-code forbid list).
    #[serde(default)]
    pub forbid: Vec<String>,
}

impl SuppressScopeConfig {
    fn default_for_test() -> Self {
        Self {
            check: Some(SuppressLevel::Allow),
            allow: Vec::new(),
            forbid: Vec::new(),
        }
    }
}

/// Suppress check level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SuppressLevel {
    /// Never allowed - any suppression fails.
    Forbid,
    /// Requires justification comment (default).
    #[default]
    Comment,
    /// Always allowed - no check.
    Allow,
}
```

**Update `parse_rust_config()` to parse suppress section.**

**Milestone:** Config parses `[rust.suppress]` section without errors.

**Verification:**
```bash
cargo build
cargo test config -- suppress
```

---

### Phase 2: Suppress Attribute Parser

Add detection for `#[allow(...)]` and `#[expect(...)]` attributes.

**Add to `crates/cli/src/adapter/rust.rs`:**

```rust
/// Suppress attribute found in source code.
#[derive(Debug, Clone)]
pub struct SuppressAttr {
    /// Line number (0-indexed).
    pub line: usize,
    /// Attribute type: "allow" or "expect".
    pub kind: &'static str,
    /// Lint codes being suppressed (e.g., ["dead_code", "unused"]).
    pub codes: Vec<String>,
    /// Whether a justification comment was found.
    pub has_comment: bool,
    /// The comment text if found.
    pub comment_text: Option<String>,
}

/// Parse suppress attributes from Rust source.
pub fn parse_suppress_attrs(content: &str, comment_pattern: Option<&str>) -> Vec<SuppressAttr> {
    let mut attrs = Vec::new();
    let lines: Vec<&str> = content.lines().collect();

    for (line_idx, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        // Match #[allow(...)] or #[expect(...)]
        if let Some(attr) = parse_suppress_line(trimmed) {
            // Check for justification comment above
            let (has_comment, comment_text) = check_justification_comment(
                &lines,
                line_idx,
                comment_pattern,
            );

            attrs.push(SuppressAttr {
                line: line_idx,
                kind: attr.kind,
                codes: attr.codes,
                has_comment,
                comment_text,
            });
        }
    }

    attrs
}

/// Parsed attribute info from a single line.
struct ParsedAttr {
    kind: &'static str,
    codes: Vec<String>,
}

/// Parse a single line for suppress attribute.
fn parse_suppress_line(line: &str) -> Option<ParsedAttr> {
    // Match #[allow(code1, code2)] or #[expect(code1, code2)]
    let kind = if line.starts_with("#[allow(") {
        "allow"
    } else if line.starts_with("#[expect(") {
        "expect"
    } else {
        return None;
    };

    // Extract codes between parentheses
    let start = line.find('(')? + 1;
    let end = line.rfind(')')?;
    if start >= end {
        return None;
    }

    let codes_str = &line[start..end];
    let codes: Vec<String> = codes_str
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    Some(ParsedAttr { kind, codes })
}

/// Check if there's a justification comment above the attribute.
fn check_justification_comment(
    lines: &[&str],
    attr_line: usize,
    required_pattern: Option<&str>,
) -> (bool, Option<String>) {
    // Look at preceding lines for a comment
    let mut check_line = attr_line;

    while check_line > 0 {
        check_line -= 1;
        let line = lines[check_line].trim();

        // Stop at blank lines or non-comment code
        if line.is_empty() {
            break;
        }

        // Check for comment
        if line.starts_with("//") {
            let comment_text = line.trim_start_matches('/').trim();

            // If a specific pattern is required, check for it
            if let Some(pattern) = required_pattern {
                let pattern_prefix = pattern.trim_start_matches('/').trim();
                if comment_text.starts_with(pattern_prefix)
                    || line.starts_with(pattern)
                {
                    return (true, Some(comment_text.to_string()));
                }
                // Continue looking for the pattern
                continue;
            }

            // Any comment counts as justification
            if !comment_text.is_empty() {
                return (true, Some(comment_text.to_string()));
            }
        } else if !line.starts_with('#') {
            // Stop at non-attribute, non-comment line
            break;
        }
    }

    (false, None)
}
```

**Milestone:** Parser correctly identifies `#[allow(...)]` and `#[expect(...)]` with associated comments.

**Verification:**
```bash
cargo test adapter::rust -- suppress
```

---

### Phase 3: Unit Tests for Suppress Parser

**Add to `crates/cli/src/adapter/rust_tests.rs`:**

```rust
mod suppress_parsing {
    use super::*;

    #[test]
    fn detects_allow_attribute() {
        let content = "#[allow(dead_code)]\nfn unused() {}";
        let attrs = parse_suppress_attrs(content, None);

        assert_eq!(attrs.len(), 1);
        assert_eq!(attrs[0].kind, "allow");
        assert_eq!(attrs[0].codes, vec!["dead_code"]);
    }

    #[test]
    fn detects_expect_attribute() {
        let content = "#[expect(unused)]\nlet _x = 42;";
        let attrs = parse_suppress_attrs(content, None);

        assert_eq!(attrs.len(), 1);
        assert_eq!(attrs[0].kind, "expect");
        assert_eq!(attrs[0].codes, vec!["unused"]);
    }

    #[test]
    fn detects_multiple_codes() {
        let content = "#[allow(dead_code, unused_variables)]\nfn f() {}";
        let attrs = parse_suppress_attrs(content, None);

        assert_eq!(attrs.len(), 1);
        assert_eq!(attrs[0].codes, vec!["dead_code", "unused_variables"]);
    }

    #[test]
    fn detects_comment_justification() {
        let content = "// This is needed for FFI compatibility\n#[allow(unsafe_code)]\nfn ffi() {}";
        let attrs = parse_suppress_attrs(content, None);

        assert_eq!(attrs.len(), 1);
        assert!(attrs[0].has_comment);
        assert_eq!(
            attrs[0].comment_text,
            Some("This is needed for FFI compatibility".to_string())
        );
    }

    #[test]
    fn no_comment_when_none_present() {
        let content = "#[allow(dead_code)]\nfn unused() {}";
        let attrs = parse_suppress_attrs(content, None);

        assert!(!attrs[0].has_comment);
        assert!(attrs[0].comment_text.is_none());
    }

    #[test]
    fn requires_specific_comment_pattern() {
        let content = "// Regular comment\n#[allow(dead_code)]\nfn f() {}";
        let attrs = parse_suppress_attrs(content, Some("// JUSTIFIED:"));

        // Regular comment doesn't match pattern
        assert!(!attrs[0].has_comment);
    }

    #[test]
    fn matches_specific_comment_pattern() {
        let content = "// JUSTIFIED: Reserved for plugin system\n#[allow(dead_code)]\nfn f() {}";
        let attrs = parse_suppress_attrs(content, Some("// JUSTIFIED:"));

        assert!(attrs[0].has_comment);
    }

    #[test]
    fn handles_multiple_attributes_on_item() {
        let content = "// Documented reason\n#[derive(Debug)]\n#[allow(dead_code)]\nstruct S;";
        let attrs = parse_suppress_attrs(content, None);

        // Should find the allow attribute and its comment (skipping #[derive])
        assert_eq!(attrs.len(), 1);
        assert!(attrs[0].has_comment);
    }

    #[test]
    fn clippy_lint_codes() {
        let content = "#[allow(clippy::unwrap_used, clippy::expect_used)]\nfn f() {}";
        let attrs = parse_suppress_attrs(content, None);

        assert_eq!(attrs[0].codes, vec!["clippy::unwrap_used", "clippy::expect_used"]);
    }
}
```

**Milestone:** All suppress parser unit tests pass.

**Verification:**
```bash
cargo test adapter::rust::tests::suppress_parsing
```

---

### Phase 4: Suppress Check Integration

Add suppress violation reporting to the escapes check.

**Update `crates/cli/src/checks/escapes.rs`:**

```rust
use crate::adapter::rust::{parse_suppress_attrs, SuppressAttr, CfgTestInfo};
use crate::config::{SuppressConfig, SuppressLevel};

/// Check suppress attributes in a Rust file.
fn check_suppress_violations(
    path: &Path,
    content: &str,
    config: &SuppressConfig,
    is_test_file: bool,
    cfg_info: Option<&CfgTestInfo>,
) -> Vec<Violation> {
    let mut violations = Vec::new();

    // Determine effective config based on source vs test
    let effective_check = if is_test_file {
        config.test.check.unwrap_or(SuppressLevel::Allow)
    } else {
        config.check
    };

    // If allow, no checking needed
    if effective_check == SuppressLevel::Allow {
        return violations;
    }

    // Parse suppress attributes
    let attrs = parse_suppress_attrs(content, config.comment.as_deref());

    for attr in attrs {
        // Check if this line is in test code (inline #[cfg(test)])
        let is_test_line = cfg_info
            .map(|info| info.is_test_line(attr.line))
            .unwrap_or(false);

        if is_test_line {
            // Use test policy
            let test_check = config.test.check.unwrap_or(SuppressLevel::Allow);
            if test_check == SuppressLevel::Allow {
                continue;
            }
        }

        // Get scope config (source or test)
        let scope_config = if is_test_file || is_test_line {
            &config.test
        } else {
            &config.source
        };

        // Check each lint code
        for code in &attr.codes {
            // Check forbid list first
            if scope_config.forbid.contains(code) {
                violations.push(Violation {
                    path: path.to_path_buf(),
                    line: attr.line + 1, // 1-indexed for output
                    pattern: format!("#[{}({})]", attr.kind, code),
                    message: format!(
                        "Suppressing `{}` is forbidden.",
                        code
                    ),
                    advice: "Remove the suppression or address the underlying issue.".to_string(),
                });
                continue;
            }

            // Check allow list (skip comment check)
            if scope_config.allow.contains(code) {
                continue;
            }

            // Check if comment is required
            if effective_check == SuppressLevel::Comment && !attr.has_comment {
                let advice = if let Some(ref pattern) = config.comment {
                    format!("Add a {} comment explaining why this suppression is needed.", pattern)
                } else {
                    "Add a comment above the attribute explaining why this suppression is needed.".to_string()
                };

                violations.push(Violation {
                    path: path.to_path_buf(),
                    line: attr.line + 1,
                    pattern: format!("#[{}({})]", attr.kind, code),
                    message: "Lint suppression requires justification.".to_string(),
                    advice,
                });
            }

            // Forbid level means all suppressions fail
            if effective_check == SuppressLevel::Forbid {
                violations.push(Violation {
                    path: path.to_path_buf(),
                    line: attr.line + 1,
                    pattern: format!("#[{}({})]", attr.kind, code),
                    message: "Lint suppressions are forbidden.".to_string(),
                    advice: "Remove the suppression and fix the underlying issue.".to_string(),
                });
            }
        }
    }

    violations
}
```

**Update `EscapesCheck::run()` to call `check_suppress_violations()` for Rust files.**

**Milestone:** Escapes check reports suppress violations based on configuration.

**Verification:**
```bash
cargo test checks::escapes -- suppress
```

---

### Phase 5: Test Fixtures

Create fixtures for behavioral specs.

**Create `tests/fixtures/rust/suppress-fail/`:**

```
Cargo.toml:
[package]
name = "suppress-fail"
version = "0.1.0"

quench.toml:
version = 1
[rust.suppress]
check = "comment"

src/lib.rs:
#[allow(dead_code)]
fn unused() {}
```

**Create `tests/fixtures/rust/suppress-ok/`:**

```
Cargo.toml:
[package]
name = "suppress-ok"
version = "0.1.0"

quench.toml:
version = 1
[rust.suppress]
check = "comment"

src/lib.rs:
// Reserved for future plugin system
#[allow(dead_code)]
fn plugin_hook() {}
```

**Create `tests/fixtures/rust/suppress-forbid/`:**

```
Cargo.toml:
[package]
name = "suppress-forbid"
version = "0.1.0"

quench.toml:
version = 1
[rust.suppress.source]
forbid = ["unsafe_code"]

src/lib.rs:
// Even with comment, this should fail
#[allow(unsafe_code)]
fn allow_unsafe() {}
```

**Milestone:** All fixtures created and accessible.

**Verification:**
```bash
ls tests/fixtures/rust/suppress-*
```

---

### Phase 6: Enable Behavioral Specs

Remove `#[ignore]` from suppress specs in `tests/specs/adapters/rust.rs`.

**Specs to enable:**
- `rust_adapter_allow_without_comment_fails_when_configured`
- `rust_adapter_allow_with_comment_passes`
- `rust_adapter_allow_in_test_code_always_passes`
- `rust_adapter_allow_list_skips_comment_check`
- `rust_adapter_forbid_list_always_fails`

**Note:** Lint policy specs (`lint_changes = "standalone"`) are out of scope for this phase.

**Milestone:** Five suppress specs pass without `#[ignore]`.

**Verification:**
```bash
cargo test --test specs rust_adapter_allow
cargo test --test specs rust_adapter_forbid
```

---

## Key Implementation Details

### Attribute Parsing Strategy

The parser uses simple string matching for `#[allow(...)]` and `#[expect(...)]`:

```
#[allow(dead_code)]           -> ["dead_code"]
#[allow(dead_code, unused)]   -> ["dead_code", "unused"]
#[expect(clippy::unwrap_used)] -> ["clippy::unwrap_used"]
```

**Limitations (acceptable for v1):**
- Multi-line attributes not fully supported
- Attributes inside macros may be missed
- Nested attributes not handled

### Comment Detection Strategy

The justification comment search looks upward from the attribute:

1. Skip blank lines (up to one)
2. Check for `//` comment lines
3. Stop at non-comment, non-attribute lines
4. If `comment` pattern specified, require it matches

```rust
// This is the justification    <- Found (line -2)
#[derive(Debug)]                 <- Skipped (attribute)
#[allow(dead_code)]              <- Target attribute
```

### Scope Resolution

Suppress policies are resolved in order:

1. **Per-code forbid**: Always fails, regardless of other settings
2. **Per-code allow**: Always passes, skip comment check
3. **Scope check level**: Test code uses `[rust.suppress.test]`, source uses base config
4. **Comment check**: If `check = "comment"`, require justification

### Integration with Test Code Detection

Uses existing infrastructure from Phase 310:

- File-level: Files in `tests/` or `*_test.rs` are test code
- Inline: Lines inside `#[cfg(test)]` blocks use test policy

```rust
pub fn f() {
    // Source code - uses [rust.suppress] policy
}

#[cfg(test)]
mod tests {
    #[allow(unused)]  // Test code - uses [rust.suppress.test] policy
    fn helper() {}
}
```

### Configuration Examples

**Default (comment required for source, allow for test):**
```toml
[rust.suppress]
check = "comment"

[rust.suppress.test]
check = "allow"
```

**Strict (custom pattern required):**
```toml
[rust.suppress]
check = "comment"
comment = "// JUSTIFIED:"
```

**With allow/forbid lists:**
```toml
[rust.suppress]
check = "comment"

[rust.suppress.source]
allow = ["dead_code", "unused"]    # No comment needed
forbid = ["unsafe_code"]            # Never allowed
```

**Forbid all suppressions:**
```toml
[rust.suppress]
check = "forbid"
```

## Verification Plan

### After Each Phase

```bash
# Compile check
cargo build

# Run relevant unit tests
cargo test adapter::rust -- suppress
cargo test config -- suppress
cargo test checks::escapes -- suppress

# Check lints
cargo clippy --all-targets --all-features -- -D warnings
```

### End-to-End Verification

```bash
# Run suppress specs
cargo test --test specs rust_adapter_allow
cargo test --test specs rust_adapter_forbid

# Full quality gates
make check
```

### Test Matrix

| Test Case | Input | Expected |
|-----------|-------|----------|
| `#[allow]` no comment | `#[allow(dead_code)]` | FAIL (comment required) |
| `#[allow]` with comment | `// reason\n#[allow(dead_code)]` | PASS |
| `#[expect]` no comment | `#[expect(unused)]` | FAIL (comment required) |
| In test file | `tests/test.rs: #[allow(...)]` | PASS (test policy) |
| In `#[cfg(test)]` | Inline test block | PASS (test policy) |
| Code in allow list | `allow = ["dead_code"]` | PASS (no comment needed) |
| Code in forbid list | `forbid = ["unsafe_code"]` | FAIL (always) |
| Custom pattern required | `// JUSTIFIED:` only | Matches pattern only |
| Forbid level | `check = "forbid"` | All suppressions fail |

### Manual Verification

```bash
# Create test project
mkdir /tmp/suppress-test && cd /tmp/suppress-test
cat > Cargo.toml << 'EOF'
[package]
name = "test"
version = "0.1.0"
EOF

cat > quench.toml << 'EOF'
version = 1
[rust.suppress]
check = "comment"
EOF

mkdir src
echo '#[allow(dead_code)]
fn unused() {}' > src/lib.rs

# Run quench
cargo run -- check --escapes

# Expected output:
# escapes: FAIL
#   src/lib.rs:1: #[allow(dead_code)]
#     Lint suppression requires justification.
#     Add a comment above the attribute explaining why this suppression is needed.
```

## Summary

| Phase | Task | Key Files | Status |
|-------|------|-----------|--------|
| 1 | Config schema | `config.rs` | [ ] Pending |
| 2 | Suppress parser | `adapter/rust.rs` | [ ] Pending |
| 3 | Parser unit tests | `adapter/rust_tests.rs` | [ ] Pending |
| 4 | Escapes integration | `checks/escapes.rs` | [ ] Pending |
| 5 | Test fixtures | `tests/fixtures/rust/suppress-*` | [ ] Pending |
| 6 | Enable specs | `tests/specs/adapters/rust.rs` | [ ] Pending |

## Future Phases

- **Phase 325**: Rust lint config policy (`lint_changes = "standalone"`)
- **Phase 330**: Shell adapter suppress patterns (`# shellcheck disable=`)
- **Phase 335**: `#[expect]` vs `#[allow]` differentiation (stricter for expect)
