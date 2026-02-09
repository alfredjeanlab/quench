// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

use super::*;

#[test]
fn basic_cfg_test_block() {
    let content = r#"
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_add() {
        assert_eq!(super::add(1, 2), 3);
    }
}
"#;
    let info = CfgTestInfo::parse(content);

    // Lines 0-4 are source (empty, pub fn, a+b, }, empty)
    // Lines 5-11 are test (#[cfg(test)], mod tests, #[test], fn, assert, }, })
    assert!(!info.is_test_line(1)); // pub fn add
    assert!(!info.is_test_line(2)); // a + b
    assert!(info.is_test_line(5)); // #[cfg(test)]
    assert!(info.is_test_line(6)); // mod tests
    assert!(info.is_test_line(11)); // closing brace
}

#[test]
fn nested_braces_in_test() {
    let content = r#"
pub fn main() {}

#[cfg(test)]
mod tests {
    fn helper() {
        if true {
            println!("nested");
        }
    }
}
"#;
    let info = CfgTestInfo::parse(content);

    assert!(!info.is_test_line(1)); // pub fn main
    assert!(info.is_test_line(3)); // #[cfg(test)]
    assert!(info.is_test_line(7)); // nested println
    assert!(info.is_test_line(10)); // closing brace of mod tests
}

#[test]
fn multiple_cfg_test_blocks() {
    let content = r#"
fn a() {}

#[cfg(test)]
mod tests_a {
    #[test]
    fn test_a() {}
}

fn b() {}

#[cfg(test)]
mod tests_b {
    #[test]
    fn test_b() {}
}
"#;
    let info = CfgTestInfo::parse(content);

    assert_eq!(info.test_ranges.len(), 2);
    assert!(!info.is_test_line(1)); // fn a()
    assert!(info.is_test_line(3)); // first #[cfg(test)]
    assert!(!info.is_test_line(9)); // fn b()
    assert!(info.is_test_line(11)); // second #[cfg(test)]
}

#[test]
fn no_cfg_test_blocks() {
    let content = r#"
pub fn main() {
    println!("Hello");
}
"#;
    let info = CfgTestInfo::parse(content);

    assert!(info.test_ranges.is_empty());
    assert!(!info.is_test_line(0));
    assert!(!info.is_test_line(1));
}

#[test]
fn cfg_test_with_spaces() {
    // #[cfg(test)] with extra whitespace inside
    let content = r#"
pub fn main() {}

#[cfg( test )]
mod tests {
    fn test() {}
}
"#;
    let info = CfgTestInfo::parse(content);

    assert!(!info.test_ranges.is_empty());
    assert!(info.is_test_line(3)); // #[cfg( test )]
}

#[test]
fn string_literals_with_braces() {
    // Test that braces in string literals are correctly skipped
    let content = r#"
fn source() {}

#[cfg(test)]
mod tests {
    #[test]
    fn test() {
        let s = "{ not a real brace }";
        assert!(true);
    }
}
"#;
    let info = CfgTestInfo::parse(content);

    // With improved string handling, this should parse correctly
    assert_eq!(info.test_ranges.len(), 1);
    assert!(info.is_test_line(3)); // #[cfg(test)]
    assert!(info.is_test_line(10)); // closing brace of mod tests
}

#[test]
fn escaped_quotes_in_strings() {
    // Test that escaped quotes don't confuse the parser
    let content = r#"
fn source() {}

#[cfg(test)]
mod tests {
    #[test]
    fn test() {
        let s = "hello \"world\" {}";
        assert!(true);
    }
}
"#;
    let info = CfgTestInfo::parse(content);

    assert_eq!(info.test_ranges.len(), 1);
    assert!(info.is_test_line(10)); // closing brace
}

#[test]
fn raw_string_with_braces() {
    // Raw strings containing braces should not affect brace counting
    let content = r#"
fn source() {}

#[cfg(test)]
mod tests {
    #[test]
    fn test_raw_string() {
        let s = r"{ not a real brace }";
        assert!(true);
    }
}
"#;
    let info = CfgTestInfo::parse(content);

    assert_eq!(info.test_ranges.len(), 1);
    assert!(info.is_test_line(3)); // #[cfg(test)]
    assert!(info.is_test_line(10)); // closing brace of mod tests
}

