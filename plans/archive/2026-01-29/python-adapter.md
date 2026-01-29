# Python Adapter Implementation Plan

Complete the Python adapter implementation: fix package name detection from pyproject.toml/setup.py and add suppress directive detection (# noqa, # type: ignore, # pylint: disable).

## Overview

The Python adapter infrastructure is largely complete, but two key features are missing integration:

1. **Package Detection** - `by_package` field not populated in cloc/escapes results
2. **Suppress Directives** - Python suppress checking not wired into escapes check

This plan unskips and passes 24 ignored tests:
- 19 tests for Phase 445 (Python Escapes)
- 5 tests for Phase 447 (Python Suppress)

Note: Phase 449 (Python Policy - 2 tests) is out of scope for this plan.

## Project Structure

```
crates/cli/src/
├── adapter/
│   └── python/
│       ├── mod.rs              # PythonAdapter, parse_pyproject_toml, detect_layout
│       ├── suppress.rs         # PythonSuppress, parse_python_suppresses (EXISTS)
│       └── package_manager.rs  # PackageManager detection (EXISTS)
├── checks/
│   └── escapes/
│       ├── mod.rs              # Main escapes check (ADD Python suppress)
│       ├── python_suppress.rs  # NEW: Python suppress integration
│       └── suppress_common.rs  # Generic suppress checking (ADD Python)
└── config/
    └── python.rs               # PythonConfig, PythonSuppressConfig (EXISTS)
```

## Dependencies

No new external dependencies required. Uses existing:
- `toml` - for parsing pyproject.toml (already used)
- `regex` - for parsing setup.py (already used)
- `globset` - for pattern matching (already used)

## Implementation Phases

### Phase 1: Implement SuppressConfigAccess for Python

**Goal**: Enable Python suppress config to work with generic suppress checking.

**Files**:
- `crates/cli/src/checks/escapes/suppress_common.rs`
- `crates/cli/src/config/mod.rs` (re-export)

**Changes**:

1. Add `SuppressConfigAccess` implementation for `PythonSuppressConfig`:

```rust
// In suppress_common.rs, add import
use crate::config::PythonSuppressConfig;

// Add implementation
impl SuppressConfigAccess for PythonSuppressConfig {
    fn check(&self) -> SuppressLevel {
        self.check
    }
    fn comment(&self) -> Option<&str> {
        self.comment.as_deref()
    }
    fn source(&self) -> &SuppressScopeConfig {
        &self.source
    }
    fn test(&self) -> &SuppressScopeConfig {
        &self.test
    }
}
```

2. Add Python fix guidance function:

```rust
fn get_python_fix_guidance(lint_code: &str) -> (&'static str, &'static str) {
    match lint_code {
        "E501" => (
            "Break long lines into smaller statements.",
            "Use implicit line continuation or extract complex expressions into variables.",
        ),
        "type-ignore" | "assignment" | "arg-type" | "return-value" => (
            "Fix the type error instead of ignoring it.",
            "Add proper type annotations or fix the type mismatch.",
        ),
        _ => (
            "Fix the underlying issue instead of suppressing the lint.",
            "Suppressions should only be used when the lint is a false positive.",
        ),
    }
}
```

3. Update `build_suppress_missing_comment_advice` to handle Python.

**Verification**: Unit tests in `suppress_common_tests.rs` pass.

---

### Phase 2: Create Python Suppress Check Module

**Goal**: Wire Python suppress directive parsing into the escapes check.

**Files**:
- `crates/cli/src/checks/escapes/python_suppress.rs` (NEW)
- `crates/cli/src/checks/escapes/mod.rs` (import and call)

**Changes**:

1. Create `python_suppress.rs`:

```rust
// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Python suppress directive checking for the escapes check.
//!
//! Checks `# noqa`, `# type: ignore`, and `# pylint: disable` directives.

use std::path::Path;

use crate::adapter::python::parse_python_suppresses;
use crate::check::{CheckContext, Violation};
use crate::config::PythonSuppressConfig;

use super::suppress_common::{UnifiedSuppressDirective, check_suppress_violations_generic};

/// Check Python suppress directives and return violations.
pub fn check_python_suppress_violations(
    ctx: &CheckContext,
    path: &Path,
    content: &str,
    config: &PythonSuppressConfig,
    is_test_file: bool,
    limit_reached: &mut bool,
) -> Vec<Violation> {
    // Parse suppress directives
    let suppresses = parse_python_suppresses(content, config.comment.as_deref());

    // Convert to unified format
    let unified: Vec<UnifiedSuppressDirective> = suppresses
        .into_iter()
        .map(|s| {
            let pattern = match s.kind {
                crate::adapter::python::PythonSuppressKind::Noqa => {
                    if s.codes.is_empty() {
                        "# noqa".to_string()
                    } else {
                        format!("# noqa: {}", s.codes.join(", "))
                    }
                }
                crate::adapter::python::PythonSuppressKind::TypeIgnore => {
                    if s.codes.is_empty() {
                        "# type: ignore".to_string()
                    } else {
                        format!("# type: ignore[{}]", s.codes.join(", "))
                    }
                }
                crate::adapter::python::PythonSuppressKind::PylintDisable => {
                    format!("# pylint: disable={}", s.codes.join(","))
                }
                crate::adapter::python::PythonSuppressKind::PragmaNoCover => {
                    "# pragma: no cover".to_string()
                }
            };
            UnifiedSuppressDirective {
                line: s.line,
                codes: s.codes,
                has_comment: s.has_comment,
                comment_text: s.comment_text,
                pattern,
            }
        })
        .collect();

    check_suppress_violations_generic(
        ctx,
        path,
        unified,
        config,
        "python",
        "suppress",
        is_test_file,
        limit_reached,
    )
}
```

2. Update `mod.rs` to call Python suppress checking:

```rust
// Add module declaration
mod python_suppress;

// Add import
use python_suppress::check_python_suppress_violations;

// In the run() method, after Ruby suppress checking (around line 217):
// Check for Python suppress directive violations
if has_extension(&file.path, &["py"]) {
    let python_violations = check_python_suppress_violations(
        ctx,
        relative,
        content,
        &ctx.config.python.suppress,
        is_test_file,
        &mut limit_reached,
    );
    violations.extend(python_violations);

    if limit_reached {
        break;
    }
}
```

**Verification**:
- Unit tests for python_suppress module
- `python_adapter_noqa_without_comment_fails_when_configured` passes
- `python_adapter_noqa_with_comment_passes` passes
- `python_adapter_type_ignore_without_comment_fails_when_configured` passes
- `python_adapter_noqa_in_test_code_always_passes` passes
- `python_adapter_pylint_disable_without_comment_fails_when_configured` passes

---

### Phase 3: Fix Python Escapes Check Activation

**Goal**: Ensure escapes check shows Python-specific patterns when Python project detected.

**Files**:
- `crates/cli/src/checks/escapes/patterns.rs`

**Changes**:

1. Verify `get_adapter_escape_patterns()` returns Python patterns when pyproject.toml/setup.py/etc. detected.

2. The existing `PYTHON_ESCAPE_PATTERNS` in `adapter/python/mod.rs` should be returned. Check that `get_adapter_escape_patterns()` detects Python projects correctly:

```rust
// In patterns.rs, ensure Python detection works
pub fn get_adapter_escape_patterns(root: &Path) -> Vec<EscapePattern> {
    // Check for Python project indicators
    let has_pyproject = root.join("pyproject.toml").exists();
    let has_setup_py = root.join("setup.py").exists();
    let has_setup_cfg = root.join("setup.cfg").exists();
    let has_requirements = root.join("requirements.txt").exists();

    if has_pyproject || has_setup_py || has_setup_cfg || has_requirements {
        return crate::adapter::python::PythonAdapter::default_escapes()
            .iter()
            .map(|p| EscapePattern { /* convert */ })
            .collect();
    }
    // ... other language detection
}
```

**Verification**:
- `python_adapter_auto_detected_when_pyproject_toml_present` passes
- `python_adapter_auto_detected_when_setup_py_present` passes
- `python_adapter_auto_detected_when_setup_cfg_present` passes
- `python_adapter_auto_detected_when_requirements_txt_present` passes

---

### Phase 4: Implement by_package for Python

**Goal**: Populate `by_package` field in cloc results for Python projects.

**Files**:
- `crates/cli/src/checks/cloc/mod.rs` (or equivalent)
- May need to create Python package detection integration

**Changes**:

1. When processing Python files, detect package from directory structure:

```rust
fn detect_python_package(path: &Path, root: &Path) -> Option<String> {
    let relative = path.strip_prefix(root).ok()?;
    let components: Vec<_> = relative.components().collect();

    // src-layout: src/package_name/...
    if components.len() >= 2 {
        if let Some(first) = components.get(0) {
            if first.as_os_str() == "src" {
                if let Some(second) = components.get(1) {
                    return Some(second.as_os_str().to_string_lossy().to_string());
                }
            }
        }
    }

    // flat-layout: package_name/...
    // Check if first component has __init__.py
    if let Some(first) = components.get(0) {
        let pkg_dir = root.join(first.as_os_str());
        if pkg_dir.join("__init__.py").exists() {
            return Some(first.as_os_str().to_string_lossy().to_string());
        }
    }

    None
}
```

2. Use `parse_pyproject_toml()` and `parse_setup_py()` to get canonical package name.

**Verification**:
- `python_adapter_detects_package_name_from_pyproject_toml` passes
- `python_adapter_detects_package_name_from_setup_py` passes
- `python_adapter_detects_src_layout_package` passes
- `python_adapter_detects_flat_layout_package` passes

---

### Phase 5: Fix Remaining Pattern Tests

**Goal**: Ensure default patterns work correctly for all Python fixtures.

**Files**:
- Test fixtures may need adjustment
- `crates/cli/src/adapter/python/mod.rs`

**Changes**:

1. Verify all default patterns work:
   - Source: `**/*.py`
   - Test: `tests/**/*.py`, `**/test_*.py`, `**/*_test.py`, `**/conftest.py`
   - Ignore: `.venv/**`, `__pycache__/**`, etc.

2. Ensure escape patterns trigger correctly:
   - `eval()` without `# EVAL:` fails
   - `eval()` with `# EVAL:` passes
   - `exec()` without `# EXEC:` fails
   - `__import__()` without `# DYNAMIC:` fails
   - `breakpoint()` always fails
   - `pdb.set_trace()` always fails
   - `import pdb` always fails

**Verification**:
- `python_adapter_default_source_pattern_matches_py_files` passes
- `python_adapter_default_test_pattern_matches_test_files` passes
- `python_adapter_default_ignores_venv_directory` passes
- `python_adapter_default_ignores_pycache_directory` passes
- `python_adapter_eval_without_comment_fails` passes
- `python_adapter_eval_with_comment_passes` passes
- `python_adapter_exec_without_comment_fails` passes
- `python_adapter_dynamic_import_without_comment_fails` passes
- `python_adapter_breakpoint_always_fails` passes
- `python_adapter_pdb_set_trace_always_fails` passes
- `python_adapter_import_pdb_always_fails` passes

---

## Key Implementation Details

### Suppress Directive Parsing

The existing `parse_python_suppresses()` in `adapter/python/suppress.rs` handles:

| Directive | Example | Codes Field |
|-----------|---------|-------------|
| noqa | `# noqa: E501` | `["E501"]` |
| type: ignore | `# type: ignore[assignment]` | `["assignment"]` |
| pylint: disable | `# pylint: disable=line-too-long` | `["line-too-long"]` |
| pragma: no cover | `# pragma: no cover` | `["coverage"]` |

### Comment Justification Detection

Comments are detected on the same line or preceding line:

```python
# Legacy API requires this
x = 1  # noqa: E501
```

Results in `has_comment = true`, `comment_text = Some("Legacy API requires this")`.

### Package Detection Priority

1. `pyproject.toml` `[project].name` (PEP 621)
2. `setup.py` `setup(name="...")`
3. Directory structure (src-layout vs flat-layout)

### Test vs Source Classification

Files are classified as test if they match test patterns:
- `tests/**/*.py`
- `**/test_*.py`
- `**/*_test.py`
- `**/conftest.py`

Test code can suppress freely by default (`[python.suppress.test] check = "allow"`).

---

## Verification Plan

### Unit Tests

1. `suppress_common_tests.rs` - Add Python-specific tests
2. `python_suppress_tests.rs` - New test file for Python suppress checking
3. `adapter/python/suppress_tests.rs` - Already exists, verify coverage

### Integration Tests (Specs)

Run each phase's tests with:

```bash
# Phase 2 - Suppress tests
cargo test --test specs python_adapter_noqa
cargo test --test specs python_adapter_type_ignore
cargo test --test specs python_adapter_pylint

# Phase 3-5 - Escapes and pattern tests
cargo test --test specs python_adapter_auto_detected
cargo test --test specs python_adapter_default
cargo test --test specs python_adapter_detects
cargo test --test specs python_adapter_eval
cargo test --test specs python_adapter_exec
cargo test --test specs python_adapter_breakpoint
cargo test --test specs python_adapter_pdb
cargo test --test specs python_adapter_import_pdb
```

### Full Test Suite

```bash
make check
```

### Manual Verification

Test on a real Python project:

```bash
cd /path/to/python-project
quench check escapes --json | jq .
quench check cloc --json | jq '.by_package'
```

---

## Test Summary

| Phase | Tests Unskipped |
|-------|-----------------|
| Phase 1 | 0 (internal) |
| Phase 2 | 5 (suppress) |
| Phase 3 | 4 (auto-detect) |
| Phase 4 | 4 (package) |
| Phase 5 | 11 (patterns + escapes) |
| **Total** | **24** |

All 24 ignored tests in `tests/specs/adapters/python.rs` should pass after implementation.
