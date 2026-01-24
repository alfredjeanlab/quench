# Checkpoint 16H: Tech Debt - Report Command

## Overview

Address remaining technical debt in the report command after the f24fda0 refactoring. Focus on:
- Consolidating duplicated test infrastructure across formatter test files
- Unifying the HtmlFormatter to use the macro pattern adopted by Text and Markdown
- Extracting repeated metric sorting/iteration patterns into helper methods

## Project Structure

```
crates/cli/src/report/
├── mod.rs           # FilteredMetrics, ReportFormatter trait, format_report functions
├── mod_tests.rs     # Tests for FilteredMetrics, human_bytes
├── text.rs          # TextFormatter with write_text_report! macro
├── text_tests.rs    # TextFormatter tests
├── markdown.rs      # MarkdownFormatter with write_markdown_report! macro
├── markdown_tests.rs
├── html.rs          # HtmlFormatter (target for macro refactoring)
├── html_tests.rs
├── json.rs          # JsonFormatter (no changes needed)
├── json_tests.rs    # (no changes needed)
└── test_support.rs  # NEW: Shared test utilities
```

## Dependencies

No new external dependencies required.

## Implementation Phases

### Phase 1: Extract Test Infrastructure

Create shared test utilities to eliminate duplication across formatter test files.

**File: `crates/cli/src/report/test_support.rs`**

```rust
// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Shared test utilities for report formatter tests.

use crate::baseline::{
    Baseline, BaselineMetrics, BuildTimeMetrics, CoverageMetrics,
    EscapesMetrics, TestTimeMetrics,
};
use crate::cli::CheckFilter;

/// Test filter that includes all checks.
pub struct AllChecks;

impl CheckFilter for AllChecks {
    fn enabled_checks(&self) -> Vec<String> {
        Vec::new() // Empty means all enabled
    }

    fn disabled_checks(&self) -> Vec<String> {
        Vec::new()
    }
}

/// Test filter that excludes specific checks.
pub struct ExcludeChecks(pub Vec<&'static str>);

impl CheckFilter for ExcludeChecks {
    fn enabled_checks(&self) -> Vec<String> {
        Vec::new()
    }

    fn disabled_checks(&self) -> Vec<String> {
        self.0.iter().map(|s| s.to_string()).collect()
    }
}

/// Create a standard test baseline with all metric types populated.
pub fn create_test_baseline() -> Baseline {
    Baseline {
        version: 1,
        updated: chrono::Utc::now(),
        commit: Some("abc1234".to_string()),
        metrics: BaselineMetrics {
            coverage: Some(CoverageMetrics {
                total: 85.5,
                by_package: None,
            }),
            escapes: Some(EscapesMetrics {
                source: [("unwrap".to_string(), 10)].into_iter().collect(),
                test: None,
            }),
            build_time: Some(BuildTimeMetrics {
                cold: 45.0,
                hot: 12.5,
            }),
            binary_size: Some([("quench".to_string(), 5_242_880)].into_iter().collect()),
            test_time: Some(TestTimeMetrics {
                total: 30.5,
                avg: 0.5,
                max: 2.0,
            }),
        },
    }
}
```

**Update `mod.rs`:**
```rust
#[cfg(test)]
mod test_support;
```

**Update each test file to use shared utilities:**
```rust
// text_tests.rs, markdown_tests.rs, html_tests.rs
use super::test_support::{AllChecks, create_test_baseline};

// mod_tests.rs (also uses ExcludeChecks)
use super::test_support::{AllChecks, ExcludeChecks, create_test_baseline};
```

**Verification:**
- `cargo test --package quench report::` passes
- No duplicated `AllChecks`/`ExcludeChecks`/`create_test_baseline` definitions in test files

---

### Phase 2: Add Sorted Iteration Helpers to FilteredMetrics

Add helper methods to encapsulate the repeated sorting pattern for HashMap iteration.

