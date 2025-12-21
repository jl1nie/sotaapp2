#!/bin/bash
# BUILD-* Tests: Verify build artifacts in Docker image

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/lib.sh"

IMAGE="${1:?Usage: $0 <image-name>}"

log_header "BUILD Tests - Build Artifacts"

# BUILD-001: Executable exists
run_test "BUILD-001" "Executable file exists" \
  docker_run_shell "$IMAGE" "test -f /app/target/release/app"

# BUILD-002: Executable is runnable
run_test "BUILD-002" "Executable has correct permissions" \
  docker_run_shell "$IMAGE" "test -x /app/target/release/app"

# BUILD-003: Migrations directory exists and has content
run_test "BUILD-003" "Migrations directory exists with SQL files" \
  docker_run_shell "$IMAGE" "ls /app/migrations/*.sql"

# BUILD-004: Static directory exists
run_test "BUILD-004" "Static directory exists" \
  docker_run_shell "$IMAGE" "test -d /app/static"

# BUILD-005: index.html exists
run_test "BUILD-005" "index.html exists" \
  docker_run_shell "$IMAGE" "test -f /app/static/index.html"

print_summary
