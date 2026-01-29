# Ruby Configuration Guide

Configuration reference for Ruby language support.

## File Patterns

```toml
[ruby]
source = ["**/*.rb", "**/*.rake", "Rakefile", "Gemfile", "*.gemspec"]
tests = ["spec/**/*_spec.rb", "test/**/*_test.rb", "test/**/test_*.rb", "features/**/*.rb"]
ignore = ["vendor/", "tmp/", "log/", "coverage/"]
```

## CLOC Advice

```toml
[ruby.cloc]
check = "error"
advice = "Custom advice for oversized Ruby files."
```

## Suppress Directives

Controls how `# rubocop:disable` and `# standard:disable` comments are handled:

- `"forbid"` — never allowed
- `"comment"` — requires justification comment (default for source)
- `"allow"` — always allowed (default for tests)

```toml
[ruby.suppress]
check = "comment"

[ruby.suppress.test]
check = "allow"
```

## Suppress with Allowlist/Denylist

Require specific comment for method length suppressions.

```toml
[ruby.suppress]
check = "comment"

[ruby.suppress.source]
allow = ["Style/FrozenStringLiteralComment"]  # No comment needed
forbid = ["Security/Eval"]                     # Never suppress

[ruby.suppress.source."Metrics/MethodLength"]
comment = "# TODO(refactor):"

[ruby.suppress.test]
check = "allow"
```

## Lint Config Policy

Require RuboCop/Standard config changes in standalone PRs.

```toml
[ruby.policy]
check = "error"
lint_changes = "standalone"
lint_config = [".rubocop.yml", ".rubocop_todo.yml", ".standard.yml"]
```

## Escape Patterns

Ruby-specific escape hatches:

```toml
[[check.escapes.patterns]]
pattern = "binding\\.pry"
action = "forbid"
in_tests = "forbid"  # Forbidden even in tests (breaks CI)
advice = "Remove debugger statement before committing."

[[check.escapes.patterns]]
pattern = "byebug"
action = "forbid"
in_tests = "forbid"
advice = "Remove debugger statement before committing."

[[check.escapes.patterns]]
pattern = "debugger"
action = "forbid"
in_tests = "forbid"
advice = "Remove debugger statement before committing."

[[check.escapes.patterns]]
pattern = "eval\\("
action = "comment"
comment = "# METAPROGRAMMING:"
advice = "Add a # METAPROGRAMMING: comment explaining why eval is necessary."

[[check.escapes.patterns]]
pattern = "instance_eval"
action = "comment"
comment = "# METAPROGRAMMING:"
advice = "Add a # METAPROGRAMMING: comment explaining the DSL or metaprogramming use case."

[[check.escapes.patterns]]
pattern = "class_eval"
action = "comment"
comment = "# METAPROGRAMMING:"
advice = "Add a # METAPROGRAMMING: comment explaining the metaprogramming use case."
```

## Coverage

RSpec or Minitest with SimpleCov:

```toml
[[check.tests.suite]]
runner = "rspec"
```

Or for Minitest:

```toml
[[check.tests.suite]]
runner = "minitest"
```

## Complete Example

```toml
[ruby]
source = ["**/*.rb", "**/*.rake", "Rakefile", "Gemfile"]
tests = ["spec/**/*_spec.rb", "test/**/*_test.rb", "features/**/*.rb"]
ignore = ["vendor/", "tmp/", "log/"]

[ruby.cloc]
check = "error"
advice = "Custom advice for Ruby files."

[ruby.suppress]
check = "comment"

[ruby.suppress.source]
allow = ["Style/FrozenStringLiteralComment"]
forbid = ["Security/Eval"]

[ruby.suppress.source."Metrics/MethodLength"]
comment = "# TODO(refactor):"

[ruby.suppress.test]
check = "allow"

[ruby.policy]
check = "error"
lint_changes = "standalone"
lint_config = [".rubocop.yml", ".standard.yml"]

[[check.escapes.patterns]]
pattern = "binding\\.pry"
action = "forbid"
in_tests = "forbid"

[[check.escapes.patterns]]
pattern = "eval\\("
action = "comment"
comment = "# METAPROGRAMMING:"

[[check.tests.suite]]
runner = "rspec"
```
