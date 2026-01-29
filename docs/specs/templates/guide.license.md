# License Configuration Guide

Configuration reference for the `license` check.

## Basic Configuration

Disabled by default; opt in by setting `check = "error"`.

```toml
[check.license]
check = "error"
license = "MIT"
copyright = "Your Organization"
```

## Common Licenses

MIT License:

```toml
[check.license]
check = "error"
license = "MIT"
copyright = "Your Organization"
```

Apache 2.0:

```toml
[check.license]
check = "error"
license = "Apache-2.0"
copyright = "Your Organization"
```

Business Source License:

```toml
[check.license]
check = "error"
license = "BUSL-1.1"
copyright = "Your Organization"
```

GPL v3:

```toml
[check.license]
check = "error"
license = "GPL-3.0-only"
copyright = "Your Organization"
```

## File Patterns

```toml
[check.license]
check = "error"
license = "MIT"
copyright = "Your Organization"

[check.license.patterns]
rust = ["**/*.rs"]
shell = ["**/*.sh", "**/*.bash", "scripts/*"]
typescript = ["**/*.ts", "**/*.tsx"]
go = ["**/*.go"]
python = ["**/*.py"]
ruby = ["**/*.rb"]
```

## Excludes

```toml
[check.license]
check = "error"
license = "MIT"
copyright = "Your Organization"
exclude = [
  "**/generated/**",
  "**/vendor/**",
  "**/node_modules/**",
  "**/target/**",
]
```

## Complete Example

```toml
[check.license]
check = "error"
license = "MIT"
copyright = "Your Organization"

[check.license.patterns]
rust = ["**/*.rs"]
shell = ["**/*.sh", "**/*.bash", "scripts/*"]
typescript = ["**/*.ts", "**/*.tsx"]
go = ["**/*.go"]

exclude = [
  "**/generated/**",
  "**/vendor/**",
  "**/node_modules/**",
  "**/target/**",
]
```
