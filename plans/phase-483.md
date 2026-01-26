# Phase 483: Ruby Adapter - Detection

Add Ruby language detection and basic adapter infrastructure to quench.

## Overview

Implement the Ruby language adapter with project detection, file classification, and default patterns. This phase establishes the foundation for Ruby support without escape patterns or suppress checks (those come in Phase 485-486).

**Reference**: `plans/.4-roadmap-ruby.md`, `docs/specs/langs/ruby.md`

## Project Structure

```
crates/cli/src/
├── adapter/
│   ├── mod.rs              # Add Ruby to ProjectLanguage enum and detection
│   ├── ruby/
│   │   └── mod.rs          # NEW: Ruby adapter implementation
│   └── ruby_tests.rs       # NEW: Ruby adapter unit tests
├── config/
│   ├── mod.rs              # Add RubyConfig to Config struct
│   └── ruby.rs             # NEW: Ruby configuration structs
tests/fixtures/
└── ruby-gem/               # NEW: Minimal Ruby gem for integration tests
    ├── Gemfile
    ├── example.gemspec
    └── lib/
        └── example.rb
```

## Dependencies

- **External crates**: None new (uses existing `globset`, `regex`)
- **Internal**: Requires Phase 201 (Generic Language Adapter) - already complete

## Implementation Phases

### Phase 1: Ruby Configuration

Add `[ruby]` config section support.

**Files to modify:**
- `crates/cli/src/config/mod.rs` - Add `RubyConfig` import and field
- `crates/cli/src/config/ruby.rs` - NEW: Ruby config structs

**Implementation:**

```rust
// crates/cli/src/config/ruby.rs

use serde::Deserialize;
use super::{CheckLevel, LangClocConfig, LintChangesPolicy, SuppressLevel, SuppressScopeConfig};

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RubyConfig {
    #[serde(default = "RubyConfig::default_source")]
    pub source: Vec<String>,

    #[serde(default = "RubyConfig::default_tests")]
    pub tests: Vec<String>,

    #[serde(default = "RubyConfig::default_ignore")]
    pub ignore: Vec<String>,

    #[serde(default)]
    pub suppress: RubySuppressConfig,

    #[serde(default)]
    pub policy: RubyPolicyConfig,

    #[serde(default)]
    pub cloc: Option<LangClocConfig>,

    #[serde(default)]
    pub cloc_advice: Option<String>,
}

impl RubyConfig {
    pub(crate) fn default_source() -> Vec<String> {
        vec![
            "**/*.rb".to_string(),
            "**/*.rake".to_string(),
            "Rakefile".to_string(),
            "Gemfile".to_string(),
            "*.gemspec".to_string(),
        ]
    }

    pub(crate) fn default_tests() -> Vec<String> {
        vec![
            "spec/**/*_spec.rb".to_string(),
            "test/**/*_test.rb".to_string(),
            "test/**/test_*.rb".to_string(),
            "features/**/*.rb".to_string(),
        ]
    }

    pub(crate) fn default_ignore() -> Vec<String> {
        vec![
            "vendor/**".to_string(),
            "tmp/**".to_string(),
            "log/**".to_string(),
            "coverage/**".to_string(),
        ]
    }

    pub(crate) fn default_cloc_advice() -> &'static str {
        "Can the code be made more concise?\n\n\
         Look for repetitive patterns that could be extracted into helper methods.\n\n\
         If not, split large files into modules or separate concerns into multiple files.\n\n\
         Avoid picking and removing individual lines to satisfy the linter,\n\
         prefer properly refactoring out testable code blocks."
    }
}
```

**Verification:**
- [ ] `cargo build` succeeds with new config module
- [ ] Config parsing accepts `[ruby]` section in `quench.toml`

### Phase 2: Ruby Adapter Core

Implement the Ruby adapter with file classification and patterns.

**Files to create/modify:**
- `crates/cli/src/adapter/ruby/mod.rs` - NEW: Adapter implementation
- `crates/cli/src/adapter/mod.rs` - Register Ruby adapter

**Implementation:**

