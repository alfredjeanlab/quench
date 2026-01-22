#!/usr/bin/env bats

@test "build script runs successfully" {
    run ./scripts/build.sh
    [ "$status" -eq 0 ]
    [[ "$output" == *"Build complete"* ]]
}

@test "deploy script accepts target argument" {
    run ./scripts/deploy.sh staging
    [ "$status" -eq 0 ]
    [[ "$output" == *"staging"* ]]
}
