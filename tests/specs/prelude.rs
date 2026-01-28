//! Test helpers for behavioral specifications.
//!
//! Provides high-level DSL for testing quench CLI behavior.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

pub use assert_cmd::prelude::*;
pub use predicates;
pub use predicates::prelude::{Predicate, PredicateBooleanExt};
use std::marker::PhantomData;
use std::path::Path;
use std::process::Command;

/// Trait for converting into a string predicate.
/// Allows passing `&str` (as contains) or any `Predicate<str>`.
pub trait IntoStrPredicate<P: Predicate<str>> {
    fn into_predicate(self) -> P;
}

impl IntoStrPredicate<predicates::str::ContainsPredicate> for &str {
    fn into_predicate(self) -> predicates::str::ContainsPredicate {
        predicates::str::contains(self)
    }
}

impl<P: Predicate<str>> IntoStrPredicate<P> for P {
    fn into_predicate(self) -> P {
        self
    }
}

/// Returns a Command configured to run the quench binary
pub fn quench_cmd() -> Command {
    Command::new(assert_cmd::cargo::cargo_bin!("quench"))
}

/// Create a check builder for the named check (runs only that check)
pub fn check(name: &str) -> CheckBuilder<Text, Single> {
    CheckBuilder::only(name)
}

/// Create a check builder that runs all checks
pub fn cli() -> CheckBuilder<Text, All> {
    CheckBuilder::all()
}

/// Create a report command builder
pub fn report() -> ReportBuilder<Text> {
    ReportBuilder::new()
}

/// Typestate markers for output mode
pub struct Text;
pub struct Json;
pub struct Html;
pub struct Markdown;

/// Typestate markers for check scope
pub struct Single(String);
pub struct All;

/// High-level check builder for fluent test assertions
pub struct CheckBuilder<Mode = Text, Scope = All> {
    scope: Scope,
    dir: Option<std::path::PathBuf>,
    args: Vec<String>,
    envs: Vec<(String, String)>,
    _mode: PhantomData<Mode>,
}

// Single check builder (Text mode)
#[allow(dead_code)]
impl CheckBuilder<Text, Single> {
    fn only(name: &str) -> Self {
        Self {
            scope: Single(name.to_string()),
            dir: None,
            args: Vec::new(),
            envs: Vec::new(),
            _mode: PhantomData,
        }
    }

    pub fn json(self) -> CheckBuilder<Json, Single> {
        CheckBuilder {
            scope: self.scope,
            dir: self.dir,
            args: self.args,
            envs: self.envs,
            _mode: PhantomData,
        }
    }

    pub fn passes(self) -> RunAssert {
        run_passes(self.command())
    }

    pub fn fails(self) -> RunAssert {
        run_fails(self.command())
    }

    pub fn exits(self, code: i32) -> RunAssert {
        run_exits(self.command(), code)
    }
}

// All checks builder (Text mode)
#[allow(dead_code)]
impl CheckBuilder<Text, All> {
    fn all() -> Self {
        Self {
            scope: All,
            dir: None,
            args: Vec::new(),
            envs: Vec::new(),
            _mode: PhantomData,
        }
    }

    pub fn json(self) -> CheckBuilder<Json, All> {
        CheckBuilder {
            scope: self.scope,
            dir: self.dir,
            args: self.args,
            envs: self.envs,
            _mode: PhantomData,
        }
    }

    pub fn passes(self) -> RunAssert {
        run_passes(self.command())
    }

    pub fn fails(self) -> RunAssert {
        run_fails(self.command())
    }

    pub fn exits(self, code: i32) -> RunAssert {
        run_exits(self.command(), code)
    }
}

// Single check builder (JSON mode) -> returns CheckJson
impl CheckBuilder<Json, Single> {
    pub fn passes(self) -> CheckJson {
        let name = self.scope.0.clone();
        let output = run_passes(self.command());
        CheckJson::new(&output.output.stdout, &name)
    }

    pub fn fails(self) -> CheckJson {
        let name = self.scope.0.clone();
        let output = run_fails(self.command());
        CheckJson::new(&output.output.stdout, &name)
    }
}

