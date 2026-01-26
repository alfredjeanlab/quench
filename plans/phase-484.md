# Phase 484: Ruby Test Fixtures

## Overview

Create test fixtures for Ruby projects to support the Ruby language adapter. This includes a standalone Ruby gem fixture, a Rails application fixture, and Ruby-specific violation files for testing escape/suppress detection.

## Project Structure

```
tests/fixtures/
├── ruby-gem/                    # Standalone Ruby gem
│   ├── quench.toml
│   ├── example_gem.gemspec
│   ├── Gemfile
│   ├── lib/
│   │   ├── example_gem.rb
│   │   └── example_gem/
│   │       └── calculator.rb
│   └── spec/
│       ├── spec_helper.rb
│       ├── example_gem_spec.rb
│       └── calculator_spec.rb
│
├── ruby-rails/                  # Rails application
│   ├── quench.toml
│   ├── Gemfile
│   ├── config/
│   │   ├── application.rb
│   │   └── routes.rb
│   ├── app/
│   │   ├── controllers/
│   │   │   └── application_controller.rb
│   │   └── models/
│   │       └── user.rb
│   └── spec/                    # RSpec tests (common in Rails)
│       ├── rails_helper.rb
│       └── models/
│           └── user_spec.rb
│
└── violations/
    └── ruby/                    # Ruby-specific violations
        ├── eval.rb              # eval without METAPROGRAMMING comment
        ├── debugger.rb          # binding.pry, byebug violations
        └── rubocop.rb           # rubocop:disable without comment
```

## Dependencies

- None (fixtures are static files requiring no runtime dependencies)
- Fixtures follow conventions established by existing Go/JS/Rust fixtures

## Implementation Phases

### Phase 1: Create ruby-gem Fixture Structure

Create the basic Ruby gem fixture following idiomatic Ruby project structure.

**Files to create:**

1. `tests/fixtures/ruby-gem/quench.toml`
```toml
version = 1

[project]
name = "ruby-gem"
```

2. `tests/fixtures/ruby-gem/example_gem.gemspec`
```ruby
Gem::Specification.new do |spec|
  spec.name          = "example_gem"
  spec.version       = "0.1.0"
  spec.authors       = ["Test Author"]
  spec.summary       = "Example gem for quench testing"
  spec.files         = Dir["lib/**/*.rb"]
  spec.require_paths = ["lib"]
end
```

3. `tests/fixtures/ruby-gem/Gemfile`
```ruby
source "https://rubygems.org"
gemspec

group :test do
  gem "rspec"
end
```

4. `tests/fixtures/ruby-gem/lib/example_gem.rb`
```ruby
require_relative "example_gem/calculator"

module ExampleGem
  VERSION = "0.1.0"
end
```

5. `tests/fixtures/ruby-gem/lib/example_gem/calculator.rb`
```ruby
module ExampleGem
  class Calculator
    def add(a, b)
      a + b
    end

    def multiply(a, b)
      a * b
    end
  end
end
```

6. `tests/fixtures/ruby-gem/spec/spec_helper.rb`
```ruby
require "example_gem"

RSpec.configure do |config|
  config.expect_with :rspec do |expectations|
    expectations.syntax = :expect
  end
end
```

7. `tests/fixtures/ruby-gem/spec/example_gem_spec.rb`
```ruby
require "spec_helper"

RSpec.describe ExampleGem do
  it "has a version number" do
    expect(ExampleGem::VERSION).not_to be_nil
  end
end
```

8. `tests/fixtures/ruby-gem/spec/calculator_spec.rb`
```ruby
require "spec_helper"

RSpec.describe ExampleGem::Calculator do
  let(:calc) { described_class.new }

  describe "#add" do
    it "adds two numbers" do
      expect(calc.add(2, 3)).to eq(5)
    end
  end

  describe "#multiply" do
    it "multiplies two numbers" do
      expect(calc.multiply(3, 4)).to eq(12)
    end
  end
end
```

### Phase 2: Create ruby-rails Fixture Structure

Create a minimal Rails application structure for testing Rails detection.

**Files to create:**

1. `tests/fixtures/ruby-rails/quench.toml`
```toml
version = 1

[project]
name = "ruby-rails"
```

2. `tests/fixtures/ruby-rails/Gemfile`
```ruby
source "https://rubygems.org"

gem "rails", "~> 7.0"

group :test do
  gem "rspec-rails"
end
```

3. `tests/fixtures/ruby-rails/config/application.rb`
```ruby
require_relative "boot"
require "rails/all"

module RubyRails
  class Application < Rails::Application
    config.load_defaults 7.0
  end
end
```

