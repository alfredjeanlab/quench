# Ruby Language Support

Ruby-specific behavior for quench checks.

## Detection

Detected when any of these exist in project root:
- `Gemfile`
- `*.gemspec`
- `config.ru` (Rack)
- `config/application.rb` (Rails)

## Profile Defaults

When using [`quench init --with ruby`](../01-cli.md#explicit-profiles) (or `--with rb`), the following opinionated defaults are configured:

```toml
[ruby]
# No build metrics for interpreted language

[ruby.suppress]
check = "comment"

[ruby.suppress.test]
check = "allow"

[ruby.policy]
lint_changes = "standalone"
lint_config = [".rubocop.yml", ".rubocop_todo.yml", ".standard.yml"]

[[check.escapes.patterns]]
pattern = "binding.pry"
action = "forbid"
in_tests = "allow"
advice = "Remove debugger statement before committing."

[[check.escapes.patterns]]
pattern = "byebug"
action = "forbid"
in_tests = "allow"
advice = "Remove debugger statement before committing."

[[check.escapes.patterns]]
pattern = "debugger"
action = "forbid"
in_tests = "allow"
advice = "Remove debugger statement before committing."

[[check.escapes.patterns]]
pattern = "eval("
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

**Landing the Plane items** (added to agent files when combined with `claude` or `cursor` profile):
- `bundle exec rubocop` (or `bundle exec standardrb` if `.standard.yml` exists)
- `bundle exec rspec` (if `spec/` exists)
- `bundle exec rake test` (if `test/` exists)
- `bundle exec rails test` (if Rails project)

## Default Patterns

```toml
[ruby]
source = ["**/*.rb", "**/*.rake", "Rakefile", "Gemfile", "*.gemspec"]
tests = [
  "spec/**/*_spec.rb",
  "test/**/*_test.rb", "test/**/test_*.rb",
  "features/**/*.rb"
]
ignore = ["vendor/", "tmp/", "log/", "coverage/"]
```

When `[ruby].tests` is not configured, patterns fall back to `[project].tests`, then to these defaults. See [Pattern Resolution](../02-config.md#pattern-resolution).

## Test Code Detection

**Test files** (entire file is test code):
- RSpec: `*_spec.rb` files, files in `spec/` directory
- Minitest: `*_test.rb` files, `test_*.rb` files, files in `test/` directory
- Cucumber: `*.rb` files in `features/` directory (step definitions)

No inline test code convention for Ruby. Test code is entirely file-based.

```ruby
# lib/math.rb       <- source LOC
module Math
  def self.add(a, b)
    a + b
  end
end
```

```ruby
# spec/math_spec.rb  <- test LOC (entire file)
require 'math'

RSpec.describe Math do
  describe '.add' do
    it 'adds two numbers' do
      expect(Math.add(1, 2)).to eq(3)
    end
  end
end
```

### Escapes in Test Code

Escape patterns (debuggers, metaprogramming) follow different rules in test code:
- **Debuggers**: Forbidden even in tests by default (common source of CI failures)
- **Metaprogramming**: Allowed in tests without comment

## Default Escape Patterns

| Pattern | Action | Comment Required |
|---------|--------|------------------|
| `binding.pry` | forbid | - |
| `byebug` | forbid | - |
| `debugger` | forbid | - |
| `eval(` | comment | `# METAPROGRAMMING:` |
| `instance_eval` | comment | `# METAPROGRAMMING:` |
| `class_eval` | comment | `# METAPROGRAMMING:` |

Quench assumes you are already running RuboCop or Standard for general linting.

- **Debugger statements**: Will cause CI failures and should never be committed
- **`eval`**: Arbitrary code execution; document why it's necessary
- **`instance_eval` / `class_eval`**: Powerful metaprogramming; document the DSL or use case

## Suppress

Controls lint directive comments:
- RuboCop: `# rubocop:disable`
- Standard: `# standard:disable`

| Setting | Behavior |
|---------|----------|
| `"forbid"` | Never allowed |
| `"comment"` | Requires justification comment (default) |
| `"allow"` | Always allowed |

Default: `"comment"` for source, `"allow"` for test code.

```ruby
# OK: Legacy API returns inconsistent types
# rubocop:disable Lint/MixedRegexpCaptureTypes
def parse_header(line)
  line.match(/(?<name>\w+):(.*)/)
end
# rubocop:enable Lint/MixedRegexpCaptureTypes

# rubocop:disable Style/Documentation  <- Missing justification -> violation
class InternalHelper
end
# rubocop:enable Style/Documentation
```

### Configuration

```toml
[ruby.suppress]
check = "comment"              # forbid | comment | allow
# comment = "# OK:"            # optional: require specific pattern (default: any)

[ruby.suppress.source]
allow = ["Style/FrozenStringLiteralComment"]  # no comment needed
forbid = ["Security/Eval"]                     # never suppress this

[ruby.suppress.test]
check = "allow"                # tests can suppress freely

# Per-cop patterns (optional)
[ruby.suppress.source."Metrics/MethodLength"]
comment = "# TODO(refactor):"  # require specific pattern for length violations
```

### Supported Patterns

```ruby
# Single cop (inline)
# rubocop:disable Style/StringLiterals

# Multiple cops
# rubocop:disable Style/StringLiterals, Style/FrozenStringLiteralComment

# Block disable
# rubocop:disable Metrics/AbcSize
def complex_method
  # ...
end
# rubocop:enable Metrics/AbcSize

# Inline disable (same line)
x = foo() # rubocop:disable Lint/UselessAssignment

# With todo comment (common pattern)
# rubocop:todo Metrics/MethodLength
def needs_refactoring
  # ...
end
# rubocop:enable Metrics/MethodLength

# Standard Ruby style
# standard:disable Style/StringLiterals
```

### Violation Messages

When a RuboCop suppression is missing a required comment, the error message encourages fixing first:
1. Primary instruction to fix the issue (imperative, actionable)
2. Context and guidance on how to fix it properly
3. Suppression as last resort with acceptable comment patterns

**Example outputs:**

```
lib/parser.rb:45: suppress_missing_comment: # rubocop:disable Metrics/MethodLength
  Refactor this method into smaller pieces.
  Extract logical sections into separate methods with clear names.
  Only if fixing is not feasible, add:
    # TODO(refactor): ...

lib/client.rb:23: suppress_missing_comment: # rubocop:disable Security/Open
  Verify the URL/path comes from a trusted source.
  Validate and sanitize the input before passing to open().
  Only if the lint is a false positive, add a comment above the directive.
```

**Default per-cop guidance** (for common RuboCop cops):

| Cop | Primary Fix Instruction | Context |
|-----|------------------------|---------|
| `Metrics/MethodLength` | Refactor this method into smaller pieces. | Extract logical sections into separate methods with clear names. |
| `Metrics/AbcSize` | Reduce this method's complexity. | Break down complex logic into smaller, focused methods. |
| `Metrics/CyclomaticComplexity` | Simplify the conditional logic. | Consider using guard clauses or extracting conditions into methods. |
| `Security/Open` | Verify the URL/path comes from a trusted source. | Validate and sanitize the input before passing to open(). |
| `Security/Eval` | Replace eval with a safer alternative. | Use send, public_send, or a dispatch table instead. |
| `Style/Documentation` | Add documentation for this class. | Document the class purpose, responsibilities, and usage. |
| `Lint/UselessAssignment` | Remove this unused assignment. | Delete the variable or use it in the method logic. |

Other cops use: "Fix the underlying issue instead of suppressing the lint."

## Policy

Enforce lint configuration hygiene.

```toml
[ruby.policy]
lint_changes = "standalone"    # lint config changes must be standalone PRs
lint_config = [                # files that trigger standalone requirement
  ".rubocop.yml",
  ".rubocop_todo.yml",
  ".standard.yml",
]
```

When `lint_changes = "standalone"`, changing any `lint_config` files alongside source/test changes fails:

```
ruby: FAIL
  lint config changes must be standalone
    Changed: .rubocop.yml
    Also changed: lib/parser.rb, lib/lexer.rb
  Submit lint config changes in a separate PR.
```

## Coverage

Ruby coverage uses SimpleCov. Coverage is collected when running test suites:

```toml
[[check.tests.suite]]
runner = "rspec"
# Implicit: covers Ruby code via SimpleCov if configured

[[check.tests.suite]]
runner = "minitest"
# Implicit: covers Ruby code via SimpleCov if configured
```

### SimpleCov Configuration

Quench expects SimpleCov to be configured in your test helper. Coverage data is read from the default SimpleCov output location (`coverage/`).

```ruby
# spec/spec_helper.rb or test/test_helper.rb
require 'simplecov'
SimpleCov.start do
  add_filter '/spec/'
  add_filter '/test/'
end
```

Coverage thresholds are configured via `[check.tests.coverage]`:

```toml
[check.tests.coverage]
check = "error"
min = 80

[check.tests.coverage.package.core]
min = 90
```

## Placeholder Tests

Quench recognizes placeholder test patterns:

```ruby
# RSpec
it 'should handle edge case' do
  pending 'implementation needed'
end

xit 'temporarily disabled' do
  # ...
end

# Minitest
def test_edge_case
  skip 'implementation needed'
end
```

These satisfy test correlation requirements, indicating planned test implementation.

## Configuration

```toml
[ruby]
# Source/test patterns (falls back to [project].tests if not set)
# source = ["**/*.rb", "**/*.rake", "Rakefile", "Gemfile", "*.gemspec"]
# tests = ["spec/**/*_spec.rb", "test/**/*_test.rb", "test/**/test_*.rb", "features/**/*.rb"]
# ignore = ["vendor/", "tmp/", "log/", "coverage/"]

[ruby.cloc]
check = "error"                  # error | warn | off
# advice = "..."                 # Custom advice for oversized Ruby files

[ruby.suppress]
check = "comment"

[ruby.suppress.test]
check = "allow"

[ruby.policy]
lint_changes = "standalone"
lint_config = [".rubocop.yml", ".rubocop_todo.yml", ".standard.yml"]
```

Test suites and coverage thresholds are configured in `[check.tests]`.
