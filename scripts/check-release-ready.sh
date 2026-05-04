#!/usr/bin/env bash
set -euo pipefail

# Pre-release validation for RustChat maintainers.
# Run this before tagging a release.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

cd "${PROJECT_ROOT}"

ERRORS=0

echo "=== Release Readiness Check ==="
echo ""

# 1. Version consistency
echo "--- Version Consistency ---"
VERSION_FILE="$(tr -d '[:space:]' < VERSION)"
CARGO_VERSION="$(grep '^version' backend/Cargo.toml | head -1 | sed 's/.*= *"\(.*\)".*/\1/')"
PKG_VERSION="$(grep '"version"' frontend/package.json | head -1 | sed 's/.*: *"\(.*\)".*/\1/')"
PROXY_VERSION=""
if [[ -f push-proxy/Cargo.toml ]]; then
  PROXY_VERSION="$(grep '^version' push-proxy/Cargo.toml | head -1 | sed 's/.*= *"\(.*\)".*/\1/')"
fi

echo "VERSION file:         ${VERSION_FILE}"
echo "backend/Cargo.toml:   ${CARGO_VERSION}"
echo "frontend/package.json: ${PKG_VERSION}"
if [[ -n "${PROXY_VERSION}" ]]; then
  echo "push-proxy/Cargo.toml: ${PROXY_VERSION}"
fi

if [[ "${VERSION_FILE}" != "${CARGO_VERSION}" || "${VERSION_FILE}" != "${PKG_VERSION}" ]]; then
  echo "ERROR: Versions do not match"
  ERRORS=$((ERRORS + 1))
else
  echo "OK: All versions match"
fi

if [[ -n "${PROXY_VERSION}" && "${PROXY_VERSION}" != "${VERSION_FILE}" ]]; then
  echo "WARNING: push-proxy version (${PROXY_VERSION}) does not match release version (${VERSION_FILE})"
fi
echo ""

# 2. Changelog has an entry for this version
echo "--- Changelog Entry ---"
if grep -q "^## \[${VERSION_FILE}\]" CHANGELOG.md; then
  echo "OK: CHANGELOG.md has entry for ${VERSION_FILE}"
else
  echo "ERROR: CHANGELOG.md does not have a section for ${VERSION_FILE}"
  ERRORS=$((ERRORS + 1))
fi
echo ""

# 3. Changelog has content for this version
echo "--- Changelog Content ---"
SECTION_CONTENT=$(awk "/^## \[${VERSION_FILE}\]/{flag=1;next}/^## \[/{flag=0}flag" CHANGELOG.md | grep -v '^[[:space:]]*$' || true)
if [[ -n "${SECTION_CONTENT}" ]]; then
  echo "OK: [${VERSION_FILE}] section has content"
else
  echo "ERROR: [${VERSION_FILE}] section is empty"
  ERRORS=$((ERRORS + 1))
fi
echo ""

# 4. Unreleased section is empty
echo "--- Unreleased Section ---"
UNRELEASED_CONTENT=$(awk '/^## \[Unreleased\]/{flag=1;next}/^## \[/{flag=0}flag' CHANGELOG.md | grep -v '^[[:space:]]*$' || true)
if [[ -n "${UNRELEASED_CONTENT}" ]]; then
  echo "WARNING: [Unreleased] section still has content. Move items to [${VERSION_FILE}] before releasing."
else
  echo "OK: [Unreleased] section is empty"
fi
echo ""

# 5. No unstaged changes
echo "--- Git Status ---"
if git diff --quiet && git diff --cached --quiet; then
  echo "OK: Working tree is clean"
else
  echo "ERROR: Working tree has uncommitted changes"
  git status --short
  ERRORS=$((ERRORS + 1))
fi
echo ""

# 6. Backend formatting and clippy
echo "--- Backend Checks ---"
cd backend
if cargo fmt --all -- --check 2>/dev/null; then
  echo "OK: cargo fmt"
else
  echo "ERROR: cargo fmt failed"
  ERRORS=$((ERRORS + 1))
fi

if cargo clippy --all-targets --all-features -- -D warnings 2>/dev/null; then
  echo "OK: cargo clippy"
else
  echo "ERROR: cargo clippy failed"
  ERRORS=$((ERRORS + 1))
fi
cd "${PROJECT_ROOT}"
echo ""

# 7. Push-proxy formatting and clippy (if it exists)
if [[ -d push-proxy/src ]]; then
  echo "--- Push Proxy Checks ---"
  cd push-proxy
  if cargo fmt --all -- --check 2>/dev/null; then
    echo "OK: cargo fmt"
  else
    echo "ERROR: cargo fmt failed"
    ERRORS=$((ERRORS + 1))
  fi

  if cargo clippy --all-targets --all-features -- -D warnings 2>/dev/null; then
    echo "OK: cargo clippy"
  else
    echo "ERROR: cargo clippy failed"
    ERRORS=$((ERRORS + 1))
  fi
  cd "${PROJECT_ROOT}"
  echo ""
fi

# 8. Docker build check
echo "--- Docker Build ---"
if docker compose config > /dev/null 2>&1; then
  echo "OK: docker compose config"
else
  echo "WARNING: docker compose config failed (or docker unavailable)"
fi
echo ""

# 9. Release notes check
echo "--- Release Notes Check ---"
if [[ -x "${SCRIPT_DIR}/release-notes-check.sh" ]]; then
  if "${SCRIPT_DIR}/release-notes-check.sh" "${VERSION_FILE}"; then
    echo "OK: release-notes-check.sh passed"
  else
    echo "ERROR: release-notes-check.sh failed"
    ERRORS=$((ERRORS + 1))
  fi
else
  echo "WARNING: release-notes-check.sh not found"
fi
echo ""

# Summary
echo "=== Summary ==="
if [[ "${ERRORS}" -eq 0 ]]; then
  echo "Ready for release."
  echo ""
  echo "Next steps:"
  echo "  git add -A"
  echo "  git commit -s -m \"chore(release): bump version to ${VERSION_FILE}\""
  echo "  git tag -s v${VERSION_FILE} -m \"Release v${VERSION_FILE}\""
  echo "  git push origin main && git push origin v${VERSION_FILE}"
  exit 0
else
  echo "${ERRORS} error(s) found. Fix before releasing."
  exit 1
fi
