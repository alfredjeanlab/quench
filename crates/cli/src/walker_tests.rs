// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

use super::*;
use crate::test_utils::create_tree;
use std::fs;
use tempfile::TempDir;
use yare::parameterized;

/// Create a test WalkerConfig with git/hidden disabled for predictable tests.
fn test_config() -> WalkerConfig {
    WalkerConfig { git_ignore: false, hidden: false, ..Default::default() }
}

#[test]
fn walks_simple_directory() {
    let tmp = TempDir::new().unwrap();
    create_tree(tmp.path(), &[("src/lib.rs", "fn main() {}"), ("src/test.rs", "fn test() {}")]);

    let walker = FileWalker::new(WalkerConfig::default());
    let (files, stats) = walker.walk_collect(tmp.path());

    assert_eq!(files.len(), 2);
    assert_eq!(stats.files_found, 2);
}

#[test]
fn respects_gitignore() {
    let tmp = TempDir::new().unwrap();
    create_tree(tmp.path(), &[("src/lib.rs", "fn main() {}"), ("src/test.rs", "fn test() {}")]);

    // Add .gitignore
    fs::write(tmp.path().join(".gitignore"), "*.rs\n").unwrap();

    // Init git repo so gitignore is respected
    fs::create_dir(tmp.path().join(".git")).unwrap();

    let walker = FileWalker::new(WalkerConfig::default());
    let (files, _) = walker.walk_collect(tmp.path());

    // .rs files should be ignored
    assert!(
        files.iter().all(|f| !f.path.extension().map(|e| e == "rs").unwrap_or(false)),
        "expected no .rs files but found: {:?}",
        files.iter().map(|f| &f.path).collect::<Vec<_>>()
    );
}

#[test]
fn respects_depth_limit() {
    let tmp = TempDir::new().unwrap();
    create_tree(
        tmp.path(),
        &[("level1/level2/level3/file.rs", "fn f() {}"), ("shallow.rs", "fn s() {}")],
    );

    let walker = FileWalker::new(WalkerConfig { max_depth: Some(2), ..test_config() });
    let (files, _) = walker.walk_collect(tmp.path());

    // Should find shallow.rs but not level1/level2/level3/file.rs
    assert_eq!(files.len(), 1);
    assert!(files[0].path.ends_with("shallow.rs"));
}

#[test]
fn custom_exclude_patterns() {
    let tmp = TempDir::new().unwrap();
    create_tree(tmp.path(), &[("src/lib.rs", "fn main() {}"), ("src/test.snapshot", "snapshot")]);

    let walker = FileWalker::new(WalkerConfig {
        exclude_patterns: vec!["*.snapshot".to_string()],
        ..test_config()
    });
    let (files, _) = walker.walk_collect(tmp.path());

    // snapshot should be excluded
    assert!(
        files.iter().all(|f| !f.path.to_string_lossy().contains(".snapshot")),
        "expected no .snapshot files but found: {:?}",
        files.iter().map(|f| &f.path).collect::<Vec<_>>()
    );
}

#[test]
fn collects_file_size() {
    let tmp = TempDir::new().unwrap();
    let content = "hello world";
    fs::write(tmp.path().join("file.txt"), content).unwrap();

    let walker = FileWalker::new(test_config());
    let (files, _) = walker.walk_collect(tmp.path());

    assert_eq!(files.len(), 1);
    assert_eq!(files[0].size, content.len() as u64);
}

#[test]
fn tracks_file_depth() {
    let tmp = TempDir::new().unwrap();
    create_tree(
        tmp.path(),
        &[("root.txt", "root"), ("a/level1.txt", "level1"), ("a/b/level2.txt", "level2")],
    );

    let walker = FileWalker::new(test_config());
    let (files, _) = walker.walk_collect(tmp.path());

    assert_eq!(files.len(), 3);

    // Sort by depth for consistent testing
    let mut files = files;
    files.sort_by_key(|f| f.depth);

    assert_eq!(files[0].depth, 1); // root.txt
    assert_eq!(files[1].depth, 2); // a/level1.txt
    assert_eq!(files[2].depth, 3); // a/b/level2.txt
}

