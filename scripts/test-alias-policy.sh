#!/usr/bin/env bash
set -euo pipefail

# Self-contained tests for scripts/check-alias-policy.sh.
#
# Each case is built from a temporary copy of the *real* workflows; the copy is
# mutated to represent a policy violation or a policy-compliant variation, then
# the checker is re-run against it through ALIAS_POLICY_ROOT. This proves the
# checker fails on every loophole a reviewer found (comment decoys, first-line
# only matching, extra ungated `latest`, head_ref / double negation / enable=true
# gates, attribute reordering, flow-style concurrency, ...) and still passes
# compliant variants.
#
# Runnable from any working directory:
#
#   scripts/test-alias-policy.sh
#
# Exit status: 0 when every case behaves as expected.

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
SRC_ROOT="$(cd -- "${SCRIPT_DIR}/.." && pwd)"
POLICY="${SCRIPT_DIR}/check-alias-policy.sh"

PASS=0
FAIL=0
NEG=0
POS=0

ok() { echo "PASS: $1"; PASS=$((PASS + 1)); }
bad() {
  echo "FAIL: $1" >&2
  FAIL=$((FAIL + 1))
}

WORK="$(mktemp -d)"
trap 'rm -rf "${WORK}"' EXIT

SQ="'"
GATE_EXPR="\${{ !contains(github.ref_name, ${SQ}-${SQ}) }}"
VERSION_TAG="type=semver,pattern={{version}}"
MINOR_TAG="type=semver,pattern={{major}}.{{minor}}"
MINOR_GATED="${MINOR_TAG},enable=${GATE_EXPR}"
LATEST_TAG="type=raw,value=latest"
LATEST_GATED="${LATEST_TAG},enable=${GATE_EXPR}"

# ---------- fixture helpers ----------

new_fixture() {
  # new_fixture <name> -> prints the fixture root
  local name="$1"
  local dir="${WORK}/${name}"
  mkdir -p "${dir}/.github/workflows"
  cp "${SRC_ROOT}/.github/workflows/release.yml" "${dir}/.github/workflows/release.yml"
  cp "${SRC_ROOT}/.github/workflows/promote.yml" "${dir}/.github/workflows/promote.yml"
  printf '%s' "${dir}"
}

# Replace the line whose trimmed content equals <old> with <new>, preserving
# the original indentation.
mutate_line() {
  local file="$1" old="$2" new="$3" tmp
  tmp="${file}.tmp"
  awk -v old="${old}" -v new="${new}" '
    {
      t = $0
      sub(/^[[:space:]]+/, "", t)
      if (t == old) {
        match($0, /^[ ]*/)
        print substr($0, 1, RLENGTH) new
        next
      }
      print
    }
  ' "${file}" >"${tmp}"
  mv "${tmp}" "${file}"
}

remove_line() {
  local file="$1" old="$2" tmp
  tmp="${file}.tmp"
  awk -v old="${old}" '
    {
      t = $0
      sub(/^[[:space:]]+/, "", t)
      if (t == old) next
      print
    }
  ' "${file}" >"${tmp}"
  mv "${tmp}" "${file}"
}

insert_after_line() {
  local file="$1" old="$2" new="$3" tmp
  tmp="${file}.tmp"
  awk -v old="${old}" -v new="${new}" '
    {
      t = $0
      sub(/^[[:space:]]+/, "", t)
      print
      if (t == old) {
        match($0, /^[ ]*/)
        print substr($0, 1, RLENGTH) new
      }
    }
  ' "${file}" >"${tmp}"
  mv "${tmp}" "${file}"
}

insert_before_line() {
  local file="$1" old="$2" new="$3" tmp
  tmp="${file}.tmp"
  awk -v old="${old}" -v new="${new}" '
    {
      t = $0
      sub(/^[[:space:]]+/, "", t)
      if (t == old) {
        match($0, /^[ ]*/)
        print substr($0, 1, RLENGTH) new
      }
      print
    }
  ' "${file}" >"${tmp}"
  mv "${tmp}" "${file}"
}

# Rewrite the workflow-level concurrency block (3 lines: key + group +
# cancel-in-progress) as a single flow mapping.
to_flow_concurrency() {
  local file="$1" flow="$2" tmp
  tmp="${file}.tmp"
  awk -v flow="${flow}" '
    $0 == "concurrency:" { print flow; skip = 2; next }
    skip > 0 { skip--; next }
    { print }
  ' "${file}" >"${tmp}"
  mv "${tmp}" "${file}"
}

# Append a decoy job that passes tags to the reusable workflow. Used to prove
# the checker anchors on the `docker-release` job instead of the whole file.
append_decoy_job() {
  local file="$1" tag="$2"
  cat >>"${file}" <<EOF
  alias-policy-decoy:
    runs-on: ubuntu-latest
    steps:
      - uses: ./.github/workflows/docker-build-push.yml
        with:
          tags: |
            ${tag}
EOF
}

# ---------- assertions ----------

