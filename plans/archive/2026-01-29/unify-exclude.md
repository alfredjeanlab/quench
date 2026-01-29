# Plan: Consolidate Ignore/Exclude into Unified Exclude System

## Overview

**Goal**: Rename "ignore" to "exclude" throughout the codebase while maintaining walker-level performance optimization and backward compatibility.

**Architecture Decision**: Keep two-level exclude system:
- **Walker-level**: Project/language excludes (prevents I/O on subtrees)
- **Check-level**: Check-specific excludes (post-walker filtering)

**Performance Target**: No regression in walker benchmarks (<2% acceptable overhead)

---

## Configuration Changes

### Before
```toml
[project]
ignore = ["tmp/**"]

[rust]
ignore = ["target/**"]

[check.cloc]
exclude = ["benches/**"]
```

### After
```toml
[project]
exclude = ["tmp/**"]        # Walker-level

[rust]
exclude = ["target/**"]     # Walker-level

[check.cloc]
exclude = ["benches/**"]    # Check-level (unchanged)
```

**Backward Compatibility**: Support legacy `ignore` field via serde `#[serde(alias = "ignore")]`

---

## Implementation Steps

### Step 1: Core Configuration Refactoring

**File**: `crates/cli/src/config/mod.rs`

1. Rename `IgnoreConfig` → `ExcludeConfig` (line 18, 593-615)
2. Add backward compatibility alias:
   ```rust
   pub struct RustConfig {
       #[serde(default, alias = "ignore")]
       pub exclude: Vec<String>,
       // ...
   }
   ```
3. Update all language configs:
   - `config/rust.rs`
   - `config/python.rs`
   - `config/ruby.rs`
   - `config/go.rs`
   - `config/shell.rs`

**Testing**: Verify both `ignore` and `exclude` parse correctly

---

### Step 2: Walker Module Updates

**File**: `crates/cli/src/walker.rs`

1. Rename field in `WalkerConfig` struct (line 72-73):
   ```rust
   pub struct WalkerConfig {
       pub exclude_patterns: Vec<String>,  // Was: ignore_patterns
       // ...
   }
   ```

2. Update pattern application in `FileWalker::walk()` (lines 248-256):
   - Keep OverrideBuilder logic unchanged
   - Update variable names: `ignore_patterns` → `exclude_patterns`

3. Update comments to use "exclude" terminology

**Testing**: Verify walker still prevents I/O on excluded subtrees

---

### Step 3: Adapter Pattern Resolution

**File**: `crates/cli/src/adapter/patterns.rs`

1. Update `ResolvedPatterns` struct (lines 25-31):
   ```rust
   pub struct ResolvedPatterns {
       pub source: Vec<String>,
       pub test: Vec<String>,
       pub exclude: Vec<String>,  // Was: ignore
   }
   ```

2. Rename trait method in `LanguageDefaults`:
   ```rust
   fn default_exclude() -> Vec<String> {  // Was: default_ignore
       vec![]
   }
   ```

3. Update `resolve_patterns()` function signature and implementation

**File**: `crates/cli/src/adapter/mod.rs`

4. Update `define_resolve_patterns!` macro (lines 389-434):
   ```rust
   patterns::resolve_patterns::<$config_type>(
       &config.$config_field.source,
       &config.$config_field.tests,
       &config.$config_field.exclude,  // Changed from .ignore
       fallback_test,
   )
   ```

**Testing**: Verify pattern resolution still works for all languages

---

### Step 4: Language Adapter Updates

**Files**: All adapters in `crates/cli/src/adapter/{rust,python,ruby,go,shell}/mod.rs`

1. Rename field: `ignore_patterns` → `exclude_patterns`
2. Rename method: `should_ignore()` → `should_exclude()` (or keep name for internal consistency)
3. Update `LanguageDefaults` impl:
   ```rust
   impl LanguageDefaults for RustDefaults {
       fn default_exclude() -> Vec<String> {  // Was: default_ignore
           vec!["target/**".to_string()]
       }
   }
   ```

**Testing**: Verify adapters correctly classify excluded files

---

### Step 5: Command Integration

**File**: `crates/cli/src/cmd_check.rs`

