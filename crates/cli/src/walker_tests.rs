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
