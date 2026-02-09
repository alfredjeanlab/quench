// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Rust suppress specs: per-lint patterns, source scope overrides,
//! module-level suppression, and suppress message format.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::prelude::*;

// =============================================================================
// PER-LINT PATTERN SPECS
// =============================================================================

/// Spec: Per-lint comment pattern for Rust suppress
///
/// > Per-lint-code comment patterns override global pattern.
/// > #[allow(dead_code)] with per-lint pattern requires that specific pattern.
#[test]
fn suppress_per_lint_pattern_respected_for_rust() {
    let temp = Project::empty();
    temp.config(
        r#"[rust.suppress]
check = "comment"

[rust.suppress.source.dead_code]
comment = "// NOTE(compat):"
"#,
    );
    temp.file("Cargo.toml", "[package]\nname = \"test\"\nversion = \"0.1.0\"");
    // Using per-lint pattern should pass
    temp.file(
        "src/lib.rs",
        "// NOTE(compat): legacy API\n#[allow(dead_code)]\nfn old_function() {}",
    );

    check("escapes").pwd(temp.path()).passes();
}

/// Spec: Per-lint comment pattern rejection
///
/// > When per-lint pattern is configured but comment doesn't match, should fail.
#[test]
fn suppress_per_lint_pattern_wrong_comment_fails() {
    let temp = Project::empty();
    temp.config(
        r#"[rust.suppress]
check = "comment"

[rust.suppress.source.dead_code]
comment = "// NOTE(compat):"
"#,
    );
    temp.file("Cargo.toml", "[package]\nname = \"test\"\nversion = \"0.1.0\"");
    // Using wrong pattern should fail
    temp.file("src/lib.rs", "// Some other comment\n#[allow(dead_code)]\nfn old_function() {}");

    let escapes = check("escapes").pwd(temp.path()).json().fails();
    let violations = escapes.require("violations").as_array().unwrap();
    assert!(
        violations.iter().any(|v| {
            v.get("type").and_then(|t| t.as_str()) == Some("suppress_missing_comment")
        }),
        "should have suppress_missing_comment violation"
    );
    // Error message should reference the per-lint pattern
    let advice = violations[0].get("advice").and_then(|a| a.as_str()).unwrap();
    assert!(advice.contains("NOTE(compat)"), "advice should mention per-lint pattern");
}

/// Spec: Fallback to global pattern when no per-lint pattern
///
/// > When no per-lint pattern is configured for a lint code, fall back to global.
#[test]
fn suppress_fallback_to_global_pattern() {
    let temp = Project::empty();
    temp.config(
        r#"[rust.suppress]
check = "comment"
comment = "// REASON:"

[rust.suppress.source.dead_code]
comment = "// NOTE(compat):"
"#,
    );
    temp.file("Cargo.toml", "[package]\nname = \"test\"\nversion = \"0.1.0\"");
    // unused_variables has no per-lint pattern, should use global
    temp.file(
        "src/lib.rs",
        "// REASON: needed for testing\n#[allow(unused_variables)]\nfn test_fn() { let x = 1; }",
    );

    check("escapes").pwd(temp.path()).passes();
}

/// Spec: Per-lint pattern for Shell suppress
///
/// > Shell shellcheck directives also support per-lint patterns.
#[test]
fn suppress_per_lint_pattern_respected_for_shell() {
    let temp = Project::empty();
    temp.config(
        r##"[shell.suppress]
check = "comment"

[shell.suppress.source.SC2034]
comment = "# UNUSED_VAR:"
"##,
    );
    // Using per-lint pattern should pass
    temp.file(
        "scripts/build.sh",
        r#"#!/bin/bash
# UNUSED_VAR: set by external caller
# shellcheck disable=SC2034
MY_VAR="value"
"#,
    );

    check("escapes").pwd(temp.path()).passes();
}

/// Spec: Per-lint pattern wrong comment for Shell
///
/// > Shell per-lint pattern should reject wrong comment patterns.
#[test]
fn suppress_per_lint_pattern_wrong_comment_fails_shell() {
    let temp = Project::empty();
    temp.config(
        r##"[shell.suppress]
check = "comment"

[shell.suppress.source.SC2034]
comment = "# UNUSED_VAR:"
"##,
    );
    // Using wrong pattern should fail
    temp.file(
        "scripts/build.sh",
        r#"#!/bin/bash
# Some other reason
# shellcheck disable=SC2034
MY_VAR="value"
"#,
    );

    let escapes = check("escapes").pwd(temp.path()).json().fails();
    let violations = escapes.require("violations").as_array().unwrap();
    assert!(
        violations.iter().any(|v| {
            v.get("type").and_then(|t| t.as_str()) == Some("shellcheck_missing_comment")
        }),
        "should have shellcheck_missing_comment violation"
    );
    // Error message should reference the per-lint pattern
    let advice = violations[0].get("advice").and_then(|a| a.as_str()).unwrap();
    assert!(advice.contains("UNUSED_VAR"), "advice should mention per-lint pattern");
}

// =============================================================================
// SOURCE SCOPE OVERRIDE SPECS
// =============================================================================

