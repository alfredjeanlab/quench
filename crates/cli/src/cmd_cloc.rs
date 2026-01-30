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
    // Per-package stats: (package_path, language_name, FileKind) -> LangStats
    let mut package_lang_stats: HashMap<(String, String, FileKind), LangStats> = HashMap::new();
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

            let pkg = if !packages.is_empty() {
                file_package(relative_path, packages)
            } else {
                None
            };

            if classification.source_lines > 0 {
                let ratio = classification.source_lines as f64 / total_nonblank as f64;
                let blank = (metrics.blank as f64 * ratio).round() as usize;
                let comment = (metrics.comment as f64 * ratio).round() as usize;
                let code = (metrics.code as f64 * ratio).round() as usize;
                let entry = stats.entry((lang.clone(), FileKind::Source)).or_default();
                entry.files += 1;
                entry.blank += blank;
                entry.comment += comment;
                entry.code += code;
                if let Some(ref pkg) = pkg {
                    let pe = package_lang_stats
                        .entry((pkg.clone(), lang.clone(), FileKind::Source))
                        .or_default();
                    pe.files += 1;
                    pe.blank += blank;
                    pe.comment += comment;
                    pe.code += code;
                }
            }
            if classification.test_lines > 0 {
                let ratio = classification.test_lines as f64 / total_nonblank as f64;
                let blank = (metrics.blank as f64 * ratio).round() as usize;
                let comment = (metrics.comment as f64 * ratio).round() as usize;
                let code = (metrics.code as f64 * ratio).round() as usize;
                let entry = stats.entry((lang.clone(), FileKind::Test)).or_default();
                entry.files += 1;
                entry.blank += blank;
                entry.comment += comment;
                entry.code += code;
                if let Some(ref pkg) = pkg {
                    let pe = package_lang_stats
                        .entry((pkg.clone(), lang.clone(), FileKind::Test))
                        .or_default();
                    pe.files += 1;
                    pe.blank += blank;
                    pe.comment += comment;
                    pe.code += code;
                }
            }
        } else {
            let entry = stats.entry((lang.clone(), file_kind)).or_default();
            entry.files += 1;
            entry.blank += metrics.blank;
            entry.comment += metrics.comment;
            entry.code += metrics.code;

            // Per-package tracking
            if !packages.is_empty()
                && let Some(pkg) = file_package(relative_path, packages)
            {
                let pe = package_lang_stats
                    .entry((pkg, lang, file_kind))
                    .or_default();
                pe.files += 1;
                pe.blank += metrics.blank;
                pe.comment += metrics.comment;
                pe.code += metrics.code;
            }
        }
    }

    // Wait for walker to finish
    let _walk_stats = handle.join();

    let package_names = &config.project.package_names;

    match args.output {
        OutputFormat::Json => print_json(&stats, &package_lang_stats, package_names)?,
        _ => print_text(&stats, &package_lang_stats, package_names),
    }

    Ok(ExitCode::Success)
}

