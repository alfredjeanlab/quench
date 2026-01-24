# JavaScript / TypeScript Language Support

JavaScript and TypeScript-specific behavior for quench checks.

## Detection

Detected when any of these exist in project root:
- `package.json`
- `tsconfig.json`
- `jsconfig.json`

## Profile Defaults

When using [`quench init --profile javascript`](../01-cli.md#profile-selection-recommended) (or `--profile js` / `--profile typescript` / `--profile ts`), the following opinionated defaults are configured:

```toml
[javascript]
bundle_size = true
build_time = true

[javascript.suppress]
check = "comment"

[javascript.suppress.test]
check = "allow"

[javascript.policy]
lint_changes = "standalone"
lint_config = [".eslintrc", ".eslintrc.js", ".eslintrc.json", ".eslintrc.yml", "eslint.config.js", "eslint.config.mjs", "biome.json", "biome.jsonc"]

[[check.escapes.patterns]]
pattern = "as unknown"
action = "comment"
comment = "// CAST:"
advice = "Add a // CAST: comment explaining why the type assertion is necessary."

[[check.escapes.patterns]]
pattern = "@ts-ignore"
action = "forbid"
in_tests = "allow"
advice = "Use @ts-expect-error instead, which fails if the error is resolved."
```

**Landing the Plane items** (added to agent files when combined with `claude` or `cursor` profile):
- `npm run lint` (or `pnpm lint` / `yarn lint` / `bun lint`)
- `npm run typecheck` (if `tsconfig.json` exists)
- `npm test`
- `npm run build`

## Default Patterns

```toml
[javascript]
source = ["**/*.js", "**/*.jsx", "**/*.ts", "**/*.tsx", "**/*.mjs", "**/*.mts", "**/*.cjs", "**/*.cts"]
tests = [
  "**/*.test.*", "**/*.spec.*",
  "**/*_test.*", "**/*_tests.*", "**/test_*.*",
  "**/__tests__/**",
  "**/test/**", "**/tests/**"
]
ignore = ["node_modules/", "dist/", "build/", ".next/", "coverage/"]
```

## Test Code Detection

**Test files** (entire file is test code):
- `*.test.*`, `*.spec.*` (any extension)
- `*_test.*`, `*_tests.*`, `test_*.*` (underscore variants)
- Files in `__tests__/` directories
- Files in `test/` or `tests/` directories

No inline test code convention for JavaScript/TypeScript. Test code is entirely file-based.

```typescript
// src/math.ts       ← source LOC
export function add(a: number, b: number): number {
  return a + b;
}
```

```typescript
// src/math.test.ts  ← test LOC (entire file)
import { add } from './math';

test('adds two numbers', () => {
  expect(add(1, 2)).toBe(3);
});
```

### Escapes in Test Code

Escape patterns (`as unknown`, `@ts-ignore`) are allowed in test code:

- **Test files**: Any file matching test patterns

## Default Escape Patterns

| Pattern | Action | Comment Required |
|---------|--------|------------------|
| `as unknown` | comment | `// CAST:` |
| `@ts-ignore` | forbid | - |

Quench assumes you are already running ESLint/Biome and TypeScript for general linting.

- **`as unknown`**: Type escape that bypasses the type checker; document why casting is safe
- **`@ts-ignore`**: Silences errors without validation; use `@ts-expect-error` instead (which is self-documenting)

## Suppress

Controls lint directive comments:
- ESLint: `eslint-disable`, `eslint-disable-next-line`
- Biome: `biome-ignore`

| Setting | Behavior |
|---------|----------|
| `"forbid"` | Never allowed |
| `"comment"` | Requires justification comment (default) |
| `"allow"` | Always allowed |

Default: `"comment"` for source, `"allow"` for test code.

```typescript
// OK: Legacy API requires mutable assignment
// eslint-disable-next-line no-param-reassign
options.timeout = 5000;

// eslint-disable-next-line @typescript-eslint/no-explicit-any  ← Missing justification → violation
function legacyWrapper(data: any) {}
```

### Configuration

```toml
[javascript.suppress]
check = "comment"              # forbid | comment | allow
# comment = "// OK:"           # optional: require specific pattern (default: any)

[javascript.suppress.source]
allow = ["no-console"]         # no comment needed for these
forbid = ["no-explicit-any"]   # never suppress this rule

[javascript.suppress.test]
check = "allow"                # tests can suppress freely
```

### Supported Patterns

**ESLint:**
```typescript
// Single rule (inline)
// eslint-disable-next-line no-unused-vars

// Multiple rules
// eslint-disable-next-line no-unused-vars, @typescript-eslint/no-explicit-any

// Block disable
/* eslint-disable no-console */
console.log('debug');
/* eslint-enable no-console */

// File-level disable
/* eslint-disable @typescript-eslint/no-require-imports */

// With reason (supported by eslint-plugin-eslint-comments)
// eslint-disable-next-line no-magic-numbers -- pagination constants
const PAGE_SIZE = 20;
```

**Biome:**
```typescript
// Single rule
// biome-ignore lint/suspicious/noExplicitAny: legacy API requires any

// Multiple rules
// biome-ignore lint/style/noUnusedVariables lint/suspicious/noExplicitAny: migration in progress

// Biome requires explanations after the colon
// biome-ignore lint/complexity/noForEach: readable in this context
items.forEach(process);
```

## Policy

Enforce lint configuration hygiene.

```toml
[javascript.policy]
lint_changes = "standalone"    # lint config changes must be standalone PRs
lint_config = [                # files that trigger standalone requirement
  ".eslintrc",
  ".eslintrc.js",
  ".eslintrc.json",
  ".eslintrc.yml",
  "eslint.config.js",
  "eslint.config.mjs",
  "biome.json",
  "biome.jsonc",
]
```

When `lint_changes = "standalone"`, changing any `lint_config` files alongside source/test changes fails:

```
javascript: FAIL
  lint config changes must be standalone
    Changed: eslint.config.js
    Also changed: src/parser.ts, src/lexer.ts
  Submit lint config changes in a separate PR.
```

## Build Metrics

JavaScript/TypeScript build metrics are part of the `build` check. See [checks/build.md](../checks/build.md) for full details.

### Bundler Detection

Quench auto-detects the bundler from config files:

| Bundler | Detection Files |
|---------|-----------------|
| Vite | `vite.config.ts`, `vite.config.js` |
| Webpack | `webpack.config.js`, `webpack.config.ts` |
| esbuild | `esbuild.config.js`, usage in `package.json` scripts |
| Rollup | `rollup.config.js`, `rollup.config.ts` |
| Parcel | `parcel` in `package.json` |
| Next.js | `next.config.js`, `next.config.mjs` |

```toml
[javascript]
bundler = "auto"               # auto | vite | webpack | esbuild | rollup | next
```

### Targets

Build targets are output files to measure. Auto-detected from bundler config or explicitly configured:

```toml
[javascript]
targets = ["dist/index.js", "dist/vendor.js"]

# Or use glob patterns
# targets = ["dist/*.js"]
```

### Bundle Size

Track bundle sizes (CI mode). Reports both raw and gzipped sizes.

```
build: size
  dist/index.js: 145 KB (42 KB gzipped)
  dist/vendor.js: 892 KB (245 KB gzipped)
```

With threshold:
```
build: FAIL
  dist/vendor.js: 1.2 MB (max: 1 MB)
    Reduce bundle size. Consider code splitting or lighter alternatives.
```

### Build Time

Track build times (CI mode):

- **Cold**: `rm -rf dist && npm run build`
- **Hot**: `npm run build` (bundler handles caching)

```
build: time
  cold: 18.4s
  hot: 2.1s
```

## Coverage

JavaScript/TypeScript runners provide built-in coverage:

| Runner | Coverage Tool |
|--------|---------------|
| `vitest` | v8 or istanbul (built-in) |
| `jest` | istanbul (built-in) |
| `bun` | built-in |

```toml
[[check.tests.suite]]
runner = "vitest"
# Implicit: covers JS/TS code via built-in coverage

[[check.tests.suite]]
runner = "jest"
# Implicit: covers JS/TS code via istanbul
```

### Coverage Configuration

Coverage is automatically collected when using supported runners. Configure thresholds via `[check.tests.coverage]`:

```toml
[check.tests.coverage]
check = "error"
min = 80

[check.tests.coverage.package.core]
min = 90
```

## Placeholder Tests

Quench recognizes placeholder test patterns:

```typescript
// Jest / Vitest
test.todo('should handle edge case');
it.todo('validates input');

// With description
test.skip('temporarily disabled', () => { /* ... */ });
```

These satisfy test correlation requirements, indicating planned test implementation.

## Configuration

```toml
[javascript]
# Source/test patterns (defaults shown)
# source = ["**/*.js", "**/*.jsx", "**/*.ts", "**/*.tsx", "**/*.mjs", "**/*.mts", "**/*.cjs", "**/*.cts"]
# tests = ["**/*.test.*", "**/*.spec.*", "**/*_test.*", "**/*_tests.*", "**/test_*.*", "**/__tests__/**", "**/test/**", "**/tests/**"]
# ignore = ["node_modules/", "dist/", "build/", ".next/", "coverage/"]

# Bundler (default: auto-detect)
# bundler = "auto"

# Build targets (default: auto-detect from bundler)
# targets = ["dist/index.js"]

# Build metrics (CI mode) - see [check.build] for thresholds
bundle_size = true
build_time = true

[javascript.suppress]
check = "comment"

[javascript.suppress.test]
check = "allow"

[javascript.policy]
lint_changes = "standalone"
lint_config = [".eslintrc", ".eslintrc.js", "eslint.config.js", "biome.json"]
```

Test suites and coverage thresholds are configured in `[check.tests]`.
