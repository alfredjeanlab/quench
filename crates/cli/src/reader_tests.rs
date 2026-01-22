#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use tempfile::TempDir;

#[test]
fn reads_small_file() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("small.txt");
    std::fs::write(&path, "hello").unwrap();

    let reader = FileReader::new();
    let content = reader.read(&path).unwrap();

    assert_eq!(content.bytes, b"hello");
    assert_eq!(content.size, 5);
}

#[test]
fn reads_large_file() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("large.txt");

    // Create file > 64KB
    let data = vec![b'x'; 100_000];
    std::fs::write(&path, &data).unwrap();

    let reader = FileReader::new();
    let content = reader.read(&path).unwrap();

    assert_eq!(content.bytes.len(), 100_000);
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
    assert_eq!(
        ReadStrategy::for_size(10 * 1024 * 1024),
        ReadStrategy::Direct
    );
    assert_eq!(
        ReadStrategy::for_size(10 * 1024 * 1024 + 1),
        ReadStrategy::Skipped
    );
}

#[test]
fn custom_max_size() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("file.txt");
    let data = vec![b'x'; 1000];
    std::fs::write(&path, &data).unwrap();

    // With max size above file size, should read
    let reader = FileReader::with_max_size(2000);
    assert!(reader.read(&path).is_ok());

    // With max size below file size, should reject
    let reader = FileReader::with_max_size(500);
    assert!(matches!(
        reader.read(&path),
        Err(Error::FileTooLarge { .. })
    ));
}

#[test]
fn should_read_checks_strategy() {
    let tmp = TempDir::new().unwrap();

    // Small file
    let small = tmp.path().join("small.txt");
    std::fs::write(&small, "hello").unwrap();

    let reader = FileReader::new();
    assert_eq!(reader.should_read(&small).unwrap(), ReadStrategy::Direct);
}

#[test]
fn handles_nonexistent_file() {
    let reader = FileReader::new();
    let result = reader.read(Path::new("/nonexistent/file.txt"));

    assert!(matches!(result, Err(Error::Io { .. })));
}
