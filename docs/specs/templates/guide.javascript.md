# JavaScript/TypeScript Configuration Guide

Configuration reference for JavaScript/TypeScript language support.

## File Patterns

```toml
[javascript]
source = ["**/*.js", "**/*.jsx", "**/*.ts", "**/*.tsx", "**/*.mjs", "**/*.mts"]
tests = ["**/*.test.*", "**/*.spec.*", "**/__tests__/**", "**/test/**"]
ignore = ["node_modules/", "dist/", "build/", ".next/", "coverage/"]
```

## Bundler

Auto-detected from `vite.config.ts`, `webpack.config.js`, etc.
Or specify explicitly: `"vite"`, `"webpack"`, `"esbuild"`, `"rollup"`, `"next"`.

```toml
[javascript]
bundler = "auto"
```

## Build Metrics

```toml
[javascript]
bundler = "vite"
bundle_size = true           # Track bundle sizes (raw and gzipped)
build_time = true            # Track build times (cold and hot)
targets = ["dist/index.js"]  # Or use globs: ["dist/*.js"]
```

## CLOC Advice

```toml
[javascript.cloc]
check = "error"
advice = "Custom advice for oversized JS/TS files."
```

## Suppress Directives

Controls how `eslint-disable` and `biome-ignore` comments are handled:

- `"forbid"` — never allowed
- `"comment"` — requires justification comment (default for source)
- `"allow"` — always allowed (default for tests)

```toml
[javascript.suppress]
check = "comment"

[javascript.suppress.test]
check = "allow"
```

## Suppress with Allowlist/Denylist

```toml
[javascript.suppress]
check = "comment"

[javascript.suppress.source]
allow = ["no-console"]        # No comment needed
forbid = ["no-explicit-any"]  # Never suppress

[javascript.suppress.test]
check = "allow"
```

## Lint Config Policy

Require ESLint/Biome config changes in standalone PRs.

```toml
[javascript.policy]
check = "error"
lint_changes = "standalone"
lint_config = [".eslintrc", "eslint.config.js", "biome.json"]
```

## Escape Patterns

TypeScript-specific escape hatches:

```toml
[[check.escapes.patterns]]
pattern = "as unknown"
action = "comment"
comment = "// CAST:"
advice = "Add a // CAST: comment explaining why the type assertion is necessary."

[[check.escapes.patterns]]
pattern = "@ts-ignore"
action = "forbid"
advice = "@ts-ignore is forbidden. Use @ts-expect-error instead."
```

## Coverage

Test runners provide built-in coverage:

```toml
[[check.tests.suite]]
runner = "vitest"  # or "jest" or "bun"
```

## Complete Example

```toml
[javascript]
source = ["**/*.js", "**/*.ts", "**/*.tsx"]
tests = ["**/*.test.*", "**/__tests__/**"]
ignore = ["node_modules/", "dist/", ".next/"]
bundler = "vite"
targets = ["dist/index.js"]
bundle_size = true
build_time = true

[javascript.cloc]
check = "error"
advice = "Custom advice for JS/TS files."

[javascript.suppress]
check = "comment"

[javascript.suppress.source]
allow = ["no-console"]
forbid = ["no-explicit-any"]

[javascript.suppress.test]
check = "allow"

[javascript.policy]
check = "error"
lint_changes = "standalone"
lint_config = ["eslint.config.js", "biome.json"]

[[check.escapes.patterns]]
pattern = "as unknown"
action = "comment"
comment = "// CAST:"

[[check.escapes.patterns]]
pattern = "@ts-ignore"
action = "forbid"

[[check.tests.suite]]
runner = "vitest"
```
