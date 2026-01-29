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