1. Update walker initialization (lines 91-238):
   ```rust
   let mut exclude_patterns = config.project.exclude.patterns.clone();

   // Merge language-specific excludes
   match detect_language(&root) {
       ProjectLanguage::Rust => {
           if !exclude_patterns.iter().any(|p| p.contains("target")) {
               exclude_patterns.push("target/**".to_string());
           }
       }
       // ... other languages
   }

   let walker_config = WalkerConfig {
       max_depth: Some(args.max_depth),
       exclude_patterns,
       ..Default::default()
   };
   ```

**Testing**: Verify walker correctly merges project + language excludes

---

### Step 6: Cache Version Bump

**File**: `crates/cli/src/cache.rs`

Bump `CACHE_VERSION` constant (pattern resolution logic changed):
```rust
pub const CACHE_VERSION: u32 = 29;  // Increment from 28
```

---

### Step 7: Documentation Updates

**Files to update**:

1. **`docs/specs/02-config.md`**:
   - Replace all `ignore` → `exclude` references
   - Add migration guide section
   - Document scope distinction (walker-level vs check-level)

2. **Init templates** (all `docs/specs/templates/init.*.toml`):
   ```toml
   # Old
   # [project]
   # ignore = ["tmp/**"]

   # New
   # [project]
   # exclude = ["tmp/**"]  # Walker-level: prevents I/O
   ```

3. **Inline comments**: Update terminology in all source files

---

## Testing Strategy

### Unit Tests (sibling `_tests.rs` files)

**New tests**:

1. `config/mod_tests.rs`:
   ```rust
   #[test]
   fn parse_config_with_legacy_ignore_field() {
       let toml = r#"
           version = 1
           [project]
           ignore = ["target/**"]
       "#;
       let config = parse(toml, Path::new("test.toml")).unwrap();
       assert_eq!(config.project.exclude.patterns, vec!["target/**"]);
   }

   #[test]
   fn parse_config_with_new_exclude_field() {
       let toml = r#"
           version = 1
           [project]
           exclude = ["target/**"]
       "#;
       let config = parse(toml, Path::new("test.toml")).unwrap();
       assert_eq!(config.project.exclude.patterns, vec!["target/**"]);
   }
   ```

2. `walker_tests.rs` (update existing):
   - Test walker with exclude_patterns
   - Verify I/O prevention on excluded subtrees

3. `adapter/patterns_tests.rs` (update existing):
   - Test pattern resolution with exclude field
   - Test language defaults

### Behavioral Specs (tests/specs/)

**New spec files**:

1. `tests/specs/config/exclude.rs`:
   - Test project-level exclude
   - Test language-level exclude
   - Test backward compatibility with `ignore`

2. `tests/specs/walker/exclude.rs`:
   ```rust
   #[test]
   fn walker_respects_project_exclude() {
       let fixture = fixture("walker-exclude");

       cli()
           .on(fixture)
           .with_config(r#"
               version = 1
               [project]
               exclude = ["excluded/**"]
           "#)
           .env("QUENCH_DEBUG_FILES", "1")
           .stdout_not_has("excluded/file.rs");
   }

   #[test]
   fn legacy_ignore_field_still_works() {
       let fixture = fixture("walker-exclude");

       cli()
           .on(fixture)
           .with_config(r#"
               version = 1
               [project]
               ignore = ["excluded/**"]  # Legacy syntax
           "#)
           .env("QUENCH_DEBUG_FILES", "1")
           .stdout_not_has("excluded/file.rs");
   }
   ```

### Performance Benchmarks

**Files to update/create**:

1. `crates/cli/benches/file_walking.rs`:
   ```rust
   fn bench_walker_with_excludes(c: &mut Criterion) {
       let mut group = c.benchmark_group("walker/excludes");
       let fixture = fixture_path("bench-large");

       let exclude_patterns = vec![
           "target/**".to_string(),
           "node_modules/**".to_string(),
           ".git/**".to_string(),
       ];

       group.bench_function("baseline_no_excludes", |b| {
           let walker = FileWalker::new(WalkerConfig::default());
           b.iter(|| {
               let (rx, handle) = walker.walk(&fixture);
               rx.iter().count();
               handle.join();
           });
       });

       group.bench_function("with_excludes", |b| {
           let walker = FileWalker::new(WalkerConfig {
               exclude_patterns: exclude_patterns.clone(),
               ..Default::default()
           });
           b.iter(|| {
               let (rx, handle) = walker.walk(&fixture);
               rx.iter().count();
               handle.join();
           });
       });
   }
   ```

