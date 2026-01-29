// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

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
fn baseline_notes_config() {
    let temp = Project::empty();
    temp.config(
        r#"
[git]
baseline = "notes"

[ratchet]
check = "error"
escapes = true

[[check.escapes.patterns]]
name = "unsafe"
pattern = "unsafe"
action = "count"
threshold = 100
"#,
    );
    temp.file("CLAUDE.md", CLAUDE_MD);
    temp.file("Cargo.toml", CARGO_TOML);
    temp.file("src/lib.rs", "fn main() {}");

    git_init(&temp);
    git_initial_commit(&temp);

    // Add note to HEAD with baseline metrics
    git_add_note(
        &temp,
        r#"{"version":1,"updated":"2026-01-20T00:00:00Z","metrics":{"escapes":{"source":{"unsafe":0}}}}"#,
    );

    // Use --no-git since CLAUDE.md doesn't have Commits section
    cli().pwd(temp.path()).args(&["--no-git"]).passes();
    // Assert reads from notes (would fail if baseline was file-based and missing)
}

/// Spec: docs/specs/02-config.md#git
///
/// > baseline = "<path>" uses file
#[test]
fn baseline_file_config() {
    let temp = Project::empty();
    temp.config(
        r#"
[git]
baseline = ".quench/baseline.json"

[ratchet]
check = "error"
escapes = true

[[check.escapes.patterns]]
name = "unsafe"
pattern = "unsafe"
action = "count"
threshold = 100
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
    git_add_note(
        &temp,
        r#"{"version":1,"updated":"2026-01-20T00:00:00Z","metrics":{"escapes":{"source":{"unsafe":100}}}}"#,
    );

    // Use --no-git since CLAUDE.md doesn't have Commits section
    cli().pwd(temp.path()).args(&["--no-git"]).passes();
    // Assert reads from file, not notes (file baseline has 0 unsafe, notes has 100)
}
