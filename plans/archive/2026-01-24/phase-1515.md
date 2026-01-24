# Phase 1515: Init Output Template

**Root Feature:** `quench-init`

## Overview

Update `quench init` to generate a full default configuration template matching `docs/specs/templates/init.default.toml`. Currently, running `quench init` without profiles outputs only `version = 1`. After this phase, it will output the complete template with all check sections, explicit check levels, and the supported languages comment block.

## Project Structure

Files to modify:

```
crates/cli/src/
├── cli.rs          # Add default_template() generator function
└── main.rs         # Update run_init to use new template

tests/specs/cli/
└── init.rs         # Enable Phase 1515 spec, verify template format
```

Reference files:

```
docs/specs/templates/
└── init.default.toml    # Target output format
```

## Dependencies

No new dependencies required. This is a pure feature implementation using existing string formatting.

## Implementation Phases

### Phase 1: Create Default Template Generator

Add a new function `default_template()` in `crates/cli/src/cli.rs` that returns the full template string matching `init.default.toml`.

```rust
/// Default template for quench init without profiles.
///
/// Matches docs/specs/templates/init.default.toml
pub fn default_template() -> &'static str {
    r#"# Quench configuration
# https://github.com/alfredjeanlab/quench
version = 1

[check.cloc]
check = "error"

[check.escapes]
check = "error"

[check.agents]
check = "error"

[check.docs]
check = "error"

[check.tests]
check = "off"  # stub in quench v0.3.0

[check.license]
check = "off"  # stub in quench v0.3.0

[git.commit]
check = "off"  # stub in quench v0.3.0

# Supported Languages:
# [rust], [golang], [javascript], [shell]
"#
}
```

Key design decisions:

| Decision | Rationale |
|----------|-----------|
| Static string | Template is fixed, no runtime allocation needed |
| Explicit `check = "error"` | Makes enabled checks visible and configurable |
| `check = "off"` with stub comments | Documents unimplemented features, easy to enable later |
| Languages comment at end | Placeholder for language sections added by detection (Phase 1520+) |

### Phase 2: Update run_init to Use Template

Modify `run_init` in `crates/cli/src/main.rs` to:

1. Use `default_template()` as base config when no profiles specified
2. Append profile sections after the template when profiles are specified

Current logic (lines 514-535):
```rust
// Build config based on profiles
let mut config = String::from("version = 1\n");

for profile in &args.with_profiles {
    match profile.as_str() {
        "rust" => { ... }
        ...
    }
}
```

New logic:
```rust
use quench::cli::default_template;

// Build config: start with default template
let mut config = if args.with_profiles.is_empty() {
    default_template().to_string()
} else {
    // With profiles: start with template, append profile sections
    let mut cfg = default_template().to_string();
    for profile in &args.with_profiles {
        match profile.as_str() {
            "rust" => {
                cfg.push('\n');
                cfg.push_str(&rust_profile_defaults());
            }
            // ... other profiles
        }
    }
    cfg
};
```

### Phase 3: Update Behavioral Specs

Enable and verify the Phase 1515 spec in `tests/specs/cli/init.rs`:

1. Remove `#[ignore = "TODO: Phase 1515 - Init Output Template"]` from `init_output_matches_template_format`
2. Verify spec assertions match the actual template output

Current spec (line 343-363):
```rust
#[test]
#[ignore = "TODO: Phase 1515 - Init Output Template"]
fn init_output_matches_template_format() {
    let temp = Project::empty();

    quench_cmd()
        .args(["init"])
        .current_dir(temp.path())
        .assert()
        .success();

    let config = std::fs::read_to_string(temp.path().join("quench.toml")).unwrap();

    // Base template fields
    assert!(config.contains("version = 1"));
    assert!(config.contains("[check.cloc]"));
    assert!(config.contains("[check.escapes]"));
    assert!(config.contains("[check.agents]"));
    assert!(config.contains("[check.docs]"));
    assert!(config.contains("# Supported Languages:"));
}
```

The existing assertions are correct for the template. No changes needed to assertions.

## Key Implementation Details

### Template Format

The template exactly matches `docs/specs/templates/init.default.toml`:

| Section | Check Level | Purpose |
|---------|-------------|---------|
| `[check.cloc]` | `"error"` | File size limits - always enabled |
| `[check.escapes]` | `"error"` | Escape hatch tracking - always enabled |
| `[check.agents]` | `"error"` | Agent file requirements - always enabled |
| `[check.docs]` | `"error"` | Documentation checks - always enabled |
| `[check.tests]` | `"off"` | Test coverage (stub) |
| `[check.license]` | `"off"` | License headers (stub) |
| `[git.commit]` | `"off"` | Commit message format (stub) |

### Stub Comments

Disabled checks include stub comments explaining why they're off:

```toml
check = "off"  # stub in quench v0.3.0
```

This tells users these features exist but aren't implemented yet.

### Languages Comment Block

The template ends with:

```toml
# Supported Languages:
# [rust], [golang], [javascript], [shell]
```

This serves as:
1. Documentation of available language profiles
2. Insertion point for Phase 1520+ language auto-detection output

### Profile Append Behavior

When `--with` is specified, profile sections append after the template. Example with `--with rust`:

```toml
# Quench configuration
# https://github.com/alfredjeanlab/quench
version = 1

[check.cloc]
check = "error"
...
# Supported Languages:
# [rust], [golang], [javascript], [shell]

[rust]
cfg_test_split = true
...
```

## Verification Plan

### 1. Unit Test: Template Function

Add test in `crates/cli/src/cli_tests.rs`:

```rust
#[test]
fn default_template_contains_required_sections() {
    let template = default_template();
    assert!(template.contains("version = 1"));
    assert!(template.contains("[check.cloc]"));
    assert!(template.contains("[check.escapes]"));
    assert!(template.contains("[check.agents]"));
    assert!(template.contains("[check.docs]"));
    assert!(template.contains("[check.tests]"));
    assert!(template.contains("[check.license]"));
    assert!(template.contains("[git.commit]"));
    assert!(template.contains("# Supported Languages:"));
    assert!(template.contains("# [rust], [golang], [javascript], [shell]"));
}

#[test]
fn default_template_has_explicit_check_levels() {
    let template = default_template();
    // Enabled checks
    assert!(template.contains("[check.cloc]\ncheck = \"error\""));
    assert!(template.contains("[check.escapes]\ncheck = \"error\""));
    // Disabled checks with stub comments
    assert!(template.contains("check = \"off\"  # stub in quench v0.3.0"));
}
```

### 2. Behavioral Spec Check

```bash
cargo test --test specs init_output_matches_template_format
```

Expected: Test passes (currently ignored).

### 3. Manual Verification

```bash
cargo build
./target/debug/quench init --force
cat quench.toml
```

Expected output:
```toml
# Quench configuration
# https://github.com/alfredjeanlab/quench
version = 1

[check.cloc]
check = "error"

[check.escapes]
check = "error"

[check.agents]
check = "error"

[check.docs]
check = "error"

[check.tests]
check = "off"  # stub in quench v0.3.0

[check.license]
check = "off"  # stub in quench v0.3.0

[git.commit]
check = "off"  # stub in quench v0.3.0

# Supported Languages:
# [rust], [golang], [javascript], [shell]
```

### 4. Profile Append Verification

```bash
./target/debug/quench init --with rust --force
cat quench.toml | head -30
```

Expected: Template followed by `[rust]` section.

### 5. Full Check

```bash
make check
```

### 6. Spec Coverage

| Spec Requirement | Test Function | Status |
|-----------------|---------------|--------|
| Output matches template format | `init_output_matches_template_format` | ✓ Enabled |
| Contains version = 1 | `init_output_matches_template_format` | ✓ Pass |
| Contains [check.cloc] | `init_output_matches_template_format` | ✓ Pass |
| Contains [check.escapes] | `init_output_matches_template_format` | ✓ Pass |
| Contains [check.agents] | `init_output_matches_template_format` | ✓ Pass |
| Contains [check.docs] | `init_output_matches_template_format` | ✓ Pass |
| Contains # Supported Languages: | `init_output_matches_template_format` | ✓ Pass |
