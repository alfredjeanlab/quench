# Phase 1101: License Check - Specs

Write behavioral specifications for the `license` check that validates and auto-fixes license headers in source files.

## Overview

This phase creates behavioral specs (black-box tests) for the license check. The specs verify:
- SPDX-License-Identifier header detection
- Missing/wrong/outdated header violations
- `--fix` mode for adding/updating headers
- Shebang preservation in shell scripts

Reference: `docs/specs/checks/license-headers.md`

## Project Structure

```
tests/
├── fixtures/
│   └── license/
│       ├── valid-headers/        # All files have correct headers
│       ├── missing-header/       # Files without any license header
│       ├── wrong-license/        # SPDX identifier doesn't match config
│       ├── outdated-year/        # Copyright year is not current year
│       ├── with-shebang/         # Shell scripts with #! lines
│       └── mixed-violations/     # Multiple violation types
└── specs/
    └── checks/
        └── license.rs            # Behavioral specs
```

## Dependencies

No new external dependencies. Uses existing test infrastructure:
- `tests/specs/prelude.rs` - Test helpers (`check()`, `Project`, etc.)
- `assert_cmd`, `predicates` - CLI testing
- `serde_json` - JSON output validation

## Implementation Phases

### Phase 1: Test Fixtures (1 of 4)

Create test fixtures for each violation scenario.

**Fixture: `license/valid-headers/`**
```
license/valid-headers/
├── quench.toml
└── src/
    └── lib.rs       # Has valid SPDX + copyright
```

`quench.toml`:
```toml
version = 1

[check.license]
check = "error"
license = "MIT"
copyright = "Test Org"
```

`src/lib.rs`:
```rust
// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Test Org

pub fn hello() {}
```

**Fixture: `license/missing-header/`**

`src/lib.rs`:
```rust
pub fn hello() {}
```

**Fixture: `license/wrong-license/`**

`src/lib.rs`:
```rust
// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Test Org

pub fn hello() {}
```

**Fixture: `license/outdated-year/`**

`src/lib.rs`:
```rust
// SPDX-License-Identifier: MIT
// Copyright (c) 2025 Test Org

pub fn hello() {}
```

**Fixture: `license/with-shebang/`**

`scripts/run.sh`:
```bash
#!/bin/bash
# SPDX-License-Identifier: MIT
# Copyright (c) 2026 Test Org

echo "hello"
```

`scripts/missing.sh`:
```bash
#!/bin/bash

echo "no license"
```

**Fixture: `license/mixed-violations/`**

Multiple files with different violation types for testing multiple violations.

### Phase 2: Detection Specs (2 of 4)

Write specs for header detection and validation.

**File: `tests/specs/checks/license.rs`**

```rust
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
#[ignore = "TODO: Phase 1105 - License Check Detection"]
fn license_detects_spdx_header() {
    check("license").on("license/valid-headers").args(&["--ci"]).passes();
}

/// Spec: docs/specs/checks/license-headers.md#validation-rules
///
/// > File has no SPDX or copyright line
#[test]
#[ignore = "TODO: Phase 1105 - License Check Detection"]
fn license_missing_header_generates_violation() {
    let license = check("license").on("license/missing-header").args(&["--ci"]).json().fails();
    assert!(license.has_violation("missing_header"));
}

/// Spec: docs/specs/checks/license-headers.md#wrong-license
///
/// > File has different SPDX identifier than configured
#[test]
#[ignore = "TODO: Phase 1105 - License Check Detection"]
fn license_wrong_license_generates_violation() {
    let license = check("license").on("license/wrong-license").args(&["--ci"]).json().fails();

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
#[ignore = "TODO: Phase 1105 - License Check Detection"]
fn license_outdated_year_generates_violation() {
    let license = check("license").on("license/outdated-year").args(&["--ci"]).json().fails();

    let violation = license.require_violation("outdated_year");
    assert!(violation.get("found").is_some());
    assert!(violation.get("expected").is_some());
}
```

### Phase 3: Fix Mode Specs (3 of 4)

Write specs for `--fix` behavior.

```rust
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
    temp.config(r#"
[check.license]
check = "error"
license = "MIT"
copyright = "Test Org"

[check.license.patterns]
rust = ["**/*.rs"]
"#);
    temp.file("src/lib.rs", "pub fn hello() {}\n");

    check("license").pwd(temp.path()).args(&["--ci", "--fix"]).passes();

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
    temp.config(r#"
[check.license]
check = "error"
license = "MIT"
copyright = "Test Org"

[check.license.patterns]
rust = ["**/*.rs"]
"#);
    temp.file("src/lib.rs", "// SPDX-License-Identifier: MIT\n// Copyright (c) 2020 Test Org\n\npub fn hello() {}\n");

    check("license").pwd(temp.path()).args(&["--ci", "--fix"]).passes();

    let content = std::fs::read_to_string(temp.path().join("src/lib.rs")).unwrap();
    assert!(content.contains("2026"), "year should be updated to current year");
}

/// Spec: docs/specs/checks/license-headers.md#header-format
///
/// > Shebangs are preserved at the top of shell scripts
#[test]
#[ignore = "TODO: Phase 1120 - License Check Fix"]
fn license_fix_preserves_shebang() {
    let temp = Project::empty();
    temp.config(r#"
[check.license]
check = "error"
license = "MIT"
copyright = "Test Org"

[check.license.patterns]
shell = ["**/*.sh"]
"#);
    temp.file("scripts/run.sh", "#!/bin/bash\n\necho 'hello'\n");

    check("license").pwd(temp.path()).args(&["--ci", "--fix"]).passes();

    let content = std::fs::read_to_string(temp.path().join("scripts/run.sh")).unwrap();
    assert!(content.starts_with("#!/bin/bash\n"), "shebang should be first line");
    assert!(content.contains("SPDX-License-Identifier: MIT"));
    // Verify header is after shebang
    let shebang_pos = content.find("#!/bin/bash").unwrap();
    let spdx_pos = content.find("SPDX-License-Identifier").unwrap();
    assert!(spdx_pos > shebang_pos, "SPDX should come after shebang");
}
```

