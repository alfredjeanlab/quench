// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! `quench cloc` command implementation.
//!
//! Walks project files and produces a cloc-like report split by language
//! and source vs test classification.

use std::collections::HashMap;
use std::path::Path;

use quench::adapter::project::apply_language_defaults;
use quench::adapter::{AdapterRegistry, FileKind, RustAdapter, patterns::LanguageDefaults};
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

/// Accumulated statistics for a single package.
#[derive(Default)]
struct PackageStats {
    source: LangStats,
    test: LangStats,
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

    // Apply language defaults (excludes + package auto-detection)
    let mut exclude_patterns = apply_language_defaults(&root, &mut config);

    // Also add check.cloc.exclude patterns (parity with check command)
    for pattern in &config.check.cloc.exclude {
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
    let mut package_stats: HashMap<String, PackageStats> = HashMap::new();
    let packages = &config.project.packages;

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

            let source_blank;
            let source_comment;
            let source_code;
            let test_blank;
            let test_comment;
            let test_code;

            if classification.source_lines > 0 {
                let ratio = classification.source_lines as f64 / total_nonblank as f64;
                source_blank = (metrics.blank as f64 * ratio).round() as usize;
                source_comment = (metrics.comment as f64 * ratio).round() as usize;
                source_code = (metrics.code as f64 * ratio).round() as usize;
                let entry = stats.entry((lang.clone(), FileKind::Source)).or_default();
                entry.files += 1;
                entry.blank += source_blank;
                entry.comment += source_comment;
                entry.code += source_code;
            } else {
                source_blank = 0;
                source_comment = 0;
                source_code = 0;
            }
            if classification.test_lines > 0 {
                let ratio = classification.test_lines as f64 / total_nonblank as f64;
                test_blank = (metrics.blank as f64 * ratio).round() as usize;
                test_comment = (metrics.comment as f64 * ratio).round() as usize;
                test_code = (metrics.code as f64 * ratio).round() as usize;
                let entry = stats.entry((lang, FileKind::Test)).or_default();
                entry.files += 1;
                entry.blank += test_blank;
                entry.comment += test_comment;
                entry.code += test_code;
            } else {
                test_blank = 0;
                test_comment = 0;
                test_code = 0;
            }

            // Per-package tracking for cfg_test split files
            if !packages.is_empty()
                && let Some(pkg) = file_package(relative_path, packages)
            {
                let pkg_entry = package_stats.entry(pkg).or_default();
                if classification.source_lines > 0 {
                    pkg_entry.source.files += 1;
                    pkg_entry.source.blank += source_blank;
                    pkg_entry.source.comment += source_comment;
                    pkg_entry.source.code += source_code;
                }
                if classification.test_lines > 0 {
                    pkg_entry.test.files += 1;
                    pkg_entry.test.blank += test_blank;
                    pkg_entry.test.comment += test_comment;
                    pkg_entry.test.code += test_code;
                }
            }
        } else {
            let entry = stats.entry((lang, file_kind)).or_default();
            entry.files += 1;
            entry.blank += metrics.blank;
            entry.comment += metrics.comment;
            entry.code += metrics.code;

            // Per-package tracking
            if !packages.is_empty()
                && let Some(pkg) = file_package(relative_path, packages)
            {
                let pkg_entry = package_stats.entry(pkg).or_default();
                match file_kind {
                    FileKind::Source => {
                        pkg_entry.source.files += 1;
                        pkg_entry.source.blank += metrics.blank;
                        pkg_entry.source.comment += metrics.comment;
                        pkg_entry.source.code += metrics.code;
                    }
                    FileKind::Test => {
                        pkg_entry.test.files += 1;
                        pkg_entry.test.blank += metrics.blank;
                        pkg_entry.test.comment += metrics.comment;
                        pkg_entry.test.code += metrics.code;
                    }
                    FileKind::Other => {}
                }
            }
        }
    }

    // Wait for walker to finish
    let _walk_stats = handle.join();

    let package_names = &config.project.package_names;

    match args.output {
        OutputFormat::Json => print_json(&stats, &package_stats, package_names)?,
        _ => print_text(&stats, &package_stats, package_names),
    }

    Ok(ExitCode::Success)
}