#[test]
fn raw_string_with_hashes() {
    // Raw strings with hash delimiters
    let content = r###"
fn source() {}

#[cfg(test)]
mod tests {
    #[test]
    fn test() {
        let s = r#"{ braces } and "quotes""#;
        let t = r##"more { braces }"##;
        assert!(true);
    }
}
"###;
    let info = CfgTestInfo::parse(content);

    assert_eq!(info.test_ranges.len(), 1);
    assert!(info.is_test_line(3)); // #[cfg(test)]
    assert!(info.is_test_line(11)); // closing brace
}

#[test]
fn char_literal_with_brace() {
    // Character literals containing braces
    let content = r#"
fn source() {}

#[cfg(test)]
mod tests {
    #[test]
    fn test_char() {
        let open = '{';
        let close = '}';
        assert_eq!(open, '{');
    }
}
"#;
    let info = CfgTestInfo::parse(content);

    assert_eq!(info.test_ranges.len(), 1);
    assert!(info.is_test_line(3)); // #[cfg(test)]
    assert!(info.is_test_line(11)); // closing brace
}

#[test]
fn char_literal_with_escaped_quote() {
    // Escaped quote in char literal shouldn't confuse parser
    let content = r#"
fn source() {}

#[cfg(test)]
mod tests {
    #[test]
    fn test() {
        let quote = '\'';
        let brace = '{';
        assert!(true);
    }
}
"#;
    let info = CfgTestInfo::parse(content);

    assert_eq!(info.test_ranges.len(), 1);
    assert!(info.is_test_line(10)); // closing brace
}

#[test]
fn mixed_string_types() {
    // Mix of regular strings, raw strings, and char literals
    let content = r###"
fn source() {}

#[cfg(test)]
mod tests {
    #[test]
    fn test() {
        let a = "{ regular }";
        let b = r"{ raw }";
        let c = r#"{ raw # }"#;
        let d = '{';
        let e = '}';
        assert!(true);
    }
}
"###;
    let info = CfgTestInfo::parse(content);

    assert_eq!(info.test_ranges.len(), 1);
    assert!(info.is_test_line(14)); // closing brace
}

#[test]
fn lifetime_not_confused_with_char() {
    // Lifetimes should not be confused with char literals
    let content = r#"
fn source<'a>(x: &'a str) -> &'a str { x }

#[cfg(test)]
mod tests {
    fn helper<'a>(x: &'a str) -> &'a str {
        x
    }
}
"#;
    let info = CfgTestInfo::parse(content);

    assert_eq!(info.test_ranges.len(), 1);
    assert!(!info.is_test_line(1)); // source function
    assert!(info.is_test_line(3)); // #[cfg(test)]
}

#[test]
fn nested_raw_strings() {
    // Raw strings can contain quote characters
    let content = r####"
fn source() {}

#[cfg(test)]
mod tests {
    #[test]
    fn test() {
        let s = r#"contains "quotes" and { braces }"#;
        assert!(true);
    }
}
"####;
    let info = CfgTestInfo::parse(content);

    assert_eq!(info.test_ranges.len(), 1);
}

#[test]
fn empty_string_and_char() {
    // Edge case: empty-ish strings
    let content = r#"
fn source() {}

#[cfg(test)]
mod tests {
    #[test]
    fn test() {
        let s = "";
        let c = ' ';
        let brace_str = "{}";
        assert!(true);
    }
}
"#;
    let info = CfgTestInfo::parse(content);

    assert_eq!(info.test_ranges.len(), 1);
}

// =============================================================================
// MULTI-LINE ATTRIBUTE TESTS
// =============================================================================

#[test]
fn multiline_cfg_test_attribute() {
    // Multi-line #[cfg(test)] should be detected
    let content = r#"
pub fn source_code() -> i32 {
    42
}

#[cfg(
    test
)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(super::source_code(), 42);
    }
}
"#;
    let info = CfgTestInfo::parse(content);

    assert_eq!(info.test_ranges.len(), 1, "should detect multi-line cfg(test)");
    assert!(!info.is_test_line(1)); // pub fn source_code
    assert!(info.is_test_line(5)); // #[cfg(
    assert!(info.is_test_line(8)); // mod tests
    assert!(info.is_test_line(13)); // closing brace
}

