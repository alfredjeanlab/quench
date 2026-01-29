// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;

#[test]
fn count_file_metrics_rust() {
    let content = "// comment\n\nfn main() {\n    println!(\"hello\");\n}\n";
    let m = count_file_metrics(content, "rs");
    assert_eq!(m.lines, 5);
    assert_eq!(m.blank, 1);
    assert_eq!(m.comment, 1);
    assert_eq!(m.code, 3);
    assert_eq!(m.nonblank, 4);
}

#[test]
fn count_file_metrics_python() {
    let content = "# comment\n\ndef foo():\n    pass\n";
    let m = count_file_metrics(content, "py");
    assert_eq!(m.lines, 4);
    assert_eq!(m.blank, 1);
    assert_eq!(m.comment, 1);
    assert_eq!(m.code, 2);
}

#[test]
fn count_file_metrics_unknown_extension() {
    let content = "line 1\n\nline 3\n";
    let m = count_file_metrics(content, "xyz");
    assert_eq!(m.lines, 3);
    assert_eq!(m.blank, 1);
    assert_eq!(m.comment, 0);
    assert_eq!(m.code, 2);
}

#[test]
fn count_file_metrics_tokens() {
    // 12 chars = 3 tokens
    let content = "hello world!";
    let m = count_file_metrics(content, "rs");
    assert_eq!(m.tokens, 3);
}

#[test]
fn language_name_known() {
    assert_eq!(language_name("rs"), "Rust");
    assert_eq!(language_name("go"), "Go");
    assert_eq!(language_name("py"), "Python");
    assert_eq!(language_name("js"), "JavaScript");
    assert_eq!(language_name("jsx"), "JavaScript");
    assert_eq!(language_name("ts"), "TypeScript");
    assert_eq!(language_name("tsx"), "TypeScript");
    assert_eq!(language_name("sh"), "Shell");
    assert_eq!(language_name("bash"), "Shell");
}

#[test]
fn language_name_unknown() {
    // Unknown extensions use the extension itself
    assert_eq!(language_name("xyz"), "xyz");
}

#[test]
fn is_text_extension_known() {
    assert!(is_text_extension("rs"));
    assert!(is_text_extension("py"));
    assert!(is_text_extension("js"));
    assert!(is_text_extension("tsx"));
    assert!(is_text_extension("sh"));
    assert!(is_text_extension("mjs"));
    assert!(is_text_extension("cts"));
}

#[test]
fn is_text_extension_unknown() {
    assert!(!is_text_extension("toml"));
    assert!(!is_text_extension("md"));
    assert!(!is_text_extension("json"));
    assert!(!is_text_extension("yaml"));
}
