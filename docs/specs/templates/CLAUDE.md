# Template Files - DO NOT MODIFY

## WARNING

**YOU MAY NOT MODIFY ANY FILES IN THIS DIRECTORY UNLESS DIRECTLY INSTRUCTED**

These template files are the source of truth specifications.
The code in `crates/cli/src/profiles.rs` must match these templates, NOT the other way around.

## If Tests Fail

If unit tests fail because they expect content from these templates:

1. **DO NOT** modify the template files
2. **DO** update the code in `profiles.rs` to match the templates
3. The templates define the spec; the code implements it

## Purpose

These `.toml` files define the canonical configuration templates for `quench init`. They serve as:

- Specifications for what the init command should generate
- Reference documentation for users
- Source of truth for test assertions

The functions in `profiles.rs` (like `default_template_base()`, `python_profile_defaults()`, etc.)
must generate output that matches these files exactly.