// All checks builder (JSON mode) -> returns ChecksJson
impl CheckBuilder<Json, All> {
    pub fn passes(self) -> ChecksJson {
        let output = run_passes(self.command());
        ChecksJson::new(&output.output.stdout)
    }

    pub fn fails(self) -> ChecksJson {
        let output = run_fails(self.command());
        ChecksJson::new(&output.output.stdout)
    }
}

// =============================================================================
// ReportBuilder
// =============================================================================

/// Report command builder for fluent test assertions
pub struct ReportBuilder<Mode = Text> {
    dir: Option<std::path::PathBuf>,
    args: Vec<String>,
    _mode: PhantomData<Mode>,
}

#[allow(dead_code)]
impl ReportBuilder<Text> {
    fn new() -> Self {
        Self {
            dir: None,
            args: Vec::new(),
            _mode: PhantomData,
        }
    }

    pub fn json(self) -> ReportBuilder<Json> {
        ReportBuilder {
            dir: self.dir,
            args: self.args,
            _mode: PhantomData,
        }
    }

    pub fn html(self) -> ReportBuilder<Html> {
        ReportBuilder {
            dir: self.dir,
            args: self.args,
            _mode: PhantomData,
        }
    }

    pub fn markdown(self) -> ReportBuilder<Markdown> {
        ReportBuilder {
            dir: self.dir,
            args: self.args,
            _mode: PhantomData,
        }
    }

    pub fn runs(self) -> RunAssert {
        run_passes(self.command())
    }
}

#[allow(dead_code)]
impl ReportBuilder<Json> {
    pub fn runs(self) -> RunAssert {
        run_passes(self.command())
    }
}

#[allow(dead_code)]
impl ReportBuilder<Html> {
    pub fn runs(self) -> RunAssert {
        run_passes(self.command())
    }
}

#[allow(dead_code)]
impl ReportBuilder<Markdown> {
    pub fn runs(self) -> RunAssert {
        run_passes(self.command())
    }
}

#[allow(dead_code)]
impl<Mode: 'static> ReportBuilder<Mode> {
    /// Set fixture directory by name
    pub fn on(mut self, fixture_name: &str) -> Self {
        self.dir = Some(fixture(fixture_name));
        self
    }

    /// Set working directory (alternative to fixture)
    pub fn pwd(mut self, path: impl Into<std::path::PathBuf>) -> Self {
        self.dir = Some(path.into());
        self
    }

    /// Add CLI arguments
    pub fn args(mut self, args: &[&str]) -> Self {
        self.args.extend(args.iter().map(|s| s.to_string()));
        self
    }

    /// Build the command
    fn command(self) -> Command {
        let is_json = std::any::TypeId::of::<Mode>() == std::any::TypeId::of::<Json>();
        let is_html = std::any::TypeId::of::<Mode>() == std::any::TypeId::of::<Html>();
        let is_markdown = std::any::TypeId::of::<Mode>() == std::any::TypeId::of::<Markdown>();

        let mut cmd = quench_cmd();
        cmd.arg("report");

        if is_json {
            cmd.args(["-o", "json"]);
        } else if is_html {
            cmd.args(["-o", "html"]);
        } else if is_markdown {
            cmd.args(["-o", "markdown"]);
        }

        cmd.args(&self.args);

        if let Some(dir) = self.dir {
            cmd.current_dir(dir);
        }

        cmd
    }
}

