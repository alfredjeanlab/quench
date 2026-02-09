// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

use std::path::PathBuf;

use tempfile::TempDir;

use super::*;

#[test]
fn is_glob_pattern_detects_asterisk() {
    assert!(is_glob_pattern("*.sh"));
    assert!(is_glob_pattern("scripts/*.sh"));
    assert!(is_glob_pattern("**/*.sh"));
}

#[test]
fn is_glob_pattern_detects_question_mark() {
    assert!(is_glob_pattern("file?.sh"));
}

#[test]
fn is_glob_pattern_detects_brackets() {
    assert!(is_glob_pattern("[abc].sh"));
    assert!(is_glob_pattern("file[0-9].sh"));
}

#[test]
fn is_glob_pattern_returns_false_for_plain_names() {
    assert!(!is_glob_pattern("myapp"));
    assert!(!is_glob_pattern("my-cli-tool"));
    assert!(!is_glob_pattern("app_v2"));
}

#[test]
fn resolve_rust_binary_from_package_name() {
    let temp = TempDir::new().unwrap();
    std::fs::write(
        temp.path().join("Cargo.toml"),
        r#"
[package]
name = "myapp"
version = "0.1.0"
"#,
    )
    .unwrap();

    let result = resolve_rust_binary("myapp", temp.path());
    assert!(result.is_ok());

    match result.unwrap() {
        ResolvedTarget::RustBinary { name, .. } => {
            assert_eq!(name, "myapp");
        }
        _ => panic!("expected RustBinary"),
    }
}

#[test]
fn resolve_rust_binary_from_bin_entry() {
    let temp = TempDir::new().unwrap();
    std::fs::write(
        temp.path().join("Cargo.toml"),
        r#"
[package]
name = "mylib"
version = "0.1.0"

[[bin]]
name = "mycli"
path = "src/main.rs"
"#,
    )
    .unwrap();

    let result = resolve_rust_binary("mycli", temp.path());
    assert!(result.is_ok());

    match result.unwrap() {
        ResolvedTarget::RustBinary { name, .. } => {
            assert_eq!(name, "mycli");
        }
        _ => panic!("expected RustBinary"),
    }
}

#[test]
fn resolve_rust_binary_fails_for_unknown_name() {
    let temp = TempDir::new().unwrap();
    std::fs::write(
        temp.path().join("Cargo.toml"),
        r#"
[package]
name = "myapp"
version = "0.1.0"
"#,
    )
    .unwrap();

    let result = resolve_rust_binary("unknown", temp.path());
    assert!(result.is_err());
    assert!(result.unwrap_err().message.contains("not a Rust binary or glob pattern"));
}

#[test]
fn resolve_shell_pattern_matches_files() {
    let temp = TempDir::new().unwrap();
    let scripts_dir = temp.path().join("scripts");
    std::fs::create_dir(&scripts_dir).unwrap();
    std::fs::write(scripts_dir.join("helper.sh"), "#!/bin/bash\necho hi").unwrap();
    std::fs::write(scripts_dir.join("util.sh"), "#!/bin/bash\necho util").unwrap();
    std::fs::write(scripts_dir.join("readme.txt"), "not a script").unwrap();

    let config = Config::default();
    let result = resolve_shell_pattern("scripts/*.sh", &config, temp.path());
    assert!(result.is_ok());

    match result.unwrap() {
        ResolvedTarget::ShellScripts { files, .. } => {
            assert_eq!(files.len(), 2);
            let names: Vec<_> = files
                .iter()
                .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
                .collect();
            assert!(names.contains(&"helper.sh".to_string()));
            assert!(names.contains(&"util.sh".to_string()));
        }
        _ => panic!("expected ShellScripts"),
    }
}

#[test]
fn resolve_shell_pattern_fails_for_no_matches() {
    let temp = TempDir::new().unwrap();
    let config = Config::default();

    let result = resolve_shell_pattern("nonexistent/*.sh", &config, temp.path());
    assert!(result.is_err());
    assert!(result.unwrap_err().message.contains("no shell scripts match"));
}

#[test]
fn resolve_target_dispatches_to_correct_resolver() {
    let temp = TempDir::new().unwrap();
    std::fs::write(
        temp.path().join("Cargo.toml"),
        r#"
[package]
name = "myapp"
version = "0.1.0"
"#,
    )
    .unwrap();

    let scripts_dir = temp.path().join("scripts");
    std::fs::create_dir(&scripts_dir).unwrap();
    std::fs::write(scripts_dir.join("test.sh"), "#!/bin/bash").unwrap();

    let config = Config::default();

    // Binary target
    let binary_result = resolve_target("myapp", &config, temp.path());
    assert!(matches!(binary_result, Ok(ResolvedTarget::RustBinary { .. })));

    // Glob target
    let glob_result = resolve_target("scripts/*.sh", &config, temp.path());
    assert!(matches!(glob_result, Ok(ResolvedTarget::ShellScripts { .. })));
}

#[test]
fn rust_binary_names_extracts_names() {
    let targets = vec![
        ResolvedTarget::RustBinary { name: "app1".to_string(), binary_path: None },
        ResolvedTarget::ShellScripts { pattern: "*.sh".to_string(), files: vec![] },
        ResolvedTarget::RustBinary {
            name: "app2".to_string(),
            binary_path: Some(PathBuf::from("/path/to/app2")),
        },
    ];

    let names = rust_binary_names(&targets);
    assert_eq!(names, vec!["app1".to_string(), "app2".to_string()]);
}

#[test]
fn shell_script_files_extracts_paths() {
    let targets = vec![
        ResolvedTarget::RustBinary { name: "app".to_string(), binary_path: None },
        ResolvedTarget::ShellScripts {
            pattern: "a/*.sh".to_string(),
            files: vec![PathBuf::from("a/one.sh"), PathBuf::from("a/two.sh")],
        },
        ResolvedTarget::ShellScripts {
            pattern: "b/*.sh".to_string(),
            files: vec![PathBuf::from("b/three.sh")],
        },
    ];

    let files = shell_script_files(&targets);
    assert_eq!(
        files,
        vec![PathBuf::from("a/one.sh"), PathBuf::from("a/two.sh"), PathBuf::from("b/three.sh"),]
    );
}

#[test]
fn resolve_targets_collects_all() {
    let temp = TempDir::new().unwrap();
    std::fs::write(
        temp.path().join("Cargo.toml"),
        r#"
[package]
name = "myapp"
version = "0.1.0"
"#,
    )
    .unwrap();

    let scripts_dir = temp.path().join("scripts");
    std::fs::create_dir(&scripts_dir).unwrap();
    std::fs::write(scripts_dir.join("test.sh"), "#!/bin/bash").unwrap();

    let config = Config::default();
    let target_strs = vec!["myapp".to_string(), "scripts/*.sh".to_string()];

    let result = resolve_targets(&target_strs, &config, temp.path());
    assert!(result.is_ok());

    let resolved = result.unwrap();
    assert_eq!(resolved.len(), 2);
    assert!(matches!(resolved[0], ResolvedTarget::RustBinary { .. }));
    assert!(matches!(resolved[1], ResolvedTarget::ShellScripts { .. }));
}
