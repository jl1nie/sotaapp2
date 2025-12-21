#!/bin/bash
# E2E Test Library - Common functions for Docker E2E tests

set -euo pipefail

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Counters
TESTS_PASSED=0
TESTS_FAILED=0

log_info() { echo -e "${GREEN}[INFO]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }
log_header() { echo -e "\n${BLUE}=== $1 ===${NC}"; }

# Run a test and track results
# Usage: run_test "TEST-ID" "Description" command args...
run_test() {
  local test_id="$1"
  local description="$2"
  shift 2

  if "$@" > /dev/null 2>&1; then
    log_info "[$test_id] $description"
    ((TESTS_PASSED++)) || true
    return 0
  else
    log_error "[$test_id] $description"
    ((TESTS_FAILED++)) || true
    return 1
  fi
}

# Run a test with output capture for pattern matching
# Usage: run_test_grep "TEST-ID" "Description" "pattern" command args...
run_test_grep() {
  local test_id="$1"
  local description="$2"
  local pattern="$3"
  shift 3

  local output
  output=$("$@" 2>&1) || true

  if echo "$output" | grep -q "$pattern"; then
    log_info "[$test_id] $description"
    ((TESTS_PASSED++)) || true
    return 0
  else
    log_error "[$test_id] $description"
    log_error "Expected pattern: $pattern"
    log_error "Actual output: $output"
    ((TESTS_FAILED++)) || true
    return 1
  fi
}

# Run a test expecting failure with specific error message
# Usage: run_test_fail "TEST-ID" "Description" "error_pattern" command args...
run_test_fail() {
  local test_id="$1"
  local description="$2"
  local pattern="$3"
  shift 3

  local output
  local exit_code=0
  output=$("$@" 2>&1) || exit_code=$?

  if [ $exit_code -ne 0 ] && echo "$output" | grep -q "$pattern"; then
    log_info "[$test_id] $description"
    ((TESTS_PASSED++)) || true
    return 0
  else
    log_error "[$test_id] $description"
    log_error "Expected failure with pattern: $pattern"
    log_error "Exit code: $exit_code, Output: $output"
    ((TESTS_FAILED++)) || true
    return 1
  fi
}

# Docker run helper
docker_run() {
  docker run --rm "$@"
}

# Docker run with shell override
docker_run_shell() {
  local image="$1"
  shift
  docker run --rm --entrypoint="" "$image" sh -c "$*"
}

# Print test summary
print_summary() {
  echo ""
  log_header "Test Summary"
  log_info "Passed: $TESTS_PASSED"
  if [ "$TESTS_FAILED" -gt 0 ]; then
    log_error "Failed: $TESTS_FAILED"
    return 1
  else
    log_info "Failed: $TESTS_FAILED"
    return 0
  fi
}

# Wait for HTTP endpoint to be ready
# Usage: wait_for_http "http://localhost:8080/health" 60
wait_for_http() {
  local url="$1"
  local timeout="${2:-60}"
  local start_time=$(date +%s)

  log_info "Waiting for $url (timeout: ${timeout}s)..."

  while true; do
    if curl -sf "$url" > /dev/null 2>&1; then
      log_info "Endpoint $url is ready"
      return 0
    fi

    local elapsed=$(($(date +%s) - start_time))
    if [ "$elapsed" -ge "$timeout" ]; then
      log_error "Timeout waiting for $url"
      return 1
    fi

    sleep 2
  done
}

# Check if required commands are available
check_requirements() {
  local missing=()

  for cmd in docker curl jq; do
    if ! command -v "$cmd" &> /dev/null; then
      missing+=("$cmd")
    fi
  done

  if [ ${#missing[@]} -gt 0 ]; then
    log_error "Missing required commands: ${missing[*]}"
    exit 1
  fi
}
