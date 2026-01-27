//! Tests for git configuration.
//!
//! Reference: docs/specs/02-config.md#git

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::prelude::*;

const CLAUDE_MD: &str =
    "# Project\n\n## Directory Structure\n\nLayout.\n\n## Landing the Plane\n\n- Done\n";
const CARGO_TOML: &str = "[package]\nname = \"test\"\nversion = \"0.1.0\"\nedition = \"2024\"\n";

/// Spec: docs/specs/02-config.md#git
///
/// > baseline = "notes" uses git notes (default)
#[test]
#[ignore = "TODO: Phase 2 - Git notes as default baseline"]
fn baseline_notes_config() {
    let temp = Project::empty();
    temp.config(
        r#"
[git]
baseline = "notes"
"#,
    );
    temp.file("CLAUDE.md", CLAUDE_MD);
    temp.file("Cargo.toml", CARGO_TOML);
    temp.file("src/lib.rs", "fn main() {}");

    git_init(&temp);
    git_initial_commit(&temp);

    // Add note to HEAD with baseline metrics
    let baseline = r#"{"version":1,"metrics":{"escapes":{"source":{"unsafe":0}}}}"#;
    std::process::Command::new("git")
        .args(["notes", "--ref=quench", "add", "-m", baseline])
        .current_dir(temp.path())
        .output()
        .expect("git notes add should succeed");

    cli().pwd(temp.path()).passes();
    // Assert reads from notes (would fail if baseline was file-based and missing)
}

/// Spec: docs/specs/02-config.md#git
///
/// > baseline = "<path>" uses file
#[test]
#[ignore = "TODO: Phase 3 - File-based baseline config"]
fn baseline_file_config() {
    let temp = Project::empty();
    temp.config(
        r#"
[git]
baseline = ".quench/baseline.json"
"#,
    );
    temp.file("CLAUDE.md", CLAUDE_MD);
    temp.file("Cargo.toml", CARGO_TOML);
    temp.file("src/lib.rs", "fn main() {}");

    // Write baseline file
    std::fs::create_dir_all(temp.path().join(".quench")).unwrap();
    std::fs::write(
        temp.path().join(".quench/baseline.json"),
        r#"{
  "version": 1,
  "updated": "2026-01-20T00:00:00Z",
  "metrics": {
    "escapes": {
      "source": { "unsafe": 0 }
    }
  }
}"#,
    )
    .unwrap();

    git_init(&temp);
    git_initial_commit(&temp);

    // Also add a note with different values to verify file takes precedence
    let note_baseline = r#"{"version":1,"metrics":{"escapes":{"source":{"unsafe":100}}}}"#;
    std::process::Command::new("git")
        .args(["notes", "--ref=quench", "add", "-m", note_baseline])
        .current_dir(temp.path())
        .output()
        .expect("git notes add should succeed");

    cli().pwd(temp.path()).passes();
    // Assert reads from file, not notes (file baseline has 0 unsafe, notes has 100)
}
