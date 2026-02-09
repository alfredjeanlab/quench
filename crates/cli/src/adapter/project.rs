// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Project-level language defaults: exclude patterns and workspace package detection.

use std::path::Path;

use super::{
    JsWorkspace, ProjectLanguage, detect_language, python::detect_package as detect_python_package,
    rust::CargoWorkspace,
};
use crate::config::Config;

/// Apply language-specific exclude patterns and auto-detect workspace packages.
///
/// Returns the complete list of exclude patterns (user-configured + language defaults).
/// Mutates `config` to populate auto-detected `packages` and `package_names`.
pub fn apply_language_defaults(root: &Path, config: &mut Config) -> Vec<String> {
    let mut exclude_patterns = config.project.exclude.patterns.clone();

    match detect_language(root) {
        ProjectLanguage::Rust => {
            // Exclude target/ directory for Rust projects
            if !exclude_patterns.iter().any(|p| p.contains("target")) {
                exclude_patterns.push("target".to_string());
            }

            // Auto-detect workspace packages if not configured
            if config.project.packages.is_empty() {
                let workspace = CargoWorkspace::from_root(root);
                if workspace.is_workspace {
                    // For workspaces, expand member patterns to get both paths and names
                    for pattern in &workspace.member_patterns {
                        if pattern.contains('*') {
                            // Expand glob patterns
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
                                            // Read package name from Cargo.toml
                                            let cargo_toml = entry.path().join("Cargo.toml");
                                            if let Ok(content) =
                                                std::fs::read_to_string(&cargo_toml)
                                                && let Ok(value) = content.parse::<toml::Value>()
                                                && let Some(name) = value
                                                    .get("package")
                                                    .and_then(|p| p.get("name"))
                                                    .and_then(|n| n.as_str())
                                            {
                                                config
                                                    .project
                                                    .package_names
                                                    .insert(rel_path.clone(), name.to_string());
                                            }
                                            config.project.packages.push(rel_path);
                                        }
                                    }
                                }
                            }
                        } else {
                            // Direct path to package
                            let pkg_dir = root.join(pattern);
                            let cargo_toml = pkg_dir.join("Cargo.toml");
                            if let Ok(content) = std::fs::read_to_string(&cargo_toml)
                                && let Ok(value) = content.parse::<toml::Value>()
                                && let Some(name) = value
                                    .get("package")
                                    .and_then(|p| p.get("name"))
                                    .and_then(|n| n.as_str())
                            {
                                config
                                    .project
                                    .package_names
                                    .insert(pattern.clone(), name.to_string());
                            }
                            config.project.packages.push(pattern.clone());
                        }
                    }
                    config.project.packages.sort();
                    tracing::debug!(
                        "auto-detected workspace packages: {:?}",
                        config.project.packages
                    );
                    tracing::debug!("package names: {:?}", config.project.package_names);
                }
            }

            // Resolve display names for explicitly configured packages that lack them
            for pkg in &config.project.packages {
                if !config.project.package_names.contains_key(pkg) {
                    let cargo_toml = root.join(pkg).join("Cargo.toml");
                    if let Ok(content) = std::fs::read_to_string(&cargo_toml)
                        && let Ok(value) = content.parse::<toml::Value>()
                        && let Some(name) = value
                            .get("package")
                            .and_then(|p| p.get("name"))
                            .and_then(|n| n.as_str())
                    {
                        config.project.package_names.insert(pkg.clone(), name.to_string());
                    }
                }
            }
        }
        ProjectLanguage::Go => {
            // Exclude vendor/ directory for Go projects
            if !exclude_patterns.iter().any(|p| p.contains("vendor")) {
                exclude_patterns.push("vendor".to_string());
            }

            // Auto-detect Go packages if not configured
            if config.project.packages.is_empty() {
                let packages = super::go::enumerate_packages(root);
                // Only populate if there are multiple packages (single-package
                // projects don't benefit from per-package breakdown)
                if packages.len() > 1 {
                    for pkg_path in packages {
                        // Use directory name as display name
                        let name = Path::new(&pkg_path)
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or(&pkg_path)
                            .to_string();
                        config.project.package_names.insert(pkg_path.clone(), name);
                        config.project.packages.push(pkg_path);
                    }
                    config.project.packages.sort();
                    tracing::debug!("auto-detected Go packages: {:?}", config.project.packages);
                    tracing::debug!("package names: {:?}", config.project.package_names);
                }
            }
        }
        ProjectLanguage::Shell => {
            // No special exclude patterns for Shell projects
        }
        ProjectLanguage::JavaScript => {
            // Exclude node_modules, dist, build for JS projects
            for pattern in ["node_modules", "dist", "build", ".next", "coverage"] {
                if !exclude_patterns.iter().any(|p| p.contains(pattern)) {
                    exclude_patterns.push(pattern.to_string());
                }
            }

            // Auto-detect workspace packages if not configured
            if config.project.packages.is_empty() {
                let workspace = JsWorkspace::from_root(root);
                if workspace.is_workspace {
                    for path in &workspace.package_paths {
                        config.project.packages.push(path.clone());
                    }
                    config.project.package_names = workspace.package_names.clone();
                    tracing::debug!(
                        "auto-detected JS workspace packages: {:?}",
                        config.project.packages
                    );
                    tracing::debug!("package names: {:?}", config.project.package_names);
                }
            }
        }
        ProjectLanguage::Python => {
            // Exclude common Python cache and build directories
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

            // Auto-detect Python package if not configured
            if config.project.packages.is_empty()
                && let Some((pkg_path, pkg_name)) = detect_python_package(root)
            {
                config.project.package_names.insert(pkg_path.clone(), pkg_name);
                config.project.packages.push(pkg_path);
                tracing::debug!("auto-detected Python package: {:?}", config.project.packages);
            }
        }
        ProjectLanguage::Ruby => {
            // Exclude vendor, tmp, log, coverage for Ruby projects
            for pattern in ["vendor", "tmp", "log", "coverage"] {
                if !exclude_patterns.iter().any(|p| p.contains(pattern)) {
                    exclude_patterns.push(pattern.to_string());
                }
            }
        }
        ProjectLanguage::Generic => {}
    }

    exclude_patterns
}