#[test]
fn multiline_cfg_test_with_extra_whitespace() {
    // Multi-line with lots of whitespace
    let content = r#"
fn main() {}

#[cfg(
        test
    )]
mod tests {
    fn test() {}
}
"#;
    let info = CfgTestInfo::parse(content);

    assert_eq!(info.test_ranges.len(), 1);
    assert!(!info.is_test_line(1)); // fn main
    assert!(info.is_test_line(3)); // #[cfg(
}

#[test]
fn multiline_cfg_not_test() {
    // Multi-line #[cfg(...)] that is NOT test
    let content = r#"
fn always() {}

#[cfg(
    feature = "foo"
)]
fn feature_foo() {}

#[cfg(test)]
mod tests {
    fn test() {}
}
"#;
    let info = CfgTestInfo::parse(content);

    // Should only have 1 range (the actual cfg(test) block)
    assert_eq!(info.test_ranges.len(), 1);
    assert!(!info.is_test_line(1)); // fn always
    assert!(!info.is_test_line(6)); // fn feature_foo
    assert!(info.is_test_line(8)); // #[cfg(test)]
}

#[test]
fn multiline_cfg_all_test() {
    // Multi-line #[cfg(all(test, ...))] should be detected
    let content = r#"
fn source() {}

#[cfg(all(
    test,
    feature = "integration"
))]
mod integration_tests {
    fn test() {}
}
"#;
    let info = CfgTestInfo::parse(content);

    assert_eq!(info.test_ranges.len(), 1);
    assert!(!info.is_test_line(1)); // fn source
    assert!(info.is_test_line(3)); // #[cfg(all(
}

#[test]
fn doc_comment_mentioning_cfg_test_not_detected() {
    // Doc comments that mention #[cfg(test)] should not be detected as cfg(test) blocks
    let content = r#"
/// Mode for handling #[cfg(test)] blocks in Rust files.
#[derive(Debug, Clone, Copy)]
pub enum CfgTestSplitMode {
    Count,
    Require,
}
"#;
    let info = CfgTestInfo::parse(content);

    // Should have no test blocks
    assert_eq!(info.test_ranges.len(), 0);
    assert!(!info.has_inline_tests());
}

#[test]
fn line_comment_mentioning_cfg_test_not_detected() {
    // Regular line comments that mention #[cfg(test)] should not be detected
    let content = r#"
fn process() {
    // Check if line is in test code (file-level OR inline #[cfg(test)])
    let is_test = false;
}
"#;
    let info = CfgTestInfo::parse(content);

    // Should have no test blocks
    assert_eq!(info.test_ranges.len(), 0);
    assert!(!info.has_inline_tests());
}

#[test]
fn block_comment_mentioning_cfg_test_not_detected() {
    // Block comments that mention #[cfg(test)] should not be detected
    let content = r#"
fn process() {
    /* This function handles #[cfg(test)] blocks
       and other conditional compilation */
    let x = 42;
}
"#;
    let info = CfgTestInfo::parse(content);

    // Should have no test blocks
    assert_eq!(info.test_ranges.len(), 0);
    assert!(!info.has_inline_tests());
}

#[test]
fn multiline_block_comment_with_cfg_test_not_detected() {
    // Multi-line block comments spanning several lines should be ignored
    let content = r#"
/*
 * Documentation about test handling:
 * - Use #[cfg(test)] for test modules
 * - Keep tests in separate files
 */
fn main() {}
"#;
    let info = CfgTestInfo::parse(content);

    // Should have no test blocks
    assert_eq!(info.test_ranges.len(), 0);
    assert!(!info.has_inline_tests());
}

// =============================================================================
// ITEM KIND DETECTION TESTS
// =============================================================================

#[test]
fn item_kind_mod() {
    let content = r#"
fn source() {}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {}
}
"#;
    let info = CfgTestInfo::parse(content);
    assert_eq!(info.blocks.len(), 1);
    assert_eq!(info.blocks[0].item_kind, CfgTestItemKind::Mod);
}

