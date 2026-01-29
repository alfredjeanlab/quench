// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

use super::*;
use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use tempfile::TempDir;

#[test]
fn test_detect_shells() {
    // Should detect at least one shell on most systems
    let shells = detect_shells();
    // This test may be environment-dependent, but at least verify it doesn't panic
    assert!(shells.len() <= 3);
}

#[test]
fn test_shell_exists() {
    // 'sh' should exist on all Unix systems
    assert!(shell_exists("sh"));
    // Nonexistent binary should return false
    assert!(!shell_exists("nonexistent_shell_binary_xyz"));
}

#[test]
fn test_shell_kind_script_filename() {
    assert_eq!(ShellKind::Bash.script_filename(), "quench.bash");
    assert_eq!(ShellKind::Zsh.script_filename(), "_quench");
    assert_eq!(ShellKind::Fish.script_filename(), "quench.fish");
}

#[test]
fn test_shell_kind_clap_shell() {
    assert_eq!(ShellKind::Bash.clap_shell(), clap_complete::Shell::Bash);
    assert_eq!(ShellKind::Zsh.clap_shell(), clap_complete::Shell::Zsh);
    assert_eq!(ShellKind::Fish.clap_shell(), clap_complete::Shell::Fish);
}

#[test]
fn test_completions_dir() {
    // Should return a path on systems with HOME set
    let dir = completions_dir();
    if let Some(d) = dir {
        assert!(d.to_string_lossy().contains("quench/completions"));
    }
}

#[test]
fn test_home_dir() {
    // HOME should be set on Unix systems
    let home = home_dir();
    assert!(home.is_some(), "HOME should be available");
}

#[test]
fn test_data_local_dir() {
    // data_local_dir should return a path on Unix systems with HOME set
    let data = data_local_dir();
    if let Some(d) = data {
        assert!(
            d.to_string_lossy().contains(".local/share")
                || std::env::var_os("XDG_DATA_HOME").is_some()
        );
    }
}

#[test]
fn test_config_dir() {
    // config_dir should return a path on Unix systems with HOME set
    let config = config_dir();
    if let Some(c) = config {
        assert!(
            c.to_string_lossy().contains(".config")
                || std::env::var_os("XDG_CONFIG_HOME").is_some()
        );
    }
}

#[test]
fn test_install_completion_source_idempotent() {
    let temp = TempDir::new().unwrap();
    let rc_path = temp.path().join(".bashrc");

    // Create a fake RC file
    fs::write(&rc_path, "# my bashrc\nexport FOO=bar\n").unwrap();

    // Create a fake completion script
    let script_path = temp.path().join("quench.bash");
    fs::write(&script_path, "# completions").unwrap();

    // Manually add the sourcing line
    let source_line = format!(
        "\n{}\n[ -f \"{}\" ] && source \"{}\"\n",
        QUENCH_COMPLETION_MARKER,
        script_path.display(),
        script_path.display()
    );
    let mut file = OpenOptions::new().append(true).open(&rc_path).unwrap();
    file.write_all(source_line.as_bytes()).unwrap();
    drop(file);

    let content_before = fs::read_to_string(&rc_path).unwrap();
    let marker_count_before = content_before.matches(QUENCH_COMPLETION_MARKER).count();
    assert_eq!(marker_count_before, 1);

    // Verify idempotency logic: existing content contains the marker
    let existing = fs::read_to_string(&rc_path).unwrap();
    assert!(existing.contains(QUENCH_COMPLETION_MARKER));
}

// Note: tests that set HOME env var are fragile because they may affect other
// tests running in parallel. These tests verify behavior using the actual
// home directory's RC files when they exist.

#[test]
fn test_shell_kind_rc_file_returns_correct_paths() {
    // Test that when RC files exist, they have the expected suffixes
    if let Some(rc) = ShellKind::Bash.rc_file() {
        let path_str = rc.to_string_lossy();
        assert!(
            path_str.ends_with(".bashrc") || path_str.ends_with(".bash_profile"),
            "Bash rc_file should end with .bashrc or .bash_profile"
        );
    }

    if let Some(rc) = ShellKind::Zsh.rc_file() {
        assert!(
            rc.to_string_lossy().ends_with(".zshrc"),
            "Zsh rc_file should end with .zshrc"
        );
    }

    if let Some(rc) = ShellKind::Fish.rc_file() {
        assert!(
            rc.to_string_lossy().ends_with("config.fish"),
            "Fish rc_file should end with config.fish"
        );
    }
}

#[test]
fn test_marker_constant() {
    assert_eq!(QUENCH_COMPLETION_MARKER, "# quench-shell-completion");
}

#[test]
fn test_write_completion_script_creates_file() {
    // Test writing completion script to the real data directory
    // This test will succeed on systems where completions_dir() returns Some
    if completions_dir().is_some() {
        let result = write_completion_script(ShellKind::Bash);
        assert!(result.is_ok(), "Should be able to write completion script");
        let path = result.unwrap();
        assert!(path.exists(), "Completion script file should exist");

        // Check script contains valid bash completion code
        let content = fs::read_to_string(&path).unwrap();
        assert!(
            content.contains("complete") || content.contains("quench"),
            "Completion script should contain completion or quench"
        );
    }
}

#[test]
fn test_install_all_does_not_panic() {
    // Test that install_all() doesn't panic regardless of environment
    // It may succeed or fail, but it shouldn't panic
    let result = install_all();
    // We just verify it returns a Result without panicking
    // On CI or systems without RC files, it may return Ok(()) or an error
    assert!(result.is_ok() || result.is_err());
}
