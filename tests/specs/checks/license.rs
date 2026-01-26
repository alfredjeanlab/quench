//! Behavioral specs for the license check.
//!
//! Tests that quench correctly:
//! - Detects SPDX-License-Identifier headers
//! - Reports missing, wrong, and outdated headers
//! - Preserves shebangs when fixing
//!
//! Reference: docs/specs/checks/license-headers.md

#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::prelude::*;

// =============================================================================
// HEADER DETECTION SPECS
// =============================================================================

/// Spec: docs/specs/checks/license-headers.md#header-format
///
/// > Uses SPDX license identifiers for standardization
#[test]
fn license_detects_spdx_header() {
    check("license")
        .on("license/valid-headers")
        .args(&["--ci"])
        .passes();
}

/// Spec: docs/specs/checks/license-headers.md#validation-rules
///
/// > File has no SPDX or copyright line
#[test]
fn license_missing_header_generates_violation() {
    let license = check("license")
        .on("license/missing-header")
        .args(&["--ci"])
        .json()
        .fails();
    assert!(license.has_violation("missing_header"));
}

/// Spec: docs/specs/checks/license-headers.md#wrong-license
///
/// > File has different SPDX identifier than configured
#[test]
fn license_wrong_license_generates_violation() {
    let license = check("license")
        .on("license/wrong-license")
        .args(&["--ci"])
        .json()
        .fails();

    let violation = license.require_violation("wrong_license");
    assert_eq!(
        violation.get("expected").and_then(|v| v.as_str()),
        Some("MIT")
    );
    assert_eq!(
        violation.get("found").and_then(|v| v.as_str()),
        Some("Apache-2.0")
    );
}

/// Spec: docs/specs/checks/license-headers.md#outdated-copyright-year
///
/// > Copyright year doesn't include current year
#[test]
fn license_outdated_year_generates_violation() {
    let license = check("license")
        .on("license/outdated-year")
        .args(&["--ci"])
        .json()
        .fails();

    let violation = license.require_violation("outdated_year");
    assert!(violation.get("found").is_some());
    assert!(violation.get("expected").is_some());
}

// =============================================================================
// FIX MODE SPECS
// =============================================================================

/// Spec: docs/specs/checks/license-headers.md#auto-fix
///
/// > Add missing headers: Insert header at file start
#[test]
#[ignore = "TODO: Phase 1120 - License Check Fix"]
fn license_fix_adds_missing_header() {
    let temp = Project::empty();
    temp.config(
        r#"
[check.license]
check = "error"
license = "MIT"
copyright = "Test Org"

[check.license.patterns]
rust = ["**/*.rs"]
"#,
    );
    temp.file("src/lib.rs", "pub fn hello() {}\n");

    check("license")
        .pwd(temp.path())
        .args(&["--ci", "--fix"])
        .passes();

    let content = std::fs::read_to_string(temp.path().join("src/lib.rs")).unwrap();
    assert!(content.contains("SPDX-License-Identifier: MIT"));
    assert!(content.contains("Copyright"));
    assert!(content.contains("Test Org"));
}

/// Spec: docs/specs/checks/license-headers.md#auto-fix
///
/// > Update copyright year: Change year to current year
#[test]
#[ignore = "TODO: Phase 1120 - License Check Fix"]
fn license_fix_updates_outdated_year() {
    let temp = Project::empty();
    temp.config(
        r#"
[check.license]
check = "error"
license = "MIT"
copyright = "Test Org"

[check.license.patterns]
rust = ["**/*.rs"]
"#,
    );
    temp.file(
        "src/lib.rs",
        "// SPDX-License-Identifier: MIT\n// Copyright (c) 2020 Test Org\n\npub fn hello() {}\n",
    );

    check("license")
        .pwd(temp.path())
        .args(&["--ci", "--fix"])
        .passes();

    let content = std::fs::read_to_string(temp.path().join("src/lib.rs")).unwrap();
    assert!(
        content.contains("2026"),
        "year should be updated to current year"
    );
}

