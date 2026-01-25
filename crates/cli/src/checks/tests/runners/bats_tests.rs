#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;

#[test]
fn parses_passing_test() {
    let output = "1..1\nok 1 example test in 45ms\n";
    let result = parse_tap_output(output, Duration::from_secs(1));

    assert!(result.passed);
    assert_eq!(result.tests.len(), 1);
    assert!(result.tests[0].passed);
    assert_eq!(result.tests[0].name, "example test");
    assert_eq!(result.tests[0].duration, Duration::from_millis(45));
}

#[test]
fn parses_failing_test() {
    let output = "1..1\nnot ok 1 should pass in 12ms\n";
    let result = parse_tap_output(output, Duration::from_secs(1));

    assert!(!result.passed);
    assert_eq!(result.tests.len(), 1);
    assert!(!result.tests[0].passed);
}

#[test]
fn parses_multiple_tests() {
    let output = r#"1..3
ok 1 first test in 10ms
ok 2 second test in 20ms
not ok 3 third test in 30ms
"#;
    let result = parse_tap_output(output, Duration::from_millis(100));

    assert!(!result.passed);
    assert_eq!(result.tests.len(), 3);
    assert_eq!(result.passed_count(), 2);
    assert_eq!(result.failed_count(), 1);
}

#[test]
fn extracts_timing_in_milliseconds() {
    let (name, duration) = extract_timing("test name in 123ms");
    assert_eq!(name, "test name");
    assert_eq!(duration, Duration::from_millis(123));
}

#[test]
fn extracts_timing_in_seconds() {
    let (name, duration) = extract_timing("test name in 1.5s");
    assert_eq!(name, "test name");
    assert_eq!(duration, Duration::from_millis(1500));
}

#[test]
fn handles_missing_timing() {
    let (name, duration) = extract_timing("test without timing");
    assert_eq!(name, "test without timing");
    assert_eq!(duration, Duration::ZERO);
}

#[test]
fn ignores_tap_comments() {
    let output = "1..1\n# diagnostic message\nok 1 test in 10ms\n";
    let result = parse_tap_output(output, Duration::from_secs(1));

    assert_eq!(result.tests.len(), 1);
}

#[test]
fn handles_empty_output() {
    let result = parse_tap_output("", Duration::from_secs(0));
    assert!(result.passed);
    assert!(result.tests.is_empty());
}

#[test]
fn parses_tap_line_ok() {
    let result = parse_tap_line("ok 1 my test in 50ms").unwrap();
    assert!(result.passed);
    assert_eq!(result.name, "my test");
    assert_eq!(result.duration, Duration::from_millis(50));
}

#[test]
fn parses_tap_line_not_ok() {
    let result = parse_tap_line("not ok 2 failing test in 100ms").unwrap();
    assert!(!result.passed);
    assert_eq!(result.name, "failing test");
    assert_eq!(result.duration, Duration::from_millis(100));
}

#[test]
fn parses_tap_line_returns_none_for_invalid() {
    assert!(parse_tap_line("1..5").is_none());
    assert!(parse_tap_line("# comment").is_none());
    assert!(parse_tap_line("random text").is_none());
}

#[test]
fn parse_duration_milliseconds() {
    assert_eq!(parse_duration("45ms"), Some(Duration::from_millis(45)));
    assert_eq!(parse_duration("0ms"), Some(Duration::from_millis(0)));
    assert_eq!(parse_duration("1000ms"), Some(Duration::from_millis(1000)));
}

#[test]
fn parse_duration_seconds() {
    assert_eq!(parse_duration("1s"), Some(Duration::from_secs(1)));
    assert_eq!(parse_duration("1.5s"), Some(Duration::from_millis(1500)));
    assert_eq!(parse_duration("0.001s"), Some(Duration::from_millis(1)));
}

#[test]
fn parse_duration_invalid() {
    assert_eq!(parse_duration("invalid"), None);
    assert_eq!(parse_duration("45"), None);
    assert_eq!(parse_duration(""), None);
}
