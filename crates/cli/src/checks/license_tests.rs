// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;

#[test]
fn year_includes_current_single_year_match() {
    assert!(year_includes_current("2026", 2026));
}

#[test]
fn year_includes_current_single_year_mismatch() {
    assert!(!year_includes_current("2025", 2026));
}

#[test]
fn year_includes_current_range_includes() {
    assert!(year_includes_current("2020-2026", 2026));
    assert!(year_includes_current("2020-2030", 2026));
}

#[test]
fn year_includes_current_range_excludes() {
    assert!(!year_includes_current("2020-2025", 2026));
}

#[test]
fn is_supported_extension_rust() {
    assert!(is_supported_extension("rs"));
}

#[test]
fn is_supported_extension_shell() {
    assert!(is_supported_extension("sh"));
    assert!(is_supported_extension("bash"));
}

#[test]
fn is_supported_extension_go() {
    assert!(is_supported_extension("go"));
}

#[test]
fn is_supported_extension_typescript() {
    assert!(is_supported_extension("ts"));
    assert!(is_supported_extension("tsx"));
}

#[test]
fn is_supported_extension_python() {
    assert!(is_supported_extension("py"));
}

#[test]
fn is_supported_extension_unsupported() {
    assert!(!is_supported_extension("txt"));
    assert!(!is_supported_extension("md"));
    assert!(!is_supported_extension("json"));
}

#[test]
fn get_header_lines_basic() {
    let content = "// Line 1\n// Line 2\n// Line 3\ncode";
    let header = get_header_lines(content, 2);
    assert_eq!(header, "// Line 1\n// Line 2");
}

#[test]
fn get_header_lines_skips_shebang() {
    let content = "#!/bin/bash\n# SPDX\n# Copyright";
    let header = get_header_lines(content, 2);
    assert_eq!(header, "# SPDX\n# Copyright");
}

#[test]
fn find_line_number_finds_pattern() {
    let content = "line1\nSPDX-License-Identifier: MIT\nline3";
    assert_eq!(find_line_number(content, "SPDX"), 2);
}

#[test]
fn find_line_number_defaults_to_one() {
    let content = "line1\nline2\nline3";
    assert_eq!(find_line_number(content, "notfound"), 1);
}