**Add to `mod.rs` in `impl<'a> FilteredMetrics<'a>`:**

```rust
/// Iterate over escape source metrics in sorted order.
/// Returns None if escapes check is filtered out.
pub fn sorted_escapes(&self) -> Option<Vec<(&str, u32)>> {
    self.escapes().map(|esc| {
        let mut items: Vec<_> = esc.source.iter()
            .map(|(k, v)| (k.as_str(), *v))
            .collect();
        items.sort_by_key(|(k, _)| *k);
        items
    })
}

/// Iterate over test escape metrics in sorted order.
/// Returns None if escapes check is filtered out or no test escapes present.
pub fn sorted_test_escapes(&self) -> Option<Vec<(&str, u32)>> {
    self.escapes().and_then(|esc| {
        esc.test.as_ref().map(|test| {
            let mut items: Vec<_> = test.iter()
                .map(|(k, v)| (k.as_str(), *v))
                .collect();
            items.sort_by_key(|(k, _)| *k);
            items
        })
    })
}

/// Iterate over coverage by package in sorted order.
/// Returns None if tests check is filtered out or no package coverage.
pub fn sorted_package_coverage(&self) -> Option<Vec<(&str, f64)>> {
    self.coverage().and_then(|cov| {
        cov.by_package.as_ref().map(|packages| {
            let mut items: Vec<_> = packages.iter()
                .map(|(k, v)| (k.as_str(), *v))
                .collect();
            items.sort_by_key(|(k, _)| *k);
            items
        })
    })
}

/// Iterate over binary sizes in sorted order.
/// Returns None if build check is filtered out or no binary sizes.
pub fn sorted_binary_sizes(&self) -> Option<Vec<(&str, u64)>> {
    self.binary_size().map(|sizes| {
        let mut items: Vec<_> = sizes.iter()
            .map(|(k, v)| (k.as_str(), *v))
            .collect();
        items.sort_by_key(|(k, _)| *k);
        items
    })
}
```

**Verification:**
- Add unit tests for each new method in `mod_tests.rs`
- `cargo test --package quench report::` passes

---

### Phase 3: Update Formatters to Use Sorted Helpers

Refactor text, markdown, and html formatters to use the new sorted iteration helpers.

**Example change in `write_text_report!` macro:**

Before:
```rust
if let Some(escapes) = $filtered.escapes() {
    let mut keys: Vec<_> = escapes.source.keys().collect();
    keys.sort();
    for name in keys {
        writeln!($writer, "escapes.{}: {}", name, escapes.source[name])?;
    }
}
```

After:
```rust
if let Some(items) = $filtered.sorted_escapes() {
    for (name, count) in items {
        writeln!($writer, "escapes.{}: {}", name, count)?;
    }
}
```

Apply similar changes to:
- `text.rs`: `write_text_report!` macro (escapes, test escapes, packages, binary sizes)
- `markdown.rs`: `write_markdown_report!` macro (same sections)
- `html.rs`: `collect_metrics` method (same sections)

**Verification:**
- `cargo test --package quench report::` passes
- Existing output format tests confirm no behavioral changes

---

### Phase 4: Unify HtmlFormatter with Macro Pattern

Convert HtmlFormatter to use a macro like Text and Markdown, eliminating duplication between `format()` and `format_to()`.

**Create `write_html_report!` macro:**

```rust
macro_rules! write_html_report {
    ($writer:expr, $baseline:expr, $filtered:expr) => {{
        let (commit, date) = Self::render_header($baseline);
        let css = Self::css();

        // Write document start
        write!($writer, r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Quench Report</title>
  <style>
    {css}
  </style>
</head>
<body>
  <div class="container">
    <header>
      <h1>Quench Report</h1>
      <div class="meta">Baseline: {commit} &middot; {date}</div>
    </header>
    <section class="cards">
"#)?;

        // Write cards inline (no intermediate Vec)
        // Coverage card
        if let Some(coverage) = $filtered.coverage() {
            writeln!($writer, "{}", Self::render_card("Coverage", &format!("{:.1}%", coverage.total), "tests"))?;
        }
        // ... escapes, build, binary, test cards

        // Write table section start
        write!($writer, r#"    </section>
    <section>
      <table>
        <thead><tr><th>Metric</th><th>Value</th></tr></thead>
        <tbody>
"#)?;

        // Write table rows inline (no intermediate Vec)
        // ... rows

        // Write document end
        write!($writer, r#"        </tbody>
      </table>
    </section>
  </div>
</body>
</html>"#)?;
    }};
}
```

