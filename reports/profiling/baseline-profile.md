# Baseline Profiling Report

Date: (run profile.sh to generate)
Commit: (run profile.sh to generate)

## Environment

- Hardware: (fill in after profiling)
- OS: (fill in after profiling)
- Rust: (fill in after profiling)

## Cold Run Profile

Fixture: stress-monorepo (~85K LOC)
Time: XXXms

### Time Breakdown

| Phase | Time (ms) | % of Total |
|-------|-----------|------------|
| File discovery | | |
| File reading | | |
| Pattern matching | | |
| Cache write | | |
| Output | | |

### Top Functions

1. `function_name` - XX%
2. ...

### Observations

- (fill in after profiling)

## Warm Run Profile

Time: XXXms

### Time Breakdown

| Phase | Time (ms) | % of Total |
|-------|-----------|------------|
| Cache load | | |
| File mtime check | | |
| Output | | |

### Top Functions

1. ...

### Observations

- (fill in after profiling)

## Identified Hotspots

### Hotspot 1: [Name]

**Where:** `src/file.rs:line`
**Time:** X% of total
**Root Cause:**
**Fix:**

## Recommendations

### P1 Optimizations (If Justified)

- [ ] Walker tuning:
- [ ] File list caching:

### P2 Optimizations (If Justified)

- [ ] Pattern combining:
- [ ] Literal prefiltering:

### Defer (Not Needed)

- P3/P4 micro-optimizations: Current memory and performance well within targets
