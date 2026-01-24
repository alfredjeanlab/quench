#!/bin/bash
# Generate benchmark test fixtures for tests correlation check
set -euo pipefail

FIXTURES_DIR="tests/fixtures"

capitalize() {
    local str="$1"
    echo "$(echo "${str:0:1}" | tr '[:lower:]' '[:upper:]')${str:1}"
}

generate_source_file() {
    local name=$1
    local num=$2
    local Name=$(capitalize "$name")
    cat << EOF
// SPDX-License-Identifier: MIT

pub fn ${name}_func_${num}(x: i32) -> i32 {
    x + ${num}
}

pub fn ${name}_helper_${num}(s: &str) -> String {
    format!("{}: {}", "${name}", s)
}

pub struct ${Name}Data${num} {
    pub value: i32,
    pub name: String,
}

impl ${Name}Data${num} {
    pub fn new(value: i32) -> Self {
        Self { value, name: "${name}".to_string() }
    }
}
EOF
}

generate_test_file() {
    local crate_name=$1
    local name=$2
    local num=$3
    local Name=$(capitalize "$name")
    cat << EOF
use ${crate_name}::${name}::{${name}_func_${num}, ${Name}Data${num}};

#[test]
fn test_${name}_func_${num}() {
    assert_eq!(${name}_func_${num}(10), 10 + ${num});
}

#[test]
fn test_${Name}Data${num}_new() {
    let data = ${Name}Data${num}::new(42);
    assert_eq!(data.value, 42);
}
EOF
}

generate_lib_rs() {
    local modules=$1
    echo "// SPDX-License-Identifier: MIT"
    echo ""
    for mod in $modules; do
        echo "pub mod ${mod};"
    done
}

# Generate medium fixture (50 files)
generate_medium() {
    echo "Generating bench-tests-medium..."
    local dir="${FIXTURES_DIR}/bench-tests-medium"
    rm -rf "$dir"
    mkdir -p "$dir/src" "$dir/tests"

    # Generate quench.toml
    cat > "$dir/quench.toml" << 'EOF'
version = 1

[project]
name = "bench-tests-medium"

[check.tests.commit]
check = "off"
EOF

    # Generate Cargo.toml
    cat > "$dir/Cargo.toml" << 'EOF'
[package]
name = "bench-tests-medium"
version = "0.1.0"
edition = "2021"

[lib]
path = "src/lib.rs"
EOF

    # Generate 50 modules with tests
    local modules=""
    for i in $(seq 1 50); do
        local name="module${i}"
        modules="${modules} ${name}"
        generate_source_file "$name" "$i" > "$dir/src/${name}.rs"
        generate_test_file "bench_tests_medium" "$name" "$i" > "$dir/tests/${name}_tests.rs"
    done

    # Generate lib.rs
    generate_lib_rs "$modules" > "$dir/src/lib.rs"
    echo "Created bench-tests-medium with 50 source files and 50 test files"
}

# Generate large fixture (500 files)
generate_large() {
    echo "Generating bench-tests-large..."
    local dir="${FIXTURES_DIR}/bench-tests-large"
    rm -rf "$dir"
    mkdir -p "$dir/src" "$dir/tests"

    # Generate quench.toml
    cat > "$dir/quench.toml" << 'EOF'
version = 1

[project]
name = "bench-tests-large"

[check.tests.commit]
check = "off"
EOF

    # Generate Cargo.toml
    cat > "$dir/Cargo.toml" << 'EOF'
[package]
name = "bench-tests-large"
version = "0.1.0"
edition = "2021"

[lib]
path = "src/lib.rs"
EOF

    # Generate 500 modules with tests in multiple subdirectories
    local subdirs="core api utils handlers models services"

    for subdir in $subdirs; do
        mkdir -p "$dir/src/$subdir" "$dir/tests/$subdir"
    done

    for i in $(seq 1 500); do
        # Distribute across subdirectories
        local subdir_idx=$(( (i - 1) % 6 ))
        local subdir=$(echo $subdirs | cut -d' ' -f$((subdir_idx + 1)))
        local name="item${i}"

        generate_source_file "$name" "$i" > "$dir/src/${subdir}/${name}.rs"
        generate_test_file "bench_tests_large::${subdir}" "$name" "$i" > "$dir/tests/${subdir}/${name}_tests.rs"
    done

    # Generate submodule mod.rs files
    for subdir in $subdirs; do
        echo "// SPDX-License-Identifier: MIT" > "$dir/src/${subdir}/mod.rs"
        for f in "$dir/src/${subdir}"/item*.rs; do
            [ -f "$f" ] || continue
            local name=$(basename "$f" .rs)
            echo "pub mod ${name};" >> "$dir/src/${subdir}/mod.rs"
        done
    done

    # Generate lib.rs
    cat > "$dir/src/lib.rs" << 'EOF'
// SPDX-License-Identifier: MIT

pub mod core;
pub mod api;
pub mod utils;
pub mod handlers;
pub mod models;
pub mod services;
EOF

    echo "Created bench-tests-large with 500 source files and 500 test files"
}

