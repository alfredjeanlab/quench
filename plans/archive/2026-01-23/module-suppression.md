# Module-Level Suppression Support

**Root Feature:** `quench-0125`

## Overview

Add support for Rust module-level lint suppressions using `#![allow(...)]` and `#![expect(...)]` inner attribute syntax. Currently, quench only recognizes outer attributes (`#[allow(...)]`) on items, but Rust also allows inner attributes that apply to the enclosing module or crate.

**Current behavior:**
- `#[allow(dead_code)]` - Supported (outer attribute on item)
- `#[expect(unused)]` - Supported (outer attribute on item)
- `#![allow(dead_code)]` - Not supported (inner attribute on module)

**Target behavior:**
- All three forms recognized and checked against suppress config
- Module-level attributes apply to the module they appear in
- Same comment requirements apply to inner and outer attributes

## Project Structure

```
quench/
├── crates/cli/src/
│   └── adapter/
│       └── rust/
│           ├── suppress.rs       # UPDATE: parse #![allow/expect] syntax
│           └── suppress_tests.rs # UPDATE: add inner attribute tests
├── docs/specs/
│   └── langs/
│       └── rust.md              # UPDATE: document #![...] support
├── tests/
│   ├── specs/
│   │   └── adapters/rust.rs     # UPDATE: add module-level specs
│   └── fixtures/
│       └── rust/
│           └── module-suppress/ # NEW: test fixture
└── plans/
    └── module-suppression.md
```

## Dependencies

No new external dependencies. Uses existing:
- String parsing already in `suppress.rs`
- `SuppressAttr` struct already captures needed info
- Existing test infrastructure in `tests/specs/`

## Implementation Phases

### Phase 1: Documentation Update

Update `docs/specs/langs/rust.md` to document module-level suppression support.

**Update the "Supported Patterns" section under Suppress:**

```markdown
### Supported Patterns

```rust
// Outer attribute (item-level)
#[allow(dead_code)]
fn unused() {}

// Inner attribute (module-level)
#![allow(clippy::unwrap_used)]

// Multi-line attribute
#[allow(
    dead_code,
    unused_variables
)]
fn unused() {}
```

**Note:** Inner attributes (`#![...]`) apply to the module or crate they appear in. They follow the same comment requirement rules as outer attributes.
```

**Milestone:** Spec documents inner attribute support.

**Verification:**
```bash
cat docs/specs/langs/rust.md | grep -A5 "Inner attribute"
```

---

### Phase 2: Test Fixture

Create fixture for module-level suppression behavioral spec.

**Create `tests/fixtures/rust/module-suppress/Cargo.toml`:**
```toml
[package]
name = "module-suppress"
version = "0.1.0"
edition = "2021"
```

**Create `tests/fixtures/rust/module-suppress/quench.toml`:**
```toml
version = 1
[rust.suppress]
check = "comment"
```

**Create `tests/fixtures/rust/module-suppress/src/lib.rs`:**
```rust
#![allow(dead_code)]

fn unused_function() {}

struct UnusedStruct;
```

**Milestone:** Fixture created with module-level `#![allow(...)]`.

**Verification:**
```bash
ls tests/fixtures/rust/module-suppress/
```

---

### Phase 3: Behavioral Specs

Add black-box tests for module-level suppression in `tests/specs/adapters/rust.rs`.

**Add specs:**

