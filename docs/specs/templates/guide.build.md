# Build Configuration Guide

Configuration reference for the `build` check.

## Global Size Threshold

```toml
[check.build]
check = "error"
size_max = "10 MB"
```

## Per-Target Size Thresholds

```toml
[check.build]
check = "error"
size_max = "10 MB"  # Default

[check.build.target.myapp]
size_max = "5 MB"   # Stricter for main app

[check.build.target.myserver]
size_max = "15 MB"  # More lenient for server
```

## Time Thresholds

```toml
[check.build]
check = "error"
time_cold_max = "60s"  # Clean build time limit
time_hot_max = "5s"    # Incremental build time limit
```

## Explicit Targets

Override auto-detection with an explicit target list.

```toml
[check.build]
check = "error"
targets = ["myapp", "myserver"]
size_max = "10 MB"
```

## JavaScript/TypeScript Bundle Analysis

```toml
[check.build]
check = "error"

[check.build.bundle]
dep_size_max = "500 KB"
forbid = ["moment", "lodash"]
chunk_max = "250 KB"
```

## Complete Example

```toml
[check.build]
check = "error"
targets = ["myapp", "myserver"]
size_max = "10 MB"
time_cold_max = "60s"
time_hot_max = "5s"

[check.build.target.myapp]
size_max = "5 MB"

[check.build.target.myserver]
size_max = "15 MB"

[check.build.bundle]
dep_size_max = "500 KB"
forbid = ["moment", "lodash"]
chunk_max = "250 KB"
```
