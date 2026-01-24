# Docs Check Specification

The `docs` check validates documentation across the project.

## Purpose

Ensure documentation is well-organized and kept in sync with code:
- **TOC validation**: Directory trees in markdown reference existing files
- **Link validation**: Markdown links point to existing files
- **Specs validation**: Structure, index, sections in `docs/specs/`
- **Doc commit** (CI): Feature commits have corresponding doc updates

## Fast Mode: TOC Validation

Validates that directory tree structures in markdown files reference existing files.

### What Gets Validated

Fenced code blocks containing directory trees. Both formats are supported:

**Box-drawing format:**
~~~markdown
```text
src/
├── parser.rs
├── lexer.rs
└── lib.rs
```
~~~

**Indentation format** (spaces or tabs):
~~~markdown
```text
src/
  parser.rs
  lexer.rs
  lib.rs
```
~~~

Each file in the tree is checked for existence.

### Resolution

Resolution applies to the entire tree block. All entries must resolve with the same strategy, or fall back to the next:

1. Relative to the markdown file's directory (`.`/`./` treated as current directory)
2. Relative to project root
3. Strip markdown file's parent directory name prefix (e.g., `quench/` for `/path/to/quench/CLAUDE.md`)

If a strategy resolves some but not all entries, that strategy fails and the next is tried. The error message notes which strategies were attempted.

### Output

```
docs: FAIL
  CLAUDE.md:72: toc path not found: checks/coverage.md
    File does not exist. Update the tree or create the file.
```

### Configuration

```toml
[check.docs.toc]
check = "error"
include = ["**/*.md", "**/*.mdc"]
exclude = ["plans/**", "plan.md", "*_plan.md", "plan_*"]
```

## Fast Mode: Link Validation

Validates that markdown links point to existing files.

### What Gets Validated

Markdown links to local files:

```markdown
See [the parser](src/parser.rs) for details.
Check [configuration](../02-config.md) for options.
```

External URLs (http/https) are not validated.

### Output

```
docs: FAIL
  README.md:45: broken link: docs/old-guide.md
    Linked file does not exist. Update the link or create the file.
```

### Configuration

```toml
[check.docs.links]
check = "error"
include = ["**/*.md", "**/*.mdc"]
exclude = ["plans/**", "plan.md", "*_plan.md", "plan_*"]
```

## Fast Mode: Specs Validation

Validates specification documents in `docs/specs/` or similar directories.

### Spec Directory

Default location: `docs/specs/`

```toml
[check.docs]
path = "docs/specs"
extension = ".md"
```

### Index File

An index file provides an overview and links to individual specs.

**Detection order** (first found wins):
1. `{path}/CLAUDE.md`
2. `docs/CLAUDE.md`
3. `{path}/[00-]{overview,summary,index}.md`
4. `docs/SPECIFICATIONS.md`
5. `docs/SPECS.md`

```toml
[check.docs]
index_file = "docs/specs/CLAUDE.md"       # Or auto-detect
index = "auto"                             # auto | toc | linked | exists
```

| Mode | Behavior |
|------|----------|
| `auto` | Try `toc` first, fall back to `linked` (default) |
| `toc` | Parse directory tree structure in index file |
| `linked` | All spec files must be reachable via markdown links (recursive) |
| `exists` | Index file must exist, no reachability check |

### TOC Format

A fenced code block with a directory tree structure. Both formats are supported:

**Box-drawing format:**
~~~markdown
```
docs/specs/
├── 00-overview.md      # Problem, philosophy
├── 01-cli.md           # Commands and flags
├── checks/
│   ├── agents.md
│   └── tests.md
└── langs/
    └── rust.md
```
~~~

**Indentation format:**
~~~markdown
```
docs/specs/
  00-overview.md        # Problem, philosophy
  01-cli.md             # Commands and flags
  checks/
    agents.md
    tests.md
  langs/
    rust.md
```
~~~

Files listed in the tree must exist. Comments after `#` are ignored.

### Section Validation

Required sections (case-insensitive matching):

```toml
[check.docs]
sections.required = ["Purpose", "Configuration"]
sections.forbid = ["TODO", "Draft*"]
```

With advice:

```toml
[[check.docs.sections.required]]
name = "Purpose"
advice = "What problem this spec addresses"
```

### Content Rules

```toml
[check.docs]
tables = "allow"         # Tables allowed (default)
box_diagrams = "allow"   # ASCII diagrams (default)
mermaid = "allow"        # Mermaid blocks (default)
max_lines = 1000
max_tokens = 20000               # use false to disable
```

### Output (Fast Mode)

```
docs: FAIL
  docs/specs/escapes.md: unreachable from index
    Add a markdown link from index or another linked file.
  docs/specs/escapes.md: missing required section "Purpose"
    Add a "## Purpose" section: What problem this spec addresses
```