# Generate worst-case fixture (pathological patterns)
generate_worst_case() {
    echo "Generating bench-tests-worst-case..."
    local dir="${FIXTURES_DIR}/bench-tests-worst-case"
    rm -rf "$dir"
    mkdir -p "$dir/src" "$dir/tests" "$dir/test"

    # Generate quench.toml with many test patterns
    cat > "$dir/quench.toml" << 'EOF'
version = 1

[project]
name = "bench-tests-worst-case"

[check.tests.commit]
check = "off"
test_patterns = [
    "tests/**/*",
    "test/**/*",
    "**/*_test.*",
    "**/*_tests.*",
    "**/*.spec.*",
    "**/test_*.*",
    "**/*Test.*",
    "**/*Tests.*",
]
source_patterns = [
    "src/**/*",
    "lib/**/*",
    "crates/**/*",
]
EOF

    # Generate Cargo.toml
    cat > "$dir/Cargo.toml" << 'EOF'
[package]
name = "bench-tests-worst-case"
version = "0.1.0"
edition = "2021"

[lib]
path = "src/lib.rs"
EOF

    # Deep nesting - 20 levels
    local nested_path="$dir/src"
    for i in $(seq 1 20); do
        nested_path="${nested_path}/level${i}"
        mkdir -p "$nested_path"
        local next=$((i+1))
        if [ $i -lt 20 ]; then
            cat > "${nested_path}/mod.rs" << EOF
// SPDX-License-Identifier: MIT
pub mod level${next};
pub fn nested_func_${i}() -> i32 { $i }
EOF
        else
            cat > "${nested_path}/mod.rs" << 'EOF'
// SPDX-License-Identifier: MIT
pub fn deepest_func() -> i32 { 42 }
EOF
        fi
    done

    # Many test naming patterns (tests for same source file in different locations)
    mkdir -p "$dir/src/multi"
    cat > "$dir/src/multi/target.rs" << 'EOF'
// SPDX-License-Identifier: MIT
pub fn target_func() -> i32 { 42 }
EOF

    # Various test file naming patterns
    cat > "$dir/tests/target_tests.rs" << 'EOF'
#[test]
fn test_target() { assert!(true); }
EOF

    cat > "$dir/tests/target_test.rs" << 'EOF'
#[test]
fn test_target() { assert!(true); }
EOF

    cat > "$dir/test/target_tests.rs" << 'EOF'
#[test]
fn test_target() { assert!(true); }
EOF

    cat > "$dir/tests/test_target.rs" << 'EOF'
#[test]
fn test_target() { assert!(true); }
EOF

    # Files that match multiple patterns
    for i in $(seq 1 30); do
        cat > "$dir/src/component${i}.rs" << EOF
// SPDX-License-Identifier: MIT
pub fn component${i}_func() -> i32 { $i }
EOF
        cat > "$dir/tests/component${i}_tests.rs" << EOF
#[test]
fn test_component${i}() { assert!(true); }
EOF
    done

    # Generate lib.rs
    cat > "$dir/src/lib.rs" << 'EOF'
// SPDX-License-Identifier: MIT

pub mod level1;
pub mod multi;
EOF

    for i in $(seq 1 30); do
        echo "pub mod component${i};" >> "$dir/src/lib.rs"
    done

    # Add multi/mod.rs
    cat > "$dir/src/multi/mod.rs" << 'EOF'
// SPDX-License-Identifier: MIT
pub mod target;
EOF

    echo "Created bench-tests-worst-case with deep nesting and multiple patterns"
}

# Main
cd "$(dirname "$0")/../.."

case "${1:-all}" in
    medium)
        generate_medium
        ;;
    large)
        generate_large
        ;;
    worst-case)
        generate_worst_case
        ;;
    all)
        generate_medium
        generate_large
        generate_worst_case
        ;;
    *)
        echo "Usage: $0 [medium|large|worst-case|all]"
        exit 1
        ;;
esac

echo "Done!"
