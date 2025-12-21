#!/bin/bash
# APP-* Tests: Verify application commands

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/lib.sh"

IMAGE="${1:?Usage: $0 <image-name>}"

log_header "APP Tests - Application Commands"

# APP-001: --help displays usage
run_test_grep "APP-001" "Help command shows usage" "Usage:" \
  docker_run "$IMAGE" --help

# APP-002: Missing env vars shows appropriate error
run_test_grep "APP-002" "Missing env vars shows error message" "必須環境変数が設定されていません" \
  docker_run "$IMAGE" serve

# APP-003: Database migration succeeds
run_test "APP-003" "Database migration succeeds" \
  docker_run \
    -e DATABASE_URL="sqlite:/tmp/test.db?mode=rwc" \
    -e FIREBASE_API_KEY="test-key" \
    -e APRSUSER="testuser" \
    -e APRSPASSWORD="testpass" \
    -e MIGRATION_PATH="/app/migrations" \
    "$IMAGE" db migrate

# APP-004: db subcommand help
run_test_grep "APP-004" "db subcommand shows help" "Database maintenance" \
  docker_run "$IMAGE" db --help

print_summary
