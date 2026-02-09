// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

use std::path::Path;

use super::*;

#[test]
fn parse_cobertura_xml_extracts_overall_coverage() {
    let xml = r#"<?xml version="1.0" ?>
<coverage line-rate="0.75" branch-rate="0" version="1.9">
    <packages>
        <package name="scripts" line-rate="0.75">
            <classes>
                <class filename="/path/to/scripts/helper.sh" line-rate="0.80">
                </class>
            </classes>
        </package>
    </packages>
</coverage>"#;

    let result = parse_cobertura_xml(xml, Duration::from_secs(1), Path::new("/path/to"));
    assert!(result.success);
    assert_eq!(result.line_coverage, Some(75.0));
}

#[test]
fn parse_cobertura_xml_extracts_per_file_coverage() {
    let xml = r#"<?xml version="1.0" ?>
<coverage line-rate="0.70" version="1.9">
    <packages>
        <package name="scripts">
            <classes>
                <class filename="/project/scripts/helper.sh" line-rate="0.80">
                </class>
                <class filename="/project/scripts/util.sh" line-rate="0.60">
                </class>
            </classes>
        </package>
    </packages>
</coverage>"#;

    let result = parse_cobertura_xml(xml, Duration::from_secs(1), Path::new("/project"));
    assert!(result.success);
    assert_eq!(result.files.len(), 2);
    assert_eq!(result.files.get("scripts/helper.sh"), Some(&80.0));
    assert_eq!(result.files.get("scripts/util.sh"), Some(&60.0));
}

#[test]
fn extract_line_rate_parses_coverage_element() {
    let xml = r#"<coverage line-rate="0.85" branch-rate="0">"#;
    assert_eq!(extract_line_rate(xml, "coverage"), Some(0.85));
}

#[test]
fn extract_line_rate_returns_none_for_missing() {
    let xml = r#"<coverage branch-rate="0">"#;
    assert_eq!(extract_line_rate(xml, "coverage"), None);
}

#[test]
fn extract_attribute_finds_value() {
    let tag = r#"<class filename="/path/to/file.sh" line-rate="0.75">"#;
    assert_eq!(extract_attribute(tag, "filename"), Some("/path/to/file.sh".to_string()));
    assert_eq!(extract_attribute(tag, "line-rate"), Some("0.75".to_string()));
}

#[test]
fn extract_attribute_returns_none_for_missing() {
    let tag = r#"<class filename="/path/to/file.sh">"#;
    assert_eq!(extract_attribute(tag, "missing"), None);
}

#[test]
fn extract_elements_finds_all_classes() {
    let xml = r#"
        <class filename="a.sh" line-rate="0.8">
        </class>
        <class filename="b.sh" line-rate="0.6">
        </class>
    "#;

    let elements = extract_elements(xml, "class");
    assert_eq!(elements.len(), 2);
    assert!(elements[0].contains("a.sh"));
    assert!(elements[1].contains("b.sh"));
}

#[test]
fn normalize_path_strips_root_prefix() {
    let path = "/home/user/project/scripts/helper.sh";
    let root = Path::new("/home/user/project");
    assert_eq!(normalize_path(path, root), "scripts/helper.sh");
}

#[test]
fn normalize_path_finds_marker() {
    let path = "/some/random/path/scripts/helper.sh";
    let root = Path::new("/different/root");
    assert_eq!(normalize_path(path, root), "scripts/helper.sh");
}

#[test]
fn normalize_path_falls_back_to_filename() {
    let path = "/unknown/path/file.sh";
    let root = Path::new("/different/root");
    assert_eq!(normalize_path(path, root), "file.sh");
}

#[test]
fn collect_shell_coverage_skips_if_no_scripts() {
    let result = collect_shell_coverage(&[], &["echo".to_string()], Path::new("/tmp"));
    assert!(result.success);
    assert!(result.line_coverage.is_none());
}

#[test]
fn parse_cobertura_computes_overall_from_files() {
    // XML with per-file data but no overall line-rate
    let xml = r#"<?xml version="1.0" ?>
<coverage version="1.9">
    <packages>
        <package name="scripts">
            <classes>
                <class filename="/project/a.sh" line-rate="0.80">
                </class>
                <class filename="/project/b.sh" line-rate="0.60">
                </class>
            </classes>
        </package>
    </packages>
</coverage>"#;

    let result = parse_cobertura_xml(xml, Duration::ZERO, Path::new("/project"));
    assert!(result.success);
    // Average of 80% and 60% = 70%
    assert_eq!(result.line_coverage, Some(70.0));
}