expect_fail() {
  # expect_fail <fixture-name> <description> [expected-message-substring]
  local name="$1" desc="$2" pattern="${3:-}"
  local dir="${WORK}/${name}" rc=0
  NEG=$((NEG + 1))
  ALIAS_POLICY_ROOT="${dir}" bash "${POLICY}" >"${dir}/out.log" 2>&1 || rc=$?
  if [[ "${rc}" -eq 0 ]]; then
    bad "${desc} (expected FAIL, checker passed)"
  elif [[ -n "${pattern}" ]] && ! grep -qF -- "${pattern}" "${dir}/out.log"; then
    bad "${desc} (failed, but not for the expected reason: '${pattern}')"
  else
    ok "${desc}"
  fi
}

expect_pass() {
  local name="$1" desc="$2"
  local dir="${WORK}/${name}" rc=0
  POS=$((POS + 1))
  ALIAS_POLICY_ROOT="${dir}" bash "${POLICY}" >"${dir}/out.log" 2>&1 || rc=$?
  if [[ "${rc}" -ne 0 ]]; then
    bad "${desc} (expected PASS, checker failed)"
    sed 's/^/    /' "${dir}/out.log" >&2
  else
    ok "${desc}"
  fi
}

# ---------- negative cases: the checker must FAIL ----------

f="$(new_fixture version-gated)"
mutate_line "${f}/.github/workflows/release.yml" "${VERSION_TAG}" "${VERSION_TAG},enable=${GATE_EXPR}"
expect_fail "version-gated" "gated {{version}} tag is rejected" "{{version}} tag must not be gated"

f="$(new_fixture version-missing)"
remove_line "${f}/.github/workflows/release.yml" "${VERSION_TAG}"
expect_fail "version-missing" "missing {{version}} tag is rejected" "exactly one"

f="$(new_fixture minor-ungated)"
mutate_line "${f}/.github/workflows/release.yml" "${MINOR_GATED}" "${MINOR_TAG}"
expect_fail "minor-ungated" "ungated {{major}}.{{minor}} tag is rejected" "must be gated"

f="$(new_fixture latest-ungated)"
mutate_line "${f}/.github/workflows/release.yml" "${LATEST_GATED}" "${LATEST_TAG}"
expect_fail "latest-ungated" "ungated raw latest tag is rejected" "'latest' tag must be gated"

f="$(new_fixture decoy-comment)"
# A YAML comment that *looks* gated must not satisfy the check for an ungated
# real tag line below it (regression: the old checker grepped the raw file).
mutate_line "${f}/.github/workflows/release.yml" "${LATEST_GATED}" "${LATEST_TAG}"
insert_before_line "${f}/.github/workflows/release.yml" "tags: |" "# ${LATEST_GATED}"
expect_fail "decoy-comment" "comment decoy above an ungated latest tag is rejected" "'latest' tag must be gated"

f="$(new_fixture extra-latest)"
# A second ungated latest after a gated one (regression: only the first
# matching line was inspected).
insert_after_line "${f}/.github/workflows/release.yml" "${LATEST_GATED}" "${LATEST_TAG}"
expect_fail "extra-latest" "a second ungated latest tag is rejected" "more than one tag source emits"

f="$(new_fixture head-ref-gate)"
mutate_line "${f}/.github/workflows/release.yml" "${LATEST_GATED}" \
  "${LATEST_TAG},enable=\${{ !contains(github.head_ref, ${SQ}-${SQ}) }}"
expect_fail "head-ref-gate" "latest gated on github.head_ref is rejected" "'latest' tag must be gated"

f="$(new_fixture double-negation)"
mutate_line "${f}/.github/workflows/release.yml" "${LATEST_GATED}" \
  "${LATEST_TAG},enable=\${{ !(!contains(github.ref_name, ${SQ}-${SQ})) }}"
expect_fail "double-negation" "double-negated latest gate is rejected" "'latest' tag must be gated"

f="$(new_fixture enable-true)"
mutate_line "${f}/.github/workflows/release.yml" "${LATEST_GATED}" "${LATEST_TAG},enable=true"
expect_fail "enable-true" "enable=true latest gate is rejected" "'latest' tag must be gated"

f="$(new_fixture flavor-latest)"
# A flavor attribute that would also emit a `latest`-suffixed tag must not
# sneak past the raw-latest check.
mutate_line "${f}/.github/workflows/release.yml" "${LATEST_GATED}" \
  "type=raw,value=release,flavor=suffix=-latest"
expect_fail "flavor-latest" "flavor suffix emitting latest is rejected" "'latest' tag must be gated"

f="$(new_fixture ungated-minor-extra)"
# Ungated minor alias added next to the gated one.
insert_after_line "${f}/.github/workflows/release.yml" "${MINOR_GATED}" "${MINOR_TAG}"
expect_fail "ungated-minor-extra" "an extra ungated {{major}}.{{minor}} tag is rejected" "must be gated"

f="$(new_fixture promote-no-concurrency)"
remove_line "${f}/.github/workflows/promote.yml" "concurrency:"
remove_line "${f}/.github/workflows/promote.yml" "group: promote-moving-aliases"
remove_line "${f}/.github/workflows/promote.yml" "cancel-in-progress: false"
expect_fail "promote-no-concurrency" "promote.yml without concurrency is rejected" "missing workflow-level concurrency group"

