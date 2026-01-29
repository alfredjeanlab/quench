# Docs Configuration Guide

Configuration reference for the `docs` check.

## TOC Validation

```toml
[check.docs.toc]
check = "error"
include = ["**/*.md", "**/*.mdc"]
exclude = ["plans/**", "plan.md", "*_plan.md", "plan_*"]
```

## Link Validation

```toml
[check.docs.links]
check = "error"
include = ["**/*.md", "**/*.mdc"]
exclude = ["plans/**", "plan.md", "*_plan.md", "plan_*"]
```

## Specs Validation

Index validation modes:

- `"auto"` — try TOC first, fall back to linked (default)
- `"toc"` — parse directory tree in index file
- `"linked"` — all specs reachable via markdown links
- `"exists"` — index file must exist, no reachability check

```toml
[check.docs.specs]
check = "error"
path = "docs/specs"
extension = ".md"
index = "auto"
```

## Specs with Index File

```toml
[check.docs.specs]
check = "error"
path = "docs/specs"
index_file = "docs/specs/CLAUDE.md"
index = "auto"
```

## Specs with Required Sections

Section names are matched case-insensitively.

```toml
[check.docs.specs]
check = "error"
path = "docs/specs"
sections.required = ["Purpose", "Configuration"]
sections.forbid = ["TODO", "Draft*"]
```

## Specs with Section Advice

```toml
[check.docs.specs]
check = "error"

[[check.docs.specs.sections.required]]
name = "Purpose"
advice = "What problem this spec addresses"

[[check.docs.specs.sections.required]]
name = "Configuration"
advice = "How to configure this feature"
```

## Specs with Content Rules

```toml
[check.docs.specs]
check = "error"
tables = "allow"
box_diagrams = "allow"
mermaid = "allow"
max_lines = 1000
max_tokens = 20000
```

## Commit Checking (CI Mode)

Disabled by default; enable explicitly. Only runs in `--ci` mode.

```toml
[check.docs.commit]
check = "error"
types = ["feat", "feature", "story", "breaking"]
```

## Area Mappings

Define areas for scoped commits. When source files in a mapped area change,
corresponding documentation must also be updated.

```toml
[check.docs.area.api]
docs = "docs/api/**"
source = "src/api/**"  # Changes here require docs in docs/api/**

[check.docs.area.cli]
docs = "docs/usage/**"
source = "src/cli/**"

[check.docs.area.parser]
docs = "docs/specs/parser.md"
source = "crates/parser/**"
```

## Complete Example

```toml
[check.docs]
check = "error"

[check.docs.toc]
check = "error"
include = ["**/*.md", "**/*.mdc"]
exclude = ["plans/**", "node_modules/**"]

[check.docs.links]
check = "error"
include = ["**/*.md", "**/*.mdc"]
exclude = ["plans/**"]

[check.docs.specs]
check = "error"
path = "docs/specs"
index_file = "docs/specs/CLAUDE.md"
index = "auto"
tables = "allow"
box_diagrams = "allow"
mermaid = "allow"
max_lines = 1000
max_tokens = 20000
sections.forbid = ["TODO", "Draft*"]

[[check.docs.specs.sections.required]]
name = "Purpose"
advice = "What problem this spec addresses"

[[check.docs.specs.sections.required]]
name = "Configuration"
advice = "How to configure this feature"

[check.docs.commit]
check = "error"
types = ["feat", "feature", "story", "breaking"]

[check.docs.area.api]
docs = "docs/api/**"
source = "src/api/**"

[check.docs.area.cli]
docs = "docs/usage/**"
source = "src/cli/**"
```
