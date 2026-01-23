// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::HashMap;

use super::*;

fn make_scope_config(
    allow: Vec<&str>,
    forbid: Vec<&str>,
    patterns: HashMap<&str, &str>,
) -> SuppressScopeConfig {
    SuppressScopeConfig {
        check: None,
        allow: allow.into_iter().map(String::from).collect(),
        forbid: forbid.into_iter().map(String::from).collect(),
        patterns: patterns
            .into_iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect(),
    }
}

#[test]
fn no_violation_when_allow_level() {
    let scope_config = make_scope_config(vec![], vec![], HashMap::new());
    let params = SuppressCheckParams {
        scope_config: &scope_config,
        scope_check: SuppressLevel::Allow,
        global_comment: None,
    };
    let attr = SuppressAttrInfo {
        codes: &["dead_code".to_string()],
        has_comment: false,
        comment_text: None,
    };

    assert!(check_suppress_attr(&params, &attr).is_none());
}

#[test]
fn forbidden_code_triggers_violation() {
    let scope_config = make_scope_config(vec![], vec!["unsafe_code"], HashMap::new());
    let params = SuppressCheckParams {
        scope_config: &scope_config,
        scope_check: SuppressLevel::Comment,
        global_comment: None,
    };
    let attr = SuppressAttrInfo {
        codes: &["unsafe_code".to_string()],
        has_comment: true,
        comment_text: Some("// Some reason"),
    };

    let result = check_suppress_attr(&params, &attr);
    assert_eq!(
        result,
        Some(SuppressViolationKind::Forbidden {
            code: "unsafe_code".to_string()
        })
    );
}

#[test]
fn allowed_code_skips_checks() {
    let scope_config = make_scope_config(vec!["dead_code"], vec![], HashMap::new());
    let params = SuppressCheckParams {
        scope_config: &scope_config,
        scope_check: SuppressLevel::Comment,
        global_comment: Some("// REASON:"),
    };
    let attr = SuppressAttrInfo {
        codes: &["dead_code".to_string()],
        has_comment: false,
        comment_text: None,
    };

    // Should pass even without comment because it's in allow list
    assert!(check_suppress_attr(&params, &attr).is_none());
}

#[test]
fn forbid_level_rejects_all() {
    let scope_config = make_scope_config(vec![], vec![], HashMap::new());
    let params = SuppressCheckParams {
        scope_config: &scope_config,
        scope_check: SuppressLevel::Forbid,
        global_comment: None,
    };
    let attr = SuppressAttrInfo {
        codes: &["dead_code".to_string()],
        has_comment: true,
        comment_text: Some("// Good reason"),
    };

    assert_eq!(
        check_suppress_attr(&params, &attr),
        Some(SuppressViolationKind::AllForbidden)
    );
}

#[test]
fn comment_level_requires_comment() {
    let scope_config = make_scope_config(vec![], vec![], HashMap::new());
    let params = SuppressCheckParams {
        scope_config: &scope_config,
        scope_check: SuppressLevel::Comment,
        global_comment: None,
    };
    let attr = SuppressAttrInfo {
        codes: &["dead_code".to_string()],
        has_comment: false,
        comment_text: None,
    };

    let result = check_suppress_attr(&params, &attr);
    assert_eq!(
        result,
        Some(SuppressViolationKind::MissingComment {
            required_pattern: None
        })
    );
}

#[test]
fn comment_level_passes_with_any_comment() {
    let scope_config = make_scope_config(vec![], vec![], HashMap::new());
    let params = SuppressCheckParams {
        scope_config: &scope_config,
        scope_check: SuppressLevel::Comment,
        global_comment: None,
    };
    let attr = SuppressAttrInfo {
        codes: &["dead_code".to_string()],
        has_comment: true,
        comment_text: Some("// Any comment works"),
    };

    assert!(check_suppress_attr(&params, &attr).is_none());
}

