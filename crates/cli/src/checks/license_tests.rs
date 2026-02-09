// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

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

// =============================================================================
// COMMENT PREFIX TESTS
// =============================================================================

#[test]
fn comment_prefix_rust() {
    assert_eq!(comment_prefix_for_extension("rs"), "// ");
}

#[test]
fn comment_prefix_typescript() {
    assert_eq!(comment_prefix_for_extension("ts"), "// ");
    assert_eq!(comment_prefix_for_extension("tsx"), "// ");
}

#[test]
fn comment_prefix_javascript() {
    assert_eq!(comment_prefix_for_extension("js"), "// ");
    assert_eq!(comment_prefix_for_extension("jsx"), "// ");
}

#[test]
fn comment_prefix_shell() {
    assert_eq!(comment_prefix_for_extension("sh"), "# ");
    assert_eq!(comment_prefix_for_extension("bash"), "# ");
}

#[test]
fn comment_prefix_python() {
    assert_eq!(comment_prefix_for_extension("py"), "# ");
}

#[test]
fn comment_prefix_yaml() {
    assert_eq!(comment_prefix_for_extension("yaml"), "# ");
    assert_eq!(comment_prefix_for_extension("yml"), "# ");
}

#[test]
fn comment_prefix_unknown_defaults_to_slashes() {
    assert_eq!(comment_prefix_for_extension("xyz"), "// ");
}

// =============================================================================
// HEADER GENERATION TESTS
// =============================================================================

#[test]
fn generate_header_rust() {
    let header = generate_header("MIT", "Test Org", 2026, "rs");
    assert_eq!(header, "// SPDX-License-Identifier: MIT\n// Copyright (c) 2026 Test Org\n");
}

#[test]
fn generate_header_shell() {
    let header = generate_header("MIT", "Test Org", 2026, "sh");
    assert_eq!(header, "# SPDX-License-Identifier: MIT\n# Copyright (c) 2026 Test Org\n");
}

#[test]
fn generate_header_python() {
    let header = generate_header("Apache-2.0", "ACME Corp", 2025, "py");
    assert_eq!(header, "# SPDX-License-Identifier: Apache-2.0\n# Copyright (c) 2025 ACME Corp\n");
}

// =============================================================================
// SHEBANG PRESERVATION TESTS
// =============================================================================

#[test]
fn insert_header_no_shebang() {
    let content = "pub fn hello() {}\n";
    let header = "// SPDX\n// Copyright\n";
    let result = insert_header_preserving_shebang(content, header);
    assert_eq!(result, "// SPDX\n// Copyright\n\npub fn hello() {}\n");
}

#[test]
fn insert_header_with_shebang() {
    let content = "#!/bin/bash\n\necho 'hello'\n";
    let header = "# SPDX\n# Copyright\n";
    let result = insert_header_preserving_shebang(content, header);
    assert!(result.starts_with("#!/bin/bash\n"));
    assert!(result.contains("# SPDX"));
    // Verify shebang comes before SPDX
    let shebang_pos = result.find("#!/bin/bash").unwrap();
    let spdx_pos = result.find("# SPDX").unwrap();
    assert!(shebang_pos < spdx_pos);
}

#[test]
fn insert_header_shebang_only() {
    let content = "#!/usr/bin/env python";
    let header = "# SPDX\n";
    let result = insert_header_preserving_shebang(content, header);
    assert!(result.starts_with("#!/usr/bin/env python"));
    assert!(result.contains("# SPDX"));
}

#[test]
fn insert_header_empty_content() {
    let content = "";
    let header = "// SPDX\n";
    let result = insert_header_preserving_shebang(content, header);
    assert_eq!(result, "// SPDX\n\n");
}

// =============================================================================
// YEAR UPDATE TESTS
// =============================================================================

#[test]
fn update_year_single_to_range() {
    let content = "// SPDX-License-Identifier: MIT\n// Copyright (c) 2020 Test Org\n\ncode";
    let result = update_copyright_year(content, 2026);
    assert!(result.contains("2020-2026"));
}

#[test]
fn update_year_extend_range() {
    let content = "// SPDX-License-Identifier: MIT\n// Copyright (c) 2020-2025 Test Org\n\ncode";
    let result = update_copyright_year(content, 2026);
    assert!(result.contains("2020-2026"));
}

#[test]
fn update_year_preserves_other_lines() {
    let content =
        "// SPDX-License-Identifier: MIT\n// Copyright (c) 2020 Test Org\n\npub fn hello() {}\n";
    let result = update_copyright_year(content, 2026);
    assert!(result.contains("SPDX-License-Identifier: MIT"));
    assert!(result.contains("pub fn hello()"));
}

#[test]
fn update_year_no_trailing_newline() {
    let content = "// Copyright (c) 2020 Test";
    let result = update_copyright_year(content, 2026);
    assert!(!result.ends_with('\n'));
    assert!(result.contains("2020-2026"));
}