```rust
/// Spec: docs/specs/langs/rust.md#supported-patterns
///
/// > Inner attributes (#![...]) apply to the module or crate they appear in.
#[test]
fn rust_adapter_inner_allow_without_comment_fails_when_configured() {
    check("escapes")
        .on("rust/module-suppress")
        .fails()
        .stdout_has("#![allow");
}

/// Spec: docs/specs/langs/rust.md#supported-patterns
///
/// > They follow the same comment requirement rules as outer attributes.
#[test]
fn rust_adapter_inner_allow_with_comment_passes() {
    let dir = temp_project();
    std::fs::write(
        dir.path().join("quench.toml"),
        r#"
version = 1
[rust.suppress]
check = "comment"
"#,
    )
    .unwrap();
    std::fs::write(
        dir.path().join("Cargo.toml"),
        "[package]\nname = \"test\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    std::fs::create_dir_all(dir.path().join("src")).unwrap();
    std::fs::write(
        dir.path().join("src/lib.rs"),
        "// Module-wide suppression for test utilities\n#![allow(dead_code)]\n\nfn helper() {}",
    )
    .unwrap();

    check("escapes").pwd(dir.path()).passes();
}

/// Spec: docs/specs/langs/rust.md#supported-patterns
///
/// > Inner expect attributes are also supported
#[test]
fn rust_adapter_inner_expect_without_comment_fails() {
    let dir = temp_project();
    std::fs::write(
        dir.path().join("quench.toml"),
        r#"
version = 1
[rust.suppress]
check = "comment"
"#,
    )
    .unwrap();
    std::fs::write(
        dir.path().join("Cargo.toml"),
        "[package]\nname = \"test\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    std::fs::create_dir_all(dir.path().join("src")).unwrap();
    std::fs::write(
        dir.path().join("src/lib.rs"),
        "#![expect(unused)]\n\nfn f() {}",
    )
    .unwrap();

    check("escapes")
        .pwd(dir.path())
        .fails()
        .stdout_has("#![expect");
}
```

**Milestone:** Specs written and will fail until parser is updated.

**Verification:**
```bash
cargo test --test specs rust_adapter_inner -- --ignored 2>&1 | grep -c "ignored"
```

---

### Phase 4: Parser Update

Update `parse_suppress_line()` in `crates/cli/src/adapter/rust/suppress.rs` to recognize inner attribute syntax.

**Current code:**
```rust
fn parse_suppress_line(line: &str) -> Option<ParsedAttr> {
    let kind = if line.starts_with("#[allow(") {
        "allow"
    } else if line.starts_with("#[expect(") {
        "expect"
    } else {
        return None;
    };
    // ...
}
```

**Updated code:**
```rust
fn parse_suppress_line(line: &str) -> Option<ParsedAttr> {
    // Match both outer (#[...]) and inner (#![...]) attributes
    let kind = if line.starts_with("#[allow(") || line.starts_with("#![allow(") {
        "allow"
    } else if line.starts_with("#[expect(") || line.starts_with("#![expect(") {
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
```

**Milestone:** Parser recognizes `#![allow(...)]` and `#![expect(...)]`.

**Verification:**
```bash
cargo build
cargo test adapter::rust::tests -- inner
```

---

### Phase 5: Unit Tests

Add unit tests for inner attribute parsing in `crates/cli/src/adapter/rust/suppress_tests.rs`.

**Add tests:**

```rust
#[test]
fn detects_inner_allow_attribute() {
    let content = "#![allow(dead_code)]\nfn unused() {}";
    let attrs = parse_suppress_attrs(content, None);

    assert_eq!(attrs.len(), 1);
    assert_eq!(attrs[0].kind, "allow");
    assert_eq!(attrs[0].codes, vec!["dead_code"]);
}

#[test]
fn detects_inner_expect_attribute() {
    let content = "#![expect(unused)]\nlet _x = 42;";
    let attrs = parse_suppress_attrs(content, None);

    assert_eq!(attrs.len(), 1);
    assert_eq!(attrs[0].kind, "expect");
    assert_eq!(attrs[0].codes, vec!["unused"]);
}

#[test]
fn detects_inner_attribute_with_comment() {
    let content = "// Module suppression for FFI compatibility\n#![allow(unsafe_code)]\n";
    let attrs = parse_suppress_attrs(content, None);

    assert_eq!(attrs.len(), 1);
    assert!(attrs[0].has_comment);
    assert_eq!(
        attrs[0].comment_text,
        Some("Module suppression for FFI compatibility".to_string())
    );
}

#[test]
fn detects_mixed_inner_and_outer_attributes() {
    let content = "#![allow(dead_code)]\n\n#[allow(unused)]\nfn f() {}";
    let attrs = parse_suppress_attrs(content, None);

    assert_eq!(attrs.len(), 2);
    assert_eq!(attrs[0].codes, vec!["dead_code"]);
    assert_eq!(attrs[1].codes, vec!["unused"]);
}

#[test]
fn inner_attribute_with_multiple_codes() {
    let content = "#![allow(dead_code, unused_variables, clippy::unwrap_used)]\n";
    let attrs = parse_suppress_attrs(content, None);

    assert_eq!(attrs.len(), 1);
    assert_eq!(
        attrs[0].codes,
        vec!["dead_code", "unused_variables", "clippy::unwrap_used"]
    );
}
```

