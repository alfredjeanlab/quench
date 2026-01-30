#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;

#[test]
fn format_duration_ms_small_values() {
    assert_eq!(format_duration_ms(0), "0ms");
    assert_eq!(format_duration_ms(1), "1ms");
    assert_eq!(format_duration_ms(500), "500ms");
    assert_eq!(format_duration_ms(2999), "2999ms");
    assert_eq!(format_duration_ms(3000), "3000ms");
}

#[test]
fn format_duration_ms_large_values() {
    assert_eq!(format_duration_ms(3001), "3.0s");
    assert_eq!(format_duration_ms(3100), "3.1s");
    assert_eq!(format_duration_ms(23287), "23.3s");
    assert_eq!(format_duration_ms(60000), "60.0s");
    assert_eq!(format_duration_ms(123456), "123.5s");
}