#[test]
fn global_pattern_enforced() {
    let scope_config = make_scope_config(vec![], vec![], HashMap::new());
    let params = SuppressCheckParams {
        scope_config: &scope_config,
        scope_check: SuppressLevel::Comment,
        global_comment: Some("// REASON:"),
    };
    let attr = SuppressAttrInfo {
        codes: &["dead_code".to_string()],
        has_comment: true,
        comment_text: Some("// Wrong pattern"),
    };

    let result = check_suppress_attr(&params, &attr);
    assert_eq!(
        result,
        Some(SuppressViolationKind::MissingComment {
            required_pattern: Some("// REASON:".to_string())
        })
    );
}

#[test]
fn global_pattern_passes_when_matched() {
    let scope_config = make_scope_config(vec![], vec![], HashMap::new());
    let params = SuppressCheckParams {
        scope_config: &scope_config,
        scope_check: SuppressLevel::Comment,
        global_comment: Some("// REASON:"),
    };
    let attr = SuppressAttrInfo {
        codes: &["dead_code".to_string()],
        has_comment: true,
        comment_text: Some("// REASON: This is needed for backwards compat"),
    };

    assert!(check_suppress_attr(&params, &attr).is_none());
}

#[test]
fn per_lint_pattern_takes_precedence() {
    let mut patterns = HashMap::new();
    patterns.insert("dead_code", "// NOTE(compat):");
    let scope_config = make_scope_config(vec![], vec![], patterns);
    let params = SuppressCheckParams {
        scope_config: &scope_config,
        scope_check: SuppressLevel::Comment,
        global_comment: Some("// REASON:"),
    };

    // Using global pattern should fail for dead_code
    let attr = SuppressAttrInfo {
        codes: &["dead_code".to_string()],
        has_comment: true,
        comment_text: Some("// REASON: wrong pattern"),
    };
    let result = check_suppress_attr(&params, &attr);
    assert_eq!(
        result,
        Some(SuppressViolationKind::MissingComment {
            required_pattern: Some("// NOTE(compat):".to_string())
        })
    );

    // Using per-lint pattern should pass
    let attr = SuppressAttrInfo {
        codes: &["dead_code".to_string()],
        has_comment: true,
        comment_text: Some("// NOTE(compat): legacy API"),
    };
    assert!(check_suppress_attr(&params, &attr).is_none());
}

#[test]
fn per_lint_pattern_fallback_to_global() {
    let mut patterns = HashMap::new();
    patterns.insert("dead_code", "// NOTE:");
    let scope_config = make_scope_config(vec![], vec![], patterns);
    let params = SuppressCheckParams {
        scope_config: &scope_config,
        scope_check: SuppressLevel::Comment,
        global_comment: Some("// REASON:"),
    };

    // unused_variables has no per-lint pattern, should use global
    let attr = SuppressAttrInfo {
        codes: &["unused_variables".to_string()],
        has_comment: true,
        comment_text: Some("// REASON: needed for tests"),
    };
    assert!(check_suppress_attr(&params, &attr).is_none());

    // Wrong pattern should fail with global pattern in error
    let attr = SuppressAttrInfo {
        codes: &["unused_variables".to_string()],
        has_comment: true,
        comment_text: Some("// NOTE: wrong"),
    };
    let result = check_suppress_attr(&params, &attr);
    assert_eq!(
        result,
        Some(SuppressViolationKind::MissingComment {
            required_pattern: Some("// REASON:".to_string())
        })
    );
}

#[test]
fn clippy_prefix_matching() {
    let scope_config = make_scope_config(vec!["clippy"], vec![], HashMap::new());
    let params = SuppressCheckParams {
        scope_config: &scope_config,
        scope_check: SuppressLevel::Comment,
        global_comment: Some("// REASON:"),
    };
    let attr = SuppressAttrInfo {
        codes: &["clippy::unwrap_used".to_string()],
        has_comment: false,
        comment_text: None,
    };

    // clippy::unwrap_used should match allow list entry "clippy"
    assert!(check_suppress_attr(&params, &attr).is_none());
}

#[test]
fn normalize_pattern_strips_comment_prefix() {
    assert_eq!(normalize_comment_pattern("// REASON:"), "REASON:");
    assert_eq!(normalize_comment_pattern("# REASON:"), "REASON:");
    assert_eq!(normalize_comment_pattern("  // REASON:  "), "REASON:");
    assert_eq!(normalize_comment_pattern("REASON:"), "REASON:");
}