```rust
// crates/cli/src/adapter/ruby/mod.rs

use std::path::Path;
use globset::GlobSet;

use super::glob::build_glob_set;
use super::{Adapter, FileKind};

pub struct RubyAdapter {
    source_patterns: GlobSet,
    test_patterns: GlobSet,
    ignore_patterns: GlobSet,
}

impl RubyAdapter {
    pub fn new() -> Self {
        Self {
            source_patterns: build_glob_set(&[
                "**/*.rb".to_string(),
                "**/*.rake".to_string(),
                "Rakefile".to_string(),
                "Gemfile".to_string(),
                "*.gemspec".to_string(),
            ]),
            test_patterns: build_glob_set(&[
                "spec/**/*_spec.rb".to_string(),
                "test/**/*_test.rb".to_string(),
                "test/**/test_*.rb".to_string(),
                "features/**/*.rb".to_string(),
            ]),
            ignore_patterns: build_glob_set(&[
                "vendor/**".to_string(),
                "tmp/**".to_string(),
                "log/**".to_string(),
                "coverage/**".to_string(),
            ]),
        }
    }

    pub fn with_patterns(patterns: super::ResolvedPatterns) -> Self {
        Self {
            source_patterns: build_glob_set(&patterns.source),
            test_patterns: build_glob_set(&patterns.test),
            ignore_patterns: build_glob_set(&patterns.ignore),
        }
    }

    pub fn should_ignore(&self, path: &Path) -> bool {
        self.ignore_patterns.is_match(path)
    }
}

impl Adapter for RubyAdapter {
    fn name(&self) -> &'static str {
        "ruby"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["rb", "rake"]
    }

    fn classify(&self, path: &Path) -> FileKind {
        if self.should_ignore(path) {
            return FileKind::Other;
        }
        if self.test_patterns.is_match(path) {
            return FileKind::Test;
        }
        if self.source_patterns.is_match(path) {
            return FileKind::Source;
        }
        FileKind::Other
    }
}
```

**Verification:**
- [ ] Adapter classifies `lib/example.rb` as Source
- [ ] Adapter classifies `spec/example_spec.rb` as Test
- [ ] Adapter classifies `vendor/bundle/gems/foo.rb` as Other

### Phase 3: Project Detection

Add Ruby to the language detection logic.

**Files to modify:**
- `crates/cli/src/adapter/mod.rs` - Add `Ruby` variant and detection

**Detection priority** (add after JavaScript, before Shell):

| File | Detection |
|------|-----------|
| `Gemfile` | Standard gem/bundler project |
| `*.gemspec` | Ruby gem library |
| `config.ru` | Rack application |
| `config/application.rb` | Rails application |

**Implementation changes to `detect_language()`:**

```rust
pub enum ProjectLanguage {
    Rust,
    Go,
    JavaScript,
    Ruby,      // NEW
    Shell,
    Generic,
}

pub fn detect_language(root: &Path) -> ProjectLanguage {
    // ... existing Rust, Go, JavaScript checks ...

    // Ruby detection (before Shell)
    if root.join("Gemfile").exists()
        || has_gemspec(root)
        || root.join("config.ru").exists()
        || root.join("config/application.rb").exists()
    {
        return ProjectLanguage::Ruby;
    }

    // Shell detection...
}

fn has_gemspec(root: &Path) -> bool {
    root.read_dir()
        .ok()
        .map(|entries| {
            entries.filter_map(|e| e.ok()).any(|entry| {
                entry.path().extension().and_then(|e| e.to_str()) == Some("gemspec")
            })
        })
        .unwrap_or(false)
}
```

**Verification:**
- [ ] Project with `Gemfile` detected as Ruby
- [ ] Project with `example.gemspec` detected as Ruby
- [ ] Project with `config.ru` detected as Ruby
- [ ] Project with `config/application.rb` detected as Ruby

### Phase 4: Gemspec Parsing

Extract gem name from `*.gemspec` files.

**Files to modify:**
- `crates/cli/src/adapter/ruby/mod.rs` - Add `parse_gemspec()` function

**Implementation:**

```rust
/// Parse *.gemspec to extract gem name.
///
/// Supports common patterns:
/// - `Gem::Specification.new do |s| s.name = "example"`
/// - `spec.name = "example"`
/// - `s.name = 'example'`
pub fn parse_gemspec(content: &str) -> Option<String> {
    // Pattern: s.name = "gem_name" or s.name = 'gem_name'
    // Also: spec.name = "gem_name"
    let re = regex::Regex::new(r#"(?:s|spec)\.name\s*=\s*['"]([^'"]+)['"]"#).ok()?;
    re.captures(content)
        .and_then(|cap| cap.get(1))
        .map(|m| m.as_str().to_string())
}

/// Find and parse the gem name from a gemspec in the given directory.
pub fn find_gem_name(root: &Path) -> Option<String> {
    let entries = root.read_dir().ok()?;
    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("gemspec") {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Some(name) = parse_gemspec(&content) {
                    return Some(name);
                }
            }
        }
    }
    None
}
```

**Verification:**
- [ ] Parses `Gem::Specification.new { |s| s.name = "example" }`
- [ ] Parses `spec.name = 'my-gem'`
- [ ] Returns `None` for invalid gemspec

### Phase 5: Integration with Config Resolution

Wire up Ruby config to pattern resolution system.

**Files to modify:**
- `crates/cli/src/adapter/mod.rs` - Add `resolve_ruby_patterns()`
- `crates/cli/src/adapter/patterns.rs` - Add `LanguageDefaults` impl for `RubyConfig`
- `crates/cli/src/config/mod.rs` - Add Ruby to `cloc_check_level_for_language()` etc.

**Implementation in patterns.rs:**

