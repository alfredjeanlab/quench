// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Unit tests for the Ruby adapter.

use std::path::Path;

use super::*;

#[test]
fn classifies_rb_files_as_source() {
    let adapter = RubyAdapter::new();
    assert_eq!(adapter.classify(Path::new("lib/app.rb")), FileKind::Source);
}

#[test]
fn classifies_rake_files_as_source() {
    let adapter = RubyAdapter::new();
    assert_eq!(adapter.classify(Path::new("tasks/deploy.rake")), FileKind::Source);
}

#[test]
fn classifies_rakefile_as_source() {
    let adapter = RubyAdapter::new();
    assert_eq!(adapter.classify(Path::new("Rakefile")), FileKind::Source);
}

#[test]
fn classifies_gemfile_as_source() {
    let adapter = RubyAdapter::new();
    assert_eq!(adapter.classify(Path::new("Gemfile")), FileKind::Source);
}

#[test]
fn classifies_gemspec_as_source() {
    let adapter = RubyAdapter::new();
    assert_eq!(adapter.classify(Path::new("myapp.gemspec")), FileKind::Source);
}

#[test]
fn classifies_spec_files_as_test() {
    let adapter = RubyAdapter::new();
    assert_eq!(adapter.classify(Path::new("spec/app_spec.rb")), FileKind::Test);
    assert_eq!(adapter.classify(Path::new("spec/models/user_spec.rb")), FileKind::Test);
}

#[test]
fn classifies_test_unit_files_as_test() {
    let adapter = RubyAdapter::new();
    assert_eq!(adapter.classify(Path::new("test/app_test.rb")), FileKind::Test);
    assert_eq!(adapter.classify(Path::new("test/models/user_test.rb")), FileKind::Test);
}

#[test]
fn classifies_test_prefix_files_as_test() {
    let adapter = RubyAdapter::new();
    assert_eq!(adapter.classify(Path::new("test/test_app.rb")), FileKind::Test);
    assert_eq!(adapter.classify(Path::new("test/models/test_user.rb")), FileKind::Test);
}

#[test]
fn classifies_cucumber_features_as_test() {
    let adapter = RubyAdapter::new();
    assert_eq!(adapter.classify(Path::new("features/login.rb")), FileKind::Test);
    assert_eq!(
        adapter.classify(Path::new("features/step_definitions/login_steps.rb")),
        FileKind::Test
    );
}

#[test]
fn ignores_vendor_directory() {
    let adapter = RubyAdapter::new();
    assert_eq!(adapter.classify(Path::new("vendor/bundle/gems/foo/lib/foo.rb")), FileKind::Other);
}

#[test]
fn ignores_tmp_directory() {
    let adapter = RubyAdapter::new();
    assert_eq!(adapter.classify(Path::new("tmp/cache/foo.rb")), FileKind::Other);
}

#[test]
fn ignores_log_directory() {
    let adapter = RubyAdapter::new();
    assert_eq!(adapter.classify(Path::new("log/debug.rb")), FileKind::Other);
}

#[test]
fn ignores_coverage_directory() {
    let adapter = RubyAdapter::new();
    assert_eq!(adapter.classify(Path::new("coverage/index.rb")), FileKind::Other);
}

#[test]
fn test_patterns_take_precedence_over_source() {
    let adapter = RubyAdapter::new();
    // A file that matches both test and source patterns should be classified as test
    assert_eq!(adapter.classify(Path::new("spec/lib_spec.rb")), FileKind::Test);
}

#[test]
fn returns_ruby_name() {
    let adapter = RubyAdapter::new();
    assert_eq!(adapter.name(), "ruby");
}

#[test]
fn returns_ruby_extensions() {
    let adapter = RubyAdapter::new();
    assert_eq!(adapter.extensions(), &["rb", "rake"]);
}

#[test]
fn default_escapes_include_debuggers() {
    let adapter = RubyAdapter::new();
    let escapes = adapter.default_escapes();

    assert!(escapes.iter().any(|e| e.name == "binding_pry"));
    assert!(escapes.iter().any(|e| e.name == "byebug"));
    assert!(escapes.iter().any(|e| e.name == "debugger"));
}

#[test]
fn default_escapes_include_metaprogramming() {
    let adapter = RubyAdapter::new();
    let escapes = adapter.default_escapes();

    assert!(escapes.iter().any(|e| e.name == "eval"));
    assert!(escapes.iter().any(|e| e.name == "instance_eval"));
    assert!(escapes.iter().any(|e| e.name == "class_eval"));
}

#[test]
fn debugger_escapes_are_forbid() {
    let adapter = RubyAdapter::new();
    let escapes = adapter.default_escapes();

    for escape in escapes
        .iter()
        .filter(|e| e.name.contains("pry") || e.name.contains("debug") || e.name == "byebug")
    {
        assert_eq!(
            escape.action,
            EscapeAction::Forbid,
            "debugger {} should be Forbid",
            escape.name
        );
    }
}

#[test]
fn metaprogramming_escapes_require_comment() {
    let adapter = RubyAdapter::new();
    let escapes = adapter.default_escapes();

    for escape in escapes.iter().filter(|e| e.name.contains("eval")) {
        assert_eq!(
            escape.action,
            EscapeAction::Comment,
            "metaprogramming {} should be Comment",
            escape.name
        );
        assert!(
            escape.comment.is_some(),
            "metaprogramming {} should have comment pattern",
            escape.name
        );
    }
}
