#!/bin/bash
# OK: Need to continue on error for cleanup
set +e
cleanup_files
set -e
echo 'done'
