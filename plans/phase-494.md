# Phase 494: JavaScript Adapter - Test Fixtures

## Overview

Create JavaScript/TypeScript test fixtures for the JavaScript adapter. This includes a simple JS/TS project (`js-simple/`), a multi-package monorepo (`js-monorepo/`), JavaScript-specific violations in the violations fixture, and updates to the fixture README.

## Project Structure

```
tests/fixtures/
├── js-simple/                    # NEW: Simple JS/TS project
│   ├── quench.toml
│   ├── package.json
│   ├── tsconfig.json
│   ├── src/
│   │   ├── index.ts
│   │   └── utils.ts
│   └── tests/
│       ├── index.test.ts
│       └── utils.test.ts
├── js-monorepo/                  # NEW: Multi-package workspace
│   ├── quench.toml
│   ├── package.json
│   ├── pnpm-workspace.yaml
│   ├── packages/
│   │   ├── core/
│   │   │   ├── package.json
│   │   │   ├── src/index.ts
│   │   │   └── tests/core.test.ts
│   │   └── cli/
│   │       ├── package.json
│   │       ├── src/main.ts
│   │       └── tests/cli.test.ts
│   └── tsconfig.json
├── violations/
│   ├── js/                       # NEW: JavaScript violations
│   │   ├── as-unknown.ts
│   │   ├── ts-ignore.ts
│   │   └── eslint-disable.ts
│   └── quench.toml               # UPDATE: Add JS patterns
└── CLAUDE.md                     # UPDATE: Document new fixtures
```

## Dependencies

- No external dependencies required
- Uses existing fixture conventions from `tests/fixtures/CLAUDE.md`
- Follows patterns established in `go-simple/` and `rust-simple/`

## Implementation Phases

### Phase 1: Create js-simple fixture

Create a minimal JavaScript/TypeScript project that passes all checks:

**Files to create:**

1. `tests/fixtures/js-simple/package.json`
```json
{
  "name": "js-simple",
  "version": "1.0.0",
  "type": "module",
  "main": "src/index.ts",
  "scripts": {
    "test": "echo 'Tests would run here'"
  }
}
```

2. `tests/fixtures/js-simple/tsconfig.json`
```json
{
  "compilerOptions": {
    "target": "ES2022",
    "module": "ESNext",
    "moduleResolution": "bundler",
    "strict": true,
    "outDir": "dist"
  },
  "include": ["src/**/*"]
}
```

3. `tests/fixtures/js-simple/quench.toml`
```toml
version = 1

[project]
name = "js-simple"
```

4. `tests/fixtures/js-simple/src/index.ts` - Main entry point with exported function
5. `tests/fixtures/js-simple/src/utils.ts` - Utility functions
6. `tests/fixtures/js-simple/tests/index.test.ts` - Tests for index
7. `tests/fixtures/js-simple/tests/utils.test.ts` - Tests for utils

**Verification:**
- `quench check` passes on the fixture
- JavaScript adapter auto-detects the project
- Source and test patterns correctly classify files

### Phase 2: Create js-monorepo fixture

Create a multi-package monorepo with pnpm workspaces:

**Files to create:**

1. `tests/fixtures/js-monorepo/package.json` - Root package with workspaces
```json
{
  "name": "js-monorepo",
  "private": true,
  "scripts": {
    "test": "echo 'Monorepo tests'"
  }
}
```

2. `tests/fixtures/js-monorepo/pnpm-workspace.yaml`
```yaml
packages:
  - 'packages/*'
```

3. `tests/fixtures/js-monorepo/tsconfig.json` - Root TypeScript config with references
4. `tests/fixtures/js-monorepo/quench.toml`
5. `tests/fixtures/js-monorepo/packages/core/package.json` - Core library package
6. `tests/fixtures/js-monorepo/packages/core/src/index.ts` - Core library exports
7. `tests/fixtures/js-monorepo/packages/core/tests/core.test.ts` - Core tests
8. `tests/fixtures/js-monorepo/packages/cli/package.json` - CLI package depending on core
9. `tests/fixtures/js-monorepo/packages/cli/src/main.ts` - CLI entry point
10. `tests/fixtures/js-monorepo/packages/cli/tests/cli.test.ts` - CLI tests

**Verification:**
- `quench check` detects all packages in workspace
- Workspace detection works for pnpm format
- Package enumeration returns both `core` and `cli`

