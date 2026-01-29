// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! `quench cloc` command implementation.
//!
//! Walks project files and produces a cloc-like report split by language
//! and source vs test classification.

use std::collections::HashMap;

use quench::adapter::{
    AdapterRegistry, FileKind, ProjectLanguage, RustAdapter, detect_language,
    patterns::LanguageDefaults, python::detect_package as detect_python_package,
    rust::CargoWorkspace,
};
use quench::cli::{ClocArgs, OutputFormat};
use quench::cloc;
use quench::config::{self, CfgTestSplitMode, RustConfig};
use quench::discovery;
use quench::error::ExitCode;
use quench::file_reader::FileContent;
use quench::walker::{FileWalker, WalkerConfig};

/// Accumulated statistics for a (language, kind) bucket.
#[derive(Default)]
struct LangStats {
    files: usize,
    blank: usize,
    comment: usize,
    code: usize,
}

/// Run the `quench cloc` command.
pub fn run(args: &ClocArgs) -> anyhow::Result<ExitCode> {
    let cwd = std::env::current_dir()?;

    let root = if args.paths.is_empty() {
        cwd.clone()
    } else {
        let path = &args.paths[0];
        if path.is_absolute() {
            path.clone()
        } else {
            cwd.join(path)
        }
    };

    // Load config
    let mut config = match discovery::find_config(&root) {
        Some(path) => config::load_with_warnings(&path)?,
        None => config::Config::default(),
    };

    // Build exclude patterns (same logic as cmd_check)
    let mut exclude_patterns = config.project.exclude.patterns.clone();

    match detect_language(&root) {
        ProjectLanguage::Rust => {
            if !exclude_patterns.iter().any(|p| p.contains("target")) {
                exclude_patterns.push("target".to_string());
            }
            if config.project.packages.is_empty() {
                let workspace = CargoWorkspace::from_root(&root);
                if workspace.is_workspace {
                    for pattern in &workspace.member_patterns {
                        if pattern.contains('*') {
                            if let Some(base) = pattern.strip_suffix("/*") {
                                let dir = root.join(base);
                                if let Ok(entries) = std::fs::read_dir(&dir) {
                                    for entry in entries.flatten() {
                                        if entry.path().is_dir() {
                                            let rel_path = format!(
                                                "{}/{}",
                                                base,
                                                entry.file_name().to_string_lossy()
                                            );
                                            config.project.packages.push(rel_path);
                                        }
                                    }
                                }
                            }
                        } else {
                            config.project.packages.push(pattern.clone());
                        }
                    }
                }
            }
        }
        ProjectLanguage::Go => {
            if !exclude_patterns.iter().any(|p| p.contains("vendor")) {
                exclude_patterns.push("vendor".to_string());
            }
        }
        ProjectLanguage::JavaScript => {
            for pattern in ["node_modules", "dist", "build", ".next", "coverage"] {
                if !exclude_patterns.iter().any(|p| p.contains(pattern)) {
                    exclude_patterns.push(pattern.to_string());
                }
            }
        }
        ProjectLanguage::Python => {
            for pattern in [
                ".venv",
                "venv",
                ".env",
                "env",
                "__pycache__",
                ".mypy_cache",
                ".pytest_cache",
                ".ruff_cache",
                "dist",
                "build",
                "*.egg-info",
                ".tox",
                ".nox",
            ] {
                if !exclude_patterns.iter().any(|p| p.contains(pattern)) {
                    exclude_patterns.push(pattern.to_string());
                }
            }
            if config.project.packages.is_empty()
                && let Some((pkg_path, _)) = detect_python_package(&root)
            {
                config.project.packages.push(pkg_path);
            }
        }
        ProjectLanguage::Ruby => {
            for pattern in ["vendor", "tmp", "log", "coverage"] {
                if !exclude_patterns.iter().any(|p| p.contains(pattern)) {
                    exclude_patterns.push(pattern.to_string());
                }
            }
        }
        ProjectLanguage::Shell | ProjectLanguage::Generic => {}
    }

    // Also add check.cloc.exclude patterns (parity with check command)
    let cloc_config = &config.check.cloc;
    for pattern in &cloc_config.exclude {
        if !exclude_patterns.contains(pattern) {
            exclude_patterns.push(pattern.clone());
        }
    }

    // Set up walker
    let walker_config = WalkerConfig {
        max_depth: Some(args.max_depth),
        exclude_patterns,
        ..Default::default()
    };
    let walker = FileWalker::new(walker_config);
    let (rx, handle) = walker.walk(&root);

    // Set up adapter registry for source/test classification
    let registry = AdapterRegistry::for_project_with_config(&root, &config);

    // Set up Rust cfg_test adapter if needed
    let rust_config = &config.rust;
    let rust_adapter = match rust_config.cfg_test_split {
        CfgTestSplitMode::Count => {
            use quench::adapter::ResolvedPatterns;
            let fallback_test = if !config.project.tests.is_empty() {
                config.project.tests.clone()
            } else {
                <RustConfig as LanguageDefaults>::default_tests()
            };
            let patterns = ResolvedPatterns {
                source: if !rust_config.source.is_empty() {
                    rust_config.source.clone()
                } else {
                    <RustConfig as LanguageDefaults>::default_source()
                },
                test: if !rust_config.tests.is_empty() {
                    rust_config.tests.clone()
                } else {
                    fallback_test
                },
                exclude: if !rust_config.exclude.is_empty() {
                    rust_config.exclude.clone()
                } else {
                    <RustConfig as LanguageDefaults>::default_exclude()
                },
            };
            Some(RustAdapter::with_patterns(patterns))
        }
        CfgTestSplitMode::Off | CfgTestSplitMode::Require => None,
    };

    // Accumulate stats: (language_name, FileKind) -> LangStats
    let mut stats: HashMap<(String, FileKind), LangStats> = HashMap::new();

    for file in rx {
        let ext = match file.path.extension().and_then(|e| e.to_str()) {
            Some(e) => e.to_lowercase(),
            None => continue,
        };

        if !cloc::is_text_extension(&ext) {
            continue;
        }

        let relative_path = file.path.strip_prefix(&root).unwrap_or(&file.path);
        let file_kind = registry.classify(relative_path);

        if file_kind == FileKind::Other {
            continue;
        }

        // Read file content
        let content = match FileContent::read(&file.path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let Some(text) = content.as_str() else {
            continue;
        };

        let lang = cloc::language_name(&ext).to_string();
        let metrics = cloc::count_file_metrics(text, &ext);

        // Handle Rust cfg_test splitting
        if let Some(adapter) = rust_adapter.as_ref()
            && ext == "rs"
            && file_kind == FileKind::Source
        {
            let classification = adapter.classify_lines(relative_path, text);

            // Split metrics proportionally between source and test
            let total_nonblank = metrics.nonblank.max(1);

            if classification.source_lines > 0 {
                let ratio = classification.source_lines as f64 / total_nonblank as f64;
                let entry = stats.entry((lang.clone(), FileKind::Source)).or_default();
                entry.files += 1;
                entry.blank += (metrics.blank as f64 * ratio).round() as usize;
                entry.comment += (metrics.comment as f64 * ratio).round() as usize;
                entry.code += (metrics.code as f64 * ratio).round() as usize;
            }
            if classification.test_lines > 0 {
                let ratio = classification.test_lines as f64 / total_nonblank as f64;
                let entry = stats.entry((lang, FileKind::Test)).or_default();
                entry.files += 1;
                entry.blank += (metrics.blank as f64 * ratio).round() as usize;
                entry.comment += (metrics.comment as f64 * ratio).round() as usize;
                entry.code += (metrics.code as f64 * ratio).round() as usize;
            }
        } else {
            let entry = stats.entry((lang, file_kind)).or_default();
            entry.files += 1;
            entry.blank += metrics.blank;
            entry.comment += metrics.comment;
            entry.code += metrics.code;
        }
    }

    // Wait for walker to finish
    let _walk_stats = handle.join();

    match args.output {
        OutputFormat::Json => print_json(&stats)?,
        _ => print_text(&stats),
    }

    Ok(ExitCode::Success)
}

/// Print the cloc report in text table format.
fn print_text(stats: &HashMap<(String, FileKind), LangStats>) {
    // Collect rows and sort: source first per language, then test; by code descending
    let mut rows: Vec<(&String, FileKind, &LangStats)> = stats
        .iter()
        .filter(|(_, s)| s.files > 0)
        .map(|((lang, kind), s)| (lang, *kind, s))
        .collect();

    rows.sort_by(|a, b| {
        // Primary: code descending
        b.2.code
            .cmp(&a.2.code)
            // Secondary: source before test for same language
            .then_with(|| kind_order(a.1).cmp(&kind_order(b.1)))
            // Tertiary: language name
            .then_with(|| a.0.cmp(b.0))
    });

    if rows.is_empty() {
        println!("No source files found.");
        return;
    }

    // Calculate column widths
    let separator = "\u{2500}".repeat(62);

    // Header
    println!("{}", separator);
    println!(
        "{:<25} {:>5}  {:>8}  {:>8}  {:>8}",
        "Language", "files", "blank", "comment", "code"
    );
    println!("{}", separator);

    // Data rows
    let mut source_totals = LangStats::default();
    let mut test_totals = LangStats::default();

    for (lang, kind, s) in &rows {
        let label = format!(
            "{} ({})",
            lang,
            match kind {
                FileKind::Source => "source",
                FileKind::Test => "tests",
                FileKind::Other => "other",
            }
        );
        println!(
            "{:<25} {:>5}  {:>8}  {:>8}  {:>8}",
            label, s.files, s.blank, s.comment, s.code
        );

        match kind {
            FileKind::Source => {
                source_totals.files += s.files;
                source_totals.blank += s.blank;
                source_totals.comment += s.comment;
                source_totals.code += s.code;
            }
            FileKind::Test => {
                test_totals.files += s.files;
                test_totals.blank += s.blank;
                test_totals.comment += s.comment;
                test_totals.code += s.code;
            }
            FileKind::Other => {}
        }
    }

    // Summary
    println!("{}", separator);
    println!(
        "{:<25} {:>5}  {:>8}  {:>8}  {:>8}",
        "Source total",
        source_totals.files,
        source_totals.blank,
        source_totals.comment,
        source_totals.code
    );
    println!(
        "{:<25} {:>5}  {:>8}  {:>8}  {:>8}",
        "Test total", test_totals.files, test_totals.blank, test_totals.comment, test_totals.code
    );
    println!("{}", separator);
    println!(
        "{:<25} {:>5}  {:>8}  {:>8}  {:>8}",
        "Total",
        source_totals.files + test_totals.files,
        source_totals.blank + test_totals.blank,
        source_totals.comment + test_totals.comment,
        source_totals.code + test_totals.code
    );
    println!("{}", separator);
}

/// Print the cloc report in JSON format.
fn print_json(stats: &HashMap<(String, FileKind), LangStats>) -> anyhow::Result<()> {
    let mut languages: Vec<serde_json::Value> = stats
        .iter()
        .filter(|(_, s)| s.files > 0)
        .map(|((lang, kind), s)| {
            serde_json::json!({
                "language": lang,
                "kind": match kind {
                    FileKind::Source => "source",
                    FileKind::Test => "test",
                    FileKind::Other => "other",
                },
                "files": s.files,
                "blank": s.blank,
                "comment": s.comment,
                "code": s.code,
            })
        })
        .collect();

    // Sort same as text output
    languages.sort_by(|a, b| {
        let a_code = a["code"].as_u64().unwrap_or(0);
        let b_code = b["code"].as_u64().unwrap_or(0);
        b_code
            .cmp(&a_code)
            .then_with(|| {
                let a_kind = a["kind"].as_str().unwrap_or("");
                let b_kind = b["kind"].as_str().unwrap_or("");
                a_kind.cmp(b_kind)
            })
            .then_with(|| {
                let a_lang = a["language"].as_str().unwrap_or("");
                let b_lang = b["language"].as_str().unwrap_or("");
                a_lang.cmp(b_lang)
            })
    });

    // Compute totals
    let mut source = LangStats::default();
    let mut test = LangStats::default();
    for ((_, kind), s) in stats.iter() {
        match kind {
            FileKind::Source => {
                source.files += s.files;
                source.blank += s.blank;
                source.comment += s.comment;
                source.code += s.code;
            }
            FileKind::Test => {
                test.files += s.files;
                test.blank += s.blank;
                test.comment += s.comment;
                test.code += s.code;
            }
            FileKind::Other => {}
        }
    }

    let output = serde_json::json!({
        "languages": languages,
        "totals": {
            "source": {
                "files": source.files,
                "blank": source.blank,
                "comment": source.comment,
                "code": source.code,
            },
            "test": {
                "files": test.files,
                "blank": test.blank,
                "comment": test.comment,
                "code": test.code,
            },
            "total": {
                "files": source.files + test.files,
                "blank": source.blank + test.blank,
                "comment": source.comment + test.comment,
                "code": source.code + test.code,
            },
        },
    });

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

/// Sort order for FileKind: Source < Test < Other.
fn kind_order(kind: FileKind) -> u8 {
    match kind {
        FileKind::Source => 0,
        FileKind::Test => 1,
        FileKind::Other => 2,
    }
}
