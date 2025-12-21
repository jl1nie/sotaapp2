#!/bin/bash
# RUNTIME-* Tests: Verify runtime dependencies

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/lib.sh"

IMAGE="${1:?Usage: $0 <image-name>}"

log_header "RUNTIME Tests - Runtime Dependencies"

# RUNTIME-001: SSL/TLS connectivity (ca-certificates)
# This test catches the missing ca-certificates issue
run_test "RUNTIME-001" "SSL/TLS connectivity to external HTTPS endpoint" \
  docker_run_shell "$IMAGE" \
    "apt-get update -qq && apt-get install -qq -y curl > /dev/null 2>&1 && \
     curl -sf https://services.swpc.noaa.gov/text/daily-geomagnetic-indices.txt > /dev/null"

# RUNTIME-002: libssl is available
run_test "RUNTIME-002" "libssl library is linked" \
  docker_run_shell "$IMAGE" "ldd /app/target/release/app | grep -q libssl"

# RUNTIME-003: CA certificates directory exists
run_test "RUNTIME-003" "CA certificates directory exists" \
  docker_run_shell "$IMAGE" "test -d /etc/ssl/certs && ls /etc/ssl/certs/*.pem > /dev/null 2>&1"

print_summary
