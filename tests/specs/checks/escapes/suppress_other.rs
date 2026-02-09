// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Shell and Go suppress message specs.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::prelude::*;

// =============================================================================
// SHELL SUPPRESS MESSAGE SPECS
// =============================================================================

/// Spec: docs/specs/langs/shell.md#violation-messages
///
/// > ShellCheck SC2086 should ask "Is unquoted expansion intentional here?"
#[test]
fn shell_suppress_sc2086_message() {
    let temp = Project::empty();
    temp.config(
        r#"[shell.suppress]
check = "comment"
"#,
    );

    temp.file("script.sh", "#!/bin/bash\n# shellcheck disable=SC2086\necho $var");

    check("escapes").pwd(temp.path()).fails().stdout_eq(
        r#"escapes: FAIL
  script.sh:2: shellcheck_missing_comment: # shellcheck disable=SC2086
    Quote the variable expansion to prevent word splitting.
    Use "$var" instead of $var unless word splitting is intentionally needed.
    Only if the lint is a false positive, add a comment above the directive.

FAIL: escapes
"#,
    );
}

/// Spec: docs/specs/langs/shell.md#violation-messages
///
/// > ShellCheck SC2154 should ask "Is this variable defined externally?"
#[test]
fn shell_suppress_sc2154_message() {
    let temp = Project::empty();
    temp.config(
        r##"[shell.suppress]
check = "comment"

[shell.suppress.source.SC2154]
comment = "# EXTERNAL:"
"##,
    );

    temp.file("script.sh", "#!/bin/bash\n# shellcheck disable=SC2154\necho $external_var");

    check("escapes").pwd(temp.path()).fails().stdout_eq(
        r##"escapes: FAIL
  script.sh:2: shellcheck_missing_comment: # shellcheck disable=SC2154
    Define this variable before use or document its external source.
    If set by the shell environment, add a comment explaining where it comes from.
    Only if fixing is not feasible, add:
      # EXTERNAL: ...

FAIL: escapes
"##,
    );
}

/// Spec: docs/specs/langs/shell.md#violation-messages
///
/// > ShellCheck SC2034 should ask "Is this unused variable needed?"
#[test]
fn shell_suppress_sc2034_message() {
    let temp = Project::empty();
    temp.config(
        r#"[shell.suppress]
check = "comment"
"#,
    );

    temp.file("script.sh", "#!/bin/bash\n# shellcheck disable=SC2034\nunused_var=1");

    check("escapes").pwd(temp.path()).fails().stdout_eq(
        r#"escapes: FAIL
  script.sh:2: shellcheck_missing_comment: # shellcheck disable=SC2034
    Remove this unused variable.
    If the variable is used externally, export it or add a comment explaining its purpose.
    Only if the lint is a false positive, add a comment above the directive.

FAIL: escapes
"#,
    );
}

/// Spec: docs/specs/langs/shell.md#violation-messages
///
/// > Unknown ShellCheck code should use generic message
#[test]
fn shell_suppress_unknown_code_message() {
    let temp = Project::empty();
    temp.config(
        r#"[shell.suppress]
check = "comment"
"#,
    );

    temp.file("script.sh", "#!/bin/bash\n# shellcheck disable=SC9999\necho test");

    check("escapes").pwd(temp.path()).fails().stdout_eq(
        r#"escapes: FAIL
  script.sh:2: shellcheck_missing_comment: # shellcheck disable=SC9999
    Fix the ShellCheck warning instead of suppressing it.
    ShellCheck warnings usually indicate real issues or portability problems.
    Only if the lint is a false positive, add a comment above the directive.

FAIL: escapes
"#,
    );
}

// =============================================================================
// GO SUPPRESS MESSAGE SPECS
// =============================================================================

/// Spec: docs/specs/langs/golang.md#violation-messages
///
/// > Go nolint without specific pattern should use inline example
#[test]
fn go_suppress_no_pattern_message() {
    let temp = Project::empty();
    temp.config(
        r#"[golang.suppress]
check = "comment"
"#,
    );

    temp.file("go.mod", "module test\ngo 1.21\n");
    temp.file("main.go", "package main\n//nolint:errcheck\nfunc test() { _ = doSomething() }");

    check("escapes").pwd(temp.path()).fails().stdout_eq(
        r#"escapes: FAIL
  main.go:2: suppress_missing_comment: //nolint:errcheck
    Handle this error properly.
    Add error handling or explicitly check and handle the error case.
    Only if the lint is a false positive, add a comment above the directive or inline (//nolint:code // reason).

FAIL: escapes
"#,
    );
}

/// Spec: docs/specs/langs/golang.md#violation-messages
///
/// > Go nolint with specific pattern configured
#[test]
fn go_suppress_with_pattern_message() {
    let temp = Project::empty();
    temp.config(
        r#"[golang.suppress]
check = "comment"

[golang.suppress.source.gosec]
comment = "// FALSE_POSITIVE:"
"#,
    );

    temp.file("go.mod", "module test\ngo 1.21\n");
    temp.file("main.go", "package main\n//nolint:gosec\nfunc test() { }");

    check("escapes").pwd(temp.path()).fails().stdout_eq(
        r#"escapes: FAIL
  main.go:2: suppress_missing_comment: //nolint:gosec
    Address the security issue identified by gosec.
    Review the security finding and apply the recommended fix.
    Only if fixing is not feasible, add:
      // FALSE_POSITIVE: ...

FAIL: escapes
"#,
    );
}
