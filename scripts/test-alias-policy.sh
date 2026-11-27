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
  # Copy every workflow: the checker scans all of them for moving-alias
  # emissions (not just release.yml / promote.yml).
  cp -R "${SRC_ROOT}/.github/workflows/." "${dir}/.github/workflows/"
  printf '%s' "${dir}"
}

trim() {
  local s="$1"
  s="${s#"${s%%[![:space:]]*}"}"
  s="${s%"${s##*[![:space:]]}"}"
  printf '%s' "${s}"
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

# Insert one or more raw lines after the first line whose trimmed content
# equals <old>, reusing that line's indentation. Values are printed verbatim
# (awk -v would interpret backslashes in regex tag values).
insert_lines_after() {
  local file="$1" old="$2"
  shift 2
  local tmp="${file}.tmp" line indent new printed=0
  : >"${tmp}"
  while IFS= read -r line; do
    printf '%s\n' "${line}" >>"${tmp}"
    if ((printed == 0)) && [[ "$(trim "${line}")" == "${old}" ]]; then
      indent="${line%%[! ]*}"
      for new in "$@"; do
        printf '%s%s\n' "${indent}" "${new}" >>"${tmp}"
      done
      printed=1
    fi
  done <"${file}"
  mv "${tmp}" "${file}"
}

# Insert a block-scalar key (at the matched line's indentation) and its
# content lines (indented two more spaces) before the first line whose
# trimmed content equals <old>.
insert_block_before() {
  local file="$1" old="$2" keyline="$3"
  shift 3
  local tmp="${file}.tmp" line indent new
  : >"${tmp}"
  while IFS= read -r line; do
    if [[ "$(trim "${line}")" == "${old}" ]]; then
      indent="${line%%[! ]*}"
      printf '%s%s\n' "${indent}" "${keyline}" >>"${tmp}"
      for new in "$@"; do
        printf '%s  %s\n' "${indent}" "${new}" >>"${tmp}"
      done
    fi
    printf '%s\n' "${line}" >>"${tmp}"
  done <"${file}"
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

# Append a job whose tags value is a GitHub expression resolved from env. The
# effective alias is invisible to a literal scan, so the checker must reject
# the indirection outside docker-build-push.yml.
append_dynamic_alias_job() {
  local file="$1"
  cat >>"${file}" <<'EOF'
  alias-policy-dynamic:
    runs-on: ubuntu-latest
    env:
      EVIL_TAGS: type=raw,value=latest
    steps:
      - uses: ./.github/workflows/docker-build-push.yml
        with:
          tags: ${{ env.EVIL_TAGS }}
EOF
}

# Append a job that hides tags inside a flow-style mapping.
append_flow_mapping_job() {
  local file="$1"
  cat >>"${file}" <<'EOF'
  alias-policy-flow:
    runs-on: ubuntu-latest
    steps:
      - uses: ./.github/workflows/docker-build-push.yml
        with: { tags: "type=raw,value=latest" }
EOF
}

# Append a job that hides tags inside a flow sequence of flow mappings.
append_flow_sequence_job() {
  local file="$1"
  cat >>"${file}" <<'EOF'
  alias-policy-flowseq:
    runs-on: ubuntu-latest
    steps: [ { uses: ./.github/workflows/docker-build-push.yml, with: { tags: "type=raw,value=latest" } } ]
EOF
}

# Append a job that bypasses metadata-action and hands a plain moving Docker
# tag reference to docker/build-push-action.
append_plain_tag_job() {
  local file="$1"
  cat >>"${file}" <<'EOF'
  alias-policy-plain:
    runs-on: ubuntu-latest
    steps:
      - uses: docker/build-push-action@10e90e3645eae34f1e60eeb005ba3a3d33f178e8 # v6.19.2
        with:
          tags: |
            ghcr.io/example/image:latest
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
expect_fail "version-gated" "gated {{version}} tag is rejected" "allowlist"

f="$(new_fixture version-missing)"
remove_line "${f}/.github/workflows/release.yml" "${VERSION_TAG}"
expect_fail "version-missing" "missing {{version}} tag is rejected" "allowlist"

f="$(new_fixture minor-ungated)"
mutate_line "${f}/.github/workflows/release.yml" "${MINOR_GATED}" "${MINOR_TAG}"
expect_fail "minor-ungated" "ungated {{major}}.{{minor}} tag is rejected" "allowlist"

f="$(new_fixture latest-ungated)"
mutate_line "${f}/.github/workflows/release.yml" "${LATEST_GATED}" "${LATEST_TAG}"
expect_fail "latest-ungated" "ungated raw latest tag is rejected" "allowlist"

f="$(new_fixture decoy-comment)"
# A YAML comment that *looks* gated must not satisfy the check for an ungated
# real tag line below it (regression: the old checker grepped the raw file).
mutate_line "${f}/.github/workflows/release.yml" "${LATEST_GATED}" "${LATEST_TAG}"
insert_before_line "${f}/.github/workflows/release.yml" "tags: |" "# ${LATEST_GATED}"
expect_fail "decoy-comment" "comment decoy above an ungated latest tag is rejected" "allowlist"

f="$(new_fixture extra-latest)"
# A second ungated latest after a gated one (regression: only the first
# matching line was inspected).
insert_after_line "${f}/.github/workflows/release.yml" "${LATEST_GATED}" "${LATEST_TAG}"
expect_fail "extra-latest" "a second ungated latest tag is rejected" "allowlist"

f="$(new_fixture head-ref-gate)"
mutate_line "${f}/.github/workflows/release.yml" "${LATEST_GATED}" \
  "${LATEST_TAG},enable=\${{ !contains(github.head_ref, ${SQ}-${SQ}) }}"
expect_fail "head-ref-gate" "latest gated on github.head_ref is rejected" "allowlist"

f="$(new_fixture double-negation)"
mutate_line "${f}/.github/workflows/release.yml" "${LATEST_GATED}" \
  "${LATEST_TAG},enable=\${{ !(!contains(github.ref_name, ${SQ}-${SQ})) }}"
expect_fail "double-negation" "double-negated latest gate is rejected" "allowlist"

f="$(new_fixture enable-true)"
mutate_line "${f}/.github/workflows/release.yml" "${LATEST_GATED}" "${LATEST_TAG},enable=true"
expect_fail "enable-true" "enable=true latest gate is rejected" "allowlist"

f="$(new_fixture flavor-latest)"
# A flavor attribute that would also emit a `latest`-suffixed tag must not
# sneak past the raw-latest check.
mutate_line "${f}/.github/workflows/release.yml" "${LATEST_GATED}" \
  "type=raw,value=release,flavor=suffix=-latest"
expect_fail "flavor-latest" "flavor suffix emitting latest is rejected" "allowlist"

f="$(new_fixture ungated-minor-extra)"
# Ungated minor alias added next to the gated one.
insert_after_line "${f}/.github/workflows/release.yml" "${MINOR_GATED}" "${MINOR_TAG}"
expect_fail "ungated-minor-extra" "an extra ungated {{major}}.{{minor}} tag is rejected" "allowlist"

f="$(new_fixture duplicate-attribute)"
# (a) Appending a duplicate `enable=true` to a gated line must fail: metadata-
# action assigns `tag.attrs[key] = value` in a plain loop, so the later value
# wins at runtime even though the first gate looks correct.
mutate_line "${f}/.github/workflows/release.yml" "${LATEST_GATED}" "${LATEST_GATED},enable=true"
expect_fail "duplicate-attribute" "a repeated enable= attribute is rejected" "repeats attribute"

f="$(new_fixture raw-rolling-extra)"
# (b) An unrecognised raw rolling alias (literal `0.5`) next to the gated
# lines must fail: the allowlist has no room for extra emitters.
insert_lines_after "${f}/.github/workflows/release.yml" "${LATEST_GATED}" "type=raw,value=0.5"
expect_fail "raw-rolling-extra" "an extra literal X.Y raw tag is rejected" "allowlist"

f="$(new_fixture match-rolling-extra)"
# (b) A match source that synthesises a two-component alias must fail too.
insert_lines_after "${f}/.github/workflows/release.yml" "${LATEST_GATED}" \
  'type=match,pattern=\d+\.\d+,group=0'
expect_fail "match-rolling-extra" "an extra two-component match tag is rejected" "allowlist"

f="$(new_fixture other-job-latest)"
# (c) A publisher job outside docker-release must not be able to move an
# alias; every workflow job is scanned, not just docker-release.
append_decoy_job "${f}/.github/workflows/release.yml" "${LATEST_TAG}"
expect_fail "other-job-latest" "an ungated latest in another job is rejected" "moving release alias"

f="$(new_fixture dynamic-tags-other-job)"
# A dynamic tags value outside docker-build-push.yml could resolve to `latest`;
# only caller-supplied inputs in the reusable workflow may be dynamic.
append_dynamic_alias_job "${f}/.github/workflows/release.yml"
expect_fail "dynamic-tags-other-job" "a dynamic tags value in another job is rejected" "dynamic tags value"

f="$(new_fixture flow-mapping-tags)"
# A flow-style `with:` mapping hides the nested tags key from the line scan.
append_flow_mapping_job "${f}/.github/workflows/release.yml"
expect_fail "flow-mapping-tags" "a flow-style with: mapping is rejected" "flow-style mapping"

f="$(new_fixture flow-sequence-tags)"
# A flow sequence of flow mappings (e.g. `steps: [{...}]`) hides the same.
append_flow_sequence_job "${f}/.github/workflows/release.yml"
expect_fail "flow-sequence-tags" "a flow-sequence steps definition is rejected" "flow-style mapping"

f="$(new_fixture plain-latest-tag)"
# Direct build-push-action tags use plain Docker references, not metadata-action
# `type=` syntax; a literal `:latest` must be rejected too.
append_plain_tag_job "${f}/.github/workflows/release.yml"
expect_fail "plain-latest-tag" "a plain :latest Docker tag in another job is rejected" "moving release alias"

f="$(new_fixture promote-backfill-gate-removed)"
# The promote allowlist keeps the backfill restriction explicit: dropping the
# gate that disables main/nightly for a backfill must fail the policy check.
mutate_line "${f}/.github/workflows/promote.yml" \
  "type=raw,value=main,enable=\${{ inputs.allow_backfill != true }}" \
  "type=raw,value=main"
expect_fail "promote-backfill-gate-removed" "removing the promote backfill gate is rejected" "allowlist"

f="$(new_fixture promote-latest-extra)"
# The development-alias allowlist has no room for a release alias.
insert_lines_after "${f}/.github/workflows/promote.yml" \
  "type=raw,value=nightly-\${{ needs.gate.outputs.sha }}" \
  "type=raw,value=latest,enable=\${{ !contains(github.ref_name, '-') }}"
expect_fail "promote-latest-extra" "an extra latest tag in promote.yml is rejected" "allowlist"

f="$(new_fixture docker-build-push-flavor-latest)"
# (d) metadata-action actually runs in docker-build-push.yml; an action-level
# `flavor: latest=true` would force latest onto prereleases.
insert_lines_after "${f}/.github/workflows/docker-build-push.yml" \
  "tags: \${{ inputs.tags }}" "flavor: latest=true"
expect_fail "docker-build-push-flavor-latest" "flavor latest=true in docker-build-push is rejected" "moving-alias metadata flavor"

f="$(new_fixture flavor-block-latest)"
# The block form of a metadata flavor must be scanned line by line too.
mutate_line "${f}/.github/workflows/docker-build-push.yml" \
  "tags: \${{ inputs.tags }}" "flavor: |"
insert_lines_after "${f}/.github/workflows/docker-build-push.yml" \
  "flavor: |" "  latest=true"
expect_fail "flavor-block-latest" "flavor block with latest=true is rejected" "moving-alias metadata flavor"

f="$(new_fixture input-default-latest)"
# (d) The reusable workflow's tags input default must stay immutable as well.
insert_lines_after "${f}/.github/workflows/docker-build-push.yml" \
  "type=semver,pattern={{version}}" "type=raw,value=latest"
expect_fail "input-default-latest" "a moving alias in the workflow_call tags default is rejected" "moving release alias"

f="$(new_fixture yaml-alias-tags)"
# A YAML anchor/alias makes the effective value invisible to a literal scan;
# reject the indirection instead of guessing what it resolves to.
insert_lines_after "${f}/.github/workflows/docker-build-push.yml" \
  "REGISTRY: ghcr.io" "EVIL_TAGS: &evil_tags |" "  type=raw,value=latest"
mutate_line "${f}/.github/workflows/docker-build-push.yml" \
  "tags: \${{ inputs.tags }}" "tags: *evil_tags"
expect_fail "yaml-alias-tags" "a YAML alias in a tags input is rejected" "YAML anchor/alias indirection"

f="$(new_fixture line-split-gate)"
# A gate expression split across source lines must fail closed instead of
# looking like a harmless extra line.
mutate_line "${f}/.github/workflows/release.yml" "${LATEST_GATED}" \
  "type=raw,value=latest,enable=\${{"
insert_lines_after "${f}/.github/workflows/release.yml" \
  "type=raw,value=latest,enable=\${{" "!contains(github.ref_name, '-') }}"
expect_fail "line-split-gate" "a line-split gate expression is rejected" "allowlist"

f="$(new_fixture duplicate-tags-key)"
# A second YAML `tags:` key in docker-release must fail closed (actionlint also
# rejects duplicate keys, but the policy check must not depend on it).
insert_block_before "${f}/.github/workflows/release.yml" "timeout-minutes: 120" \
  "tags: |" "${LATEST_GATED}"
expect_fail "duplicate-tags-key" "a duplicated tags: key in docker-release is rejected" "exactly one tags: block"

f="$(new_fixture folded-tags-block)"
# Folded scalars hide per-line tag structure; require an explicit literal block.
mutate_line "${f}/.github/workflows/release.yml" "tags: |" "tags: >"
expect_fail "folded-tags-block" "a folded (>) docker-release tags block is rejected" "folded"

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
expect_fail "decoy-other-job" "a compliant tags block in another job does not mask docker-release" "allowlist"

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

f="$(new_fixture extra-immutable-job)"
# A tags block in another job is allowed as long as it only emits immutable
# tags; an ungated moving alias there is a negative case above.
append_decoy_job "${f}/.github/workflows/release.yml" "type=sha,format=long"
expect_pass "extra-immutable-job" "an immutable tags block outside docker-release is allowed"

# ---------- totals ----------

echo ""
echo "alias-policy tests: ${PASS} passed, ${FAIL} failed (${NEG} negative, ${POS} positive)"

if [[ "${NEG}" -lt 20 ]]; then
  bad "expected at least 20 negative cases, found ${NEG}"
fi
if [[ "${POS}" -lt 6 ]]; then
  bad "expected at least 6 positive cases, found ${POS}"
fi

[[ "${FAIL}" -eq 0 ]]