/// Spec: docs/specs/checks/license-headers.md#header-format
///
/// > Shebangs are preserved at the top of shell scripts
#[test]
#[ignore = "TODO: Phase 1120 - License Check Fix"]
fn license_fix_preserves_shebang() {
    let temp = Project::empty();
    temp.config(
        r#"
[check.license]
check = "error"
license = "MIT"
copyright = "Test Org"

[check.license.patterns]
shell = ["**/*.sh"]
"#,
    );
    temp.file("scripts/run.sh", "#!/bin/bash\n\necho 'hello'\n");

    check("license")
        .pwd(temp.path())
        .args(&["--ci", "--fix"])
        .passes();

    let content = std::fs::read_to_string(temp.path().join("scripts/run.sh")).unwrap();
    assert!(
        content.starts_with("#!/bin/bash\n"),
        "shebang should be first line"
    );
    assert!(content.contains("SPDX-License-Identifier: MIT"));
    // Verify header is after shebang
    let shebang_pos = content.find("#!/bin/bash").unwrap();
    let spdx_pos = content.find("SPDX-License-Identifier").unwrap();
    assert!(spdx_pos > shebang_pos, "SPDX should come after shebang");
}

// =============================================================================
// OUTPUT FORMAT SPECS
// =============================================================================

/// Spec: docs/specs/checks/license-headers.md#output
///
/// > Missing header shows file and advice
#[test]
fn exact_missing_header_text() {
    check("license")
        .on("license/missing-header")
        .args(&["--ci"])
        .fails()
        .stdout_has("license: FAIL")
        .stdout_has("missing license header");
}

/// Spec: docs/specs/checks/license-headers.md#json-output
///
/// > Violation types: `missing_header`, `outdated_year`, `wrong_license`
#[test]
fn license_violation_types_are_expected_values() {
    let license = check("license")
        .on("license/mixed-violations")
        .args(&["--ci"])
        .json()
        .fails();
    let violations = license.require("violations").as_array().unwrap();

    let valid_types = ["missing_header", "outdated_year", "wrong_license"];
    for v in violations {
        let vtype = v.get("type").and_then(|t| t.as_str()).unwrap();
        assert!(
            valid_types.contains(&vtype),
            "unexpected violation type: {}",
            vtype
        );
    }
}

/// Spec: docs/specs/checks/license-headers.md#json-output
///
/// > Metrics include files_checked, files_with_headers, etc.
#[test]
fn license_json_includes_metrics() {
    let license = check("license")
        .on("license/valid-headers")
        .args(&["--ci"])
        .json()
        .passes();

    let metrics = license.require("metrics");
    assert!(metrics.get("files_checked").is_some());
    assert!(metrics.get("files_with_headers").is_some());
}

/// Spec: docs/specs/checks/license-headers.md#fixed
///
/// > FIXED output shows counts
#[test]
#[ignore = "TODO: Phase 1120 - License Check Fix"]
fn exact_fix_output_text() {
    let temp = Project::empty();
    temp.config(
        r#"
[check.license]
check = "error"
license = "MIT"
copyright = "Test Org"

[check.license.patterns]
rust = ["**/*.rs"]
"#,
    );
    temp.file("src/lib.rs", "pub fn hello() {}\n");

    check("license")
        .pwd(temp.path())
        .args(&["--ci", "--fix"])
        .passes()
        .stdout_has("FIXED");
}

// =============================================================================
// CI-ONLY BEHAVIOR SPECS
// =============================================================================

/// Spec: docs/specs/checks/license-headers.md
///
/// > CI-only. This check only runs in `--ci` mode. It is skipped in fast mode.
#[test]
fn license_skipped_without_ci_flag() {
    // Without --ci, license check passes silently (CI-only check)
    // It doesn't run violation detection, just passes
    let license = check("license")
        .on("license/missing-header")
        .json()
        .passes();

    // Verify no violations were detected (check didn't actually run)
    assert!(
        license.violations().is_empty(),
        "license check should not detect violations without --ci"
    );

    // With --ci, the same fixture should fail (has missing header)
    check("license")
        .on("license/missing-header")
        .args(&["--ci"])
        .json()
        .fails();
}

/// Spec: docs/specs/checks/license-headers.md#configuration
///
/// > Disabled by default. Enable explicitly when your project requires license headers.
#[test]
fn license_disabled_by_default() {
    let temp = Project::empty();
    temp.config(""); // No license config
    temp.file("src/lib.rs", "pub fn hello() {}\n");

    // Should pass even with --ci because license check is off by default
    check("license").pwd(temp.path()).args(&["--ci"]).passes();
}