#[test]
fn item_kind_pub_mod() {
    let content = r#"
#[cfg(test)]
pub mod tests {
    fn test() {}
}
"#;
    let info = CfgTestInfo::parse(content);
    assert_eq!(info.blocks.len(), 1);
    assert_eq!(info.blocks[0].item_kind, CfgTestItemKind::Mod);
}

#[test]
fn item_kind_fn() {
    let content = r#"
#[cfg(test)]
fn test_helper() -> i32 {
    42
}
"#;
    let info = CfgTestInfo::parse(content);
    assert_eq!(info.blocks.len(), 1);
    assert_eq!(info.blocks[0].item_kind, CfgTestItemKind::Fn);
}

#[test]
fn item_kind_pub_fn() {
    let content = r#"
#[cfg(test)]
pub fn test_helper() {
}
"#;
    let info = CfgTestInfo::parse(content);
    assert_eq!(info.blocks.len(), 1);
    assert_eq!(info.blocks[0].item_kind, CfgTestItemKind::Fn);
}

#[test]
fn item_kind_async_fn() {
    let content = r#"
#[cfg(test)]
async fn async_helper() {
}
"#;
    let info = CfgTestInfo::parse(content);
    assert_eq!(info.blocks.len(), 1);
    assert_eq!(info.blocks[0].item_kind, CfgTestItemKind::Fn);
}

#[test]
fn item_kind_unsafe_fn() {
    let content = r#"
#[cfg(test)]
unsafe fn unsafe_helper() {
}
"#;
    let info = CfgTestInfo::parse(content);
    assert_eq!(info.blocks.len(), 1);
    assert_eq!(info.blocks[0].item_kind, CfgTestItemKind::Fn);
}

#[test]
fn item_kind_pub_async_unsafe_fn() {
    let content = r#"
#[cfg(test)]
pub async unsafe fn complex_fn() {
}
"#;
    let info = CfgTestInfo::parse(content);
    assert_eq!(info.blocks.len(), 1);
    assert_eq!(info.blocks[0].item_kind, CfgTestItemKind::Fn);
}

#[test]
fn item_kind_const_fn() {
    let content = r#"
#[cfg(test)]
const fn const_helper() -> i32 {
    42
}
"#;
    let info = CfgTestInfo::parse(content);
    assert_eq!(info.blocks.len(), 1);
    assert_eq!(info.blocks[0].item_kind, CfgTestItemKind::Fn);
}

#[test]
fn item_kind_extern_fn() {
    let content = r#"
#[cfg(test)]
extern "C" fn extern_helper() {
}
"#;
    let info = CfgTestInfo::parse(content);
    assert_eq!(info.blocks.len(), 1);
    assert_eq!(info.blocks[0].item_kind, CfgTestItemKind::Fn);
}

#[test]
fn item_kind_impl() {
    let content = r#"
#[cfg(test)]
impl TestStruct {
    fn helper(&self) {}
}
"#;
    let info = CfgTestInfo::parse(content);
    assert_eq!(info.blocks.len(), 1);
    assert_eq!(info.blocks[0].item_kind, CfgTestItemKind::Impl);
}

#[test]
fn item_kind_impl_with_generics() {
    let content = r#"
#[cfg(test)]
impl<T> TestStruct<T> {
    fn helper(&self) {}
}
"#;
    let info = CfgTestInfo::parse(content);
    assert_eq!(info.blocks.len(), 1);
    assert_eq!(info.blocks[0].item_kind, CfgTestItemKind::Impl);
}

#[test]
fn item_kind_struct() {
    let content = r#"
#[cfg(test)]
struct TestFixture {
    value: i32,
}
"#;
    let info = CfgTestInfo::parse(content);
    assert_eq!(info.blocks.len(), 1);
    assert_eq!(info.blocks[0].item_kind, CfgTestItemKind::Struct);
}

#[test]
fn item_kind_pub_crate_struct() {
    let content = r#"
#[cfg(test)]
pub(crate) struct TestFixture {
    value: i32,
}
"#;
    let info = CfgTestInfo::parse(content);
    assert_eq!(info.blocks.len(), 1);
    assert_eq!(info.blocks[0].item_kind, CfgTestItemKind::Struct);
}

#[test]
fn item_kind_enum() {
    let content = r#"
#[cfg(test)]
enum TestEnum {
    A,
    B,
}
"#;
    let info = CfgTestInfo::parse(content);
    assert_eq!(info.blocks.len(), 1);
    assert_eq!(info.blocks[0].item_kind, CfgTestItemKind::Enum);
}

