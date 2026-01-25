#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;

#[test]
fn classify_small_file() {
    // 1KB should be Small
    assert_eq!(FileSizeClass::from_size(1024), FileSizeClass::Small);
    // 63KB should be Small
    assert_eq!(FileSizeClass::from_size(63 * 1024), FileSizeClass::Small);
    // Exactly at threshold is still Small (<=)
    assert_eq!(
        FileSizeClass::from_size(MMAP_THRESHOLD),
        FileSizeClass::Small
    );
}

#[test]
fn classify_normal_file() {
    // Just over mmap threshold
    assert_eq!(
        FileSizeClass::from_size(MMAP_THRESHOLD + 1),
        FileSizeClass::Normal
    );
    // 500KB should be Normal
    assert_eq!(FileSizeClass::from_size(500 * 1024), FileSizeClass::Normal);
    // Exactly at soft limit is still Normal (<=)
    assert_eq!(
        FileSizeClass::from_size(SOFT_LIMIT_SIZE),
        FileSizeClass::Normal
    );
}

#[test]
fn classify_oversized_file() {
    // Just over soft limit
    assert_eq!(
        FileSizeClass::from_size(SOFT_LIMIT_SIZE + 1),
        FileSizeClass::Oversized
    );
    // 5MB should be Oversized
    assert_eq!(
        FileSizeClass::from_size(5 * 1024 * 1024),
        FileSizeClass::Oversized
    );
    // Exactly at max is still Oversized (<=)
    assert_eq!(
        FileSizeClass::from_size(MAX_FILE_SIZE),
        FileSizeClass::Oversized
    );
}

#[test]
fn classify_too_large_file() {
    // Just over max
    assert_eq!(
        FileSizeClass::from_size(MAX_FILE_SIZE + 1),
        FileSizeClass::TooLarge
    );
    // 15MB should be TooLarge
    assert_eq!(
        FileSizeClass::from_size(15 * 1024 * 1024),
        FileSizeClass::TooLarge
    );
    // 1GB should be TooLarge
    assert_eq!(
        FileSizeClass::from_size(1024 * 1024 * 1024),
        FileSizeClass::TooLarge
    );
}

#[test]
fn human_size_bytes() {
    assert_eq!(human_size(0, false), "0B");
    assert_eq!(human_size(512, false), "512B");
    assert_eq!(human_size(1023, false), "1023B");
}

#[test]
fn human_size_kilobytes() {
    assert_eq!(human_size(1024, false), "1.0KB");
    assert_eq!(human_size(1536, false), "1.5KB");
    assert_eq!(human_size(500 * 1024, false), "500.0KB");
}

#[test]
fn human_size_megabytes() {
    assert_eq!(human_size(1024 * 1024, false), "1.0MB");
    assert_eq!(human_size(15 * 1024 * 1024, false), "15.0MB");
    assert_eq!(human_size(10 * 1024 * 1024 + 512 * 1024, false), "10.5MB");
}

#[test]
fn human_size_spaced() {
    assert_eq!(human_size(0, true), "0 B");
    assert_eq!(human_size(1024, true), "1.0 KB");
    assert_eq!(human_size(1024 * 1024, true), "1.0 MB");
}

#[test]
fn constants_are_correct() {
    // Verify the constants match docs/specs/20-performance.md
    assert_eq!(MMAP_THRESHOLD, 64 * 1024); // 64KB
    assert_eq!(SOFT_LIMIT_SIZE, 1024 * 1024); // 1MB
    assert_eq!(MAX_FILE_SIZE, 10 * 1024 * 1024); // 10MB
}