/// Spec: Source scope check should override base level
///
/// > When [rust.suppress].check = "allow" but [rust.suppress.source].check = "comment",
/// > source files should require comments for suppressions.
#[test]
fn rust_suppress_source_scope_overrides_base() {
    check("escapes")
        .on("rust/source-scope-override")
        .fails()
        .stdout_has("dead_code")
        .stdout_has("// KEEP UNTIL:"); // dead_code has default pattern
}

// =============================================================================
// MODULE-LEVEL SUPPRESSION SPECS
// =============================================================================

/// Spec: Module-level suppressions should be detected
///
/// > Inner attributes #![allow(...)] and #![expect(...)] should be checked
/// > the same as outer attributes #[allow(...)].
#[test]
fn rust_suppress_detects_module_level_allow() {
    check("escapes")
        .on("rust/module-allow")
        .fails()
        .stdout_has("dead_code")
        .stdout_has("// KEEP UNTIL:");
}

/// Spec: docs/specs/checks/escape-hatches.md#lint-suppression-messages
///
/// > Suppress missing comment messages provide:
/// > 1. Primary instruction to fix the issue
/// > 2. Context and guidance on how to fix
/// > 3. Suppression as last resort with acceptable patterns
#[test]
fn suppress_missing_comment_message_format() {
    let temp = Project::empty();
    temp.config(
        r#"[rust.suppress]
check = "comment"
"#,
    );
    temp.file("Cargo.toml", "[package]\nname = \"test\"\nversion = \"0.1.0\"");

    // Test dead_code with multiple patterns
    temp.file("src/lib.rs", "#[allow(dead_code)]\nfn unused() {}");

    check("escapes").pwd(temp.path()).fails().stdout_eq(
        r#"escapes: FAIL
  src/lib.rs:1: suppress_missing_comment: #[allow(dead_code)]
    Remove this dead code.
    Dead code should be deleted to keep the codebase clean and maintainable.
    Only if fixing is not feasible, add one of:
      // KEEP UNTIL: ...
      // NOTE(compat): ...
      // NOTE(compatibility): ...
      // NOTE(lifetime): ...

FAIL: escapes
"#,
    );
}

/// Spec: docs/specs/checks/escape-hatches.md#lint-suppression-messages
///
/// > Single pattern should use "If not, add:" format
#[test]
fn suppress_missing_comment_single_pattern() {
    let temp = Project::empty();
    temp.config(
        r#"[rust.suppress]
check = "comment"
"#,
    );
    temp.file("Cargo.toml", "[package]\nname = \"test\"\nversion = \"0.1.0\"");

    temp.file(
        "src/lib.rs",
        "#[allow(clippy::too_many_arguments)]\nfn many_args(a: i32, b: i32, c: i32, d: i32, e: i32, f: i32, g: i32, h: i32) {}",
    );

    check("escapes").pwd(temp.path()).fails().stdout_eq(
        r#"escapes: FAIL
  src/lib.rs:1: suppress_missing_comment: #[allow(clippy::too_many_arguments)]
    Refactor this function to use fewer arguments.
    Consider grouping related parameters into a struct or using the builder pattern.
    Only if fixing is not feasible, add:
      // TODO(refactor): ...

FAIL: escapes
"#,
    );
}

/// Spec: docs/specs/checks/escape-hatches.md#lint-suppression-messages
///
/// > Cast truncation should ask "Is this cast safe?"
#[test]
fn suppress_missing_comment_cast_truncation() {
    let temp = Project::empty();
    temp.config(
        r#"[rust.suppress]
check = "comment"
"#,
    );
    temp.file("Cargo.toml", "[package]\nname = \"test\"\nversion = \"0.1.0\"");

    temp.file(
        "src/lib.rs",
        "#[allow(clippy::cast_possible_truncation)]\nfn cast_fn() { let _x = 1000_u64 as u8; }",
    );

    check("escapes").pwd(temp.path()).fails().stdout_eq(
        r#"escapes: FAIL
  src/lib.rs:1: suppress_missing_comment: #[allow(clippy::cast_possible_truncation)]
    Verify this cast is safe and won't truncate data.
    Add explicit bounds checking or use safe conversion methods (e.g., try_into).
    Only if fixing is not feasible, add one of:
      // CORRECTNESS: ...
      // SAFETY: ...

FAIL: escapes
"#,
    );
}

/// Spec: docs/specs/checks/escape-hatches.md#lint-suppression-messages
///
/// > No specific pattern: generic message with example
#[test]
fn suppress_missing_comment_no_specific_pattern() {
    let temp = Project::empty();
    temp.config(
        r#"[rust.suppress]
check = "comment"
# No global pattern, no per-lint patterns for unused_variables
"#,
    );
    temp.file("Cargo.toml", "[package]\nname = \"test\"\nversion = \"0.1.0\"");

    temp.file("src/lib.rs", "#[allow(unused_variables)]\nfn test() { let x = 1; }");

    check("escapes").pwd(temp.path()).fails().stdout_eq(
        r#"escapes: FAIL
  src/lib.rs:1: suppress_missing_comment: #[allow(unused_variables)]
    Fix the underlying issue instead of suppressing the lint.
    Suppressions should only be used when the lint is a false positive.
    Only if the lint is a false positive, add a comment above the attribute.

FAIL: escapes
"#,
    );
}
