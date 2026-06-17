#!/usr/bin/env bash
set -euo pipefail

# P0 production-readiness CI gates.
# Lightweight shell checks that prevent regressions of security-critical
# behaviors implemented in tasks 1.1-6.1. This script must not invoke any
# compiler, package manager, or Docker build.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

cd "${PROJECT_ROOT}"

ERRORS=0

fail() {
  echo "FAIL: $1" >&2
  ERRORS=$((ERRORS + 1))
}

pass() {
  echo "PASS: $1"
}

# 1. Frontend container runs as a non-root user.
if grep -qE '^USER[[:space:]]+rustchat([[:space:]]|$)' docker/frontend.Dockerfile; then
  pass "docker/frontend.Dockerfile sets USER rustchat"
else
  fail "docker/frontend.Dockerfile must set USER rustchat"
fi

if grep -qE '^USER[[:space:]]+root([[:space:]]|$)' docker/frontend.Dockerfile; then
  fail "docker/frontend.Dockerfile must not set USER root"
else
  pass "docker/frontend.Dockerfile does not set USER root"
fi

# 2. Dev CORS is documented as opt-in.
if grep -qE '^\s*#?\s*RUSTCHAT_ALLOW_DEV_CORS\s*=' .env.example; then
  pass ".env.example documents RUSTCHAT_ALLOW_DEV_CORS"
else
  fail ".env.example must document RUSTCHAT_ALLOW_DEV_CORS"
fi

# 3. Retention orphan scan env vars are documented.
for var in \
  RUSTCHAT_RETENTION_ORPHAN_SCAN_ENABLED \
  RUSTCHAT_RETENTION_ORPHAN_SCAN_INTERVAL_HOURS \
  RUSTCHAT_RETENTION_ORPHAN_SCAN_PAGE_SIZE \
  RUSTCHAT_RETENTION_ORPHAN_SCAN_PAGE_DELAY_MS; do
  if grep -qE "^\s*#?\s*${var}\s*=" .env.example; then
    pass ".env.example documents ${var}"
  else
    fail ".env.example must document ${var}"
  fi
done

# 4. Default environment is production.
# Extract the function body with a brace counter so nested blocks do not prematurely end the scan.
if awk '/fn default_environment\(\)/ {p=1} p {print; brace+=gsub(/\{/,"{")-gsub(/\}/,"}")} p && brace==0 {p=0}' backend/src/config/mod.rs | grep -q '"production"'; then
  pass "backend/src/config/mod.rs default_environment returns production"
else
  fail "backend/src/config/mod.rs default_environment must return \"production\""
fi

# 5. Webhook/slash-command URLs are validated at request time.
if grep -qE '(pub\(crate\) async fn|fn) validate_callback_url_at_request_time' backend/src/services/webhooks.rs; then
  pass "backend/src/services/webhooks.rs validates callback URLs at request time"
else
  fail "backend/src/services/webhooks.rs must contain validate_callback_url_at_request_time"
fi

# 6. Retention cleanup deletes S3 objects and has an orphan scanner.
if grep -qE '(async fn|pub async fn) run_orphan_scan' backend/src/jobs/retention.rs; then
  pass "backend/src/jobs/retention.rs contains run_orphan_scan"
else
  fail "backend/src/jobs/retention.rs must contain run_orphan_scan"
fi

if grep -qE 'storage\.delete_object|delete_object\(' backend/src/jobs/retention.rs; then
  pass "backend/src/jobs/retention.rs uses delete_object"
else
  fail "backend/src/jobs/retention.rs must use delete_object"
fi

if [[ "${ERRORS}" -eq 0 ]]; then
  echo ""
  echo "All P0 production-readiness gates passed."
  exit 0
else
  echo ""
  echo "${ERRORS} P0 gate(s) failed."
  exit 1
fi