f="$(new_fixture promote-cancel-true)"
mutate_line "${f}/.github/workflows/promote.yml" "cancel-in-progress: false" "cancel-in-progress: true"
expect_fail "promote-cancel-true" "promote.yml cancel-in-progress: true is rejected" "cancel-in-progress: true is forbidden"

f="$(new_fixture release-no-concurrency)"
remove_line "${f}/.github/workflows/release.yml" "concurrency:"
remove_line "${f}/.github/workflows/release.yml" "group: release-moving-aliases"
remove_line "${f}/.github/workflows/release.yml" "cancel-in-progress: false"
expect_fail "release-no-concurrency" "release.yml without concurrency is rejected" "missing workflow-level concurrency group"

f="$(new_fixture release-cancel-true)"
mutate_line "${f}/.github/workflows/release.yml" "cancel-in-progress: false" "cancel-in-progress: true"
expect_fail "release-cancel-true" "release.yml cancel-in-progress: true is rejected" "cancel-in-progress: true is forbidden"

f="$(new_fixture decoy-other-job)"
# A compliant copy in another job must not mask a violation in docker-release.
mutate_line "${f}/.github/workflows/release.yml" "${LATEST_GATED}" "${LATEST_TAG}"
append_decoy_job "${f}/.github/workflows/release.yml" "${LATEST_GATED}"
expect_fail "decoy-other-job" "a compliant tags block in another job does not mask docker-release" "'latest' tag must be gated"

# ---------- positive cases: the checker must PASS ----------

f="$(new_fixture baseline)"
expect_pass "baseline" "unmodified workflows pass"

f="$(new_fixture attribute-order)"
# docker/metadata-action is order-insensitive; so is the checker.
mutate_line "${f}/.github/workflows/release.yml" "${LATEST_GATED}" \
  "type=raw,enable=${GATE_EXPR},value=latest"
mutate_line "${f}/.github/workflows/release.yml" "${MINOR_GATED}" \
  "type=semver,enable=${GATE_EXPR},pattern={{major}}.{{minor}}"
expect_pass "attribute-order" "reordered tag attributes pass"

f="$(new_fixture flow-concurrency)"
to_flow_concurrency "${f}/.github/workflows/promote.yml" \
  "concurrency: {group: promote-moving-aliases, cancel-in-progress: false}"
to_flow_concurrency "${f}/.github/workflows/release.yml" \
  "concurrency: {group: release-moving-aliases, cancel-in-progress: false}"
expect_pass "flow-concurrency" "flow-style concurrency maps pass"

f="$(new_fixture comment-mentions-true)"
# Comments that merely mention cancel-in-progress: true must not fail the
# policy, and gated-looking tag comments must not be required to exist.
insert_before_line "${f}/.github/workflows/promote.yml" "concurrency:" \
  "# NB: cancel-in-progress: true would abort an in-flight alias push"
insert_before_line "${f}/.github/workflows/release.yml" "concurrency:" \
  "# NB: cancel-in-progress: true would abort an in-flight alias push"
insert_before_line "${f}/.github/workflows/release.yml" "tags: |" "# ${LATEST_GATED}"
expect_pass "comment-mentions-true" "comments mentioning cancel-in-progress: true do not fail"

f="$(new_fixture trailing-comments)"
mutate_line "${f}/.github/workflows/promote.yml" "group: promote-moving-aliases" \
  "group: promote-moving-aliases  # serialize promotions"
mutate_line "${f}/.github/workflows/promote.yml" "cancel-in-progress: false" \
  "cancel-in-progress: false  # never abort an in-flight push"
mutate_line "${f}/.github/workflows/release.yml" "group: release-moving-aliases" \
  "group: release-moving-aliases  # serialize stable releases"
mutate_line "${f}/.github/workflows/release.yml" "cancel-in-progress: false" \
  "cancel-in-progress: false  # never abort an in-flight push"
mutate_line "${f}/.github/workflows/release.yml" "tags: |" "tags: |  # rolling aliases"
expect_pass "trailing-comments" "trailing comments do not break or satisfy checks"

f="$(new_fixture decoy-unrelated-job)"
# An ungated tags block in a different job must be ignored: only the
# docker-release job publishes the release aliases.
append_decoy_job "${f}/.github/workflows/release.yml" "${LATEST_TAG}"
expect_pass "decoy-unrelated-job" "tags block outside docker-release is ignored"

# ---------- totals ----------

echo ""
echo "alias-policy tests: ${PASS} passed, ${FAIL} failed (${NEG} negative, ${POS} positive)"

if [[ "${NEG}" -lt 8 ]]; then
  bad "expected at least 8 negative cases, found ${NEG}"
fi
if [[ "${POS}" -lt 4 ]]; then
  bad "expected at least 4 positive cases, found ${POS}"
fi

[[ "${FAIL}" -eq 0 ]]
