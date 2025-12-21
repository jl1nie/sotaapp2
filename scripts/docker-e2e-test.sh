#!/bin/bash
# Docker E2E Test Suite
# Runs all E2E tests against a Docker image
#
# Usage:
#   ./scripts/docker-e2e-test.sh <image-name> [port]
#
# Examples:
#   ./scripts/docker-e2e-test.sh sotaapp2:test
#   ./scripts/docker-e2e-test.sh jl1nie/sotaapp2:latest 18080

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/e2e/lib.sh"

IMAGE="${1:?Usage: $0 <image-name> [port]}"
PORT="${2:-18080}"

log_header "Docker E2E Test Suite"
log_info "Image: $IMAGE"
log_info "Test port: $PORT"

# Check requirements
check_requirements

# Track overall results
TOTAL_PASSED=0
TOTAL_FAILED=0

run_phase() {
  local phase_name="$1"
  local script="$2"
  shift 2

  log_header "Phase: $phase_name"

  if "$script" "$@"; then
    log_info "Phase '$phase_name' completed successfully"
  else
    log_error "Phase '$phase_name' failed"
    return 1
  fi
}

# Phase 1: Build artifacts
if run_phase "Build Artifacts" "$SCRIPT_DIR/e2e/test-build.sh" "$IMAGE"; then
  ((TOTAL_PASSED++)) || true
else
  ((TOTAL_FAILED++)) || true
fi

# Phase 2: Runtime dependencies
if run_phase "Runtime Dependencies" "$SCRIPT_DIR/e2e/test-runtime.sh" "$IMAGE"; then
  ((TOTAL_PASSED++)) || true
else
  ((TOTAL_FAILED++)) || true
fi

# Phase 3: Application commands
if run_phase "Application Commands" "$SCRIPT_DIR/e2e/test-app.sh" "$IMAGE"; then
  ((TOTAL_PASSED++)) || true
else
  ((TOTAL_FAILED++)) || true
fi

# Phase 4: Server and API (only if previous phases passed)
if [ "$TOTAL_FAILED" -eq 0 ]; then
  if run_phase "Server & API" "$SCRIPT_DIR/e2e/test-server.sh" "$IMAGE" "$PORT"; then
    ((TOTAL_PASSED++)) || true
  else
    ((TOTAL_FAILED++)) || true
  fi
else
  log_warn "Skipping Server & API tests due to previous failures"
fi

# Final summary
echo ""
log_header "E2E Test Suite Summary"
log_info "Phases passed: $TOTAL_PASSED"
if [ "$TOTAL_FAILED" -gt 0 ]; then
  log_error "Phases failed: $TOTAL_FAILED"
  exit 1
else
  log_info "Phases failed: $TOTAL_FAILED"
  log_info "All E2E tests passed!"
  exit 0
fi
