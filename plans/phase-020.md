# Phase 020: File Walking - Implementation

**Root Feature:** `quench-6920`

## Overview

Implement parallel, gitignore-aware file walking using the `ignore` crate. This phase builds the core file discovery infrastructure that respects `.gitignore`, custom ignore patterns, handles symlink loops, and limits traversal depth. Additionally, implement size-gated file reading with direct I/O for small files and memory-mapped I/O for larger files.

**Current State**: Behavioral specs exist from Phase 015 (all marked `#[ignore]`). The `ignore` crate is available as a dev dependency for benchmarks. Config parsing exists but lacks ignore pattern support.

**End State**: Full file walking implementation with all Phase 015 specs passing. Files discovered efficiently with proper filtering, size-gated reading, and timeout protection.

## Project Structure

```
crates/cli/src/
├── lib.rs                    # Updated exports
├── config.rs                 # Extended with ignore patterns
├── walker.rs                 # NEW: File walking module
├── walker_tests.rs           # NEW: Walker unit tests
├── reader.rs                 # NEW: Size-gated file reading
└── reader_tests.rs           # NEW: Reader unit tests
```

## Dependencies

Add to `crates/cli/Cargo.toml`:

```toml
[dependencies]
ignore = "0.4"                # Parallel gitignore-aware walking
memmap2 = "0.9"               # Memory-mapped file I/O
crossbeam-channel = "0.5"     # Bounded channels for streaming pipeline
```

## Implementation Phases

### Phase 20.1: Config Extensions for Ignore Patterns

**Goal**: Extend config parsing to support custom ignore patterns.

**Tasks**:
1. Add `[project.ignore]` section to config schema
2. Parse glob patterns from config
3. Validate patterns at load time
4. Add tests for ignore pattern config

**Files**:

```rust
// crates/cli/src/config.rs - additions

/// Project-level configuration.
#[derive(Debug, Default, Deserialize)]
pub struct ProjectConfig {
    /// Project name.
    pub name: Option<String>,

    /// Custom ignore patterns.
    #[serde(default)]
    pub ignore: IgnoreConfig,
}

/// Ignore pattern configuration.
#[derive(Debug, Default, Deserialize)]
pub struct IgnoreConfig {
    /// Glob patterns to ignore (e.g., "*.snapshot", "testdata/", "**/fixtures/**").
    #[serde(default)]
    pub patterns: Vec<String>,
}
```

Update `KNOWN_KEYS` to include `project.ignore`.

**Verification**:
```bash
cargo test --lib config
```

### Phase 20.2: Core Walker Module

**Goal**: Create the file walking module using the `ignore` crate.

**Tasks**:
1. Create `walker.rs` with `FileWalker` struct
2. Configure `WalkBuilder` with gitignore, depth limit, parallel walking
3. Add custom ignore pattern support via `add_custom_ignore_file` or overrides
4. Implement streaming file discovery via channels
5. Handle walker errors gracefully

**Files**:

