// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

use super::*;

#[test]
fn phase_timing_format_text() {
    let timing = PhaseTiming { discovery_ms: 10, checking_ms: 50, output_ms: 5, total_ms: 65 };
    let text = timing.format_text();
    assert!(text.contains("discovery: 10ms"));
    assert!(text.contains("checking: 50ms"));
    assert!(text.contains("output: 5ms"));
    assert!(text.contains("total: 65ms"));
}

#[test]
fn timing_info_format_cache_with_hits() {
    let info = TimingInfo { cache_hits: 5, ..Default::default() };
    assert_eq!(info.format_cache(3), "cache: 5/8");
}

#[test]
fn timing_info_format_cache_zero_total() {
    let info = TimingInfo::default();
    assert_eq!(info.format_cache(0), "cache: 0/0");
}

#[test]
fn timing_info_format_cache_all_misses() {
    let info = TimingInfo::default();
    assert_eq!(info.format_cache(10), "cache: 0/10");
}