**Regression targets**:
- Walker overhead with excludes: <50ms additional
- Cold run: <2s (unchanged)
- Warm run: <500ms (unchanged)

---

## Verification Steps

### Before Changes (Baseline)
```bash
# 1. Run benchmarks and save baseline
./scripts/benchmark
cp reports/benchmark-baseline.json /tmp/before-exclude-refactor.json

# 2. Profile current file walking
./scripts/profile-repo . > /tmp/before-profile.txt

# 3. Run existing tests
make check
```

### After Changes (Validation)
```bash
# 1. Run all tests
make check

# 2. Run benchmarks and compare
./scripts/benchmark
diff /tmp/before-exclude-refactor.json reports/benchmark-baseline.json

# 3. Profile new file walking
./scripts/profile-repo . > /tmp/after-profile.txt
diff /tmp/before-profile.txt /tmp/after-profile.txt

# 4. Verify backward compatibility
echo '[project]\nignore = ["tmp/**"]' > test.toml
cargo run -- check  # Should work without error

# 5. Verify new syntax
echo '[project]\nexclude = ["tmp/**"]' > test.toml
cargo run -- check  # Should work without error

# 6. Run regression benchmarks
cargo bench --bench regression
```

**Success Criteria**:
- ✅ All unit tests pass
- ✅ All behavioral specs pass
- ✅ Benchmarks show <2% regression (acceptable)
- ✅ Both `ignore` and `exclude` configs work
- ✅ Cache invalidation works correctly

---

## Critical Files to Modify

### Configuration (5 files)
- `crates/cli/src/config/mod.rs` - Core config structs, add ExcludeConfig
- `crates/cli/src/config/rust.rs` - Rust language config
- `crates/cli/src/config/python.rs` - Python language config
- `crates/cli/src/config/ruby.rs` - Ruby language config
- `crates/cli/src/config/go.rs` - Go language config

### Walker (1 file)
- `crates/cli/src/walker.rs` - Rename ignore_patterns → exclude_patterns

### Adapter (7 files)
- `crates/cli/src/adapter/patterns.rs` - Pattern resolution logic
- `crates/cli/src/adapter/mod.rs` - Resolve patterns macro
- `crates/cli/src/adapter/rust/mod.rs` - Rust adapter
- `crates/cli/src/adapter/python/mod.rs` - Python adapter
- `crates/cli/src/adapter/ruby/mod.rs` - Ruby adapter
- `crates/cli/src/adapter/go/mod.rs` - Go adapter
- `crates/cli/src/adapter/shell/mod.rs` - Shell adapter

### Command (1 file)
- `crates/cli/src/cmd_check.rs` - Walker initialization

### Cache (1 file)
- `crates/cli/src/cache.rs` - Bump version to 29

### Documentation (8+ files)
- `docs/specs/02-config.md` - Config specification
- `docs/specs/templates/init.*.toml` - All init templates (7 files)

---

## Migration Notes

**Backward Compatibility**: The `ignore` field will continue to work via serde aliases. No breaking changes for existing users.

**Future Deprecation**: In a future major version, the `ignore` alias can be removed with appropriate deprecation warnings.

**Config Migration**: Users can migrate at their leisure:
```bash
# Automated migration (if desired)
sed -i '' 's/^ignore = /exclude = /g' quench.toml
sed -i '' 's/^\[.*\]\.ignore/\[.*\].exclude/g' quench.toml
```

---

## Rollout Checklist

- [ ] Step 1: Core configuration refactoring
- [ ] Step 2: Walker module updates
- [ ] Step 3: Adapter pattern resolution
- [ ] Step 4: Language adapter updates
- [ ] Step 5: Command integration
- [ ] Step 6: Cache version bump
- [ ] Step 7: Documentation updates
- [ ] Run unit tests
- [ ] Add behavioral specs
- [ ] Run performance benchmarks
- [ ] Verify backward compatibility
- [ ] Update CACHE_VERSION
- [ ] Run `make check`