fn run_passes(mut cmd: Command) -> RunAssert {
    let output = cmd.output().expect("command should run");
    assert!(
        output.status.success(),
        "expected check to pass, got exit code {:?}\nstdout: {}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    RunAssert { output }
}

fn run_fails(mut cmd: Command) -> RunAssert {
    let output = cmd.output().expect("command should run");
    assert!(
        !output.status.success(),
        "expected check to fail, but it passed\nstdout: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    RunAssert { output }
}

fn run_exits(mut cmd: Command, code: i32) -> RunAssert {
    let output = cmd.output().expect("command should run");
    assert_eq!(
        output.status.code(),
        Some(code),
        "expected exit code {}, got {:?}\nstdout: {}\nstderr: {}",
        code,
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    RunAssert { output }
}

/// Single check JSON output
pub struct CheckJson {
    root: serde_json::Value,
    name: String,
}

#[allow(dead_code)]
impl CheckJson {
    fn new(stdout: &[u8], name: &str) -> Self {
        let root: serde_json::Value = serde_json::from_slice(stdout).expect("valid JSON");
        Self {
            root,
            name: name.to_string(),
        }
    }

    /// Get the root JSON value
    pub fn value(&self) -> &serde_json::Value {
        &self.root
    }

    /// Get the raw JSON as a pretty-printed string (for Exact output tests)
    pub fn raw_json(&self) -> String {
        serde_json::to_string_pretty(&self.root).expect("valid JSON")
    }

    /// Get the check object
    pub fn check(&self) -> &serde_json::Value {
        self.root
            .get("checks")
            .and_then(|v| v.as_array())
            .unwrap()
            .iter()
            .find(|c| c.get("name").and_then(|n| n.as_str()) == Some(&self.name))
            .unwrap_or_else(|| panic!("check '{}' not found", self.name))
    }

    /// Get field from the check, returns None if missing
    pub fn get(&self, key: &str) -> Option<&serde_json::Value> {
        self.check().get(key)
    }

    /// Get field from the check, panics if missing
    pub fn require(&self, key: &str) -> &serde_json::Value {
        self.check()
            .get(key)
            .unwrap_or_else(|| panic!("expected '{}' in check JSON", key))
    }

    // =========================================================================
    // Violation Helpers
    // =========================================================================

    /// Get all violations as a slice
    pub fn violations(&self) -> &[serde_json::Value] {
        self.get("violations")
            .and_then(|v| v.as_array())
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Returns true if any violation has this type
    pub fn has_violation(&self, vtype: &str) -> bool {
        self.violations()
            .iter()
            .any(|v| v.get("type").and_then(|t| t.as_str()) == Some(vtype))
    }

    /// Panics if no violation of this type exists, returns it otherwise
    pub fn require_violation(&self, vtype: &str) -> &serde_json::Value {
        self.violations()
            .iter()
            .find(|v| v.get("type").and_then(|t| t.as_str()) == Some(vtype))
            .unwrap_or_else(|| panic!("expected violation of type '{}'", vtype))
    }

    /// Get all violations of a specific type
    pub fn violations_of_type(&self, vtype: &str) -> Vec<&serde_json::Value> {
        self.violations()
            .iter()
            .filter(|v| v.get("type").and_then(|t| t.as_str()) == Some(vtype))
            .collect()
    }

    /// Returns true if any violation references this file (by suffix match)
    pub fn has_violation_for_file(&self, file_suffix: &str) -> bool {
        self.violations().iter().any(|v| {
            v.get("file")
                .and_then(|f| f.as_str())
                .map(|f| f.ends_with(file_suffix))
                .unwrap_or(false)
        })
    }

    /// Panics if no violation references this file, returns it otherwise
    pub fn require_violation_for_file(&self, file_suffix: &str) -> &serde_json::Value {
        self.violations()
            .iter()
            .find(|v| {
                v.get("file")
                    .and_then(|f| f.as_str())
                    .map(|f| f.ends_with(file_suffix))
                    .unwrap_or(false)
            })
            .unwrap_or_else(|| panic!("expected violation for file ending with '{}'", file_suffix))
    }
}

/// All checks JSON output
pub struct ChecksJson {
    root: serde_json::Value,
}

#[allow(dead_code)]
impl ChecksJson {
    fn new(stdout: &[u8]) -> Self {
        let root: serde_json::Value = serde_json::from_slice(stdout).expect("valid JSON");
        Self { root }
    }

    /// Get the root JSON value
    pub fn value(&self) -> &serde_json::Value {
        &self.root
    }

    /// Get all checks as array
    pub fn checks(&self) -> &Vec<serde_json::Value> {
        self.root.get("checks").and_then(|v| v.as_array()).unwrap()
    }
}

/// Trait for getting check name from scope
pub trait ScopeName {
    fn check_name(&self) -> Option<&str>;
}

impl ScopeName for Single {
    fn check_name(&self) -> Option<&str> {
        Some(&self.0)
    }
}

impl ScopeName for All {
    fn check_name(&self) -> Option<&str> {
        None
    }
}

#[allow(dead_code)]
impl<Mode: 'static, Scope: ScopeName> CheckBuilder<Mode, Scope> {
    /// Set fixture directory by name
    pub fn on(mut self, fixture_name: &str) -> Self {
        self.dir = Some(fixture(fixture_name));
        self
    }

    /// Set working directory (alternative to fixture)
    pub fn pwd(mut self, path: impl Into<std::path::PathBuf>) -> Self {
        self.dir = Some(path.into());
        self
    }

    /// Add CLI arguments
    pub fn args(mut self, args: &[&str]) -> Self {
        self.args.extend(args.iter().map(|s| s.to_string()));
        self
    }

    /// Set environment variable
    pub fn env(mut self, key: &str, value: &str) -> Self {
        self.envs.push((key.to_string(), value.to_string()));
        self
    }

    /// Build the command without running it
    fn command(self) -> Command {
        let is_json = std::any::TypeId::of::<Mode>() == std::any::TypeId::of::<Json>();
        let mut cmd = quench_cmd();
        cmd.arg("check");

        if let Some(name) = self.scope.check_name() {
            cmd.arg(format!("--{}", name));
        }

        cmd.arg("--no-cache");

        if is_json {
            cmd.args(["-o", "json"]);
        }

        cmd.args(&self.args);

        if let Some(dir) = self.dir {
            cmd.current_dir(dir);
        }

        for (key, value) in &self.envs {
            cmd.env(key, value);
        }

        cmd
    }
}

/// Result of a check run for chaining assertions
pub struct RunAssert {
    output: std::process::Output,
}

#[allow(dead_code)]
impl RunAssert {
    /// Get stdout as string
    pub fn stdout(&self) -> String {
        String::from_utf8_lossy(&self.output.stdout).into_owned()
    }

    /// Get stderr as string
    pub fn stderr(&self) -> String {
        String::from_utf8_lossy(&self.output.stderr).into_owned()
    }

    /// Assert stdout equals expected (with diff on failure)
    pub fn stdout_eq(self, expected: &str) -> Self {
        let stdout = String::from_utf8_lossy(&self.output.stdout);
        similar_asserts::assert_eq!(stdout, expected);
        self
    }

    /// Assert stderr equals expected (with diff on failure)
    pub fn stderr_eq(self, expected: &str) -> Self {
        let stderr = String::from_utf8_lossy(&self.output.stderr);
        similar_asserts::assert_eq!(stderr, expected);
        self
    }

    /// Assert stdout matches predicate.
    /// Prefer `stdout_eq` for strict matching.
    ///
    /// ```ignore
    /// .stdout_has("FAIL")  // contains
    /// .stdout_has(predicates::str::is_match(r"^\d+ checks").unwrap())
    /// ```
    pub fn stdout_has<I, P>(self, predicate: I) -> Self
    where
        I: IntoStrPredicate<P>,
        P: Predicate<str>,
    {
        let stdout = String::from_utf8_lossy(&self.output.stdout);
        assert!(
            predicate.into_predicate().eval(&stdout),
            "stdout predicate failed:\n{}",
            stdout
        );
        self
    }

    /// Assert stdout does not match predicate.
    /// Prefer `stdout_eq` for strict matching.
    ///
    /// ```ignore
    /// .stdout_lacks("\x1b[")  // doesn't contain
    /// ```
    pub fn stdout_lacks<I, P>(self, predicate: I) -> Self
    where
        I: IntoStrPredicate<P>,
        P: Predicate<str>,
    {
        let stdout = String::from_utf8_lossy(&self.output.stdout);
        assert!(
            !predicate.into_predicate().eval(&stdout),
            "stdout should NOT match predicate:\n{}",
            stdout
        );
        self
    }

    /// Assert stderr matches predicate.
    /// Prefer `stderr_eq` for strict matching.
    ///
    /// ```ignore
    /// .stderr_has("error")  // contains
    /// .stderr_has(predicates::str::is_empty().not())
    /// ```
    pub fn stderr_has<I, P>(self, predicate: I) -> Self
    where
        I: IntoStrPredicate<P>,
        P: Predicate<str>,
    {
        let stderr = String::from_utf8_lossy(&self.output.stderr);
        assert!(
            predicate.into_predicate().eval(&stderr),
            "stderr predicate failed:\n{}",
            stderr
        );
        self
    }

    /// Assert stderr does not match predicate.
    /// Prefer `stderr_eq` for strict matching.
    pub fn stderr_lacks<I, P>(self, predicate: I) -> Self
    where
        I: IntoStrPredicate<P>,
        P: Predicate<str>,
    {
        let stderr = String::from_utf8_lossy(&self.output.stderr);
        assert!(
            !predicate.into_predicate().eval(&stderr),
            "stderr should NOT match predicate:\n{}",
            stderr
        );
        self
    }
}

/// Get path to a test fixture directory
pub fn fixture(name: &str) -> std::path::PathBuf {
    let manifest_dir =
        std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR should be set");
    std::path::PathBuf::from(manifest_dir)
        .parent()
        .expect("parent should exist")
        .parent()
        .expect("grandparent should exist")
        .join("tests")
        .join("fixtures")
        .join(name)
}

/// Creates a temp directory with quench.toml and minimal CLAUDE.md
pub fn default_project() -> Project {
    Project::with_defaults()
}

// =============================================================================
// Project
// =============================================================================

/// Temporary test project directory with helper methods.
///
/// Reduces boilerplate by:
/// - Auto-creating parent directories
/// - Adding `version = 1` prefix to config
/// - Panicking on errors (we're in tests)
///
/// # Examples
///
/// ```ignore
/// // Project with defaults
/// let temp = default_project();
/// temp.config("[check.cloc]\nmax_lines = 5");
/// temp.file("src/lib.rs", "fn main() {}");
/// check("cloc").pwd(temp.path()).fails();
///
/// // Empty project (for init tests)
/// let temp = Project::empty();
/// quench_cmd().args(["init"]).current_dir(temp.path());
/// ```
pub struct Project {
    dir: tempfile::TempDir,
}

impl Project {
    /// Create an empty project with no files
    pub fn empty() -> Self {
        Self {
            dir: tempfile::tempdir().unwrap(),
        }
    }

    /// Create a minimal Cargo project with tests check configured.
    ///
    /// Creates:
    /// - `Cargo.toml` with package name and edition
    /// - `quench.toml` with cargo test suite
    /// - `src/lib.rs` with a simple function
    /// - `tests/basic.rs` with one passing test
    ///
    /// # Example
    /// ```ignore
    /// let temp = Project::cargo("my_project");
    /// check("tests").pwd(temp.path()).passes();
    /// ```
    pub fn cargo(name: &str) -> Self {
        let temp = Self::empty();
        temp.config(
            r#"
[[check.tests.suite]]
runner = "cargo"
"#,
        );
        temp.file(
            "Cargo.toml",
            &format!(
                r#"
[package]
name = "{name}"
version = "0.1.0"
edition = "2021"
"#
            ),
        );
        temp.file("src/lib.rs", "pub fn add(a: i32, b: i32) -> i32 { a + b }");
        temp.file(
            "tests/basic.rs",
            &format!(
                r#"
#[test]
fn test_add() {{ assert_eq!({name}::add(1, 2), 3); }}
"#
            ),
        );
        temp
    }

    /// Create a project with default quench.toml and CLAUDE.md
    pub fn with_defaults() -> Self {
        let temp = Self::empty();
        temp.file("quench.toml", "version = 1\n");
        temp.file(
            "CLAUDE.md",
            "# Project\n\n## Directory Structure\n\nMinimal.\n\n## Landing the Plane\n\n- Done\n",
        );
        temp
    }

    /// Get the project path
    pub fn path(&self) -> &Path {
        self.dir.path()
    }

    /// Write quench.toml (auto-prefixes with `version = 1` if not present)
    pub fn config(&self, content: &str) {
        let content = if content.contains("version") {
            content.to_string()
        } else {
            format!("version = 1\n{}", content)
        };
        std::fs::write(self.dir.path().join("quench.toml"), content).unwrap();
    }

    /// Write a file at the given path (parent directories created automatically)
    pub fn file(&self, path: impl AsRef<Path>, content: &str) {
        let full_path = self.dir.path().join(path.as_ref());
        if let Some(parent) = full_path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(full_path, content).unwrap();
    }
}

/// Extract check names from JSON output
pub fn check_names(json: &serde_json::Value) -> Vec<&str> {
    json.get("checks")
        .and_then(|v| v.as_array())
        .unwrap()
        .iter()
        .filter_map(|c| c.get("name").and_then(|n| n.as_str()))
        .collect()
}

/// Minimal config that disables agents check (for tests not focused on agents)
pub const MINIMAL_CONFIG: &str = r#"[check.agents]
required = []
"#;

// =============================================================================
// GIT TEST HELPERS
// =============================================================================

/// Initialize a git repo with minimal config
pub fn git_init(project: &Project) {
    std::process::Command::new("git")
        .args(["init", "-b", "main"])
        .current_dir(project.path())
        .output()
        .expect("git init should succeed");

    std::process::Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(project.path())
        .output()
        .expect("git config email should succeed");

    std::process::Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(project.path())
        .output()
        .expect("git config name should succeed");
}

/// Create main branch with initial commit (requires files to exist)
pub fn git_initial_commit(project: &Project) {
    std::process::Command::new("git")
        .args(["add", "."])
        .current_dir(project.path())
        .output()
        .expect("git add should succeed");

    std::process::Command::new("git")
        .args(["commit", "-m", "feat: initial commit"])
        .current_dir(project.path())
        .output()
        .expect("git commit should succeed");
}

/// Create a feature branch
pub fn git_branch(project: &Project, name: &str) {
    std::process::Command::new("git")
        .args(["checkout", "-b", name])
        .current_dir(project.path())
        .output()
        .expect("git checkout -b should succeed");
}

/// Add a commit with the given message
pub fn git_commit(project: &Project, message: &str) {
    // Touch a file to make a change
    let id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("time should work")
        .as_nanos();
    let dummy_file = project.path().join(format!("dummy_{}.txt", id));
    std::fs::write(&dummy_file, "dummy").expect("write should succeed");

    std::process::Command::new("git")
        .args(["add", "."])
        .current_dir(project.path())
        .output()
        .expect("git add should succeed");

    std::process::Command::new("git")
        .args(["commit", "-m", message])
        .current_dir(project.path())
        .output()
        .expect("git commit should succeed");
}

/// Checkout an existing branch
pub fn git_checkout(project: &Project, branch: &str) {
    std::process::Command::new("git")
        .args(["checkout", branch])
        .current_dir(project.path())
        .output()
        .expect("git checkout should succeed");
}

/// Stage all changes
pub fn git_add_all(project: &Project) {
    std::process::Command::new("git")
        .args(["add", "-A"])
        .current_dir(project.path())
        .output()
        .expect("git add should succeed");
}

/// Add a git note to HEAD with the given content
pub fn git_add_note(project: &Project, content: &str) {
    std::process::Command::new("git")
        .args(["notes", "--ref=quench", "add", "-m", content])
        .current_dir(project.path())
        .output()
        .expect("git notes add should succeed");
}

/// Read the git note from HEAD
#[allow(dead_code)]
pub fn git_read_note(project: &Project) -> Option<String> {
    let output = std::process::Command::new("git")
        .args(["notes", "--ref=quench", "show"])
        .current_dir(project.path())
        .output()
        .expect("git notes show should succeed");

    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        None
    }
}
