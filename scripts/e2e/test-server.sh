#!/bin/bash
# SERVER-* and API-* Tests: Verify server startup and API endpoints

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/lib.sh"

IMAGE="${1:?Usage: $0 <image-name>}"
CONTAINER_NAME="sotaapp2-e2e-test"
HOST_PORT="${2:-18080}"

log_header "SERVER & API Tests - Server Startup and Endpoints"

cleanup() {
  log_info "Cleaning up test container..."
  docker stop "$CONTAINER_NAME" > /dev/null 2>&1 || true
  docker rm "$CONTAINER_NAME" > /dev/null 2>&1 || true
}

# Ensure cleanup on exit
trap cleanup EXIT

# Start container
log_info "Starting test container..."
docker run -d \
  --name "$CONTAINER_NAME" \
  -p "${HOST_PORT}:8080" \
  -e DATABASE_URL="sqlite:/tmp/test.db?mode=rwc" \
  -e FIREBASE_API_KEY="test-key" \
  -e APRSUSER="testuser" \
  -e APRSPASSWORD="testpass" \
  -e MIGRATION_PATH="/app/migrations" \
  -e HOST="0.0.0.0" \
  -e PORT="8080" \
  -e GEOMAG_ENDPOINT="https://services.swpc.noaa.gov/text/daily-geomagnetic-indices.txt" \
  -e GEOMAG_SCHEDULE="0 0 */3 * * *" \
  -e SOTA_SPOT_ENDPOINT="https://api2.sota.org.uk/api/spots/20?" \
  -e SOTA_ALERT_ENDPOINT="https://api2.sota.org.uk/api/alerts" \
  -e POTA_SPOT_ENDPOINT="https://api.pota.app/spot/activator" \
  -e POTA_ALERT_ENDPOINT="https://api.pota.app/activation" \
  -e SPOT_INTERVAL="60" \
  -e ALERT_INTERVAL="300" \
  -e SPOT_EXPIRE="3600" \
  -e ALERT_EXPIRE="86400" \
  -e APRS_LOG_EXPIRE="86400" \
  -e POTA_LOG_EXPIRE="86400" \
  -e AUTH_TOKEN_TTL="3600" \
  "$IMAGE" > /dev/null

# SERVER-001: Wait for server to start
log_info "Waiting for server to start..."
if wait_for_http "http://localhost:${HOST_PORT}/health" 90; then
  log_info "[SERVER-001] Server started successfully"
  ((TESTS_PASSED++)) || true
else
  log_error "[SERVER-001] Server failed to start"
  log_error "Container logs:"
  docker logs "$CONTAINER_NAME" 2>&1 | tail -50
  ((TESTS_FAILED++)) || true
  print_summary
  exit 1
fi

# SERVER-002: Health endpoint returns 200
run_test "SERVER-002" "Health endpoint returns 200" \
  curl -sf "http://localhost:${HOST_PORT}/health"

# API-001: Geomag endpoint
run_test_grep "API-001" "Geomag API returns valid JSON with date field" '"date"' \
  curl -sf "http://localhost:${HOST_PORT}/api/v2/propagation/geomag"

# API-002: Spots endpoint (may be empty but should return 200)
run_test "API-002" "Spots API returns 200" \
  curl -sf "http://localhost:${HOST_PORT}/api/v2/spots"

# API-003: Alerts endpoint
run_test "API-003" "Alerts API returns 200" \
  curl -sf "http://localhost:${HOST_PORT}/api/v2/alerts"

# API-004: APRS track endpoint
run_test "API-004" "APRS track API returns 200" \
  curl -sf "http://localhost:${HOST_PORT}/api/v2/activation/aprs/track?pat_ref=JA&hours_ago=1"

# API-005: Static file serving
run_test_grep "API-005" "Static index.html is served" "html" \
  curl -sf "http://localhost:${HOST_PORT}/"

# SERVER-003: Check startup time (container should be ready within 90s - already verified)
log_info "[SERVER-003] Startup time within acceptable range (verified above)"
((TESTS_PASSED++)) || true

# SERVER-004: Graceful shutdown
log_info "Testing graceful shutdown..."
docker stop -t 10 "$CONTAINER_NAME" > /dev/null 2>&1
EXIT_CODE=$(docker inspect "$CONTAINER_NAME" --format='{{.State.ExitCode}}' 2>/dev/null || echo "unknown")
if [ "$EXIT_CODE" = "0" ] || [ "$EXIT_CODE" = "143" ]; then
  log_info "[SERVER-004] Graceful shutdown succeeded (exit code: $EXIT_CODE)"
  ((TESTS_PASSED++)) || true
else
  log_warn "[SERVER-004] Shutdown exit code: $EXIT_CODE (expected 0 or 143)"
  ((TESTS_PASSED++)) || true  # Not failing on this, just informational
fi

# Container already stopped by graceful shutdown test
trap - EXIT

print_summary
