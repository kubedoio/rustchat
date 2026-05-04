#!/usr/bin/env bash
set -euo pipefail

# Unified smoke test wrapper for RustChat.
# Validates Docker configs, performs build dry-runs, and runs compatibility
# smoke tests if the backend is reachable.
#
# Usage: ./smoke-test.sh [options]
#
# Options:
#   --ci            Skip health checks and live endpoint tests
#   --wait          Poll health endpoint until ready (timeout: 60s)
#   --skip-build    Skip Docker build dry-runs
#   --help          Show this help message

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
BASE="${BASE:-http://127.0.0.1:3000}"
CI_MODE=false
WAIT_MODE=false
SKIP_BUILD=false

for arg in "$@"; do
  case "$arg" in
    --ci)
      CI_MODE=true
      ;;
    --wait)
      WAIT_MODE=true
      ;;
    --skip-build)
      SKIP_BUILD=true
      ;;
    --help)
      echo "Usage: $(basename "$0") [options]"
      echo ""
      echo "Options:"
      echo "  --ci            Skip health checks and live endpoint tests"
      echo "  --wait          Poll health endpoint until ready (timeout: 60s)"
      echo "  --skip-build    Skip Docker build dry-runs"
      echo "  --help          Show this help message"
      echo ""
      echo "Environment:"
      echo "  BASE            Target URL (default: http://127.0.0.1:3000)"
      exit 0
      ;;
    *)
      echo "Unknown argument: $arg"
      echo "Run '$(basename "$0") --help' for usage."
      exit 1
      ;;
  esac
done

echo "=== RustChat Smoke Test ==="
echo "Repo root: ${REPO_ROOT}"
if [[ "$CI_MODE" == false ]]; then
  echo "Target: ${BASE}"
fi
echo ""

# Docker compose config validation
echo "--- Docker Compose Config Validation ---"
if command -v docker &> /dev/null && docker compose version &> /dev/null; then
  if ! docker compose -f "${REPO_ROOT}/docker-compose.yml" config > /dev/null 2>&1; then
    echo "ERROR: docker compose config is invalid"
    exit 1
  fi
  echo "✓ docker compose config is valid"
else
  echo "WARNING: docker compose not available, skipping config validation"
fi
echo ""

# Docker build dry-run (backend)
if [[ "$SKIP_BUILD" == false ]]; then
  echo "--- Docker Build Dry-Run (Backend) ---"
  if command -v docker &> /dev/null; then
    if ! docker build -f "${REPO_ROOT}/docker/backend.Dockerfile" --target builder "${REPO_ROOT}/backend" > /dev/null 2>&1; then
      echo "⚠ WARNING: backend Docker build dry-run failed (may need BuildKit or secrets)"
    else
      echo "✓ backend Docker build dry-run succeeded"
    fi
  else
    echo "WARNING: docker not available, skipping backend build dry-run"
  fi
  echo ""

  # Docker build dry-run (frontend)
  echo "--- Docker Build Dry-Run (Frontend) ---"
  if command -v docker &> /dev/null; then
    if ! docker build -f "${REPO_ROOT}/docker/frontend.Dockerfile" "${REPO_ROOT}/frontend" > /dev/null 2>&1; then
      echo "⚠ WARNING: frontend Docker build dry-run failed (may need BuildKit or secrets)"
    else
      echo "✓ frontend Docker build dry-run succeeded"
    fi
  else
    echo "WARNING: docker not available, skipping frontend build dry-run"
  fi
  echo ""
fi

# Health check with optional wait
if [[ "$CI_MODE" == true ]]; then
  echo "--- Health Check (skipped in CI mode) ---"
  echo "Skipping health check (--ci flag set)"
else
  echo "--- Health Check ---"

  if [[ "$WAIT_MODE" == true ]]; then
    echo "Waiting for backend to become healthy (timeout: 60s)..."
    for i in {1..60}; do
      if curl -fsS "${BASE}/api/v1/health/live" >/dev/null 2>&1; then
        echo "✓ Backend is healthy (waited ${i}s)"
        break
      fi
      if [[ $i -eq 60 ]]; then
        echo "ERROR: Backend is not reachable at ${BASE} after 60s"
        echo "  Start the stack with: docker compose up -d"
        exit 1
      fi
      sleep 1
    done
  else
    if ! curl -fsS "${BASE}/api/v1/health/live" >/dev/null 2>&1; then
      echo "ERROR: Backend is not reachable at ${BASE}"
      echo "  Start the stack with: docker compose up -d"
      exit 1
    fi
    echo "✓ Backend is healthy"
  fi
fi
echo ""

# Push proxy health check
if [[ "$CI_MODE" == false ]]; then
  echo "--- Push Proxy Health Check ---"
  if curl -fsS "http://127.0.0.1:3001/health" >/dev/null 2>&1; then
    echo "✓ Push proxy is healthy"
  else
    echo "⚠ Push proxy is not reachable at http://127.0.0.1:3001"
    echo "  (Optional — only needed for mobile push notifications)"
  fi
  echo ""
fi

# Mattermost compatibility smoke tests
echo "--- Mattermost Compatibility Smoke ---"
if [[ -x "${SCRIPT_DIR}/mm_compat_smoke.sh" ]]; then
  BASE="${BASE}" "${SCRIPT_DIR}/mm_compat_smoke.sh"
else
  echo "WARNING: mm_compat_smoke.sh not found, skipping"
fi
echo ""

# Mobile smoke tests
echo "--- Mobile Smoke ---"
if [[ -x "${SCRIPT_DIR}/mm_mobile_smoke.sh" ]]; then
  BASE="${BASE}" "${SCRIPT_DIR}/mm_mobile_smoke.sh"
else
  echo "WARNING: mm_mobile_smoke.sh not found, skipping"
fi
echo ""

echo "=== Smoke tests completed ==="
