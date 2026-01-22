#!/bin/bash
# Script with shell escape hatch violations

# VIOLATION: shellcheck disable without justification
# shellcheck disable=SC2086
echo $UNQUOTED_VAR

# VIOLATION: set +e without OK comment
set +e
risky_command_that_might_fail
set -e

# Proper set +e with comment (should pass)
# OK: We intentionally ignore errors here to collect all results
set +e
optional_command || true
set -e
