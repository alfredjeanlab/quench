//! Behavioral specs for --timing flag.
//!
//! Reference: docs/specs/01-cli.md (--timing flag)
//! Reference: docs/specs/20-performance.md (Performance Model)

#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::prelude::*;

// =============================================================================
// PHASE BREAKDOWN SPECS
// =============================================================================

/// Spec: --timing shows phase breakdown (discovery, reading, checking, output)
///
/// Per docs/specs/20-performance.md:
/// "Total Time = File Discovery + File Reading + Pattern Matching + Aggregation"
#[test]
#[ignore = "TODO: Phase 1398 - implement --timing flag"]
fn timing_shows_phase_breakdown() {
    let temp = Project::empty();
    temp.config(
        r#"
        [check.cloc]
        max_lines = 1000
    "#,
    );
    temp.file("src/main.rs", "fn main() {}");

    cli()
        .args(&["--timing"])
        .pwd(temp.path())
        .passes()
        .stderr_has("discovery:")
        .stderr_has("checking:")
        .stderr_has("output:")
        .stderr_has("total:");
}

/// Spec: Phase breakdown shows millisecond timing
#[test]
#[ignore = "TODO: Phase 1398 - implement --timing flag"]
fn timing_phases_show_milliseconds() {
    let temp = Project::empty();
    temp.config(
        r#"
        [check.cloc]
        max_lines = 1000
    "#,
    );
    temp.file("src/main.rs", "fn main() {}");

    cli()
        .args(&["--timing"])
        .pwd(temp.path())
        .passes()
        .stderr_has("ms");
}

// =============================================================================
// PER-CHECK TIMING SPECS
// =============================================================================

/// Spec: --timing shows per-check timing
#[test]
#[ignore = "TODO: Phase 1398 - implement --timing flag"]
fn timing_shows_per_check_breakdown() {
    let temp = Project::empty();
    temp.config(
        r#"
        [check.cloc]
        max_lines = 1000

        [check.escapes]
        check = "error"
    "#,
    );
    temp.file("src/main.rs", "fn main() {}");

    cli()
        .args(&["--timing"])
        .pwd(temp.path())
        .passes()
        .stderr_has("cloc:")
        .stderr_has("escapes:");
}

/// Spec: Per-check timing only shows enabled checks
#[test]
#[ignore = "TODO: Phase 1398 - implement --timing flag"]
fn timing_only_shows_enabled_checks() {
    let temp = Project::empty();
    temp.config(
        r#"
        [check.cloc]
        max_lines = 1000

        [check.escapes]
        check = "off"
    "#,
    );
    temp.file("src/main.rs", "fn main() {}");

    cli()
        .args(&["--timing"])
        .pwd(temp.path())
        .passes()
        .stderr_has("cloc:")
        .stderr_lacks("escapes:");
}

// =============================================================================
// JSON OUTPUT SPECS
// =============================================================================