/// Print the cloc report in text table format.
fn print_text(
    stats: &HashMap<(String, FileKind), LangStats>,
    package_stats: &HashMap<String, PackageStats>,
    package_names: &HashMap<String, String>,
) {
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

    // Per-package breakdown (only when packages are configured)
    if !package_stats.is_empty() {
        println!();
        println!("{}", separator);
        println!(
            "{:<25} {:>8}  {:>8}  {:>8}",
            "Package", "source", "test", "ratio"
        );
        println!("{}", separator);

        // Collect and sort packages alphabetically by display name
        let mut pkg_rows: Vec<(&String, &PackageStats)> = package_stats.iter().collect();
        pkg_rows.sort_by(|a, b| {
            let name_a = package_names.get(a.0).unwrap_or(a.0);
            let name_b = package_names.get(b.0).unwrap_or(b.0);
            name_a.cmp(name_b)
        });

        for (pkg_path, ps) in &pkg_rows {
            let display_name = package_names.get(*pkg_path).unwrap_or(pkg_path);
            let ratio = if ps.source.code > 0 {
                ps.test.code as f64 / ps.source.code as f64
            } else {
                0.0
            };
            println!(
                "{:<25} {:>8}  {:>8}  {:>7.2}x",
                display_name, ps.source.code, ps.test.code, ratio
            );
        }

        // Show unpackaged row if there are files not in any package
        let packaged_source: usize = package_stats.values().map(|ps| ps.source.code).sum();
        let packaged_test: usize = package_stats.values().map(|ps| ps.test.code).sum();
        let unpackaged_source = source_totals.code.saturating_sub(packaged_source);
        let unpackaged_test = test_totals.code.saturating_sub(packaged_test);
        if unpackaged_source > 0 || unpackaged_test > 0 {
            let ratio = if unpackaged_source > 0 {
                unpackaged_test as f64 / unpackaged_source as f64
            } else {
                0.0
            };
            println!(
                "{:<25} {:>8}  {:>8}  {:>7.2}x",
                "(unpackaged)", unpackaged_source, unpackaged_test, ratio
            );
        }

        println!("{}", separator);
    }
}

/// Print the cloc report in JSON format.
fn print_json(
    stats: &HashMap<(String, FileKind), LangStats>,
    package_stats: &HashMap<String, PackageStats>,
    package_names: &HashMap<String, String>,
) -> anyhow::Result<()> {
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

    let mut output = serde_json::json!({
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

    // Add packages when configured/detected
    if !package_stats.is_empty() {
        let mut packages = serde_json::Map::new();
        for (pkg_path, ps) in package_stats {
            let display_name = package_names.get(pkg_path).unwrap_or(pkg_path).clone();
            let ratio = if ps.source.code > 0 {
                ps.test.code as f64 / ps.source.code as f64
            } else {
                0.0
            };
            packages.insert(
                display_name,
                serde_json::json!({
                    "source": {
                        "files": ps.source.files,
                        "blank": ps.source.blank,
                        "comment": ps.source.comment,
                        "code": ps.source.code,
                    },
                    "test": {
                        "files": ps.test.files,
                        "blank": ps.test.blank,
                        "comment": ps.test.comment,
                        "code": ps.test.code,
                    },
                    "ratio": (ratio * 100.0).round() / 100.0,
                }),
            );
        }
        output["packages"] = serde_json::Value::Object(packages);
    }

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

/// Determine which package a file belongs to based on its path prefix.
fn file_package(path: &Path, packages: &[String]) -> Option<String> {
    for pkg in packages {
        if pkg == "." {
            return Some(pkg.clone());
        }
        if path.starts_with(pkg) {
            return Some(pkg.clone());
        }
    }
    None
}
