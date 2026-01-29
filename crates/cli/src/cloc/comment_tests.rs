// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;

// =============================================================================
// Rust comment detection
// =============================================================================

#[test]
fn rust_single_line_comments() {
    let style = comment_style("rs").unwrap();
    let content = "// this is a comment\nfn main() {}\n// another comment\n";
    let counts = count_lines(content, &style);
    assert_eq!(
        counts,
        LineCounts {
            blank: 0,
            comment: 2,
            code: 1
        }
    );
}

#[test]
fn rust_block_comment() {
    let style = comment_style("rs").unwrap();
    let content = "/* block\n   comment */\nfn main() {}\n";
    let counts = count_lines(content, &style);
    assert_eq!(
        counts,
        LineCounts {
            blank: 0,
            comment: 2,
            code: 1
        }
    );
}

#[test]
fn rust_mixed_blank_comment_code() {
    let style = comment_style("rs").unwrap();
    let content = "\n// comment\n\nfn foo() {}\n\n// end\n";
    let counts = count_lines(content, &style);
    assert_eq!(
        counts,
        LineCounts {
            blank: 3,
            comment: 2,
            code: 1
        }
    );
}

#[test]
fn rust_block_comment_with_code_after_close() {
    let style = comment_style("rs").unwrap();
    let content = "/* comment */ let x = 1;\n";
    let counts = count_lines(content, &style);
    // Opening line has code after close -> code
    assert_eq!(
        counts,
        LineCounts {
            blank: 0,
            comment: 0,
            code: 1
        }
    );
}

#[test]
fn rust_multiline_block_with_code_after_close() {
    let style = comment_style("rs").unwrap();
    // Line 1: "/* start" -> opens block comment (comment)
    // Line 2: "  middle" -> inside block (comment)
    // Line 3: "  end */ let x = 1;" -> closes block, code after (code)
    // Line 4: "fn foo() {}" -> code
    let content = "/* start\n  middle\n  end */ let x = 1;\nfn foo() {}\n";
    let counts = count_lines(content, &style);
    assert_eq!(
        counts,
        LineCounts {
            blank: 0,
            comment: 2,
            code: 2
        }
    );
}

// =============================================================================
// Python comment detection
// =============================================================================

#[test]
fn python_hash_comments() {
    let style = comment_style("py").unwrap();
    let content = "# comment\ndef foo():\n    # inline\n    pass\n";
    let counts = count_lines(content, &style);
    assert_eq!(
        counts,
        LineCounts {
            blank: 0,
            comment: 2,
            code: 2
        }
    );
}

#[test]
fn python_no_block_comments() {
    let style = comment_style("py").unwrap();
    // Python doesn't have block comments in our model
    let content = "# comment\ndef foo():\n    pass\n";
    let counts = count_lines(content, &style);
    assert_eq!(
        counts,
        LineCounts {
            blank: 0,
            comment: 1,
            code: 2
        }
    );
}

// =============================================================================
// Go comment detection
// =============================================================================

#[test]
fn go_comments() {
    let style = comment_style("go").unwrap();
    let content = "// Package main\npackage main\n\n/* multi\n   line */\nfunc main() {}\n";
    let counts = count_lines(content, &style);
    assert_eq!(
        counts,
        LineCounts {
            blank: 1,
            comment: 3,
            code: 2
        }
    );
}

// =============================================================================
// Lua comment detection
// =============================================================================

#[test]
fn lua_comments() {
    let style = comment_style("lua").unwrap();
    let content = "-- single line\n--[[ block\ncomment ]]\nlocal x = 1\n";
    let counts = count_lines(content, &style);
    assert_eq!(
        counts,
        LineCounts {
            blank: 0,
            comment: 3,
            code: 1
        }
    );
}

// =============================================================================
// PHP comment detection
// =============================================================================

#[test]
fn php_dual_line_comments() {
    let style = comment_style("php").unwrap();
    let content = "// C-style\n# shell-style\n$x = 1;\n/* block */\n";
    let counts = count_lines(content, &style);
    assert_eq!(
        counts,
        LineCounts {
            blank: 0,
            comment: 3,
            code: 1
        }
    );
}

// =============================================================================
// Edge cases
// =============================================================================

#[test]
fn empty_content() {
    let style = comment_style("rs").unwrap();
    let counts = count_lines("", &style);
    assert_eq!(
        counts,
        LineCounts {
            blank: 0,
            comment: 0,
            code: 0
        }
    );
}

#[test]
fn all_blank() {
    let style = comment_style("rs").unwrap();
    let counts = count_lines("\n\n\n", &style);
    assert_eq!(
        counts,
        LineCounts {
            blank: 3,
            comment: 0,
            code: 0
        }
    );
}

#[test]
fn unknown_extension() {
    assert!(comment_style("xyz").is_none());
}

#[test]
fn single_line_block_comment() {
    let style = comment_style("rs").unwrap();
    let content = "/* single-line block comment */\ncode();\n";
    let counts = count_lines(content, &style);
    assert_eq!(
        counts,
        LineCounts {
            blank: 0,
            comment: 1,
            code: 1
        }
    );
}

#[test]
fn nested_looking_block_comment() {
    // We don't support true nesting; just find the close delimiter
    let style = comment_style("rs").unwrap();
    let content = "/* outer /* inner */ code();\n";
    let counts = count_lines(content, &style);
    // The first `*/` closes the block, `code();` is code on same line
    assert_eq!(
        counts,
        LineCounts {
            blank: 0,
            comment: 0,
            code: 1
        }
    );
}

#[test]
fn bat_rem_comment() {
    let style = comment_style("bat").unwrap();
    let content = "REM this is a comment\n:: another comment\necho hello\n";
    let counts = count_lines(content, &style);
    assert_eq!(
        counts,
        LineCounts {
            blank: 0,
            comment: 2,
            code: 1
        }
    );
}

#[test]
fn vue_html_comment() {
    let style = comment_style("vue").unwrap();
    let content = "<!-- comment -->\n<template>\n</template>\n";
    let counts = count_lines(content, &style);
    assert_eq!(
        counts,
        LineCounts {
            blank: 0,
            comment: 1,
            code: 2
        }
    );
}

#[test]
fn perl_pod_block() {
    let style = comment_style("pl").unwrap();
    let content = "=pod\nDocumentation here\n=cut\nmy $x = 1;\n";
    let counts = count_lines(content, &style);
    assert_eq!(
        counts,
        LineCounts {
            blank: 0,
            comment: 3,
            code: 1
        }
    );
}

#[test]
fn powershell_block_comment() {
    let style = comment_style("ps1").unwrap();
    let content = "<#\nblock comment\n#>\n$x = 1\n";
    let counts = count_lines(content, &style);
    assert_eq!(
        counts,
        LineCounts {
            blank: 0,
            comment: 3,
            code: 1
        }
    );
}