4. `tests/fixtures/ruby-rails/config/routes.rb`
```ruby
Rails.application.routes.draw do
  resources :users, only: [:index, :show]
end
```

5. `tests/fixtures/ruby-rails/app/controllers/application_controller.rb`
```ruby
class ApplicationController < ActionController::Base
  before_action :set_locale

  private

  def set_locale
    I18n.locale = params[:locale] || I18n.default_locale
  end
end
```

6. `tests/fixtures/ruby-rails/app/models/user.rb`
```ruby
class User < ApplicationRecord
  validates :email, presence: true, uniqueness: true
  validates :name, presence: true, length: { minimum: 2 }

  def display_name
    name.titleize
  end
end
```

7. `tests/fixtures/ruby-rails/spec/rails_helper.rb`
```ruby
require "spec_helper"
ENV["RAILS_ENV"] ||= "test"

RSpec.configure do |config|
  config.use_transactional_fixtures = true
  config.infer_spec_type_from_file_location!
end
```

8. `tests/fixtures/ruby-rails/spec/models/user_spec.rb`
```ruby
require "rails_helper"

RSpec.describe User, type: :model do
  describe "validations" do
    it { is_expected.to validate_presence_of(:email) }
    it { is_expected.to validate_presence_of(:name) }
  end

  describe "#display_name" do
    it "titleizes the name" do
      user = User.new(name: "john doe")
      expect(user.display_name).to eq("John Doe")
    end
  end
end
```

### Phase 3: Add Ruby Violations

Create Ruby-specific violation files in the violations fixture.

**Files to create:**

1. `tests/fixtures/violations/ruby/eval.rb`
```ruby
# VIOLATION: eval without METAPROGRAMMING comment
class DynamicLoader
  def load_code(code_string)
    eval(code_string)
  end

  def dynamic_method(obj, code)
    obj.instance_eval(code)
  end

  def extend_class(klass, code)
    klass.class_eval(code)
  end
end
```

2. `tests/fixtures/violations/ruby/debugger.rb`
```ruby
# VIOLATION: debugger statements forbidden in source code
class BuggyService
  def process(data)
    binding.pry
    result = transform(data)
    byebug
    save(result)
    debugger
    result
  end

  private

  def transform(data)
    data.to_s.upcase
  end

  def save(result)
    # save logic
  end
end
```

3. `tests/fixtures/violations/ruby/rubocop.rb`
```ruby
# VIOLATION: rubocop:disable without justification comment
# rubocop:disable Metrics/MethodLength
def long_method
  step_one
  step_two
  step_three
  step_four
  step_five
  step_six
  step_seven
  step_eight
  step_nine
  step_ten
end
# rubocop:enable Metrics/MethodLength

# VIOLATION: rubocop:todo without justification
# rubocop:todo Style/FrozenStringLiteralComment
MUTABLE_STRING = "hello"
```

### Phase 4: Update violations/quench.toml

Add Ruby-specific escape patterns to the violations fixture config.

**Edit:** `tests/fixtures/violations/quench.toml`

Add after the JavaScript patterns section:

```toml
# Ruby-specific escape patterns
[[check.escapes.patterns]]
name = "ruby_eval"
pattern = "\\beval\\("
action = "comment"
comment = "# METAPROGRAMMING:"
source = ["**/*.rb"]

[[check.escapes.patterns]]
name = "ruby_instance_eval"
pattern = "\\.instance_eval"
action = "comment"
comment = "# METAPROGRAMMING:"
source = ["**/*.rb"]

[[check.escapes.patterns]]
name = "ruby_class_eval"
pattern = "\\.class_eval"
action = "comment"
comment = "# METAPROGRAMMING:"
source = ["**/*.rb"]

[[check.escapes.patterns]]
name = "ruby_binding_pry"
pattern = "binding\\.pry"
action = "forbid"
source = ["**/*.rb"]

[[check.escapes.patterns]]
name = "ruby_byebug"
pattern = "\\bbyebug\\b"
action = "forbid"
source = ["**/*.rb"]

[[check.escapes.patterns]]
name = "ruby_debugger"
pattern = "\\bdebugger\\b"
action = "forbid"
source = ["**/*.rb"]

[ruby.suppress]
check = "comment"
```

### Phase 5: Update Fixture README

Update `tests/fixtures/CLAUDE.md` to document the new Ruby fixtures.

**Add to Fixture Index table:**

| `ruby-gem/` | Small Ruby gem | cloc, tests |
| `ruby-rails/` | Rails application | cloc, tests |

**Add new sections:**

