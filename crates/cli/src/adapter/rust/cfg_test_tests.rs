// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

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
