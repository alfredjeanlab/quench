// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Edge case specs: dedup, embedded comments, and comment-only false positives.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::prelude::*;

// =============================================================================
// EDGE CASE SPECS - Checkpoint 3C Fixes
// =============================================================================

/// Spec: Edge case - pattern in both code and comment
///
/// > When escape pattern appears in code AND in a comment on the same line,
/// > only one violation should be reported for that line.
#[test]
fn escapes_single_violation_per_line_even_with_pattern_in_comment() {
    let temp = Project::empty();
    temp.config(
        r#"[[check.escapes.patterns]]
name = "unwrap"
pattern = "\\.unwrap\\(\\)"
action = "forbid"
"#,
    );
    // Pattern appears twice on same line: in code AND in comment
    temp.file(
        "src/lib.rs",
        "pub fn f() { None::<i32>.unwrap() } // using .unwrap() here\n",
    );

    let escapes = check("escapes").pwd(temp.path()).json().fails();
    let violations = escapes.require("violations").as_array().unwrap();

    // Should only have ONE violation, not two
    assert_eq!(
        violations.len(),
        1,
        "should have exactly one violation, not multiple for same line"
    );
}

/// Spec: Edge case - embedded comment pattern
///
/// > Comment pattern embedded in other text should NOT satisfy the requirement.
/// > For example, `// VIOLATION: missing // SAFETY:` should not match `// SAFETY:`.
#[test]
fn escapes_comment_embedded_in_text_does_not_satisfy() {
    let temp = Project::empty();
    temp.config(
        r#"[[check.escapes.patterns]]
name = "unsafe"
pattern = "unsafe\\s*\\{"
action = "comment"
comment = "// SAFETY:"
"#,
    );
    // The // SAFETY: is embedded in another comment, not at comment start
    temp.file(
        "src/lib.rs",
        "unsafe { }  // VIOLATION: missing // SAFETY: comment\n",
    );

    // This should FAIL because the embedded // SAFETY: should not count
    let escapes = check("escapes").pwd(temp.path()).json().fails();
    let violations = escapes.require("violations").as_array().unwrap();

    assert!(
        !violations.is_empty(),
        "should have violation - embedded pattern should not satisfy requirement"
    );
    assert!(
        violations
            .iter()
            .any(|v| { v.get("type").and_then(|t| t.as_str()) == Some("missing_comment") }),
        "should be missing_comment violation"
    );
}

/// Spec: Edge case - comment at start is valid
///
/// > Comment pattern at start of inline comment should satisfy requirement.
#[test]
fn escapes_comment_at_start_of_inline_comment_satisfies() {
    let temp = Project::empty();
    temp.config(
        r#"[[check.escapes.patterns]]
name = "unsafe"
pattern = "unsafe\\s*\\{"
action = "comment"
comment = "// SAFETY:"
"#,
    );
    // The // SAFETY: is at start of the inline comment
    temp.file("src/lib.rs", "unsafe { }  // SAFETY: pointer is valid\n");

    // This should PASS
    check("escapes").pwd(temp.path()).passes();
}

// =============================================================================
// COMMENT-ONLY FALSE POSITIVE SPECS
// =============================================================================

/// Spec: Pattern in comment only should not trigger violation
///
/// > When an escape pattern appears only in a comment (not in actual code),
/// > it should NOT generate a violation. This prevents false positives from
/// > documentation or explanatory comments.
#[test]
fn escapes_pattern_in_comment_only_does_not_trigger_violation() {
    let temp = Project::empty();
    temp.config(
        r#"[[check.escapes.patterns]]
name = "unsafe"
pattern = "unsafe\\s*\\{"
action = "comment"
comment = "// SAFETY:"
"#,
    );
    // Pattern "unsafe {" appears only in the comment, not in actual code
    temp.file(
        "src/lib.rs",
        r#"
// Don't use unsafe { } blocks without a SAFETY comment
pub fn safe_function() -> i32 {
    42
}
"#,
    );

    // Should PASS - no actual unsafe block in code, only mentioned in comment
    check("escapes").pwd(temp.path()).passes();
}

/// Spec: Shell pattern in comment only should not trigger violation
///
/// > Shell escape patterns like `eval` appearing only in comments should not
/// > generate violations.
#[test]
fn escapes_shell_pattern_in_comment_only_does_not_trigger_violation() {
    let temp = default_project();
    // "eval" appears only in comment text, not as actual code
    temp.file(
        "scripts/build.sh",
        r#"#!/bin/bash
# This variable is used with eval in the calling script
export MY_VAR="value"
"#,
    );

    // Should PASS - no actual eval in code, only mentioned in comment
    check("escapes").pwd(temp.path()).passes();
}

/// Spec: Pattern in code triggers violation even with same pattern in comment
///
/// > When pattern appears in both code AND comment, the code occurrence
/// > should still trigger a violation (unless properly justified).
#[test]
fn escapes_pattern_in_code_triggers_even_when_also_in_comment() {
    let temp = default_project();
    // "eval" appears in comment AND in actual code
    temp.file(
        "scripts/build.sh",
        r#"#!/bin/bash
# Using eval here
eval "$CMD"
"#,
    );

    // Should FAIL - actual eval in code without # OK: comment
    check("escapes")
        .pwd(temp.path())
        .fails()
        .stdout_has("missing_comment");
}