```markdown
### ruby-gem/

A minimal Ruby gem with idiomatic structure. Good baseline for testing Ruby detection and default behavior.

- `example_gem.gemspec` with gem metadata
- `Gemfile` with RSpec test dependency
- `lib/example_gem.rb` with main entry point
- `lib/example_gem/calculator.rb` with utility class
- `spec/` with RSpec tests
- Under 750 lines (passes cloc)
- Proper test coverage (passes tests)

### ruby-rails/

A minimal Rails application structure for testing Rails-specific detection.

- `config/application.rb` with Rails application class
- `config/routes.rb` with route definitions
- `app/controllers/` with ApplicationController
- `app/models/` with User model
- `spec/` with RSpec Rails tests
- Tests Rails framework detection path
```

**Add to violations table:**

| escapes | `ruby/eval.rb` | `eval(` without METAPROGRAMMING |
| escapes | `ruby/debugger.rb` | `binding.pry`, `byebug`, `debugger` (forbidden) |
| suppress | `ruby/rubocop.rb` | `rubocop:disable` without justification |

## Key Implementation Details

### Ruby Project Detection Hierarchy

The fixtures demonstrate the detection hierarchy from the roadmap:

1. **Gemfile** - Any Ruby project (ruby-gem, ruby-rails)
2. ***.gemspec** - Ruby gem specifically (ruby-gem)
3. **config/application.rb** - Rails application (ruby-rails)
4. **config.ru** - Rack application (not included, future fixture)

### Test Pattern Conventions

Ruby has two major test conventions:

| Convention | Directory | File Pattern | Framework |
|------------|-----------|--------------|-----------|
| RSpec | `spec/` | `*_spec.rb` | rspec |
| Minitest | `test/` | `*_test.rb` | minitest |

Both fixtures use RSpec (`spec/` directory) as it's more common in the Ruby ecosystem.

### Escape Patterns

Ruby escape patterns follow the METAPROGRAMMING comment convention:

```ruby
# METAPROGRAMMING: Dynamic method generation for API clients
eval("def #{method_name}; end")
```

Debugger statements (`binding.pry`, `byebug`, `debugger`) use `forbid` action since they should never appear in committed source code.

### RuboCop Suppress Patterns

The violations fixture demonstrates RuboCop directive patterns:

- `# rubocop:disable Cop/Name` - Single cop disable
- `# rubocop:disable Cop1, Cop2` - Multiple cops (not in current fixture)
- `# rubocop:todo` - Deferred fixes
- `# standard:disable` - Standard Ruby (not in current fixture)

## Verification Plan

### Phase 1 Verification: ruby-gem Fixture

```bash
# Verify fixture structure exists
ls -la tests/fixtures/ruby-gem/
ls -la tests/fixtures/ruby-gem/lib/
ls -la tests/fixtures/ruby-gem/spec/

# Verify gemspec is valid Ruby syntax
ruby -c tests/fixtures/ruby-gem/example_gem.gemspec

# Verify all Ruby files have valid syntax
find tests/fixtures/ruby-gem -name "*.rb" -exec ruby -c {} \;
```

### Phase 2 Verification: ruby-rails Fixture

```bash
# Verify fixture structure exists
ls -la tests/fixtures/ruby-rails/
ls -la tests/fixtures/ruby-rails/config/
ls -la tests/fixtures/ruby-rails/app/

# Verify all Ruby files have valid syntax
find tests/fixtures/ruby-rails -name "*.rb" -exec ruby -c {} \;
```

### Phase 3 Verification: Ruby Violations

```bash
# Verify violation files exist
ls -la tests/fixtures/violations/ruby/

# Verify Ruby syntax (should be valid Ruby even with violations)
ruby -c tests/fixtures/violations/ruby/eval.rb
ruby -c tests/fixtures/violations/ruby/debugger.rb
ruby -c tests/fixtures/violations/ruby/rubocop.rb

# Verify violations contain expected patterns
grep -n "eval(" tests/fixtures/violations/ruby/eval.rb
grep -n "binding.pry" tests/fixtures/violations/ruby/debugger.rb
grep -n "rubocop:disable" tests/fixtures/violations/ruby/rubocop.rb
```

### Phase 4-5 Verification: Config and README Updates

```bash
# Verify quench.toml is valid TOML
cat tests/fixtures/violations/quench.toml | python3 -c "import sys, tomllib; tomllib.load(sys.stdin.buffer)"

# Verify README contains new fixture documentation
grep -q "ruby-gem" tests/fixtures/CLAUDE.md
grep -q "ruby-rails" tests/fixtures/CLAUDE.md
```

### Full Verification

```bash
# Run the full check suite (should not regress existing tests)
make check
```