#[test]
fn handles_empty_directory() {
    let tmp = TempDir::new().unwrap();

    let walker = FileWalker::new(WalkerConfig::default());
    let (files, stats) = walker.walk_collect(tmp.path());

    assert!(files.is_empty());
    assert_eq!(stats.files_found, 0);
    assert_eq!(stats.errors, 0);
}

#[test]
fn from_exclude_config() {
    let exclude = ExcludeConfig { patterns: vec!["*.log".to_string(), "tmp/".to_string()] };

    let walker = FileWalker::from_exclude_config(&exclude);
    assert_eq!(walker.config.exclude_patterns, exclude.patterns);
}

// Adaptive parallel/sequential tests

#[test]
fn should_use_parallel_on_large_directory() {
    let tmp = TempDir::new().unwrap();

    // Create many top-level entries (threshold/10 = 100 by default)
    for i in 0..150 {
        fs::write(tmp.path().join(format!("file{}.txt", i)), "content").unwrap();
    }

    let walker = FileWalker::new(WalkerConfig::default());
    assert!(
        walker.should_use_parallel(tmp.path()),
        "expected parallel mode for directory with {} entries",
        150
    );
}

#[test]
fn should_use_sequential_on_small_directory() {
    let tmp = TempDir::new().unwrap();

    // Create few top-level entries (less than threshold/10 = 100)
    for i in 0..10 {
        fs::write(tmp.path().join(format!("file{}.txt", i)), "content").unwrap();
    }

    let walker = FileWalker::new(WalkerConfig::default());
    assert!(
        !walker.should_use_parallel(tmp.path()),
        "expected sequential mode for directory with {} entries",
        10
    );
}

#[parameterized(
    force_parallel = { true, false, true },
    force_sequential = { false, true, false },
)]
fn force_flags_override_heuristic(force_parallel: bool, force_sequential: bool, expected: bool) {
    let tmp = TempDir::new().unwrap();
    create_tree(tmp.path(), &[("file.txt", "content")]);

    let walker =
        FileWalker::new(WalkerConfig { force_parallel, force_sequential, ..Default::default() });

    assert_eq!(
        walker.should_use_parallel(tmp.path()),
        expected,
        "force_parallel={} force_sequential={} should give {}",
        force_parallel,
        force_sequential,
        expected
    );
}

#[test]
fn parallel_and_sequential_produce_same_files() {
    let tmp = TempDir::new().unwrap();
    create_tree(
        tmp.path(),
        &[
            ("src/main.rs", "fn main() {}"),
            ("src/lib.rs", "pub fn lib() {}"),
            ("tests/test.rs", "#[test] fn t() {}"),
            ("README.md", "# Readme"),
        ],
    );

    // Walk with force_parallel
    let walker_parallel = FileWalker::new(WalkerConfig { force_parallel: true, ..test_config() });
    let (parallel_files, parallel_stats) = walker_parallel.walk_collect(tmp.path());

    // Walk with force_sequential
    let walker_sequential =
        FileWalker::new(WalkerConfig { force_sequential: true, ..test_config() });
    let (sequential_files, sequential_stats) = walker_sequential.walk_collect(tmp.path());

    // Same number of files
    assert_eq!(
        parallel_files.len(),
        sequential_files.len(),
        "parallel and sequential should find same number of files"
    );

    // Same stats
    assert_eq!(
        parallel_stats.files_found, sequential_stats.files_found,
        "file counts should match"
    );

    // Same file paths (sorted for comparison since order may differ)
    let mut parallel_paths: Vec<_> = parallel_files.iter().map(|f| &f.path).collect();
    let mut sequential_paths: Vec<_> = sequential_files.iter().map(|f| &f.path).collect();
    parallel_paths.sort();
    sequential_paths.sort();

    assert_eq!(
        parallel_paths, sequential_paths,
        "parallel and sequential should find identical files"
    );
}

#[test]
fn custom_parallel_threshold() {
    let tmp = TempDir::new().unwrap();

    // Create 20 top-level entries
    for i in 0..20 {
        fs::write(tmp.path().join(format!("file{}.txt", i)), "content").unwrap();
    }

    // With default threshold (1000), should be sequential (need 100 entries)
    let walker_default = FileWalker::new(WalkerConfig::default());
    assert!(
        !walker_default.should_use_parallel(tmp.path()),
        "default threshold should use sequential for 20 entries"
    );

    // With lower threshold (100), should be parallel (need 10 entries)
    let walker_low =
        FileWalker::new(WalkerConfig { parallel_threshold: 100, ..Default::default() });
    assert!(
        walker_low.should_use_parallel(tmp.path()),
        "low threshold should use parallel for 20 entries"
    );
}

