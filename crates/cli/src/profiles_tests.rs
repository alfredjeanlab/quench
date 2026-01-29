// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use tempfile::TempDir;

fn setup_dir() -> TempDir {
    TempDir::new().unwrap()
}

// =============================================================================
// JAVASCRIPT LANDING ITEMS TESTS
// =============================================================================

#[test]
fn javascript_landing_items_returns_npm_commands() {
    let items = javascript_landing_items();
    assert_eq!(items.len(), 4);
    assert_eq!(items[0], "npm run lint");
    assert_eq!(items[1], "npm run typecheck");
    assert_eq!(items[2], "npm test");
    assert_eq!(items[3], "npm run build");
}

#[test]
fn javascript_landing_items_for_detects_npm() {
    let dir = setup_dir();
    std::fs::write(dir.path().join("package-lock.json"), "{}").unwrap();

    let items = javascript_landing_items_for(dir.path());
    assert_eq!(items.len(), 4);
    assert_eq!(items[0], "npm run lint");
    assert_eq!(items[1], "npm run typecheck");
    assert_eq!(items[2], "npm test");
    assert_eq!(items[3], "npm run build");
}

#[test]
fn javascript_landing_items_for_detects_yarn() {
    let dir = setup_dir();
    std::fs::write(dir.path().join("yarn.lock"), "").unwrap();

    let items = javascript_landing_items_for(dir.path());
    assert_eq!(items.len(), 4);
    // Yarn uses `yarn <script>` without "run"
    assert_eq!(items[0], "yarn lint");
    assert_eq!(items[1], "yarn typecheck");
    assert_eq!(items[2], "yarn test");
    assert_eq!(items[3], "yarn build");
}

#[test]
fn javascript_landing_items_for_detects_pnpm() {
    let dir = setup_dir();
    std::fs::write(dir.path().join("pnpm-lock.yaml"), "").unwrap();

    let items = javascript_landing_items_for(dir.path());
    assert_eq!(items.len(), 4);
    assert_eq!(items[0], "pnpm run lint");
    assert_eq!(items[1], "pnpm run typecheck");
    assert_eq!(items[2], "pnpm test");
    assert_eq!(items[3], "pnpm run build");
}

#[test]
fn javascript_landing_items_for_detects_bun() {
    let dir = setup_dir();
    std::fs::write(dir.path().join("bun.lock"), "").unwrap();

    let items = javascript_landing_items_for(dir.path());
    assert_eq!(items.len(), 4);
    assert_eq!(items[0], "bun run lint");
    assert_eq!(items[1], "bun run typecheck");
    assert_eq!(items[2], "bun test");
    assert_eq!(items[3], "bun run build");
}

#[test]
fn javascript_landing_items_for_defaults_to_npm() {
    let dir = setup_dir();
    // No lock file - defaults to npm

    let items = javascript_landing_items_for(dir.path());
    assert_eq!(items[0], "npm run lint");
    assert_eq!(items[2], "npm test");
}

// =============================================================================
// PROFILE REGISTRY TESTS
// =============================================================================

#[test]
fn profile_registry_includes_javascript() {
    let available = ProfileRegistry::available();
    assert!(available.contains(&"javascript"));
}

#[test]
fn profile_registry_get_javascript() {
    let profile = ProfileRegistry::get("javascript");
    assert!(profile.is_some());

    let profile = profile.unwrap();
    assert!(profile.contains("[javascript]"));
    assert!(profile.contains("source = "));
}

#[test]
fn profile_registry_aliases_work() {
    assert!(ProfileRegistry::get("js").is_some());
    assert!(ProfileRegistry::get("typescript").is_some());
    assert!(ProfileRegistry::get("ts").is_some());
}

// =============================================================================
// PYTHON PROFILE TESTS
// =============================================================================

#[test]
fn profile_registry_includes_python() {
    let available = ProfileRegistry::available();
    assert!(available.contains(&"python"));
}

#[test]
fn profile_registry_get_python() {
    let profile = ProfileRegistry::get("python");
    assert!(profile.is_some());

    let profile = profile.unwrap();
    assert!(profile.contains("[python]"));
    assert!(profile.contains("[python.suppress]"));
    assert!(profile.contains("[python.policy]"));
}

#[test]
fn profile_registry_python_aliases_work() {
    assert!(ProfileRegistry::get("py").is_some());
}

#[test]
fn python_profile_has_escape_patterns() {
    let profile = python_profile_defaults();
    assert!(profile.contains("eval"));
    assert!(profile.contains("exec"));
    assert!(profile.contains("breakpoint"));
    assert!(profile.contains("# EVAL:"));
}

// =============================================================================
// PYTHON LANDING ITEMS TESTS
// =============================================================================

#[test]
fn python_landing_items_returns_default_commands() {
    let items = python_landing_items();
    assert_eq!(items.len(), 4);
    assert_eq!(items[0], "ruff check .");
    assert_eq!(items[1], "ruff format --check .");
    assert_eq!(items[2], "mypy .");
    assert_eq!(items[3], "pytest");
}

