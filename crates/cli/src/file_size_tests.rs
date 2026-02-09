// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

use super::*;

#[test]
fn file_size_classification() {
    let cases = [
        // Small class
        (1024, FileSizeClass::Small, "1KB"),
        (63 * 1024, FileSizeClass::Small, "63KB"),
        (MMAP_THRESHOLD, FileSizeClass::Small, "at mmap threshold"),
        // Normal class
        (MMAP_THRESHOLD + 1, FileSizeClass::Normal, "just over mmap threshold"),
        (500 * 1024, FileSizeClass::Normal, "500KB"),
        (SOFT_LIMIT_SIZE, FileSizeClass::Normal, "at soft limit"),
        // Oversized class
        (SOFT_LIMIT_SIZE + 1, FileSizeClass::Oversized, "just over soft limit"),
        (5 * 1024 * 1024, FileSizeClass::Oversized, "5MB"),
        (MAX_FILE_SIZE, FileSizeClass::Oversized, "at max"),
        // TooLarge class
        (MAX_FILE_SIZE + 1, FileSizeClass::TooLarge, "just over max"),
        (15 * 1024 * 1024, FileSizeClass::TooLarge, "15MB"),
        (1024 * 1024 * 1024, FileSizeClass::TooLarge, "1GB"),
    ];

    for (size, expected, desc) in cases {
        assert_eq!(
            FileSizeClass::from_size(size),
            expected,
            "Failed for {} ({} bytes)",
            desc,
            size
        );
    }
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
fn human_size_gigabytes() {
    assert_eq!(human_size(1024 * 1024 * 1024, false), "1.0GB");
    assert_eq!(human_size(2 * 1024 * 1024 * 1024, false), "2.0GB");
    assert_eq!(human_size(1024 * 1024 * 1024 + 512 * 1024 * 1024, false), "1.5GB");
}

#[test]
fn human_size_spaced() {
    assert_eq!(human_size(0, true), "0 B");
    assert_eq!(human_size(1024, true), "1.0 KB");
    assert_eq!(human_size(1024 * 1024, true), "1.0 MB");
    assert_eq!(human_size(1024 * 1024 * 1024, true), "1.0 GB");
}

#[test]
fn constants_are_correct() {
    // Verify the constants match docs/specs/20-performance.md
    assert_eq!(MMAP_THRESHOLD, 64 * 1024); // 64KB
    assert_eq!(SOFT_LIMIT_SIZE, 1024 * 1024); // 1MB
    assert_eq!(MAX_FILE_SIZE, 10 * 1024 * 1024); // 10MB
}