/// Print the cloc report in text table format.
fn print_text(
    stats: &HashMap<(String, FileKind), LangStats>,
    package_lang_stats: &HashMap<(String, String, FileKind), LangStats>,
    package_names: &HashMap<String, String>,
) {
    // Collect rows and sort: group by language total code desc, source before test
    let mut rows: Vec<(&String, FileKind, &LangStats)> = stats
        .iter()
        .filter(|(_, s)| s.files > 0)
        .map(|((lang, kind), s)| (lang, *kind, s))
        .collect();

    // Precompute per-language total code for group sorting
    let mut lang_totals: HashMap<&String, usize> = HashMap::new();
    for (lang, _, s) in &rows {
        *lang_totals.entry(lang).or_default() += s.code;
    }

    rows.sort_by(|a, b| {
        let a_total = lang_totals.get(a.0).copied().unwrap_or(0);
        let b_total = lang_totals.get(b.0).copied().unwrap_or(0);
        b_total
            .cmp(&a_total)
            .then_with(|| a.0.cmp(b.0))
            .then_with(|| kind_order(a.1).cmp(&kind_order(b.1)))
    });

    if rows.is_empty() {
        println!("No source files found.");
        return;
    }

    let separator = "\u{2500}".repeat(62);

    // Header
    println!("{}", separator);
    println!(
        "{:<25} {:>5}  {:>8}  {:>8}  {:>8}",
        "Language", "files", "blank", "comment", "code"
    );
    println!("{}", separator);

    // Data rows with inline package breakdown
    let mut source_totals = LangStats::default();
    let mut test_totals = LangStats::default();
    let has_packages = !package_lang_stats.is_empty();

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

        // Inline per-package rows (indented)
        if has_packages {
            let mut pkg_rows: Vec<(&String, &LangStats)> = package_lang_stats
                .iter()
                .filter(|((_, l, k), ps)| l == *lang && *k == *kind && ps.files > 0)
                .map(|((p, _, _), ps)| (p, ps))
                .collect();
            pkg_rows.sort_by(|a, b| {
                let name_a = package_names.get(a.0).unwrap_or(a.0);
                let name_b = package_names.get(b.0).unwrap_or(b.0);
                name_a.cmp(name_b)
            });
            for (pkg_path, ps) in &pkg_rows {
                let display_name = package_names.get(*pkg_path).unwrap_or(pkg_path);
                println!(
                    "  {:<23} {:>5}  {:>8}  {:>8}  {:>8}",
                    display_name, ps.files, ps.blank, ps.comment, ps.code
                );
            }
        }

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
fn print_json(
    stats: &HashMap<(String, FileKind), LangStats>,
    package_lang_stats: &HashMap<(String, String, FileKind), LangStats>,
    package_names: &HashMap<String, String>,
) -> anyhow::Result<()> {
    let has_packages = !package_lang_stats.is_empty();

    // Collect and sort rows with same logic as text output
    let mut rows: Vec<(&String, FileKind, &LangStats)> = stats
        .iter()
        .filter(|(_, s)| s.files > 0)
        .map(|((lang, kind), s)| (lang, *kind, s))
        .collect();

    let mut lang_totals: HashMap<&String, usize> = HashMap::new();
    for (lang, _, s) in &rows {
        *lang_totals.entry(lang).or_default() += s.code;
    }

    rows.sort_by(|a, b| {
        let a_total = lang_totals.get(a.0).copied().unwrap_or(0);
        let b_total = lang_totals.get(b.0).copied().unwrap_or(0);
        b_total
            .cmp(&a_total)
            .then_with(|| a.0.cmp(b.0))
            .then_with(|| kind_order(a.1).cmp(&kind_order(b.1)))
    });

    let languages: Vec<serde_json::Value> = rows
        .iter()
        .map(|(lang, kind, s)| {
            let mut entry = serde_json::json!({
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
            });

            if has_packages {
                let mut pkg_rows: Vec<(&String, &LangStats)> = package_lang_stats
                    .iter()
                    .filter(|((_, l, k), ps)| l == *lang && *k == *kind && ps.files > 0)
                    .map(|((p, _, _), ps)| (p, ps))
                    .collect();
                pkg_rows.sort_by(|a, b| {
                    let name_a = package_names.get(a.0).unwrap_or(a.0);
                    let name_b = package_names.get(b.0).unwrap_or(b.0);
                    name_a.cmp(name_b)
                });
                if !pkg_rows.is_empty() {
                    let packages: Vec<serde_json::Value> = pkg_rows
                        .iter()
                        .map(|(pkg_path, ps)| {
                            let display_name = package_names.get(*pkg_path).unwrap_or(pkg_path);
                            serde_json::json!({
                                "name": display_name,
                                "files": ps.files,
                                "blank": ps.blank,
                                "comment": ps.comment,
                                "code": ps.code,
                            })
                        })
                        .collect();
                    entry["packages"] = serde_json::Value::Array(packages);
                }
            }

            entry
        })
        .collect();

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