### Phase 3: Add JavaScript violations

Add JavaScript-specific violations to `fixtures/violations/`:

**Files to create:**

1. `tests/fixtures/violations/js/as-unknown.ts`
```typescript
// VIOLATION: `as unknown` without // CAST: comment
const data = fetch('/api/data') as unknown as UserData;

interface UserData {
  name: string;
}
```

2. `tests/fixtures/violations/js/ts-ignore.ts`
```typescript
// VIOLATION: @ts-ignore in source code (forbidden)
// @ts-ignore
const result: number = "not a number";
```

3. `tests/fixtures/violations/js/eslint-disable.ts`
```typescript
// VIOLATION: eslint-disable without justification comment
/* eslint-disable @typescript-eslint/no-explicit-any */
const value: any = getUntypedValue();

function getUntypedValue() {
  return "test";
}
```

**Update `tests/fixtures/violations/quench.toml`:**

Add JavaScript-specific escape patterns:
```toml
# JavaScript/TypeScript escape patterns
[[check.escapes.patterns]]
name = "ts_as_unknown"
pattern = "as\\s+unknown"
action = "comment"
comment = "// CAST:"
source = ["**/*.ts", "**/*.tsx"]

[[check.escapes.patterns]]
name = "ts_ignore"
pattern = "@ts-ignore"
action = "forbid"
source = ["**/*.ts", "**/*.tsx"]

[javascript.suppress]
check = "comment"
```

**Verification:**
- Escapes check fails on `as-unknown.ts` (missing CAST comment)
- Escapes check fails on `ts-ignore.ts` (forbidden pattern)
- Suppress check fails on `eslint-disable.ts` (missing justification)

### Phase 4: Update fixture README

Update `tests/fixtures/CLAUDE.md` to document new fixtures:

1. Add `js-simple/` and `js-monorepo/` to the Fixture Index table
2. Add detailed sections describing each fixture
3. Add JavaScript violations to the violations table

## Key Implementation Details

### Fixture Design Principles

1. **Minimal but complete**: Each fixture should be as small as possible while still exercising the intended behavior
2. **No real dependencies**: Do not include `node_modules/` or lock files; use mock scripts for `npm test`
3. **Consistent structure**: Follow TypeScript + src/tests convention matching industry standards
4. **Self-documenting**: File contents should make the test scenario obvious

### Workspace Detection

The `js-monorepo/` fixture uses pnpm workspaces format:
- `pnpm-workspace.yaml` with `packages: ['packages/*']` glob
- Each package has its own `package.json` with name field
- Root `package.json` is marked `private: true`

This tests the pnpm detection path. The existing `javascript/workspace-npm/` fixture tests npm workspaces (using `workspaces` field in package.json).

### Violation Patterns

JavaScript violations follow established patterns from Go/Rust:

| Pattern | Escape Type | Comment Required | Scope |
|---------|-------------|------------------|-------|
| `as unknown` | Type escape | `// CAST:` | `.ts`/`.tsx` |
| `@ts-ignore` | Type bypass | (forbidden) | `.ts`/`.tsx` |
| `eslint-disable` | Lint suppress | Justification | All JS/TS |
| `biome-ignore` | Lint suppress | Justification | All JS/TS |

## Verification Plan

### Unit Verification (Phase 1-3)

After creating each fixture, verify:
```bash
# Check fixture structure
ls -la tests/fixtures/js-simple/
ls -la tests/fixtures/js-monorepo/

# Verify quench can process them (when adapter is implemented)
cargo run -- check tests/fixtures/js-simple
cargo run -- check tests/fixtures/js-monorepo
cargo run -- check tests/fixtures/violations
```

### Integration Verification

Once Phase 493 (JavaScript Detection) is complete:
```bash
# Run behavioral specs that use these fixtures
cargo test --test specs javascript
```

### Fixture Checklist

- [ ] `js-simple/` has package.json, tsconfig.json, src/, tests/
- [ ] `js-simple/` passes `quench check` with no violations
- [ ] `js-monorepo/` has root workspace config and 2 packages
- [ ] `js-monorepo/` workspace detection enumerates both packages
- [ ] `violations/js/as-unknown.ts` triggers escape check failure
- [ ] `violations/js/ts-ignore.ts` triggers forbidden pattern failure
- [ ] `violations/js/eslint-disable.ts` triggers suppress check failure
- [ ] `CLAUDE.md` documents all new fixtures
