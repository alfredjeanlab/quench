# Tech Debt: Language Config Consolidation

## Problem

All five language config files follow identical patterns with only default values differing. Each reimplements ~150-180 lines of boilerplate.

## Files Affected

| File | Lines | Unique Content |
|------|-------|----------------|
| `config/rust.rs` | 175 | Default patterns only |
| `config/go.rs` | 175 | Default patterns only |
| `config/shell.rs` | 172 | Default patterns only |
| `config/javascript.rs` | 142 | Default patterns only |
| `config/ruby.rs` | 184 | Default patterns only |

## Duplicated Patterns

### 1. Config Struct Shape (~30 lines x 5)
```rust
#[derive(Debug, Clone, Default, Deserialize)]
pub struct LangConfig {
    #[serde(default = "LangConfig::default_source")]
    pub source: Vec<String>,
    #[serde(default = "LangConfig::default_tests")]
    pub tests: Vec<String>,
    #[serde(default)]
    pub suppress: LangSuppressConfig,
    #[serde(default)]
    pub policy: LangPolicyConfig,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cloc: Option<PathBuf>,
    pub cloc_advice: Option<String>,
}
```

### 2. Default Impl (~15 lines x 5)
```rust
impl Default for LangConfig {
    fn default() -> Self {
        Self {
            source: Self::default_source(),
            tests: Self::default_tests(),
            suppress: Default::default(),
            policy: Default::default(),
            cloc: None,
            cloc_advice: None,
        }
    }
}
```

### 3. Default Methods (~30 lines x 5)
```rust
impl LangConfig {
    pub(crate) fn default_source() -> Vec<String> { vec![...] }
    pub(crate) fn default_tests() -> Vec<String> { vec![...] }
    pub(crate) fn default_ignore() -> Vec<String> { vec![...] }
    pub(crate) fn default_cloc_advice() -> &'static str { "..." }
}
```

### 4. Suppress Config (~25 lines x 5)
```rust
#[derive(Debug, Clone, Default, Deserialize)]
pub struct LangSuppressConfig {
    #[serde(default)]
    pub check: SuppressLevel,
    pub comment: Option<String>,
    #[serde(default)]
    pub source: SuppressScopeConfig,
    #[serde(default)]
    pub test: SuppressScopeConfig,
}
```

### 5. Policy Config (~25 lines x 5)
```rust
#[derive(Debug, Clone, Deserialize)]
pub struct LangPolicyConfig {
    #[serde(default)]
    pub lint_changes: LintChangesPolicy,
    #[serde(default)]
    pub lint_config: Vec<String>,
}

impl Default for LangPolicyConfig { ... }

impl PolicyConfig for LangPolicyConfig {
    fn lint_changes(&self) -> LintChangesPolicy { self.lint_changes }
    fn lint_config(&self) -> &[String] { &self.lint_config }
}
```

## Proposed Solution

### Create `config/lang_common.rs`:

```rust
/// Trait for language-specific defaults
pub trait LanguageDefaults {
    fn default_source() -> Vec<String>;
    fn default_tests() -> Vec<String>;
    fn default_ignore() -> Vec<String>;
    fn default_cloc_advice() -> &'static str;
    fn default_lint_config() -> Vec<String> { vec![] }
}

/// Generic language config struct
#[derive(Debug, Clone, Deserialize)]
pub struct LangConfig<D: LanguageDefaults> {
    #[serde(default = "D::default_source")]
    pub source: Vec<String>,
    #[serde(default = "D::default_tests")]
    pub tests: Vec<String>,
    #[serde(default)]
    pub suppress: SuppressConfig,
    #[serde(default)]
    pub policy: PolicyConfig,
    pub cloc: Option<PathBuf>,
    pub cloc_advice: Option<String>,
    #[serde(skip)]
    _marker: std::marker::PhantomData<D>,
}

// Single SuppressConfig used by all languages
#[derive(Debug, Clone, Default, Deserialize)]
pub struct SuppressConfig {
    #[serde(default)]
    pub check: SuppressLevel,
    pub comment: Option<String>,
    #[serde(default)]
    pub source: SuppressScopeConfig,
    #[serde(default)]
    pub test: SuppressScopeConfig,
}

// Single PolicyConfig used by all languages
#[derive(Debug, Clone, Default, Deserialize)]
pub struct PolicyConfigData {
    #[serde(default)]
    pub lint_changes: LintChangesPolicy,
    #[serde(default)]
    pub lint_config: Vec<String>,
}
```

### Language-Specific Files Become Minimal:

```rust
// config/rust.rs - Now ~30 lines instead of 175

pub struct RustDefaults;

impl LanguageDefaults for RustDefaults {
    fn default_source() -> Vec<String> {
        vec!["src/**/*.rs".into(), "lib/**/*.rs".into()]
    }
    fn default_tests() -> Vec<String> {
        vec!["tests/**/*.rs".into(), "src/**/*_test.rs".into()]
    }
    fn default_ignore() -> Vec<String> {
        vec!["target/**".into()]
    }
    fn default_cloc_advice() -> &'static str {
        "Add `cloc = \"target/debug/crate_name\"` to [rust] section"
    }
}

pub type RustConfig = LangConfig<RustDefaults>;
pub type RustSuppressConfig = SuppressConfig;
pub type RustPolicyConfig = PolicyConfigData;
```

## Alternative: Macro Approach

```rust
macro_rules! define_lang_config {
    ($lang:ident, $source:expr, $tests:expr, $ignore:expr, $cloc_advice:expr) => {
        paste::paste! {
            #[derive(Debug, Clone, Default, Deserialize)]
            pub struct [<$lang Config>] {
                #[serde(default = "[<$lang Config>]::default_source")]
                pub source: Vec<String>,
                // ... rest of fields
            }

            impl [<$lang Config>] {
                pub(crate) fn default_source() -> Vec<String> { $source }
                pub(crate) fn default_tests() -> Vec<String> { $tests }
                pub(crate) fn default_ignore() -> Vec<String> { $ignore }
                pub(crate) fn default_cloc_advice() -> &'static str { $cloc_advice }
            }

            // Generate suppress and policy configs too...
        }
    };
}

// Usage:
define_lang_config!(Rust,
    vec!["src/**/*.rs".into()],
    vec!["tests/**/*.rs".into()],
    vec!["target/**".into()],
    "Add cloc = ..."
);
```

## Implementation Steps

1. Create `config/lang_common.rs` with:
   - `LanguageDefaults` trait
   - `LangConfig<D>` generic struct
   - Shared `SuppressConfig` and `PolicyConfigData`

2. Update each language config (can be done incrementally):
   - `config/rust.rs` → type alias + defaults impl
   - `config/go.rs` → type alias + defaults impl
   - `config/shell.rs` → type alias + defaults impl
   - `config/javascript.rs` → type alias + defaults impl
   - `config/ruby.rs` → type alias + defaults impl

3. Update imports in dependent files

4. Update tests

## Impact

- **Lines removed:** ~600 LOC
- **Files modified:** 6 (5 lang configs + 1 new common)
- **Risk:** Medium (public API types, needs careful migration)
- **Benefit:** Adding new language support becomes ~30 lines

## Verification

```bash
cargo test --all -- config
cargo check --all
```

## Priority

**MEDIUM** - Large reduction but low bug risk (mostly type aliases).