```rust
impl LanguageDefaults for crate::config::RubyConfig {
    fn default_source() -> Vec<String> {
        crate::config::RubyConfig::default_source()
    }

    fn default_tests() -> Vec<String> {
        crate::config::RubyConfig::default_tests()
    }

    fn default_ignore() -> Vec<String> {
        crate::config::RubyConfig::default_ignore()
    }
}
```

**Verification:**
- [ ] `AdapterRegistry::for_project_with_config()` creates Ruby adapter with resolved patterns
- [ ] Config overrides apply correctly

### Phase 6: Test Fixture and Unit Tests

Create test fixture and comprehensive unit tests.

**Files to create:**
- `tests/fixtures/ruby-gem/Gemfile`
- `tests/fixtures/ruby-gem/example.gemspec`
- `tests/fixtures/ruby-gem/lib/example.rb`
- `tests/fixtures/ruby-gem/spec/example_spec.rb`
- `crates/cli/src/adapter/ruby_tests.rs`

**Fixture structure:**

```ruby
# tests/fixtures/ruby-gem/Gemfile
source 'https://rubygems.org'
gemspec

# tests/fixtures/ruby-gem/example.gemspec
Gem::Specification.new do |s|
  s.name = 'example'
  s.version = '0.1.0'
  s.summary = 'Example gem for quench testing'
  s.authors = ['Test']
end

# tests/fixtures/ruby-gem/lib/example.rb
module Example
  def self.hello
    'Hello, Ruby!'
  end
end

# tests/fixtures/ruby-gem/spec/example_spec.rb
require 'example'

RSpec.describe Example do
  it 'says hello' do
    expect(Example.hello).to eq('Hello, Ruby!')
  end
end
```

**Unit tests to write:**

```rust
// crates/cli/src/adapter/ruby_tests.rs

#[parameterized(
    lib_file = { "lib/example.rb", FileKind::Source },
    app_file = { "app/models/user.rb", FileKind::Source },
    rake_file = { "lib/tasks/deploy.rake", FileKind::Source },
    gemfile = { "Gemfile", FileKind::Source },
    gemspec = { "example.gemspec", FileKind::Source },
    rakefile = { "Rakefile", FileKind::Source },
    spec_file = { "spec/example_spec.rb", FileKind::Test },
    test_file = { "test/example_test.rb", FileKind::Test },
    test_prefix = { "test/test_example.rb", FileKind::Test },
    features = { "features/step_definitions/example.rb", FileKind::Test },
    vendor = { "vendor/bundle/gems/foo/lib/foo.rb", FileKind::Other },
    tmp = { "tmp/cache/foo.rb", FileKind::Other },
    log = { "log/development.log", FileKind::Other },
    coverage = { "coverage/index.html", FileKind::Other },
)]
fn classify_path(path: &str, expected: FileKind) { ... }

#[test]
fn parses_gem_name_from_gemspec() {
    let content = r#"
Gem::Specification.new do |s|
  s.name = "my_gem"
  s.version = "1.0.0"
end
"#;
    assert_eq!(parse_gemspec(content), Some("my_gem".to_string()));
}
```

**Verification:**
- [ ] All unit tests pass
- [ ] `detect_language()` returns `Ruby` for `tests/fixtures/ruby-gem/`
- [ ] `find_gem_name()` returns `"example"` for fixture

## Key Implementation Details

### Detection Order

Ruby detection is placed after JavaScript (to avoid misdetecting JS projects with some Ruby tooling) but before Shell:

```
Cargo.toml → Rust
go.mod → Go
package.json/tsconfig.json → JavaScript
Gemfile/*.gemspec/config.ru/config/application.rb → Ruby
*.sh in root/bin/scripts → Shell
(fallback) → Generic
```

### Gemspec Parsing Strategy

Use a simple regex that handles the common patterns:
- `s.name = "name"` or `s.name = 'name'`
- `spec.name = "name"` or `spec.name = 'name'`

This covers 99% of real-world gemspecs without needing a full Ruby parser.

### Ignore Pattern Behavior

Ruby's ignore patterns follow the same semantics as Go's `vendor/`:
- Files in ignored directories are classified as `FileKind::Other`
- They are excluded from source/test LOC counting
- They are excluded from escape pattern checking

## Verification Plan

### Unit Tests

```bash
cargo test adapter::ruby
```

- [ ] `classify_path` tests for all file type combinations
- [ ] `should_ignore` tests for vendor/tmp/log/coverage
- [ ] `parse_gemspec` tests for various formats
- [ ] `find_gem_name` test with fixture

### Integration Tests

```bash
cargo test --test specs
```

- [ ] Ruby project detection (`detect_language()`)
- [ ] Adapter registration in `AdapterRegistry::for_project()`
- [ ] Pattern resolution with config overrides

### Manual Verification

```bash
# Verify detection
cd tests/fixtures/ruby-gem
../../target/debug/quench check --cloc

# Should show Ruby adapter being used and classify files correctly
```

### Checklist Before Commit

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --all-targets --all-features -- -D warnings`
- [ ] `cargo test --all`
- [ ] `make check` passes
- [ ] Gem name extraction works on fixture