#[test]
fn python_landing_items_for_with_ruff_project() {
    let dir = setup_dir();
    std::fs::write(dir.path().join("ruff.toml"), "[lint]\nselect = [\"E\"]").unwrap();
    std::fs::create_dir(dir.path().join("tests")).unwrap();

    let items = python_landing_items_for(dir.path());
    assert!(items.iter().any(|i| i.contains("ruff check")));
    assert!(items.iter().any(|i| i.contains("ruff format")));
    assert!(items.iter().any(|i| i.contains("pytest")));
}

#[test]
fn python_landing_items_for_with_poetry() {
    let dir = setup_dir();
    std::fs::write(dir.path().join("poetry.lock"), "").unwrap();
    std::fs::write(dir.path().join("ruff.toml"), "").unwrap();
    std::fs::create_dir(dir.path().join("tests")).unwrap();

    let items = python_landing_items_for(dir.path());
    // Poetry projects should use `poetry run` prefix
    assert!(items.iter().any(|i| i.starts_with("poetry run")));
}

#[test]
fn python_landing_items_for_with_uv() {
    let dir = setup_dir();
    std::fs::write(dir.path().join("uv.lock"), "").unwrap();
    std::fs::write(dir.path().join("ruff.toml"), "").unwrap();
    std::fs::create_dir(dir.path().join("tests")).unwrap();

    let items = python_landing_items_for(dir.path());
    // uv projects should use `uv run` prefix
    assert!(items.iter().any(|i| i.starts_with("uv run")));
}

#[test]
fn python_landing_items_for_with_pip() {
    let dir = setup_dir();
    std::fs::write(dir.path().join("ruff.toml"), "").unwrap();
    std::fs::create_dir(dir.path().join("tests")).unwrap();

    let items = python_landing_items_for(dir.path());
    // pip projects should NOT have a run prefix
    assert!(items.iter().any(|i| i == "ruff check ."));
    assert!(!items.iter().any(|i| i.starts_with("pip run")));
}

#[test]
fn python_landing_items_for_prefers_ruff_over_flake8() {
    let dir = setup_dir();
    std::fs::write(dir.path().join("ruff.toml"), "").unwrap();
    std::fs::write(dir.path().join(".flake8"), "[flake8]").unwrap();

    let items = python_landing_items_for(dir.path());
    // Should have ruff, not flake8
    assert!(items.iter().any(|i| i.contains("ruff check")));
    assert!(!items.iter().any(|i| i.contains("flake8")));
}

#[test]
fn python_landing_items_for_uses_flake8_when_no_ruff() {
    let dir = setup_dir();
    std::fs::write(dir.path().join(".flake8"), "[flake8]").unwrap();

    let items = python_landing_items_for(dir.path());
    assert!(items.iter().any(|i| i.contains("flake8")));
}

#[test]
fn python_landing_items_for_uses_black_when_no_ruff() {
    let dir = setup_dir();
    std::fs::write(
        dir.path().join("pyproject.toml"),
        "[tool.black]\nline-length = 88\n",
    )
    .unwrap();

    let items = python_landing_items_for(dir.path());
    assert!(items.iter().any(|i| i.contains("black --check")));
}

#[test]
fn python_landing_items_for_includes_build_when_configured() {
    let dir = setup_dir();
    std::fs::write(
        dir.path().join("pyproject.toml"),
        "[build-system]\nrequires = [\"setuptools\"]\n",
    )
    .unwrap();

    let items = python_landing_items_for(dir.path());
    assert!(items.iter().any(|i| i.contains("python -m build")));
}

#[test]
fn python_landing_items_for_omits_build_when_not_configured() {
    let dir = setup_dir();
    std::fs::write(
        dir.path().join("pyproject.toml"),
        "[project]\nname = \"test\"\n",
    )
    .unwrap();

    let items = python_landing_items_for(dir.path());
    assert!(!items.iter().any(|i| i.contains("python -m build")));
}

#[test]
fn python_landing_items_for_with_pipenv() {
    let dir = setup_dir();
    std::fs::write(dir.path().join("Pipfile"), "[packages]").unwrap();
    std::fs::write(dir.path().join("ruff.toml"), "").unwrap();
    std::fs::create_dir(dir.path().join("tests")).unwrap();

    let items = python_landing_items_for(dir.path());
    // Pipenv projects should use `pipenv run` prefix
    assert!(items.iter().any(|i| i.starts_with("pipenv run")));
}

#[test]
fn python_landing_items_for_returns_defaults_when_nothing_detected() {
    let dir = setup_dir();
    // Empty directory - nothing configured

    let items = python_landing_items_for(dir.path());
    // Should return sensible defaults
    assert!(items.iter().any(|i| i.contains("ruff check")));
    assert!(items.iter().any(|i| i.contains("ruff format")));
    assert!(items.iter().any(|i| i.contains("pytest")));
}
