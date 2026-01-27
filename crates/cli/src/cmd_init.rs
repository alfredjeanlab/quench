// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Implementation of the `quench init` command.

use std::path::Path;

use anyhow::Result;

use crate::cli::InitArgs;
use crate::error::ExitCode;
use crate::init::{DetectedAgent, DetectedLanguage, detect_agents, detect_languages};
use crate::profiles::{
    ProfileRegistry, agents_section, default_template_base, default_template_suffix,
    golang_detected_section, javascript_detected_section, ruby_detected_section,
    rust_detected_section, shell_detected_section,
};

/// Default entries to add to .gitignore.
const DEFAULT_GITIGNORE_ENTRIES: &[&str] = &[".quench/"];

/// Ensure .quench/ is in .gitignore.
fn ensure_gitignored(root: &Path) -> Result<()> {
    let gitignore = root.join(".gitignore");
    let content = if gitignore.exists() {
        std::fs::read_to_string(&gitignore)?
    } else {
        String::new()
    };

    let mut entries_to_add = Vec::new();
    for entry in DEFAULT_GITIGNORE_ENTRIES {
        // Check if entry is already present (as a line on its own)
        let entry_line = format!("\n{}\n", entry);
        let content_with_newlines = format!("\n{}\n", content);
        if !content_with_newlines.contains(&entry_line)
            && !content.starts_with(*entry)
            && !content.starts_with(&format!("{}\n", entry))
        {
            entries_to_add.push(*entry);
        }
    }

    if !entries_to_add.is_empty() {
        let mut new_content = content;
        // Add newline if file doesn't end with one
        if !new_content.is_empty() && !new_content.ends_with('\n') {
            new_content.push('\n');
        }
        // Add a blank line separator if file has content
        if !new_content.is_empty() {
            new_content.push('\n');
        }
        // Add comment and entries
        new_content.push_str("# Quench cache (managed by quench init)\n");
        for entry in entries_to_add {
            new_content.push_str(entry);
            new_content.push('\n');
        }
        std::fs::write(&gitignore, new_content)?;
    }

    Ok(())
}

/// Run the `init` command to create a quench.toml configuration file.
pub fn run(args: &InitArgs) -> Result<ExitCode> {
    let cwd = std::env::current_dir()?;
    let config_path = cwd.join("quench.toml");

    if config_path.exists() && !args.force {
        eprintln!("quench.toml already exists. Use --force to overwrite.");
        return Ok(ExitCode::ConfigError);
    }

    // Determine what to include
    let (config, message) = if !args.with_profiles.is_empty() {
        // --with specified: use full profiles, skip detection
        // Separate agent profiles from language profiles since agents replace agents section
        let mut agent_required: Vec<&str> = Vec::new();
        let mut lang_config = String::new();

        for profile in &args.with_profiles {
            if ProfileRegistry::is_agent_profile(profile) {
                // Agent profile: collect required files
                match profile.to_lowercase().as_str() {
                    "claude" => {
                        if !agent_required.contains(&"CLAUDE.md") {
                            agent_required.push("CLAUDE.md");
                        }
                    }
                    "cursor" => {
                        if !agent_required.contains(&".cursorrules") {
                            agent_required.push(".cursorrules");
                        }
                    }
                    _ => {}
                }
            } else if let Some(content) = ProfileRegistry::get(profile) {
                // Language profile: append to config
                lang_config.push('\n');
                lang_config.push_str(&content);
            } else {
                // Unknown profile: warn and suggest
                if let Some(suggestion) = ProfileRegistry::suggest(profile) {
                    eprintln!(
                        "quench: warning: unknown profile '{}', did you mean '{}'?",
                        profile, suggestion
                    );
                } else {
                    eprintln!("quench: warning: unknown profile '{}', skipping", profile);
                }
            }
        }

        // Build final config
        let mut cfg = default_template_base().to_string();
        if !agent_required.is_empty() {
            cfg.push_str(&format!(
                "[check.agents]\ncheck = \"error\"\nrequired = {:?}\n",
                agent_required
            ));
        } else {
            cfg.push_str(&agents_section(&[]));
        }
        cfg.push_str(default_template_suffix());
        cfg.push_str(&lang_config);

        let msg = format!(
            "Created quench.toml with profile(s): {}",
            args.with_profiles.join(", ")
        );
        (cfg, msg)
    } else {
        // No --with: run auto-detection for both languages and agents
        let detected_langs = detect_languages(&cwd);
        let detected_agents = detect_agents(&cwd);

        // Build config with proper agents section placement
        let mut cfg = default_template_base().to_string();
        cfg.push_str(&agents_section(&detected_agents));
        cfg.push_str(default_template_suffix());

        // Add language sections (after # Supported Languages:)
        for lang in &detected_langs {
            cfg.push('\n');
            match lang {
                DetectedLanguage::Rust => cfg.push_str(rust_detected_section()),
                DetectedLanguage::Golang => cfg.push_str(golang_detected_section()),
                DetectedLanguage::JavaScript => cfg.push_str(javascript_detected_section()),
                DetectedLanguage::Shell => cfg.push_str(shell_detected_section()),
                DetectedLanguage::Ruby => cfg.push_str(ruby_detected_section()),
            }
        }

        // Build message listing detected items
        let mut detected_names = Vec::new();
        for lang in &detected_langs {
            detected_names.push(match lang {
                DetectedLanguage::Rust => "rust",
                DetectedLanguage::Golang => "golang",
                DetectedLanguage::JavaScript => "javascript",
                DetectedLanguage::Shell => "shell",
                DetectedLanguage::Ruby => "ruby",
            });
        }
        for agent in &detected_agents {
            detected_names.push(match agent {
                DetectedAgent::Claude => "claude",
                DetectedAgent::Cursor(_) => "cursor",
            });
        }

        let msg = if detected_names.is_empty() {
            "Created quench.toml".to_string()
        } else {
            format!(
                "Created quench.toml (detected: {})",
                detected_names.join(", ")
            )
        };
        (cfg, msg)
    };

    std::fs::write(&config_path, config)?;

    // Ensure .quench/ is in .gitignore
    if let Err(e) = ensure_gitignored(&cwd) {
        eprintln!("quench: warning: failed to update .gitignore: {}", e);
    }

    println!("{}", message);
    Ok(ExitCode::Success)
}