/// Spec: --timing works with -o json (adds timing field)
#[test]
#[ignore = "TODO: Phase 1398 - implement --timing flag"]
fn timing_with_json_adds_timing_field() {
    let temp = Project::empty();
    temp.config(
        r#"
        [check.cloc]
        max_lines = 1000
    "#,
    );
    temp.file("src/main.rs", "fn main() {}");

    cli()
        .args(&["--timing", "-o", "json"])
        .pwd(temp.path())
        .passes()
        .stdout_has(r#""timing":"#);
}

/// Spec: JSON timing includes phase breakdown
#[test]
#[ignore = "TODO: Phase 1398 - implement --timing flag"]
fn timing_json_includes_phases() {
    let temp = Project::empty();
    temp.config(
        r#"
        [check.cloc]
        max_lines = 1000
    "#,
    );
    temp.file("src/main.rs", "fn main() {}");

    cli()
        .args(&["--timing", "-o", "json"])
        .pwd(temp.path())
        .passes()
        .stdout_has(r#""discovery_ms":"#)
        .stdout_has(r#""checking_ms":"#)
        .stdout_has(r#""total_ms":"#);
}

/// Spec: JSON timing includes per-check breakdown
#[test]
#[ignore = "TODO: Phase 1398 - implement --timing flag"]
fn timing_json_includes_per_check() {
    let temp = Project::empty();
    temp.config(
        r#"
        [check.cloc]
        max_lines = 1000
    "#,
    );
    temp.file("src/main.rs", "fn main() {}");

    cli()
        .args(&["--timing", "-o", "json"])
        .pwd(temp.path())
        .passes()
        .stdout_has(r#""checks":"#);
}

// =============================================================================
// CACHE STATISTICS SPECS
// =============================================================================

/// Spec: --timing shows file count and cache hit rate
#[test]
#[ignore = "TODO: Phase 1398 - implement --timing flag"]
fn timing_shows_file_count() {
    let temp = Project::empty();
    temp.config(
        r#"
        [check.cloc]
        max_lines = 1000
    "#,
    );
    temp.file("src/main.rs", "fn main() {}");
    temp.file("src/lib.rs", "pub fn hello() {}");

    cli()
        .args(&["--timing"])
        .pwd(temp.path())
        .passes()
        .stderr_has("files:");
}

/// Spec: --timing shows cache statistics
#[test]
#[ignore = "TODO: Phase 1398 - implement --timing flag"]
fn timing_shows_cache_stats() {
    let temp = Project::empty();
    temp.config(
        r#"
        [check.cloc]
        max_lines = 1000
    "#,
    );
    temp.file("src/main.rs", "fn main() {}");

    // First run - cold cache
    cli()
        .args(&["--timing"])
        .pwd(temp.path())
        .passes()
        .stderr_has("cache:");

    // Second run - warm cache (should show hits)
    cli()
        .args(&["--timing"])
        .pwd(temp.path())
        .passes()
        .stderr_has("cache:");
}

/// Spec: JSON timing includes cache statistics
#[test]
#[ignore = "TODO: Phase 1398 - implement --timing flag"]
fn timing_json_includes_cache_stats() {
    let temp = Project::empty();
    temp.config(
        r#"
        [check.cloc]
        max_lines = 1000
    "#,
    );
    temp.file("src/main.rs", "fn main() {}");

    cli()
        .args(&["--timing", "-o", "json"])
        .pwd(temp.path())
        .passes()
        .stdout_has(r#""files":"#)
        .stdout_has(r#""cache_hits":"#);
}

// =============================================================================
// EDGE CASE SPECS
// =============================================================================

/// Spec: --timing works with failing checks
#[test]
#[ignore = "TODO: Phase 1398 - implement --timing flag"]
fn timing_works_with_failures() {
    let temp = Project::empty();
    temp.config(
        r#"
        [check.cloc]
        max_lines = 5
    "#,
    );
    // Create file that exceeds max_lines
    temp.file(
        "src/main.rs",
        "fn main() {\n    let a = 1;\n    let b = 2;\n    let c = 3;\n    let d = 4;\n    let e = 5;\n    let f = 6;\n}",
    );

    cli()
        .args(&["--timing"])
        .pwd(temp.path())
        .fails()
        .stderr_has("total:");
}

/// Spec: --timing with --no-cache shows zero cache hits
#[test]
#[ignore = "TODO: Phase 1398 - implement --timing flag"]
fn timing_no_cache_shows_zero_hits() {
    let temp = Project::empty();
    temp.config(
        r#"
        [check.cloc]
        max_lines = 1000
    "#,
    );
    temp.file("src/main.rs", "fn main() {}");

    // Warm the cache
    cli().pwd(temp.path()).passes();

    // Run with --no-cache
    cli()
        .args(&["--timing", "--no-cache"])
        .pwd(temp.path())
        .passes()
        .stderr_has("cache: 0/");
}

/// Spec: --timing without checks shows only discovery phase
#[test]
#[ignore = "TODO: Phase 1398 - implement --timing flag"]
fn timing_config_only_shows_discovery() {
    let temp = Project::empty();
    temp.config(
        r#"
        [check.cloc]
        max_lines = 1000
    "#,
    );
    temp.file("src/main.rs", "fn main() {}");

    cli()
        .args(&["--timing", "--config"])
        .pwd(temp.path())
        .passes()
        .stderr_lacks("discovery:") // --config doesn't walk files
        .stderr_lacks("checking:");
}
