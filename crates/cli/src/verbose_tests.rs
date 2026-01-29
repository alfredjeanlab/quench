#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;

#[test]
fn disabled_logger_reports_not_enabled() {
    let logger = VerboseLogger::new(false);
    assert!(!logger.is_enabled());
}

#[test]
fn enabled_logger_reports_enabled() {
    let logger = VerboseLogger::new(true);
    assert!(logger.is_enabled());
}

#[test]
fn log_does_nothing_when_disabled() {
    let logger = VerboseLogger::new(false);
    // This should not panic and should not output anything
    logger.log("test message");
}

#[test]
fn section_does_nothing_when_disabled() {
    let logger = VerboseLogger::new(false);
    // This should not panic and should not output anything
    logger.section("Test Section");
}

#[test]
fn log_outputs_with_prefix_when_enabled() {
    let logger = VerboseLogger::new(true);
    // We can't easily capture stderr in unit tests, but we can verify it doesn't panic
    logger.log("test message");
}

#[test]
fn section_outputs_with_header_when_enabled() {
    let logger = VerboseLogger::new(true);
    // We can't easily capture stderr in unit tests, but we can verify it doesn't panic
    logger.section("Test Section");
}
