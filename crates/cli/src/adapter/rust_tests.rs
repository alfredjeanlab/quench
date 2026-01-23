#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::Path;

use super::*;

mod classification {
    use super::*;

    #[test]
    fn source_file_in_src() {
        let adapter = RustAdapter::new();
        assert_eq!(adapter.classify(Path::new("src/lib.rs")), FileKind::Source);
        assert_eq!(adapter.classify(Path::new("src/main.rs")), FileKind::Source);
        assert_eq!(
            adapter.classify(Path::new("src/foo/bar.rs")),
            FileKind::Source
        );
    }

    #[test]
    fn test_file_in_tests_dir() {
        let adapter = RustAdapter::new();
        assert_eq!(
            adapter.classify(Path::new("tests/integration.rs")),
            FileKind::Test
        );
        assert_eq!(
            adapter.classify(Path::new("tests/foo/bar.rs")),
            FileKind::Test
        );
    }

    #[test]
    fn test_file_with_suffix() {
        let adapter = RustAdapter::new();
        assert_eq!(
            adapter.classify(Path::new("src/lib_test.rs")),
            FileKind::Test
        );
        assert_eq!(
            adapter.classify(Path::new("src/lib_tests.rs")),
            FileKind::Test
        );
    }

    #[test]
    fn ignored_target_dir() {
        let adapter = RustAdapter::new();
        assert_eq!(
            adapter.classify(Path::new("target/debug/deps/foo.rs")),
            FileKind::Other
        );
        assert_eq!(
            adapter.classify(Path::new("target/release/build/bar.rs")),
            FileKind::Other
        );
    }

    #[test]
    fn non_rust_file() {
        let adapter = RustAdapter::new();
        assert_eq!(adapter.classify(Path::new("Cargo.toml")), FileKind::Other);
        assert_eq!(adapter.classify(Path::new("README.md")), FileKind::Other);
    }
}

mod ignore_patterns {
    use super::*;

    #[test]
    fn target_dir_ignored() {
        let adapter = RustAdapter::new();
        assert!(adapter.should_ignore(Path::new("target/debug/foo.rs")));
        assert!(adapter.should_ignore(Path::new("target/release/bar.rs")));
    }

    #[test]
    fn src_not_ignored() {
        let adapter = RustAdapter::new();
        assert!(!adapter.should_ignore(Path::new("src/lib.rs")));
        assert!(!adapter.should_ignore(Path::new("tests/test.rs")));
    }
}

mod workspace {
    use super::*;
    use tempfile::TempDir;

    fn create_workspace(dir: &Path, manifest: &str) {
        std::fs::write(dir.join("Cargo.toml"), manifest).unwrap();
    }

    fn create_package(dir: &Path, name: &str) {
        let pkg_dir = dir.join(name);
        std::fs::create_dir_all(&pkg_dir).unwrap();
        std::fs::write(
            pkg_dir.join("Cargo.toml"),
            format!(
                r#"[package]
name = "{name}"
version = "0.1.0"
"#
            ),
        )
        .unwrap();
    }

    #[test]
    fn single_package() {
        let dir = TempDir::new().unwrap();
        create_workspace(
            dir.path(),
            r#"[package]
name = "my-project"
version = "0.1.0"
"#,
        );

        let workspace = CargoWorkspace::from_root(dir.path());
        assert!(!workspace.is_workspace);
        assert_eq!(workspace.packages, vec!["my-project"]);
        assert!(workspace.member_patterns.is_empty());
    }

    #[test]
    fn workspace_with_explicit_members() {
        let dir = TempDir::new().unwrap();
        std::fs::create_dir_all(dir.path().join("crates")).unwrap();
        create_package(&dir.path().join("crates"), "core");
        create_package(&dir.path().join("crates"), "cli");

        create_workspace(
            dir.path(),
            r#"[workspace]
members = ["crates/core", "crates/cli"]
"#,
        );

        let workspace = CargoWorkspace::from_root(dir.path());
        assert!(workspace.is_workspace);
        assert_eq!(workspace.packages, vec!["cli", "core"]);
        assert_eq!(workspace.member_patterns, vec!["crates/core", "crates/cli"]);
    }

    #[test]
    fn workspace_with_glob_members() {
        let dir = TempDir::new().unwrap();
        std::fs::create_dir_all(dir.path().join("crates")).unwrap();
        create_package(&dir.path().join("crates"), "core");
        create_package(&dir.path().join("crates"), "cli");

        create_workspace(
            dir.path(),
            r#"[workspace]
members = ["crates/*"]
"#,
        );

        let workspace = CargoWorkspace::from_root(dir.path());
        assert!(workspace.is_workspace);
        assert_eq!(workspace.packages, vec!["cli", "core"]);
        assert_eq!(workspace.member_patterns, vec!["crates/*"]);
    }

    #[test]
    fn no_cargo_toml() {
        let dir = TempDir::new().unwrap();
        let workspace = CargoWorkspace::from_root(dir.path());
        assert!(!workspace.is_workspace);
        assert!(workspace.packages.is_empty());
        assert!(workspace.member_patterns.is_empty());
    }
}
