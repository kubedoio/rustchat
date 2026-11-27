#!/usr/bin/env bash
set -euo pipefail

# Static regression guard for the release/promotion alias policy documented in
# docs/release-process.md.
#
# The policy:
#   * a prerelease tag (anything with a `-` suffix, e.g. v0.5.1-rc.1) publishes
#     only the immutable `{{version}}` image tag — it must never move the
#     rolling `{{major}}.{{minor}}` tag or `latest`;
#   * moving-alias promotions (`promote.yml`) serialize: overlapping runs must
#     queue instead of cancelling an in-flight alias push.
#
# This script asserts the *workflow source* encodes that policy. It is
# intentionally dependency-free (bash + grep/awk) so it can run in any CI job
# and locally from the repository root:
#
#   scripts/check-alias-policy.sh
#
# Exit status: 0 when every check passes, 1 with a `FAIL:` line otherwise.

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd -- "${SCRIPT_DIR}/.." && pwd)"

RELEASE_YML="${ROOT}/.github/workflows/release.yml"
PROMOTE_YML="${ROOT}/.github/workflows/promote.yml"

status=0

pass() { printf 'ok: %s\n' "$*"; }

fail() {
  printf 'FAIL: %s\n' "$*" >&2
  status=1
}

if [[ ! -f "${RELEASE_YML}" ]]; then
  fail "missing workflow: ${RELEASE_YML}"
fi
if [[ ! -f "${PROMOTE_YML}" ]]; then
  fail "missing workflow: ${PROMOTE_YML}"
fi
if [[ "${status}" -ne 0 ]]; then
  echo "Alias policy check failed." >&2
  exit 1
fi

# --- release.yml -----------------------------------------------------------
#
# Expected shape of the `tags:` input passed to docker-build-push.yml:
#
#   type=semver,pattern={{version}}
#   type=semver,pattern={{major}}.{{minor}},enable=${{ !contains(github.ref_name, '-') }}
#   type=raw,value=latest,enable=${{ !contains(github.ref_name, '-') }}
#
# `enable=` is evaluated per tag by docker/metadata-action, so the allow/deny
# decision lives right next to each rolling tag.

# The immutable version tag must stay enabled for *every* release, including
# prereleases. An `enable=` gate here would (depending on its value) silently
# stop publishing the RC image itself.
version_line="$(grep -F 'type=semver,pattern={{version}}' "${RELEASE_YML}" | head -n1 || true)"
if [[ -z "${version_line}" ]]; then
  fail "release.yml: missing 'type=semver,pattern={{version}}' tag (every release needs an immutable version tag)"
elif [[ "${version_line}" == *'enable='* ]]; then
  fail "release.yml: the {{version}} tag must not be gated by enable= (prereleases must publish their exact version tag)"
else
  pass "release.yml: {{version}} tag is published for all releases"
fi

# Shared check: the tag line must exist and its enable= expression must negate
# a prerelease check (`!contains(<ref>, '-')`), i.e. be false for RCs.
check_prerelease_gate() {
  local line="$1" label="$2" enable_expr
  if [[ -z "${line}" ]]; then
    fail "release.yml: missing ${label} tag"
    return
  fi
  if [[ "${line}" != *'enable='* ]]; then
    fail "release.yml: ${label} tag has no enable= gate (prereleases would move the rolling alias)"
    return
  fi
  enable_expr="${line#*enable=}"
  if [[ "${enable_expr}" != *'!contains('* ]]; then
    fail "release.yml: ${label} enable= must negate a prerelease check, got: ${enable_expr}"
    return
  fi
  if [[ "${enable_expr}" != *"'-'"* && "${enable_expr}" != *'"-"'* ]]; then
    fail "release.yml: ${label} enable= must test for a '-' prerelease suffix, got: ${enable_expr}"
    return
  fi
  pass "release.yml: ${label} tag is gated on a non-prerelease ref"
}

minor_line="$(grep -F 'type=semver,pattern={{major}}.{{minor}}' "${RELEASE_YML}" | head -n1 || true)"
check_prerelease_gate "${minor_line}" "{{major}}.{{minor}}"

# `type=raw,value=latest` must exist and be gated. `(,|$)` allows the tag
# attributes that follow the value.
latest_line="$(grep -E '^[[:space:]]*type=raw,value=latest(,|[[:space:]]|$)' "${RELEASE_YML}" | head -n1 || true)"
check_prerelease_gate "${latest_line}" "raw latest"

# --- promote.yml -----------------------------------------------------------
#
# Promotions must serialize. The top-level `concurrency:` block is extracted
# verbatim (it ends at the next unindented, non-comment line) so the `group:`
# and `cancel-in-progress:` keys are proven to belong to it.

concurrency_block="$(awk '
  /^concurrency:[[:space:]]*$/ { inside = 1; print; next }
  inside && /^[^[:space:]#]/ { inside = 0 }
  inside { print }
' "${PROMOTE_YML}")"

if [[ -z "${concurrency_block}" ]]; then
  fail "promote.yml: missing workflow-level concurrency: block (concurrent promotions can race)"
else
  if grep -qE '^[[:space:]]+group:[[:space:]]*[^[:space:]#]' <<<"${concurrency_block}"; then
    pass "promote.yml: promotions are serialized by a concurrency group"
  else
    fail "promote.yml: concurrency block has no non-empty group:"
  fi
  if grep -qE '^[[:space:]]+cancel-in-progress:[[:space:]]*false([[:space:]]|$)' <<<"${concurrency_block}"; then
    pass "promote.yml: cancel-in-progress is false (queued, never aborted)"
  else
    fail "promote.yml: concurrency must set cancel-in-progress: false so an in-flight promotion is never aborted"
  fi
fi

if grep -qE 'cancel-in-progress:[[:space:]]*true' "${PROMOTE_YML}"; then
  fail "promote.yml: cancel-in-progress: true is forbidden for alias promotions"
fi

if [[ "${status}" -ne 0 ]]; then
  echo "Alias policy check failed." >&2
  exit 1
fi

echo "Alias policy check passed."