**Simplify `ReportFormatter` impl:**

```rust
impl ReportFormatter for HtmlFormatter {
    fn format(&self, baseline: &Baseline, filter: &dyn CheckFilter) -> anyhow::Result<String> {
        use std::fmt::Write;
        let filtered = FilteredMetrics::new(baseline, filter);
        let capacity = HTML_BASE_SIZE + filtered.count() * (HTML_CARD_SIZE + HTML_ROW_SIZE);
        let mut output = String::with_capacity(capacity);
        write_html_report!(&mut output, baseline, &filtered);
        Ok(output)
    }

    fn format_to(
        &self,
        writer: &mut dyn std::io::Write,
        baseline: &Baseline,
        filter: &dyn CheckFilter,
    ) -> anyhow::Result<()> {
        let filtered = FilteredMetrics::new(baseline, filter);
        write_html_report!(writer, baseline, &filtered);
        Ok(())
    }

    fn format_empty(&self) -> String { /* unchanged */ }
}
```

**Benefits:**
- Removes `collect_metrics()` method and its Vec allocations
- Removes `render_document()` method (template now in macro only)
- Single source of truth for HTML structure
- Estimated ~60 lines reduction

**Verification:**
- `cargo test --package quench report::html` passes
- `html_format_to_matches_format` test confirms consistency
- Output format unchanged

---

### Phase 5: Cleanup and Documentation

Final cleanup pass:

1. Remove any dead code or unused helper methods
2. Ensure consistent doc comments across all formatter modules
3. Run `cargo fmt` and `cargo clippy`
4. Run full `make check`

**Verification:**
- `make check` passes
- No clippy warnings
- Code coverage maintained

## Key Implementation Details

### Macro Pattern for Formatters

The macro pattern allows sharing formatting logic between `fmt::Write` (String) and `io::Write` (files/stdout):

```rust
macro_rules! write_format_report {
    ($writer:expr, ...) => {
        writeln!($writer, ...)?;  // Works with both trait bounds
    };
}
```

This works because:
- `writeln!` on `&mut String` uses `fmt::Write`
- `writeln!` on `&mut dyn io::Write` uses `io::Write`
- Both return `Result<(), _>` with `?` propagation

### Test Infrastructure Sharing

Using `#[cfg(test)]` module for shared test utilities:

```rust
// In mod.rs
#[cfg(test)]
mod test_support;

// In test files
use super::test_support::*;
```

This keeps test utilities compiled only in test mode while making them accessible to sibling test modules.

## Verification Plan

1. **Unit Tests**: Run `cargo test --package quench report::` after each phase
2. **Integration**: Run `make check` after Phase 5
3. **Output Stability**: The `*_format_to_matches_format` tests ensure streaming matches buffered output
4. **No Behavioral Changes**: All existing assertion tests should pass unchanged

## Summary of Changes

| Phase | Files Modified | Lines Changed (est.) |
|-------|---------------|---------------------|
| 1     | mod.rs, test_support.rs (new), 4 test files | +60, -100 |
| 2     | mod.rs, mod_tests.rs | +50 |
| 3     | text.rs, markdown.rs, html.rs | -30 |
| 4     | html.rs | -60 |
| 5     | Various | Minimal |

**Net result**: ~80 fewer lines, better code organization, consistent patterns across all formatters.