```rust
// crates/cli/src/walker.rs

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use crossbeam_channel::{Receiver, Sender, bounded};
use ignore::{WalkBuilder, WalkState};

use crate::config::IgnoreConfig;

/// Default maximum directory depth.
pub const DEFAULT_MAX_DEPTH: usize = 100;

/// Walker configuration.
#[derive(Debug, Clone)]
pub struct WalkerConfig {
    /// Maximum directory depth (default: 100).
    pub max_depth: Option<usize>,

    /// Custom ignore patterns from config.
    pub ignore_patterns: Vec<String>,

    /// Whether to respect gitignore files.
    pub git_ignore: bool,

    /// Whether to include hidden files.
    pub hidden: bool,

    /// Number of threads (0 = auto).
    pub threads: usize,
}

impl Default for WalkerConfig {
    fn default() -> Self {
        Self {
            max_depth: Some(DEFAULT_MAX_DEPTH),
            ignore_patterns: Vec::new(),
            git_ignore: true,
            hidden: true,  // Skip hidden files by default
            threads: 0,    // Auto-detect
        }
    }
}

/// File discovered by the walker.
#[derive(Debug)]
pub struct WalkedFile {
    /// Path to the file.
    pub path: PathBuf,

    /// File size in bytes.
    pub size: u64,

    /// Directory depth from root.
    pub depth: usize,
}

/// Statistics from a walk operation.
#[derive(Debug, Default)]
pub struct WalkStats {
    /// Total files discovered.
    pub files_found: usize,

    /// Files skipped due to ignore patterns.
    pub files_ignored: usize,

    /// Directories skipped due to depth limit.
    pub depth_limited: usize,

    /// Symlink loops detected.
    pub symlink_loops: usize,

    /// Errors encountered.
    pub errors: usize,
}

/// Parallel file walker with gitignore support.
pub struct FileWalker {
    config: WalkerConfig,
}

impl FileWalker {
    /// Create a new walker with the given configuration.
    pub fn new(config: WalkerConfig) -> Self {
        Self { config }
    }

    /// Create a walker from project ignore config.
    pub fn from_ignore_config(ignore: &IgnoreConfig) -> Self {
        Self::new(WalkerConfig {
            ignore_patterns: ignore.patterns.clone(),
            ..Default::default()
        })
    }

    /// Walk the given root directory, returning a receiver of discovered files.
    ///
    /// Files are streamed through the channel as they're discovered.
    /// Returns (receiver, stats) where stats is populated after walking completes.
    pub fn walk(&self, root: &Path) -> (Receiver<WalkedFile>, WalkStats) {
        let (tx, rx) = bounded(1000);
        let stats = WalkStats::default();

        let mut builder = WalkBuilder::new(root);
        builder
            .hidden(self.config.hidden)
            .git_ignore(self.config.git_ignore)
            .git_exclude(true)
            .git_global(true);

        if let Some(depth) = self.config.max_depth {
            builder.max_depth(Some(depth));
        }

        if self.config.threads > 0 {
            builder.threads(self.config.threads);
        }

        // Add custom ignore patterns
        for pattern in &self.config.ignore_patterns {
            let mut override_builder = ignore::overrides::OverrideBuilder::new(root);
            // Negate to ignore: !pattern means "do not ignore", pattern means "ignore"
            if let Ok(()) = override_builder.add(&format!("!{}", pattern)) {
                if let Ok(overrides) = override_builder.build() {
                    builder.overrides(overrides);
                }
            }
        }

        let walker = builder.build_parallel();

        // Track stats atomically
        let files_found = AtomicUsize::new(0);
        let errors = AtomicUsize::new(0);

        walker.run(|| {
            let tx = tx.clone();
            let files_found = &files_found;
            let errors = &errors;

            Box::new(move |entry| {
                match entry {
                    Ok(entry) => {
                        // Skip directories
                        if !entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
                            return WalkState::Continue;
                        }

                        // Get metadata for size
                        let size = entry.metadata()
                            .map(|m| m.len())
                            .unwrap_or(0);

                        let walked = WalkedFile {
                            path: entry.into_path(),
                            size,
                            depth: entry.depth(),
                        };

                        files_found.fetch_add(1, Ordering::Relaxed);

                        // Non-blocking send, drop if full (bounded backpressure)
                        let _ = tx.try_send(walked);

                        WalkState::Continue
                    }
                    Err(err) => {
                        // Log error but continue walking
                        tracing::warn!("Walk error: {}", err);
                        errors.fetch_add(1, Ordering::Relaxed);
                        WalkState::Continue
                    }
                }
            })
        });

        drop(tx); // Close sender to signal completion

        let final_stats = WalkStats {
            files_found: files_found.load(Ordering::Relaxed),
            errors: errors.load(Ordering::Relaxed),
            ..Default::default()
        };

        (rx, final_stats)
    }

    /// Walk and collect all files (convenience method for small directories).
    pub fn walk_collect(&self, root: &Path) -> (Vec<WalkedFile>, WalkStats) {
        let (rx, stats) = self.walk(root);
        let files: Vec<_> = rx.iter().collect();
        (files, stats)
    }
}

#[cfg(test)]
#[path = "walker_tests.rs"]
mod tests;
```

**Verification**:
```bash
cargo test --lib walker
cargo test --test specs file_walking_respects_gitignore -- --ignored
```

### Phase 20.3: Size-Gated File Reading

**Goal**: Implement file reading with size-based strategy selection.

**Tasks**:
1. Create `reader.rs` with `FileReader` struct
2. Check file size from metadata before reading
3. Skip files > 10MB with warning
4. Direct read for files < 64KB
5. Memory-mapped I/O for 64KB - 10MB
6. Add timeout wrapper for file operations

**Files**:

```rust
// crates/cli/src/reader.rs

use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::time::Duration;

use memmap2::Mmap;

use crate::error::{Error, Result};

/// Size thresholds for read strategies.
pub const MMAP_THRESHOLD: u64 = 64 * 1024;      // 64KB
pub const LARGE_FILE_WARN: u64 = 1024 * 1024;   // 1MB
pub const MAX_FILE_SIZE: u64 = 10 * 1024 * 1024; // 10MB

/// Default per-file processing timeout.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(5);

/// File content with metadata.
#[derive(Debug)]
pub struct FileContent {
    /// The file content as bytes.
    pub bytes: Vec<u8>,

    /// File size in bytes.
    pub size: u64,

    /// Whether the file was memory-mapped.
    pub mmap_used: bool,
}

/// Read strategy used for a file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadStrategy {
    /// Direct read into buffer (< 64KB).
    Direct,

    /// Memory-mapped I/O (64KB - 10MB).
    Mmap,

    /// Skipped due to size (> 10MB).
    Skipped,
}

impl ReadStrategy {
    /// Determine the read strategy for a file of the given size.
    pub fn for_size(size: u64) -> Self {
        if size > MAX_FILE_SIZE {
            ReadStrategy::Skipped
        } else if size > MMAP_THRESHOLD {
            ReadStrategy::Mmap
        } else {
            ReadStrategy::Direct
        }
    }
}

/// Size-gated file reader.
pub struct FileReader {
    /// Maximum file size to read.
    max_size: u64,

    /// Threshold for memory-mapped I/O.
    mmap_threshold: u64,
}

impl Default for FileReader {
    fn default() -> Self {
        Self {
            max_size: MAX_FILE_SIZE,
            mmap_threshold: MMAP_THRESHOLD,
        }
    }
}

impl FileReader {
    /// Create a new file reader with default thresholds.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a reader with custom thresholds.
    pub fn with_thresholds(mmap_threshold: u64, max_size: u64) -> Self {
        Self {
            max_size,
            mmap_threshold,
        }
    }

    /// Read a file, selecting strategy based on size.
    ///
    /// Returns `Err(FileTooLarge)` for files exceeding max_size.
    pub fn read(&self, path: &Path) -> Result<FileContent> {
        let metadata = std::fs::metadata(path).map_err(|e| Error::Io {
            path: path.to_path_buf(),
            source: e,
        })?;

        let size = metadata.len();

        // Check size before reading
        if size > self.max_size {
            return Err(Error::FileTooLarge {
                path: path.to_path_buf(),
                size,
                max_size: self.max_size,
            });
        }

        // Report large files (1MB - 10MB)
        if size > LARGE_FILE_WARN {
            tracing::info!(
                path = %path.display(),
                size_mb = size as f64 / 1_000_000.0,
                "Reading large file"
            );
        }

        // Select strategy
        let (bytes, mmap_used) = if size > self.mmap_threshold {
            (self.read_mmap(path)?, true)
        } else {
            (self.read_direct(path, size)?, false)
        };

        Ok(FileContent {
            bytes,
            size,
            mmap_used,
        })
    }

    /// Read file directly into buffer.
    fn read_direct(&self, path: &Path, size: u64) -> Result<Vec<u8>> {
        let mut file = File::open(path).map_err(|e| Error::Io {
            path: path.to_path_buf(),
            source: e,
        })?;

        let mut buffer = Vec::with_capacity(size as usize);
        file.read_to_end(&mut buffer).map_err(|e| Error::Io {
            path: path.to_path_buf(),
            source: e,
        })?;

        Ok(buffer)
    }

    /// Read file using memory mapping.
    fn read_mmap(&self, path: &Path) -> Result<Vec<u8>> {
        let file = File::open(path).map_err(|e| Error::Io {
            path: path.to_path_buf(),
            source: e,
        })?;

        // SAFETY: File is opened read-only, we copy the content immediately
        let mmap = unsafe {
            Mmap::map(&file).map_err(|e| Error::Io {
                path: path.to_path_buf(),
                source: e,
            })?
        };

        // Copy to owned Vec to avoid lifetime issues
        Ok(mmap.to_vec())
    }

    /// Check if a file should be read based on size.
    pub fn should_read(&self, path: &Path) -> Result<ReadStrategy> {
        let metadata = std::fs::metadata(path).map_err(|e| Error::Io {
            path: path.to_path_buf(),
            source: e,
        })?;

        Ok(ReadStrategy::for_size(metadata.len()))
    }
}

#[cfg(test)]
#[path = "reader_tests.rs"]
mod tests;
```

