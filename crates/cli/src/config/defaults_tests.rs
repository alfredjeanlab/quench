// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::defaults;

#[test]
fn size_constants_are_sane() {
    assert_eq!(defaults::size::MAX_LINES, 800);
    assert_eq!(defaults::size::MAX_LINES_TEST, 1000);
    assert_eq!(defaults::size::MAX_TOKENS, 20000);
    assert_eq!(defaults::size::MAX_LINES_SPEC, 1000);
    const { assert!(defaults::size::MAX_LINES < defaults::size::MAX_LINES_TEST) };
}

#[test]
fn target_range_for_750() {
    let range = defaults::advice::target_range(750);
    // 750/5=150, 750/3=250
    assert_eq!(range, "150–250 lines");
}

#[test]
fn target_range_for_500() {
    let range = defaults::advice::target_range(500);
    // 500/5=100, 500/3=166 -> rounded to 100, 160
    assert_eq!(range, "100–160 lines");
}

#[test]
fn target_range_for_small_threshold() {
    // Below 100, no rounding
    let range = defaults::advice::target_range(50);
    // 50/5=10, 50/3=16
    assert_eq!(range, "10–16 lines");
}

#[test]
fn cloc_source_advice_contains_range() {
    let advice = defaults::advice::cloc_source(750);
    assert!(advice.contains("150–250 lines"));
}

#[test]
fn cloc_test_advice_contains_range() {
    let advice = defaults::advice::cloc_test(1000);
    // 1000/5=200, 1000/3=333 -> 200, 330
    assert!(advice.contains("200–330 lines"));
}

#[test]
fn generic_test_patterns_non_empty() {
    let patterns = defaults::test_patterns::generic();
    assert!(!patterns.is_empty());
    assert!(patterns.contains(&"**/tests/**".to_string()));
    assert!(patterns.contains(&"**/*_test.*".to_string()));
}
