#!/usr/bin/env bats
# shellcheck disable=SC2034
UNUSED_VAR=1

@test "build script runs" {
    run ./scripts/build.sh
    [ "$status" -eq 0 ]
}