## CI Mode: Commit Checking

In `--ci` mode, also checks that feature commits have corresponding documentation updates.

**Disabled by default.** Enable via `[check.docs.commit]`.

### How It Works

1. Examine commits on the branch (compared to base branch)
2. Identify commits with `feat:` or `feat(area):` prefixes
3. Check if the branch includes changes to documentation
4. Report when feature commits lack corresponding doc changes

### Commit Types

Which commit types trigger the documentation requirement:

```toml
[check.docs.commit]
check = "error"
# types = ["feat", "feature", "story", "breaking"]   # default
```

**Note**: `breaking` commits require documentation because breaking changes should always be clearly documented for users.

### Area Mapping

By default, any change in `docs/` satisfies the commit requirement.

Use area mappings to require specific documentation for:
- Scoped commits (e.g., `feat(api):`) - matched by area name
- Source file changes - matched by `source` glob

```toml
[check.docs.area.api]
docs = "docs/api/**"
source = "src/api/**"            # changes here also require docs here

[check.docs.area.cli]
docs = "docs/usage/**"

[check.docs.area.parser]
docs = "docs/specs/parser.md"
source = "crates/parser/**"
```

Area name doubles as commit scope: `feat(api):` triggers the `api` area.

Areas are defined at `[check.docs.area.*]`, separate from commit checking, so they can be reused by other features.

### Check Levels

| Level | Behavior |
|-------|----------|
| `error` | Feature commits must have doc changes (fail if missing) |
| `warn` | Report missing docs but don't fail |
| `off` | Disable commit checking (default) |

```toml
[check.docs.commit]
check = "warn"  # error | warn | off
```

### Output (CI Mode)

```
docs: FAIL
  Branch has feature commits without documentation:
    abc123: feat(api): add export endpoint
    def456: feat: new user settings
  Update docs/ or add area mapping in quench.toml.
```

With area mapping:

```
docs: FAIL
  feat(api) commits require changes in docs/api/**
    No changes found in docs/api/
    Update docs/api/ with the new API functionality.
```

## Configuration

```toml
[check.docs]
check = "error"

# TOC validation (directory trees in markdown)
[check.docs.toc]
check = "error"
include = ["**/*.md", "**/*.mdc"]
exclude = ["plans/**", "plan.md", "*_plan.md", "plan_*"]

# Link validation (markdown links)
[check.docs.links]
check = "error"
include = ["**/*.md", "**/*.mdc"]
exclude = ["plans/**", "plan.md", "*_plan.md", "plan_*"]

# Specs validation
[check.docs.specs]
path = "docs/specs"
extension = ".md"
index = "auto"                   # auto | toc | linked | exists
sections.required = ["Purpose"]
sections.forbid = ["TODO"]
tables = "allow"
max_lines = 1000

# Commit checking (CI mode)
[check.docs.commit]
check = "off"                    # error | warn | off
# types = ["feat", "feature", "story", "breaking"]   # default

# Area mappings (reusable, default: any change in docs/)
[check.docs.area.api]
docs = "docs/api/**"
source = "src/api/**"
```

## JSON Output

```json
{
  "name": "docs",
  "passed": false,
  "violations": [
    {
      "file": "docs/specs/escapes.md",
      "line": null,
      "type": "missing_section",
      "section": "Purpose",
      "advice": "Add a \"## Purpose\" section: What problem this spec addresses"
    },
    {
      "file": "CLAUDE.md",
      "line": 72,
      "type": "broken_toc",
      "path": "checks/coverage.md",
      "advice": "File does not exist. Update the tree or create the file."
    },
    {
      "file": "README.md",
      "line": 45,
      "type": "broken_link",
      "target": "docs/old-guide.md",
      "advice": "Linked file does not exist. Update the link or create the file."
    },
    {
      "file": null,
      "line": null,
      "type": "missing_docs",
      "commit": "abc123",
      "message": "feat(api): add export endpoint",
      "expected_docs": "docs/api/**",
      "advice": "Update docs/api/ with the new API functionality."
    }
  ],
  "metrics": {
    "index_file": "docs/specs/00-overview.md",
    "spec_files": 12,
    "feature_commits": 1,
    "with_docs": 0
  }
}
```

**Violation types**: `missing_section`, `forbidden_section`, `broken_toc`, `broken_link`, `missing_docs`

**Note**: `missing_docs` violations (CI mode) have `file: null` with `commit` field instead.

## Comparison to `agents` Check

| Aspect | `agents` | `docs` |
|--------|----------|--------|
| Purpose | AI agent context | Technical specifications |
| Default path | Project root | `docs/specs/` |
| Tables | Forbidden | Allowed |
| Index file | N/A | Required by default |
| Sync behavior | Yes | No |
| Token limits | Strict (20000) | Relaxed (20000) |
| Correlation | N/A | CI mode (opt-in) |
