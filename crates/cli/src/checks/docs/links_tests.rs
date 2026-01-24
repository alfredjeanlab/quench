// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;

// =============================================================================
// LINK EXTRACTION TESTS
// =============================================================================

#[test]
fn extracts_simple_link() {
    let links = extract_links("[text](file.md)");
    assert_eq!(links.len(), 1);
    assert_eq!(links[0].target, "file.md");
    assert_eq!(links[0].line, 1);
}

#[test]
fn extracts_multiple_links_per_line() {
    let links = extract_links("[a](x.md) and [b](y.md)");
    assert_eq!(links.len(), 2);
    assert_eq!(links[0].target, "x.md");
    assert_eq!(links[1].target, "y.md");
}

#[test]
fn extracts_links_across_lines() {
    let content = "Line 1: [first](a.md)\nLine 2\nLine 3: [second](b.md)";
    let links = extract_links(content);
    assert_eq!(links.len(), 2);
    assert_eq!(links[0].line, 1);
    assert_eq!(links[1].line, 3);
}

#[test]
fn extracts_link_with_fragment() {
    let links = extract_links("[text](file.md#section)");
    assert_eq!(links.len(), 1);
    assert_eq!(links[0].target, "file.md#section");
}

#[test]
fn extracts_nested_brackets() {
    let links = extract_links("[[nested]](file.md)");
    assert_eq!(links.len(), 1);
    assert_eq!(links[0].target, "file.md");
}

#[test]
fn extracts_link_with_path() {
    let links = extract_links("[text](../sibling/file.md)");
    assert_eq!(links.len(), 1);
    assert_eq!(links[0].target, "../sibling/file.md");
}

#[test]
fn skips_links_in_fenced_code_blocks() {
    let content = r#"
Before: [real](real.md)

```markdown
See [example](src/parser.rs) for details.
```

After: [also-real](also-real.md)
"#;
    let links = extract_links(content);
    assert_eq!(links.len(), 2);
    assert_eq!(links[0].target, "real.md");
    assert_eq!(links[1].target, "also-real.md");
}

#[test]
fn skips_links_in_code_blocks_with_language_tags() {
    let content = r#"
[before](before.md)

```rust
// [comment](not-a-link.rs)
```

[after](after.md)
"#;
    let links = extract_links(content);
    assert_eq!(links.len(), 2);
    assert_eq!(links[0].target, "before.md");
    assert_eq!(links[1].target, "after.md");
}

#[test]
fn skips_links_in_markdown_code_block() {
    // Code blocks with markdown language tag should skip link extraction
    let content = r#"Before the block.

```markdown
See [the parser](src/parser.rs) for details.
Check [configuration](../02-config.md) for options.
```

After the block.
"#;
    let links = extract_links(content);
    // Should extract ZERO links since they're all inside a code block
    assert_eq!(links.len(), 0, "Expected no links, got: {:?}", links);
}

// =============================================================================
// LOCAL VS EXTERNAL DETECTION TESTS
// =============================================================================

#[test]
fn skips_https_urls() {
    assert!(!is_local_link("https://example.com"));
    assert!(!is_local_link("https://example.com/path"));
}

#[test]
fn skips_http_urls() {
    assert!(!is_local_link("http://example.com"));
    assert!(!is_local_link("http://example.com/path"));
}

#[test]
fn skips_other_protocols() {
    assert!(!is_local_link("mailto:foo@bar.com"));
    assert!(!is_local_link("ftp://example.com/file"));
}

#[test]
fn skips_protocol_relative_urls() {
    assert!(!is_local_link("//cdn.example.com/file.js"));
}

#[test]
fn skips_fragment_only_links() {
    assert!(!is_local_link("#section"));
    assert!(!is_local_link("#"));
}

#[test]
fn accepts_local_file_links() {
    assert!(is_local_link("file.md"));
    assert!(is_local_link("./file.md"));
    assert!(is_local_link("../file.md"));
    assert!(is_local_link("path/to/file.md"));
}

#[test]
fn accepts_local_links_with_fragment() {
    assert!(is_local_link("file.md#section"));
    assert!(is_local_link("./file.md#section"));
}

#[test]
fn accepts_directory_links() {
    assert!(is_local_link("dir/"));
    assert!(is_local_link("./dir/"));
}

// =============================================================================
// FRAGMENT STRIPPING TESTS
// =============================================================================

#[test]
fn strips_fragment_from_path() {
    assert_eq!(strip_fragment("file.md#section"), "file.md");
    assert_eq!(strip_fragment("path/file.md#anchor"), "path/file.md");
}

#[test]
fn handles_no_fragment() {
    assert_eq!(strip_fragment("file.md"), "file.md");
    assert_eq!(strip_fragment("path/to/file"), "path/to/file");
}

#[test]
fn handles_empty_fragment() {
    assert_eq!(strip_fragment("file.md#"), "file.md");
}

#[test]
fn handles_multiple_hashes() {
    // Only strips at first #
    assert_eq!(strip_fragment("file.md#sec#tion"), "file.md");
}

// =============================================================================
// PATH RESOLUTION TESTS
// =============================================================================

#[test]
fn resolves_relative_to_file_directory() {
    let md_file = Path::new("/project/docs/README.md");
    let resolved = resolve_link(md_file, "guide.md");
    assert_eq!(resolved, Path::new("/project/docs/guide.md"));
}

#[test]
fn resolves_parent_directory() {
    let md_file = Path::new("/project/docs/specs/overview.md");
    let resolved = resolve_link(md_file, "../config.md");
    assert_eq!(resolved, Path::new("/project/docs/specs/../config.md"));
}

#[test]
fn resolves_dot_prefix() {
    let md_file = Path::new("/project/docs/README.md");
    let resolved = resolve_link(md_file, "./guide.md");
    assert_eq!(resolved, Path::new("/project/docs/guide.md"));
}

#[test]
fn resolves_with_fragment() {
    let md_file = Path::new("/project/docs/README.md");
    let resolved = resolve_link(md_file, "guide.md#section");
    // Fragment should be stripped
    assert_eq!(resolved, Path::new("/project/docs/guide.md"));
}

#[test]
fn resolves_subdirectory_path() {
    let md_file = Path::new("/project/README.md");
    let resolved = resolve_link(md_file, "docs/guide.md");
    assert_eq!(resolved, Path::new("/project/docs/guide.md"));
}
