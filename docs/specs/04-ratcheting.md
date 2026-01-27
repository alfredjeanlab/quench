# Ratcheting Specification

Ratcheting prevents quality regressions while allowing gradual improvement.

## Concept

A ratchet only turns one way:
- Metrics can improve (coverage up, escapes down, binary smaller)
- Metrics cannot regress past the baseline
- When metrics improve, the baseline auto-updates

This enables incremental quality improvement without manual threshold maintenance.

## Supported Metrics

| Metric | Good Direction | Ratchet Behavior | Default |
|--------|----------------|------------------|---------|
| Coverage | Higher | Floor rises with improvements | On |
| Escape hatch counts | Lower | Ceiling drops as you clean up | On |
| Binary size | Smaller | Ceiling drops on optimization | Off |
| Build time (cold) | Faster | Ceiling drops on improvement | Off |
| Build time (hot) | Faster | Ceiling drops on improvement | Off |
| Test time (total) | Faster | Ceiling drops on improvement | Off |
| Test time (avg) | Faster | Ceiling drops on improvement | Off |
| Test time (max) | Faster | Ceiling drops on improvement | Off |

## Baseline Storage

Ratcheting requires a stored baseline. By default, quench uses git notes (`refs/notes/quench`) to store per-commit metrics. This provides automatic history tracking without committing baseline files.

Configure the baseline source in `[git]`:

```toml
[git]
# Baseline source: "notes" (default) or path to file
baseline = "notes"

# Optional: explicit file path for file-based baseline
# baseline = ".quench/baseline.json"
```

### Git Notes (Default)

Git notes store metrics attached to each commit:
- Per-commit history without polluting commit history
- Easy to push/pull: `git push origin refs/notes/quench`
- Works well with CI: each commit gets its own baseline
- Falls back to file baseline when notes unavailable

### File-Based Baseline

For teams preferring committed baselines:
- Single `.quench/baseline.json` file
- Visible in code review
- Works without git notes support
- Enable with `baseline = ".quench/baseline.json"`

### Local Cache

When using git notes, `.quench/latest.json` caches the most recent metrics locally for faster access. This file is auto-generated and should be gitignored.

## Configuration

Enable ratcheting per-metric:

```toml
[ratchet]
check = "error"          # error | warn | off

# Which metrics to ratchet (defaults shown)
coverage = true          # Coverage can't drop
escapes = true           # Escape counts can't increase
binary_size = false      # Opt-in: binaries can't grow

# Build time (granular)
build_time_cold = false
build_time_hot = false

# Test time (granular)
test_time_total = false
test_time_avg = false
test_time_max = false    # e.g., ratchet only slowest test
```

### Tolerance

Allow small regressions to handle noise:

```toml
[ratchet]
coverage_tolerance = 0.5       # Allow 0.5% coverage drop
binary_size_tolerance = "100KB" # Allow 100KB size increase
build_time_tolerance = "5s"     # Allow 5s build time increase
```

### Stale Baseline Warning

Configure when to warn about old baselines:

```toml
[ratchet]
stale_days = 30    # Warn if baseline > 30 days old (default)
stale_days = 0     # Disable stale warning
```

When the baseline is older than `stale_days`, a warning is printed to stderr:

```
warning: baseline is 45 days old. Consider refreshing with --fix.
```

This helps teams maintain accurate baselines that reflect current project norms.

### Warn Level

Use warn level to see regressions without failing:

```toml
[ratchet]
check = "warn"     # Report regressions, exit 0
```

This is useful for:
- Gradual adoption of ratcheting
- Informational CI runs on feature branches
- Understanding impact before enforcement

When check level is `warn`, regressions show `WARN` instead of `FAIL` and the exit code remains 0:

```
ratchet: WARN
  escapes.unsafe: 5 (max: 3 from baseline)
    Reduce unsafe blocks or add // SAFETY: comments.
```

### Per-Package

Ratcheting respects per-package breakdown:

```toml
[ratchet.package.core]
coverage = true          # Ratchet core coverage

[ratchet.package.cli]
coverage = false         # Don't ratchet CLI coverage (still developing)
```

## Behavior

### On Check (quench)

```
escapes: FAIL
  unsafe: 5 (max: 3 from baseline)
    Escape hatch count increased. Clean up or update baseline.
```

### On Fix (quench --fix)

When `--fix` is run and metrics have improved:

```
ratchet: updated baseline
  coverage: 78.4% → 82.1% (new floor)
  escapes.unsafe: 5 → 3 (new ceiling)
```

The baseline file is updated automatically.

### CI Workflow

Using git notes (default, recommended):

```yaml
- name: Fetch baseline notes
  run: git fetch origin refs/notes/quench:refs/notes/quench || true

- name: Check quality
  run: quench check --ci

- name: Update baseline on main
  if: github.ref == 'refs/heads/main'
  run: |
    quench check --ci --fix
    git push origin refs/notes/quench
```

Using file-based baseline (alternative):

```yaml
- name: Check quality
  run: quench check --ci --no-notes

- name: Update baseline on main
  if: github.ref == 'refs/heads/main'
  run: |
    quench check --ci --fix --no-notes
    git add .quench/baseline.json
    git commit -m "chore: update quality baseline" || true
    git push
```

## Output

### Pass (within baseline)

```
coverage: 82.1% (baseline: 78.4%) ✓
escapes: 3 unsafe (baseline: 5) ✓
```

### Fail (regression)

```
coverage: FAIL
  76.2% (baseline: 78.4%, min with tolerance: 77.9%)
    Coverage dropped below ratcheted baseline.
```

### JSON Output

```json
{
  "ratchet": {
    "coverage": {
      "current": 76.2,
      "baseline": 78.4,
      "tolerance": 0.5,
      "min_allowed": 77.9,
      "passed": false
    },
    "escapes": {
      "unsafe": {
        "current": 3,
        "baseline": 5,
        "passed": true,
        "improved": true
      }
    }
  }
}
```

## Baseline File Format

```json
{
  "version": 1,
  "updated": "2026-01-21T10:30:00Z",
  "commit": "abc123",
  "metrics": {
    "coverage": {
      "total": 78.4,
      "by_package": {
        "core": 82.3,
        "cli": 68.9
      }
    },
    "escapes": {
      "source": {
        "unsafe": 5,
        "unwrap": 0,
        "allow": 12
      }
    },
    "binary_size": {
      "quench": 4404019
    },
    "build_time": {
      "cold": 45.2,
      "hot": 1.8
    },
    "test_time": {
      "total": 12.4,
      "avg": 0.045,
      "max": 2.1
    }
  }
}
```

## Notes

- Coverage and escapes ratcheting are **on by default**; other metrics are opt-in
- Tolerance prevents failing on noise (especially build time)
- Per-package ratcheting allows different policies for different maturity levels
- `--fix` updates baseline only when metrics improve (never on regression)
- Baseline should be committed to repo for team visibility
