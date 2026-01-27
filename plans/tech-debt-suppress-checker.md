# Tech Debt: Suppress Checker Consolidation

## Problem

All four language-specific suppress checkers have nearly identical structure (~100 lines each) with only the parser function differing.

## Files Affected

| File | Lines | Pattern |
|------|-------|---------|
| `checks/escapes/go_suppress.rs` | 129 | Identical loop structure |
| `checks/escapes/javascript_suppress.rs` | 142 | Identical loop structure |
| `checks/escapes/shell_suppress.rs` | 115 | Identical loop structure |
| `checks/escapes/ruby_suppress.rs` | 116 | Identical loop structure |

## Duplicated Patterns

### 1. Scope Detection (~10 lines x 4)
```rust
let (scope_config, scope_check) = if is_test_file {
    (&config.test, config.test.check.unwrap_or(SuppressLevel::Allow))
} else {
    (&config.source, config.source.check.unwrap_or(config.check))
};

if scope_check == SuppressLevel::Allow {
    return violations;
}
```

### 2. Violation Loop (~60 lines x 4)
```rust
for suppress in suppresses {
    if *limit_reached { break; }

    let params = SuppressCheckParams { scope_config, scope_check, global_comment };
    let attr_info = SuppressAttrInfo { codes, has_comment, comment_text };

    if let Some(violation_kind) = check_suppress_attr(&params, &attr_info) {
        // ... build pattern and advice (another 20 lines) ...
        if let Some(v) = try_create_violation(...) {
            violations.push(v);
        } else {
            *limit_reached = true;
        }
    }
}
```

### 3. Advice Message Building (~20 lines x 4)
```rust
let (violation_type, advice) = match violation_kind {
    SuppressViolation::MissingComment => { ... }
    SuppressViolation::MissingCode => { ... }
    SuppressViolation::InvalidCode(code) => { ... }
};
```

## Proposed Solution

### Option A: Generic Function (Recommended)

Create `checks/escapes/suppress_common.rs`:

```rust
pub trait SuppressParser {
    fn parse(content: &str, comment_style: Option<&str>) -> Vec<Suppress>;
}

pub fn check_suppress_violations<P: SuppressParser, C: SuppressConfig>(
    ctx: &CheckContext,
    path: &Path,
    content: &str,
    config: &C,
    is_test_file: bool,
    limit_reached: &mut bool,
) -> Vec<Violation> {
    let (scope_config, scope_check) = config.get_scope(is_test_file);
    if scope_check == SuppressLevel::Allow {
        return Vec::new();
    }

    let suppresses = P::parse(content, config.comment());
    check_suppress_list(ctx, path, &suppresses, scope_config, scope_check, limit_reached)
}

fn check_suppress_list(
    ctx: &CheckContext,
    path: &Path,
    suppresses: &[Suppress],
    scope_config: &SuppressScopeConfig,
    scope_check: SuppressLevel,
    limit_reached: &mut bool,
) -> Vec<Violation> {
    // Common loop logic here
}
```

### Option B: Macro

```rust
macro_rules! impl_suppress_checker {
    ($name:ident, $parser:path, $config:ty) => {
        pub fn $name(
            ctx: &CheckContext,
            path: &Path,
            content: &str,
            config: &$config,
            is_test_file: bool,
            limit_reached: &mut bool,
        ) -> Vec<Violation> {
            super::suppress_common::check_suppress_violations::<$parser, _>(
                ctx, path, content, config, is_test_file, limit_reached
            )
        }
    };
}
```

## Implementation Steps

1. Create `checks/escapes/suppress_common.rs` with:
   - `SuppressConfig` trait (get_scope, comment methods)
   - `check_suppress_list()` - the common loop
   - `build_violation_advice()` - message formatting

2. Update each language config to impl `SuppressConfig`:
   - `config/go.rs` - impl for GoSuppressConfig
   - `config/javascript.rs` - impl for JavaScriptSuppressConfig
   - `config/shell.rs` - impl for ShellSuppressConfig
   - `config/ruby.rs` - impl for RubySuppressConfig

3. Update suppress checkers to use common:
   - `go_suppress.rs` - reduce to ~15 lines
   - `javascript_suppress.rs` - reduce to ~15 lines
   - `shell_suppress.rs` - reduce to ~15 lines
   - `ruby_suppress.rs` - reduce to ~15 lines

4. Move tests to common test file where applicable

## Impact

- **Lines removed:** ~400 LOC
- **Files modified:** 9 (4 suppress, 4 config, 1 new common)
- **Risk:** Medium (refactoring logic, needs careful testing)
- **Benefit:** Adding new language suppress checking becomes trivial

## Verification

```bash
cargo test --all -- suppress
cargo test --test specs -- escapes
```

## Priority

**HIGH** - This is the largest single duplication in the codebase.