**Milestone:** All inner attribute unit tests pass.

**Verification:**
```bash
cargo test adapter::rust::tests::detects_inner
cargo test adapter::rust::tests::inner_attribute
```

---

### Phase 6: Verification & Cleanup

Run full test suite and verify specs pass.

**Enable behavioral specs** (remove `#[ignore]` if added):
- `rust_adapter_inner_allow_without_comment_fails_when_configured`
- `rust_adapter_inner_allow_with_comment_passes`
- `rust_adapter_inner_expect_without_comment_fails`

**Milestone:** All tests pass, no regressions.

**Verification:**
```bash
make check
cargo test --test specs rust_adapter_inner
```

---

## Key Implementation Details

### Inner vs Outer Attribute Syntax

| Syntax | Type | Applies To |
|--------|------|------------|
| `#[allow(...)]` | Outer | Following item |
| `#![allow(...)]` | Inner | Enclosing module/crate |

Both forms follow the same suppression rules:
- Check level (forbid/comment/allow)
- Comment requirements
- Allow/forbid lists

### Parser Changes Summary

The only change needed is in `parse_suppress_line()`:

```rust
// Before: only outer attributes
line.starts_with("#[allow(")

// After: both inner and outer
line.starts_with("#[allow(") || line.starts_with("#![allow(")
```

This minimal change works because:
1. The parentheses extraction logic (`line.find('(')`) handles both forms
2. The comment detection already searches upward from any line
3. The `SuppressAttr` struct doesn't need to distinguish inner/outer

### Scope Behavior

Module-level (`#!`) attributes conceptually apply to all items in the module. However, quench treats them the same as item-level attributes for violation checking:
- Both require justification comments (if configured)
- Both respect allow/forbid lists
- Both respect source vs test scope

This simplification is acceptable because quench's goal is ensuring suppressions are documented, not enforcing Rust's scope rules.

## Verification Plan

### After Each Phase

```bash
# Compile check
cargo build

# Run unit tests
cargo test adapter::rust -- suppress

# Check lints
cargo clippy --all-targets --all-features -- -D warnings
```

### End-to-End Verification

```bash
# Run all suppress-related specs
cargo test --test specs rust_adapter_allow
cargo test --test specs rust_adapter_inner

# Full quality gates
make check
```

### Test Matrix

| Test Case | Input | Expected |
|-----------|-------|----------|
| `#![allow]` no comment | `#![allow(dead_code)]` | FAIL (comment required) |
| `#![allow]` with comment | `// reason\n#![allow(dead_code)]` | PASS |
| `#![expect]` no comment | `#![expect(unused)]` | FAIL (comment required) |
| Mixed inner/outer | Both in same file | Both checked |
| In test file | `tests/test.rs: #![allow(...)]` | PASS (test policy) |
| Code in forbid list | `#![allow(unsafe_code)]` with forbid | FAIL (always) |

### Manual Verification

```bash
# Create test project
mkdir /tmp/inner-suppress-test && cd /tmp/inner-suppress-test
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
echo '#![allow(dead_code)]
fn unused() {}' > src/lib.rs

# Run quench
quench check --escapes

# Expected output:
# escapes: FAIL
#   src/lib.rs:1: #![allow(dead_code)]
#     Lint suppression requires justification.
```

## Summary

| Phase | Task | Key Files | Status |
|-------|------|-----------|--------|
| 1 | Documentation | `docs/specs/langs/rust.md` | [ ] Pending |
| 2 | Test fixture | `tests/fixtures/rust/module-suppress/` | [ ] Pending |
| 3 | Behavioral specs | `tests/specs/adapters/rust.rs` | [ ] Pending |
| 4 | Parser update | `adapter/rust/suppress.rs` | [ ] Pending |
| 5 | Unit tests | `adapter/rust/suppress_tests.rs` | [ ] Pending |
| 6 | Verification | Full test suite | [ ] Pending |
