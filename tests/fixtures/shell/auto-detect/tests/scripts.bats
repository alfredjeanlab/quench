#!/usr/bin/env bats

@test "build script runs" {
    run ./scripts/build.sh
    [ "$status" -eq 0 ]
}
