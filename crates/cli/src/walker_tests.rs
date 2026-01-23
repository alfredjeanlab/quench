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
    assert!(
        files
            .iter()
            .all(|f| !f.path.extension().map(|e| e == "rs").unwrap_or(false)),
        "expected no .rs files but found: {:?}",
        files.iter().map(|f| &f.path).collect::<Vec<_>>()
    );
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
        git_ignore: false,
        hidden: false,
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
        git_ignore: false,
        hidden: false,
        ..Default::default()
    });
    let (files, _) = walker.walk_collect(tmp.path());

    // snapshot should be ignored
    assert!(
        files
            .iter()
            .all(|f| !f.path.to_string_lossy().contains(".snapshot")),
        "expected no .snapshot files but found: {:?}",
        files.iter().map(|f| &f.path).collect::<Vec<_>>()
    );
}

#[test]
fn collects_file_size() {
    let tmp = TempDir::new().unwrap();
    let content = "hello world";
    fs::write(tmp.path().join("file.txt"), content).unwrap();

    let walker = FileWalker::new(WalkerConfig {
        git_ignore: false,
        hidden: false,
        ..Default::default()
    });
    let (files, _) = walker.walk_collect(tmp.path());

    assert_eq!(files.len(), 1);
    assert_eq!(files[0].size, content.len() as u64);
}

#[test]
fn tracks_file_depth() {
    let tmp = TempDir::new().unwrap();

    // Create nested structure
    fs::create_dir_all(tmp.path().join("a/b")).unwrap();
    fs::write(tmp.path().join("root.txt"), "root").unwrap();
    fs::write(tmp.path().join("a/level1.txt"), "level1").unwrap();
    fs::write(tmp.path().join("a/b/level2.txt"), "level2").unwrap();

    let walker = FileWalker::new(WalkerConfig {
        git_ignore: false,
        hidden: false,
        ..Default::default()
    });
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
fn from_ignore_config() {
    let ignore = IgnoreConfig {
        patterns: vec!["*.log".to_string(), "tmp/".to_string()],
    };

    let walker = FileWalker::from_ignore_config(&ignore);
    assert_eq!(walker.config.ignore_patterns, ignore.patterns);
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

#[test]
fn force_parallel_overrides_heuristic() {
    let tmp = TempDir::new().unwrap();

    // Create a small directory that would normally use sequential
    fs::write(tmp.path().join("file.txt"), "content").unwrap();

    let walker = FileWalker::new(WalkerConfig {
        force_parallel: true,
        ..Default::default()
    });

    assert!(
        walker.should_use_parallel(tmp.path()),
        "force_parallel should override heuristic"
    );
}

#[test]
fn force_sequential_overrides_heuristic() {
    let tmp = TempDir::new().unwrap();

    // Create many top-level entries that would normally use parallel
    for i in 0..150 {
        fs::write(tmp.path().join(format!("file{}.txt", i)), "content").unwrap();
    }

    let walker = FileWalker::new(WalkerConfig {
        force_sequential: true,
        ..Default::default()
    });

    assert!(
        !walker.should_use_parallel(tmp.path()),
        "force_sequential should override heuristic"
    );
}

#[test]
fn parallel_and_sequential_produce_same_files() {
    let tmp = TempDir::new().unwrap();

    // Create a test directory structure
    fs::create_dir_all(tmp.path().join("src")).unwrap();
    fs::create_dir_all(tmp.path().join("tests")).unwrap();
    fs::write(tmp.path().join("src/main.rs"), "fn main() {}").unwrap();
    fs::write(tmp.path().join("src/lib.rs"), "pub fn lib() {}").unwrap();
    fs::write(tmp.path().join("tests/test.rs"), "#[test] fn t() {}").unwrap();
    fs::write(tmp.path().join("README.md"), "# Readme").unwrap();

    // Walk with force_parallel
    let walker_parallel = FileWalker::new(WalkerConfig {
        force_parallel: true,
        git_ignore: false,
        hidden: false,
        ..Default::default()
    });
    let (parallel_files, parallel_stats) = walker_parallel.walk_collect(tmp.path());

    // Walk with force_sequential
    let walker_sequential = FileWalker::new(WalkerConfig {
        force_sequential: true,
        git_ignore: false,
        hidden: false,
        ..Default::default()
    });
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
    let walker_low = FileWalker::new(WalkerConfig {
        parallel_threshold: 100,
        ..Default::default()
    });
    assert!(
        walker_low.should_use_parallel(tmp.path()),
        "low threshold should use parallel for 20 entries"
    );
}