### Phase 4: Output Format Specs (4 of 4)

Write specs for exact output format and JSON structure.

```rust
// =============================================================================
// OUTPUT FORMAT SPECS
// =============================================================================

/// Spec: docs/specs/checks/license-headers.md#output
///
/// > Missing header shows file and advice
#[test]
#[ignore = "TODO: Phase 1105 - License Check Detection"]
fn exact_missing_header_text() {
    check("license").on("license/missing-header").args(&["--ci"]).fails().stdout_has(
        "license: FAIL"
    ).stdout_has(
        "missing license header"
    );
}

/// Spec: docs/specs/checks/license-headers.md#json-output
///
/// > Violation types: `missing_header`, `outdated_year`, `wrong_license`
#[test]
#[ignore = "TODO: Phase 1105 - License Check Detection"]
fn license_violation_types_are_expected_values() {
    let license = check("license").on("license/mixed-violations").args(&["--ci"]).json().fails();
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
#[ignore = "TODO: Phase 1105 - License Check Detection"]
fn license_json_includes_metrics() {
    let license = check("license").on("license/valid-headers").args(&["--ci"]).json().passes();

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
    temp.config(r#"
[check.license]
check = "error"
license = "MIT"
copyright = "Test Org"

[check.license.patterns]
rust = ["**/*.rs"]
"#);
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
#[ignore = "TODO: Phase 1105 - License Check Detection"]
fn license_skipped_without_ci_flag() {
    // Without --ci, license check should not run (fast mode)
    let result = cli().on("license/missing-header").json().passes();
    let checks = result.checks();

    // License check should not appear in fast mode output
    let license_check = checks.iter().find(|c| {
        c.get("name").and_then(|n| n.as_str()) == Some("license")
    });
    assert!(
        license_check.is_none() ||
        license_check.unwrap().get("skipped").and_then(|s| s.as_bool()) == Some(true),
        "license check should be skipped without --ci"
    );
}

/// Spec: docs/specs/checks/license-headers.md#configuration
///
/// > Disabled by default. Enable explicitly when your project requires license headers.
#[test]
#[ignore = "TODO: Phase 1105 - License Check Detection"]
fn license_disabled_by_default() {
    let temp = Project::empty();
    temp.config(""); // No license config
    temp.file("src/lib.rs", "pub fn hello() {}\n");

    // Should pass even with --ci because license check is off by default
    check("license").pwd(temp.path()).args(&["--ci"]).passes();
}
```

## Key Implementation Details

### Header Format Pattern

The spec requires detecting this format:
```
// SPDX-License-Identifier: <LICENSE>
// Copyright (c) <YEAR> <HOLDER>
```

For shell scripts with shebang:
```
#!/bin/bash
# SPDX-License-Identifier: <LICENSE>
# Copyright (c) <YEAR> <HOLDER>
```

### Comment Syntax by Extension

| Extensions | Comment Style |
|------------|---------------|
| `.rs`, `.ts`, `.tsx`, `.js`, `.go`, `.c`, `.cpp`, `.h` | `// ` |
| `.sh`, `.bash`, `.py`, `.rb`, `.yaml`, `.yml` | `# ` |
| `.html`, `.xml` | `<!-- -->` |

### Violation Types

Exactly three violation types:
- `missing_header` - No SPDX-License-Identifier found
- `wrong_license` - SPDX identifier doesn't match configured license
- `outdated_year` - Copyright year doesn't include current year

### Shebang Detection

When fixing, must detect lines starting with `#!` and insert header after them.

## Verification Plan

### Running Specs

```bash
# Run all license specs (will show ignored count)
cargo test --test specs license -- --ignored

# Verify specs compile
cargo test --test specs license -- --list
```

### Checklist

- [ ] All fixtures created in `tests/fixtures/license/`
- [ ] Spec file `tests/specs/checks/license.rs` added
- [ ] Module declared in `tests/specs/checks/mod.rs`
- [ ] All specs have `#[ignore = "TODO: Phase N - ..."]`
- [ ] Doc comments reference spec document sections
- [ ] `cargo test --test specs` compiles without errors
- [ ] Fixture README updated if needed

### Spec Count

Expected: ~12 specs covering:
- Detection (4): valid headers, missing, wrong license, outdated year
- Fix mode (3): add header, update year, preserve shebang
- Output (3): text format, violation types, metrics
- Behavior (2): CI-only, disabled by default
