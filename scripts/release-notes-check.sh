#!/usr/bin/env bash
set -euo pipefail

# Validate that CHANGELOG.md is ready for a release.
# Usage: ./scripts/release-notes-check.sh [VERSION]
# If VERSION is omitted, uses the VERSION file.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

cd "${PROJECT_ROOT}"

VERSION="${1:-$(cat VERSION | tr -d '[:space:]')}"

echo "=== Release Notes Check ==="
echo "Expected version: ${VERSION}"
echo ""

ERRORS=0

# Check that CHANGELOG has the version section
echo "--- Checking CHANGELOG.md ---"
if grep -q "^## \[${VERSION}\]" CHANGELOG.md; then
  echo "OK: Found section for ${VERSION}"
else
  echo "ERROR: CHANGELOG.md is missing a section for [${VERSION}]"
  ERRORS=$((ERRORS + 1))
fi

# Check that the Unreleased section is not empty (it should either have content or be removed)
UNRELEASED_CONTENT=$(awk '/^## \[Unreleased\]/{flag=1;next}/^## \[/{flag=0}flag' CHANGELOG.md | grep -v '^[[:space:]]*$' || true)
if [[ -n "${UNRELEASED_CONTENT}" ]]; then
  echo "WARNING: [Unreleased] section still has content. Move items to [${VERSION}] before releasing."
else
  echo "OK: [Unreleased] section is empty"
fi
echo ""

# Check for empty version section
SECTION_CONTENT=$(awk "/^## \[${VERSION}\]/{flag=1;next}/^## \[/{flag=0}flag" CHANGELOG.md | grep -v '^[[:space:]]*$' || true)
if [[ -n "${SECTION_CONTENT}" ]]; then
  echo "OK: [${VERSION}] section has content"
else
  echo "ERROR: [${VERSION}] section is empty"
  ERRORS=$((ERRORS + 1))
fi
echo ""

# Summary
echo "=== Summary ==="
if [[ "${ERRORS}" -eq 0 ]]; then
  echo "CHANGELOG.md looks ready for v${VERSION}."
  exit 0
else
  echo "${ERRORS} error(s) found. Fix CHANGELOG.md before releasing."
  exit 1
fi