#[test]
fn item_kind_trait() {
    let content = r#"
#[cfg(test)]
trait TestTrait {
    fn method(&self);
}
"#;
    let info = CfgTestInfo::parse(content);
    assert_eq!(info.blocks.len(), 1);
    assert_eq!(info.blocks[0].item_kind, CfgTestItemKind::Trait);
}

#[test]
fn item_kind_multiple_blocks() {
    let content = r#"
fn source() {}

#[cfg(test)]
mod tests {
    fn test() {}
}

#[cfg(test)]
fn helper() {
}

#[cfg(test)]
struct Fixture {
    x: i32,
}
"#;
    let info = CfgTestInfo::parse(content);
    assert_eq!(info.blocks.len(), 3);
    assert_eq!(info.blocks[0].item_kind, CfgTestItemKind::Mod);
    assert_eq!(info.blocks[1].item_kind, CfgTestItemKind::Fn);
    assert_eq!(info.blocks[2].item_kind, CfgTestItemKind::Struct);
}

#[test]
fn item_kind_with_path_attr() {
    // Extra attributes between #[cfg(test)] and the item should be skipped
    let content = r#"
#[cfg(test)]
#[path = "tests.rs"]
mod tests {
    fn test() {}
}
"#;
    let info = CfgTestInfo::parse(content);
    assert_eq!(info.blocks.len(), 1);
    assert_eq!(info.blocks[0].item_kind, CfgTestItemKind::Mod);
}

#[test]
fn item_kind_pub_super() {
    let content = r#"
#[cfg(test)]
pub(super) fn test_helper() {
}
"#;
    let info = CfgTestInfo::parse(content);
    assert_eq!(info.blocks.len(), 1);
    assert_eq!(info.blocks[0].item_kind, CfgTestItemKind::Fn);
}

#[test]
fn item_kind_pub_in_path() {
    let content = r#"
#[cfg(test)]
pub(in crate::module) fn test_helper() {
}
"#;
    let info = CfgTestInfo::parse(content);
    assert_eq!(info.blocks.len(), 1);
    assert_eq!(info.blocks[0].item_kind, CfgTestItemKind::Fn);
}

#[test]
fn item_kind_macro_invocation() {
    let content = r#"
#[cfg(test)]
thread_local! {
    static MOCK: std::cell::Cell<Option<bool>> = const { std::cell::Cell::new(None) };
}
"#;
    let info = CfgTestInfo::parse(content);
    assert_eq!(info.blocks.len(), 1);
    assert_eq!(info.blocks[0].item_kind, CfgTestItemKind::Macro);
}

#[test]
fn item_kind_lazy_static_macro() {
    let content = r#"
#[cfg(test)]
lazy_static! {
    static ref DB: Mutex<MockDb> = Mutex::new(MockDb::new());
}
"#;
    let info = CfgTestInfo::parse(content);
    assert_eq!(info.blocks.len(), 1);
    assert_eq!(info.blocks[0].item_kind, CfgTestItemKind::Macro);
}

#[test]
fn non_mod_items_not_in_test_ranges() {
    // Only `mod` blocks contribute to test_ranges; other item kinds
    // (helpers, fixtures, macros) are test infrastructure, not tests.
    let content = r#"
fn source() {}

#[cfg(test)]
mod tests {
    fn test() {}
}

#[cfg(test)]
fn helper() {
}

#[cfg(test)]
thread_local! {
    static MOCK: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

#[cfg(test)]
struct Fixture {
    x: i32,
}
"#;
    let info = CfgTestInfo::parse(content);
    assert_eq!(info.blocks.len(), 4);
    // Only the mod block is in test_ranges
    assert_eq!(info.test_ranges.len(), 1);
    assert!(info.is_test_line(3)); // #[cfg(test)] mod tests
    assert!(!info.is_test_line(9)); // #[cfg(test)] fn helper - NOT test
    assert!(!info.is_test_line(13)); // #[cfg(test)] thread_local! - NOT test
    assert!(!info.is_test_line(18)); // #[cfg(test)] struct Fixture - NOT test
}