#[test]
fn skips_files_over_10mb() {
    use std::fs::File;

    let tmp = TempDir::new().unwrap();

    // Create a normal sized file
    fs::write(tmp.path().join("small.txt"), "hello").unwrap();

    // Create a sparse file larger than 10MB (15MB)
    let large_file = File::create(tmp.path().join("huge.txt")).unwrap();
    large_file.set_len(15 * 1024 * 1024).unwrap();

    let walker = FileWalker::new(WalkerConfig {
        force_sequential: true, // Test sequential path
        ..test_config()
    });
    let (files, stats) = walker.walk_collect(tmp.path());

    // Should only find the small file
    assert_eq!(files.len(), 1, "should only find small file");
    assert!(files[0].path.ends_with("small.txt"), "found file should be small.txt");

    // Stats should reflect the skipped file
    assert_eq!(stats.files_found, 1);
    assert_eq!(stats.files_skipped_size, 1);
}

#[test]
fn skips_files_over_10mb_parallel() {
    use std::fs::File;

    let tmp = TempDir::new().unwrap();

    // Create a normal sized file
    fs::write(tmp.path().join("small.txt"), "hello").unwrap();

    // Create a sparse file larger than 10MB
    let large_file = File::create(tmp.path().join("huge.txt")).unwrap();
    large_file.set_len(15 * 1024 * 1024).unwrap();

    let walker = FileWalker::new(WalkerConfig {
        force_parallel: true, // Test parallel path
        ..test_config()
    });
    let (files, stats) = walker.walk_collect(tmp.path());

    // Should only find the small file
    assert_eq!(files.len(), 1, "should only find small file");
    assert!(files[0].path.ends_with("small.txt"), "found file should be small.txt");

    // Stats should reflect the skipped file
    assert_eq!(stats.files_found, 1);
    assert_eq!(stats.files_skipped_size, 1);
}

#[test]
fn processes_files_just_under_10mb() {
    use crate::file_size::MAX_FILE_SIZE;
    use std::fs::File;

    let tmp = TempDir::new().unwrap();

    // Create a file just under the limit (10MB - 1 byte)
    let borderline_file = File::create(tmp.path().join("borderline.txt")).unwrap();
    borderline_file.set_len(MAX_FILE_SIZE - 1).unwrap();

    let walker = FileWalker::new(test_config());
    let (files, stats) = walker.walk_collect(tmp.path());

    // Should find the borderline file
    assert_eq!(files.len(), 1, "should find borderline file");
    assert_eq!(stats.files_found, 1);
    assert_eq!(stats.files_skipped_size, 0, "no files should be skipped");
}

#[test]
fn assigns_correct_size_class() {
    use crate::file_size::FileSizeClass;
    use std::fs::File;

    let tmp = TempDir::new().unwrap();

    // Create files of different sizes
    // Small: < 64KB
    fs::write(tmp.path().join("small.txt"), "hello").unwrap();

    // Normal: 64KB - 1MB (use 100KB)
    let normal_file = File::create(tmp.path().join("normal.txt")).unwrap();
    normal_file.set_len(100 * 1024).unwrap();

    // Oversized: 1MB - 10MB (use 5MB)
    let oversized_file = File::create(tmp.path().join("oversized.txt")).unwrap();
    oversized_file.set_len(5 * 1024 * 1024).unwrap();

    let walker = FileWalker::new(test_config());
    let (files, _) = walker.walk_collect(tmp.path());

    // Find files by name and check their size_class
    for file in &files {
        let name = file.path.file_name().unwrap().to_str().unwrap();
        match name {
            "small.txt" => assert_eq!(file.size_class, FileSizeClass::Small),
            "normal.txt" => assert_eq!(file.size_class, FileSizeClass::Normal),
            "oversized.txt" => assert_eq!(file.size_class, FileSizeClass::Oversized),
            _ => panic!("unexpected file: {}", name),
        }
    }
}
