#!/usr/bin/env bats

setup() {
    cargo build --quiet 2>/dev/null || true
}

@test "mixed shows usage without args" {
    run cargo run --quiet --
    [ "$status" -eq 0 ]
    [[ "$output" == *"Usage"* ]]
}

@test "mixed hello prints greeting" {
    run cargo run --quiet -- hello
    [ "$status" -eq 0 ]
    [[ "$output" == *"Hello"* ]]
}
