// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;

#[test]
fn parses_passing_tests() {
    let stderr = r#"
test_add (test_app.TestApp.test_add) ... ok
test_sub (test_app.TestApp.test_sub) ... ok

----------------------------------------------------------------------
Ran 2 tests in 0.001s

OK
"#;
    let result = parse_unittest_output(stderr, "", Duration::from_secs(1));

    assert!(result.passed);
    assert_eq!(result.tests.len(), 2);
    assert!(result.tests[0].passed);
    assert_eq!(result.tests[0].name, "test_app.TestApp.test_add");
    assert!(result.tests[1].passed);
    assert_eq!(result.tests[1].name, "test_app.TestApp.test_sub");
}

#[test]
fn parses_failing_tests() {
    let stderr = r#"
test_add (test_app.TestApp.test_add) ... ok
test_sub (test_app.TestApp.test_sub) ... FAIL

======================================================================
FAIL: test_sub (test_app.TestApp.test_sub)
----------------------------------------------------------------------
Traceback (most recent call last):
  File "test_app.py", line 10, in test_sub
    self.assertEqual(1, 2)
AssertionError: 1 != 2

----------------------------------------------------------------------
Ran 2 tests in 0.001s

FAILED (failures=1)
"#;
    let result = parse_unittest_output(stderr, "", Duration::from_secs(1));

    assert!(!result.passed);
    assert_eq!(result.tests.len(), 2);
    assert!(result.tests[0].passed);
    assert!(!result.tests[1].passed);
}

#[test]
fn parses_error_tests() {
    let stderr = r#"
test_add (test_app.TestApp.test_add) ... ok
test_error (test_app.TestApp.test_error) ... ERROR

======================================================================
ERROR: test_error (test_app.TestApp.test_error)
----------------------------------------------------------------------
Traceback (most recent call last):
  File "test_app.py", line 15, in test_error
    raise RuntimeError("oops")
RuntimeError: oops

----------------------------------------------------------------------
Ran 2 tests in 0.001s

FAILED (errors=1)
"#;
    let result = parse_unittest_output(stderr, "", Duration::from_secs(1));

    assert!(!result.passed);
    assert!(result.error.is_some());
    assert!(result.error.unwrap().contains("error"));
}

#[test]
fn parses_skipped_tests() {
    let stderr = r#"
test_add (test_app.TestApp.test_add) ... ok
test_skip (test_app.TestApp.test_skip) ... skipped 'not implemented'

----------------------------------------------------------------------
Ran 2 tests in 0.001s

OK (skipped=1)
"#;
    let result = parse_unittest_output(stderr, "", Duration::from_secs(1));

    assert!(result.passed);
    assert_eq!(result.tests.len(), 2);
    assert!(result.tests[0].passed);
    assert!(result.tests[1].skipped);
}

#[test]
fn parses_test_line_ok() {
    let (name, status) = parse_test_line("test_add (test_app.TestApp.test_add) ... ok").unwrap();
    assert_eq!(name, "test_app.TestApp.test_add");
    assert_eq!(status, TestStatus::Ok);
}

#[test]
fn parses_test_line_fail() {
    let (name, status) = parse_test_line("test_sub (test_app.TestApp.test_sub) ... FAIL").unwrap();
    assert_eq!(name, "test_app.TestApp.test_sub");
    assert_eq!(status, TestStatus::Fail);
}

#[test]
fn parses_test_line_error() {
    let (name, status) =
        parse_test_line("test_error (test_app.TestApp.test_error) ... ERROR").unwrap();
    assert_eq!(name, "test_app.TestApp.test_error");
    assert_eq!(status, TestStatus::Error);
}

#[test]
fn parses_test_line_skipped() {
    let (name, status) =
        parse_test_line("test_skip (test_app.TestApp.test_skip) ... skipped 'reason'").unwrap();
    assert_eq!(name, "test_app.TestApp.test_skip");
    assert_eq!(status, TestStatus::Skipped);
}

#[test]
fn parses_test_line_without_parentheses() {
    // Some unittest runners may not include the full path in parentheses
    let (name, status) = parse_test_line("test_simple ... ok").unwrap();
    assert_eq!(name, "test_simple");
    assert_eq!(status, TestStatus::Ok);
}

#[test]
fn parses_summary_line() {
    let (count, duration) = parse_summary_line("Ran 5 tests in 0.123s").unwrap();
    assert_eq!(count, 5);
    assert_eq!(duration, Duration::from_millis(123));
}

#[test]
fn parses_summary_line_single_test() {
    let (count, duration) = parse_summary_line("Ran 1 test in 0.001s").unwrap();
    assert_eq!(count, 1);
    assert_eq!(duration, Duration::from_millis(1));
}

#[test]
fn summary_returns_none_for_non_summary() {
    assert!(parse_summary_line("test_add ... ok").is_none());
    assert!(parse_summary_line("OK").is_none());
    assert!(parse_summary_line("").is_none());
}

#[test]
fn parses_duration_seconds() {
    assert_eq!(parse_duration("0.001s"), Some(Duration::from_millis(1)));
    assert_eq!(parse_duration("1.0s"), Some(Duration::from_secs(1)));
    assert_eq!(parse_duration("0.123s"), Some(Duration::from_millis(123)));
}

#[test]
fn parses_duration_invalid() {
    assert!(parse_duration("invalid").is_none());
    assert!(parse_duration("123ms").is_none());
    assert!(parse_duration("").is_none());
}

#[test]
fn handles_empty_output() {
    let result = parse_unittest_output("", "", Duration::from_secs(0));
    // Empty output is considered passed (no failures detected)
    assert!(result.passed);
    assert!(result.tests.is_empty());
}

#[test]
fn handles_mixed_stdout_stderr() {
    // unittest typically outputs to stderr, but some info may go to stdout
    let stderr = "test_add (test_app.TestApp.test_add) ... ok\n";
    let stdout = "----------------------------------------------------------------------\nRan 1 test in 0.001s\n\nOK\n";

    let result = parse_unittest_output(stderr, stdout, Duration::from_secs(1));
    assert!(result.passed);
    assert_eq!(result.tests.len(), 1);
}