**Verification**:
```bash
cargo test --lib reader
```

### Phase 20.4: Error Types and CLI Integration

**Goal**: Add error types and integrate walker/reader into CLI.

**Tasks**:
1. Add `FileTooLarge` error variant
2. Add `--debug-files` flag to list scanned files
3. Add `--max-depth` flag
4. Add `--verbose` for detailed logging
5. Wire walker into check command

**Files**:

```rust
// crates/cli/src/error.rs - additions

#[derive(Debug, thiserror::Error)]
pub enum Error {
    // ... existing variants

    /// File exceeds maximum size limit.
    #[error("file too large: {} ({} bytes, max: {} bytes)", .path.display(), .size, .max_size)]
    FileTooLarge {
        path: PathBuf,
        size: u64,
        max_size: u64,
    },

    /// Walker error.
    #[error("walk error: {message}")]
    Walk {
        message: String,
    },
}
```

```rust
// crates/cli/src/cli.rs - additions to CheckArgs

#[derive(Debug, Args)]
pub struct CheckArgs {
    // ... existing args

    /// Maximum directory depth to traverse.
    #[arg(long, default_value_t = 100)]
    pub max_depth: usize,

    /// List scanned files (for debugging).
    #[arg(long, hide = true)]
    pub debug_files: bool,
}
```

**Verification**:
```bash
cargo build
./target/debug/quench check --help
./target/debug/quench check --debug-files tests/fixtures/rust-simple
```

### Phase 20.5: Custom Ignore Pattern Integration

**Goal**: Wire custom ignore patterns from config into the walker.

**Tasks**:
1. Load ignore patterns from config in check command
2. Create `.quenchignore` file support (like `.gitignore`)
3. Pass patterns to walker builder
4. Add integration tests

**Files**:

The walker already accepts ignore patterns. Integration:

```rust
// In check command handler

let config = load_config()?;
let walker_config = WalkerConfig {
    max_depth: Some(args.max_depth),
    ignore_patterns: config.project.ignore.patterns.clone(),
    ..Default::default()
};

let walker = FileWalker::new(walker_config);
let (rx, stats) = walker.walk(&project_root);

if args.debug_files {
    for file in rx {
        println!("{}", file.path.display());
    }
    return Ok(());
}

// Process files...
```

**Verification**:
```bash
cargo test --test specs file_walking_respects_custom_ignore_patterns -- --ignored
```

### Phase 20.6: Unit Tests and Edge Cases

**Goal**: Comprehensive unit tests for walker and reader.

**Tasks**:
1. Create `walker_tests.rs` with temp directory tests
2. Create `reader_tests.rs` with file size tests
3. Test symlink loop handling
4. Test depth limiting
5. Test custom patterns

**Files**:

```rust
// crates/cli/src/walker_tests.rs

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use std::fs;
use tempfile::TempDir;

fn create_test_tree(dir: &Path) {
    fs::create_dir_all(dir.join("src")).unwrap();
    fs::write(dir.join("src/lib.rs"), "fn main() {}").unwrap();
    fs::write(dir.join("src/test.rs"), "fn test() {}").unwrap();
}

#[test]
fn walks_simple_directory() {
    let tmp = TempDir::new().unwrap();
    create_test_tree(tmp.path());

    let walker = FileWalker::new(WalkerConfig::default());
    let (files, stats) = walker.walk_collect(tmp.path());

    assert_eq!(files.len(), 2);
    assert_eq!(stats.files_found, 2);
}

#[test]
fn respects_gitignore() {
    let tmp = TempDir::new().unwrap();
    create_test_tree(tmp.path());

    // Add .gitignore
    fs::write(tmp.path().join(".gitignore"), "*.rs\n").unwrap();

    // Init git repo so gitignore is respected
    fs::create_dir(tmp.path().join(".git")).unwrap();

    let walker = FileWalker::new(WalkerConfig::default());
    let (files, _) = walker.walk_collect(tmp.path());

    // .rs files should be ignored
    assert!(files.iter().all(|f| !f.path.extension().map(|e| e == "rs").unwrap_or(false)));
}

#[test]
fn respects_depth_limit() {
    let tmp = TempDir::new().unwrap();

    // Create nested structure: level1/level2/level3/file.rs
    let deep = tmp.path().join("level1/level2/level3");
    fs::create_dir_all(&deep).unwrap();
    fs::write(deep.join("file.rs"), "fn f() {}").unwrap();

    // Shallow file
    fs::write(tmp.path().join("shallow.rs"), "fn s() {}").unwrap();

    let walker = FileWalker::new(WalkerConfig {
        max_depth: Some(2),
        ..Default::default()
    });
    let (files, _) = walker.walk_collect(tmp.path());

    // Should find shallow.rs but not level1/level2/level3/file.rs
    assert_eq!(files.len(), 1);
    assert!(files[0].path.ends_with("shallow.rs"));
}

#[test]
fn custom_ignore_patterns() {
    let tmp = TempDir::new().unwrap();
    fs::create_dir_all(tmp.path().join("src")).unwrap();
    fs::write(tmp.path().join("src/lib.rs"), "fn main() {}").unwrap();
    fs::write(tmp.path().join("src/test.snapshot"), "snapshot").unwrap();

    let walker = FileWalker::new(WalkerConfig {
        ignore_patterns: vec!["*.snapshot".to_string()],
        git_ignore: false,  // Don't need git
        ..Default::default()
    });
    let (files, _) = walker.walk_collect(tmp.path());

    // snapshot should be ignored
    assert!(files.iter().all(|f| !f.path.to_string_lossy().contains(".snapshot")));
}

#[test]
fn collects_file_size() {
    let tmp = TempDir::new().unwrap();
    let content = "hello world";
    fs::write(tmp.path().join("file.txt"), content).unwrap();

    let walker = FileWalker::new(WalkerConfig::default());
    let (files, _) = walker.walk_collect(tmp.path());

    assert_eq!(files.len(), 1);
    assert_eq!(files[0].size, content.len() as u64);
}
```

```rust
// crates/cli/src/reader_tests.rs

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use tempfile::TempDir;

#[test]
fn reads_small_file_directly() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("small.txt");
    std::fs::write(&path, "hello").unwrap();

    let reader = FileReader::new();
    let content = reader.read(&path).unwrap();

    assert_eq!(content.bytes, b"hello");
    assert_eq!(content.size, 5);
    assert!(!content.mmap_used);
}

#[test]
fn reads_large_file_with_mmap() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("large.txt");

    // Create file > 64KB
    let data = vec![b'x'; 100_000];
    std::fs::write(&path, &data).unwrap();

    let reader = FileReader::new();
    let content = reader.read(&path).unwrap();

    assert_eq!(content.bytes.len(), 100_000);
    assert!(content.mmap_used);
}

#[test]
fn rejects_oversized_file() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("huge.txt");

    // Create file > 10MB
    let data = vec![b'x'; 11_000_000];
    std::fs::write(&path, &data).unwrap();

    let reader = FileReader::new();
    let result = reader.read(&path);

    assert!(matches!(result, Err(Error::FileTooLarge { .. })));
}

#[test]
fn strategy_selection() {
    assert_eq!(ReadStrategy::for_size(100), ReadStrategy::Direct);
    assert_eq!(ReadStrategy::for_size(64 * 1024), ReadStrategy::Direct);
    assert_eq!(ReadStrategy::for_size(64 * 1024 + 1), ReadStrategy::Mmap);
    assert_eq!(ReadStrategy::for_size(10 * 1024 * 1024), ReadStrategy::Mmap);
    assert_eq!(ReadStrategy::for_size(10 * 1024 * 1024 + 1), ReadStrategy::Skipped);
}
```

**Verification**:
```bash
cargo test --lib walker_tests
cargo test --lib reader_tests
```

## Key Implementation Details

### Parallel Walking Architecture

The `ignore` crate's `WalkBuilder::build_parallel()` provides:
- Work-stealing thread pool for directory traversal
- Built-in gitignore parsing and matching
- Symlink loop detection (reports error, continues walking)
- Depth limiting (iterative, not recursive - no stack overflow)

```rust
// Key configuration for parallel walking
WalkBuilder::new(root)
    .hidden(true)           // Skip hidden files
    .git_ignore(true)       // Respect .gitignore
    .git_exclude(true)      // Respect .git/info/exclude
    .git_global(true)       // Respect global gitignore
    .max_depth(Some(100))   // Limit depth
    .build_parallel()       // Use all cores
```

### Custom Ignore Pattern Integration

Custom patterns from `[project.ignore]` are added via `OverrideBuilder`:

```rust
let mut override_builder = ignore::overrides::OverrideBuilder::new(root);
for pattern in &config.ignore.patterns {
    // Patterns are negated globs (ignore = don't match)
    override_builder.add(&format!("!{}", pattern))?;
}
builder.overrides(override_builder.build()?);
```

### Size-Gated Reading Strategy

| Size | Strategy | Rationale |
|------|----------|-----------|
| < 64KB | Direct read | Small allocation, fast |
| 64KB - 10MB | Memory map | Efficient for larger files |
| > 10MB | Skip with warning | Protect against pathological cases |

Memory-mapped reading:
```rust
let mmap = unsafe { Mmap::map(&file)? };
let bytes = mmap.to_vec(); // Copy to owned buffer
```

### Streaming Pipeline

Files are streamed through a bounded channel to avoid unbounded memory:

```rust
let (tx, rx) = crossbeam_channel::bounded(1000);

// Producer: parallel walking
walker.run(|| Box::new(|entry| { tx.send(entry); ... }));

// Consumer: processing
for file in rx {
    check_file(&file)?;
}
```

### Symlink Loop Handling

The `ignore` crate detects symlink loops automatically. When encountered:
1. Error is logged via `tracing::warn!`
2. Walking continues with other files
3. Stats track symlink_loops count
4. `--verbose` displays loop warnings

### Debug Output

The `--debug-files` flag lists all discovered files:

```bash
$ quench check --debug-files
src/lib.rs
src/main.rs
tests/test.rs
```

This enables behavioral specs to verify file discovery behavior.

## Verification Plan

### Phase Completion Checklist

- [ ] `ignore` and `memmap2` added to dependencies
- [ ] `[project.ignore]` config parsing works
- [ ] `FileWalker` discovers files with gitignore filtering
- [ ] `FileWalker` respects custom ignore patterns
- [ ] `FileWalker` limits depth correctly
- [ ] `FileWalker` handles symlink loops without hanging
- [ ] `FileReader` uses direct read for small files
- [ ] `FileReader` uses mmap for medium files
- [ ] `FileReader` rejects files > 10MB
- [ ] `--debug-files` flag lists discovered files
- [ ] `--max-depth` flag configurable
- [ ] All Phase 015 behavioral specs pass
- [ ] Unit tests cover edge cases
- [ ] Benchmarks run on bench-medium fixture

### Behavioral Specs to Enable

Remove `#[ignore]` from these specs (tests/specs/file_walking.rs):

1. `file_walking_respects_gitignore`
2. `file_walking_ignores_gitignore_glob_patterns`
3. `file_walking_respects_nested_gitignore`
4. `file_walking_respects_custom_ignore_patterns`
5. `file_walking_respects_custom_directory_patterns`
6. `file_walking_respects_double_star_patterns`
7. `file_walking_detects_symlink_loops`
8. `file_walking_reports_symlink_loops_in_verbose_mode`
9. `file_walking_scans_normal_files_despite_symlink_loops`
10. `file_walking_respects_default_depth_limit`
11. `file_walking_respects_custom_depth_limit`
12. `file_walking_warns_on_depth_limit_in_verbose`
13. `file_walking_handles_empty_directory`
14. `file_walking_uses_iterative_traversal`

### Running Verification

```bash
# Run unit tests
cargo test --lib walker reader config

# Run behavioral specs (should all pass)
cargo test --test specs file_walking

# Run benchmarks
cargo bench --bench file_walking

# Verify CLI flags
cargo run -- check --debug-files tests/fixtures/rust-simple
cargo run -- check --max-depth 5 tests/fixtures/bench-deep
cargo run -- check --verbose tests/fixtures/symlink-loop

# Full check
make check
```

### Expected Benchmark Results

For `bench-medium` fixture (500 files, 50K LOC):

| Metric | Single-threaded | Parallel |
|--------|-----------------|----------|
| Walk time | ~50ms | ~15ms |
| Files/sec | ~10K | ~30K |

Actual numbers depend on hardware; focus on parallel speedup ratio (2-4x expected).

## Summary

Phase 020 implements the core file discovery infrastructure:

1. **Parallel walker** using `ignore` crate with gitignore support
2. **Custom ignore patterns** from `[project.ignore]` config
3. **Size-gated reader** with direct read (< 64KB) and mmap (64KB - 10MB)
4. **Edge case handling** for symlinks, deep trees, large files
5. **CLI integration** with `--debug-files`, `--max-depth`, `--verbose`

All 14 behavioral specs from Phase 015 should pass after implementation.
