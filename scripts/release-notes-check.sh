#!/usr/bin/env bash
set -euo pipefail

# Validate that CHANGELOG.md is ready to release a given version.
#
# Usage: release-notes-check.sh [VERSION]
#   VERSION defaults to the VERSION file.
#
# Checks (all must pass):
#   1. CHANGELOG.md has a "## [VERSION]" section.
#   2. That section has content.
#   3. The [Unreleased] section is empty (items moved to the version section).
#
# Exits non-zero with ERROR lines on failure.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
cd "${PROJECT_ROOT}"

VERSION="${1:-$(tr -d '[:space:]' < VERSION)}"

ERRORS=0

echo "=== Release Notes Check (v${VERSION}) ==="
echo ""

# 1. Section exists
if grep -q "^## \[${VERSION}\]" CHANGELOG.md; then
  echo "OK: CHANGELOG.md has entry for [${VERSION}]"
else
  echo "ERROR: CHANGELOG.md does not have a section for [${VERSION}]"
  ERRORS=$((ERRORS + 1))
fi

# 2. Section has content
SECTION_CONTENT=$(awk "/^## \[${VERSION}\]/{flag=1;next}/^## \[/{flag=0}flag" CHANGELOG.md | grep -v '^[[:space:]]*$' || true)
if [[ -n "${SECTION_CONTENT}" ]]; then
  echo "OK: [${VERSION}] section has content"
elif grep -q "^## \[${VERSION}\]" CHANGELOG.md; then
  echo "ERROR: [${VERSION}] section is empty"
  ERRORS=$((ERRORS + 1))
else
  echo "SKIP: [${VERSION}] section not present, already reported above"
fi

# 3. [Unreleased] is empty
UNRELEASED_CONTENT=$(awk '/^## \[Unreleased\]/{flag=1;next}/^## \[/{flag=0}flag' CHANGELOG.md | grep -v '^[[:space:]]*$' || true)
if [[ -z "${UNRELEASED_CONTENT}" ]]; then
  echo "OK: [Unreleased] section is empty"
else
  echo "ERROR: [Unreleased] section still has content — move items to [${VERSION}] before releasing"
  ERRORS=$((ERRORS + 1))
fi

echo ""
if [[ "${ERRORS}" -eq 0 ]]; then
  echo "Release notes check passed."
  exit 0
else
  echo "${ERRORS} error(s) found. Fix before releasing."
  exit 1
fi
