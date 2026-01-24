// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;

#[test]
fn suggest_check_name_escapes_typo() {
    assert_eq!(suggest_check_name("escaps"), Some("escapes"));
    assert_eq!(suggest_check_name("escape"), Some("escapes"));
    assert_eq!(suggest_check_name("escap"), Some("escapes"));
    assert_eq!(suggest_check_name("esc"), Some("escapes"));
}

#[test]
fn suggest_check_name_agents_typo() {
    assert_eq!(suggest_check_name("agent"), Some("agents"));
    assert_eq!(suggest_check_name("claude"), Some("agents"));
    assert_eq!(suggest_check_name("cursor"), Some("agents"));
}

#[test]
fn suggest_check_name_tests_typo() {
    assert_eq!(suggest_check_name("test"), Some("tests"));
    assert_eq!(suggest_check_name("testing"), Some("tests"));
}

#[test]
fn suggest_check_name_docs_typo() {
    assert_eq!(suggest_check_name("doc"), Some("docs"));
    assert_eq!(suggest_check_name("documentation"), Some("docs"));
}

#[test]
fn suggest_check_name_cloc_typo() {
    assert_eq!(suggest_check_name("loc"), Some("cloc"));
    assert_eq!(suggest_check_name("lines"), Some("cloc"));
    assert_eq!(suggest_check_name("code"), Some("cloc"));
}

#[test]
fn suggest_check_name_git_typo() {
    assert_eq!(suggest_check_name("commit"), Some("git"));
    assert_eq!(suggest_check_name("commits"), Some("git"));
}

#[test]
fn suggest_check_name_build_typo() {
    assert_eq!(suggest_check_name("builds"), Some("build"));
    assert_eq!(suggest_check_name("binary"), Some("build"));
    assert_eq!(suggest_check_name("compile"), Some("build"));
}

#[test]
fn suggest_check_name_license_typo() {
    assert_eq!(suggest_check_name("licenses"), Some("license"));
    assert_eq!(suggest_check_name("lic"), Some("license"));
    assert_eq!(suggest_check_name("header"), Some("license"));
}

#[test]
fn suggest_check_name_prefix_match() {
    // Prefix matching
    assert_eq!(suggest_check_name("clo"), Some("cloc"));
    assert_eq!(suggest_check_name("age"), Some("agents"));
    assert_eq!(suggest_check_name("bui"), Some("build"));
}

#[test]
fn suggest_check_name_no_match() {
    assert_eq!(suggest_check_name("foobar"), None);
    assert_eq!(suggest_check_name("xyz"), None);
    assert_eq!(suggest_check_name(""), None);
}
