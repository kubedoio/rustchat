#!/usr/bin/env bash
set -euo pipefail

# Deterministic negative tests for the repository-integrity gate model
# (RI-C01 / RI-C03).
#
# 1. A required merge-gate failure propagates to the CI/Security aggregates.
# 2. A required post-merge health failure blocks moving-alias promotion.
#
# These tests run without network: the aggregate is exercised directly and the
# promotion gate is exercised with fixture check-run payloads.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
AGG="${SCRIPT_DIR}/ci-aggregate.sh"
PROMO="${SCRIPT_DIR}/promotion-gate.sh"

PASS=0
FAIL=0

ok()  { echo "PASS: $1"; PASS=$((PASS + 1)); }
bad() { echo "FAIL: $1" >&2; FAIL=$((FAIL + 1)); }

# ---------- 1. Merge aggregate ----------

run_agg_expect() {
  # run_agg_expect <expected> <desc> <env assignments...>
  local expected="$1"; shift
  local desc="$1"; shift
  local actual=0
  env "$@" "${AGG}" \
    DETECT_CHANGES_RESULT SECURITY_REGRESSION_GUARDS_RESULT \
    BACKEND_CHECK_RESULT FRONTEND_CHECK_RESULT FRONTEND_E2E_RESULT \
    PUSH_PROXY_CHECK_RESULT DOCKER_VALIDATE_RESULT BUILD_RELEASE_RESULT \
    REPO_INTEGRITY_RESULT MIGRATION_MATRIX_RESULT BACKUP_RESTORE_RESULT \
    >/dev/null 2>&1 || actual=$?
  if [[ "${actual}" -eq "${expected}" ]]; then ok "${desc}"; else bad "${desc} (exit ${actual})"; fi
}

# Mirrors the full ci-complete member set in .github/workflows/ci.yml so a
# mis-wiring of any single member (not just BUILD_RELEASE_RESULT) is caught.
ALL_RESULTS=(DETECT_CHANGES_RESULT=success SECURITY_REGRESSION_GUARDS_RESULT=success \
  BACKEND_CHECK_RESULT=success FRONTEND_CHECK_RESULT=success \
  FRONTEND_E2E_RESULT=success PUSH_PROXY_CHECK_RESULT=success \
  DOCKER_VALIDATE_RESULT=success BUILD_RELEASE_RESULT=success \
  REPO_INTEGRITY_RESULT=success MIGRATION_MATRIX_RESULT=success \
  BACKUP_RESTORE_RESULT=success)

run_agg_expect 0 "aggregate: all members success" "${ALL_RESULTS[@]}"

run_agg_expect 0 "aggregate: conditional jobs skipped is acceptable" \
  DETECT_CHANGES_RESULT=success SECURITY_REGRESSION_GUARDS_RESULT=success \
  BACKEND_CHECK_RESULT=skipped FRONTEND_CHECK_RESULT=skipped \
  FRONTEND_E2E_RESULT=skipped PUSH_PROXY_CHECK_RESULT=skipped \
  DOCKER_VALIDATE_RESULT=skipped BUILD_RELEASE_RESULT=skipped \
  REPO_INTEGRITY_RESULT=success MIGRATION_MATRIX_RESULT=skipped \
  BACKUP_RESTORE_RESULT=skipped

run_agg_expect 1 "aggregate: one required backend failure blocks CI Complete" \
  DETECT_CHANGES_RESULT=success SECURITY_REGRESSION_GUARDS_RESULT=success \
  BACKEND_CHECK_RESULT=failure FRONTEND_CHECK_RESULT=success \
  FRONTEND_E2E_RESULT=success PUSH_PROXY_CHECK_RESULT=success \
  DOCKER_VALIDATE_RESULT=success BUILD_RELEASE_RESULT=success \
  REPO_INTEGRITY_RESULT=success MIGRATION_MATRIX_RESULT=success \
  BACKUP_RESTORE_RESULT=success

run_agg_expect 1 "aggregate: missing required member is a wiring failure" \
  DETECT_CHANGES_RESULT=success SECURITY_REGRESSION_GUARDS_RESULT=success \
  BACKEND_CHECK_RESULT=success FRONTEND_CHECK_RESULT=success \
  FRONTEND_E2E_RESULT=success PUSH_PROXY_CHECK_RESULT=success \
  DOCKER_VALIDATE_RESULT=success REPO_INTEGRITY_RESULT=success \
  MIGRATION_MATRIX_RESULT=success BACKUP_RESTORE_RESULT=success

# Security aggregate variant.
sec_agg_expect() {
  local expected="$1"; shift
  local desc="$1"; shift
  local actual=0
  env "$@" "${AGG}" \
    CODEQL_RESULT CARGO_AUDIT_BACKEND_RESULT CARGO_AUDIT_PUSH_PROXY_RESULT \
    CARGO_DENY_BACKEND_RESULT CARGO_DENY_PUSH_PROXY_RESULT NPM_AUDIT_RESULT \
    DEPENDENCY_REVIEW_RESULT >/dev/null 2>&1 || actual=$?
  if [[ "${actual}" -eq "${expected}" ]]; then ok "${desc}"; else bad "${desc} (exit ${actual})"; fi
}

sec_agg_expect 1 "aggregate(security): cargo-audit failure propagates to Security Complete" \
  CODEQL_RESULT=success CARGO_AUDIT_BACKEND_RESULT=success \
  CARGO_AUDIT_PUSH_PROXY_RESULT=failure CARGO_DENY_BACKEND_RESULT=success \
  CARGO_DENY_PUSH_PROXY_RESULT=success NPM_AUDIT_RESULT=success \
  DEPENDENCY_REVIEW_RESULT=success

# ---------- 2. Promotion gate (fixture-driven) ----------

FIXTURE_DIR="$(mktemp -d)"
trap 'rm -rf "${FIXTURE_DIR}"' EXIT

cat > "${FIXTURE_DIR}/all_green.json" <<'JSON'
{"total_count":4,"check_runs":[
  {"name":"CI Complete","status":"completed","conclusion":"success"},
  {"name":"Security Complete","status":"completed","conclusion":"success"},
  {"name":"dco-check","status":"completed","conclusion":"success"},
  {"name":"Backend Integration Tests","status":"completed","conclusion":"success"}
]}
JSON

cat > "${FIXTURE_DIR}/integration_failed.json" <<'JSON'
{"total_count":4,"check_runs":[
  {"name":"CI Complete","status":"completed","conclusion":"success"},
  {"name":"Security Complete","status":"completed","conclusion":"success"},
  {"name":"dco-check","status":"completed","conclusion":"success"},
  {"name":"Backend Integration Tests","status":"completed","conclusion":"failure"}
]}
JSON

cat > "${FIXTURE_DIR}/integration_pending.json" <<'JSON'
{"total_count":4,"check_runs":[
  {"name":"CI Complete","status":"completed","conclusion":"success"},
  {"name":"Security Complete","status":"completed","conclusion":"success"},
  {"name":"dco-check","status":"completed","conclusion":"success"},
  {"name":"Backend Integration Tests","status":"in_progress","conclusion":null}
]}
JSON

cat > "${FIXTURE_DIR}/missing.json" <<'JSON'
{"total_count":3,"check_runs":[
  {"name":"CI Complete","status":"completed","conclusion":"success"},
  {"name":"Security Complete","status":"completed","conclusion":"success"},
  {"name":"dco-check","status":"completed","conclusion":"success"}
]}
JSON

cat > "${FIXTURE_DIR}/http.sh" <<'SH'
#!/usr/bin/env bash
cat "${PROMO_FIXTURE_FILE}"
SH
chmod +x "${FIXTURE_DIR}/http.sh"

REQ=(--required "CI Complete" --required "Security Complete" --required "dco-check" --required "Backend Integration Tests")

promo_expect() {
  # promo_expect <case> <expected> <desc>
  local case="$1"; shift
  local expected="$1"; shift
  local desc="$1"; shift
  local actual=0
  env GITHUB_TOKEN=test PROMOTION_GATE_HTTP="${FIXTURE_DIR}/http.sh" \
    PROMO_FIXTURE_FILE="${FIXTURE_DIR}/${case}.json" \
    "${PROMO}" --repo org/repo --sha abc --token test "${REQ[@]}" \
    >/dev/null 2>&1 || actual=$?
  if [[ "${actual}" -eq "${expected}" ]]; then ok "${desc}"; else bad "${desc} (exit ${actual})"; fi
}

promo_expect all_green           0 "promotion: all checks green -> promotion allowed"
promo_expect integration_failed  1 "promotion: post-merge integration failure -> moving alias blocked"
promo_expect integration_pending 1 "promotion: post-merge integration pending -> moving alias blocked"
promo_expect missing             1 "promotion: missing required promotion check -> moving alias blocked"

# ---------- 3. Promotion gate multi-page pagination ----------
# >100 check-runs on a release commit spans multiple API pages; the gate must
# parse beyond page 1 (regression: concatenated page bodies broke json.load and
# dropped checks after the first 100).

MP_FIXTURE_DIR="$(mktemp -d)"
trap 'rm -rf "${MP_FIXTURE_DIR}" "${FIXTURE_DIR}"' EXIT

# Page 1 holds exactly 100 runs (2 required + 98 dummy); page 2 the other two
# required runs. Generated via Python so the JSON is always well-formed.
python3 - <<'PY' > "${MP_FIXTURE_DIR}/p1.json"
import json
runs = [
    {"name": "CI Complete", "status": "completed", "conclusion": "success"},
    {"name": "Security Complete", "status": "completed", "conclusion": "success"},
] + [
    {"name": f"dummy-{i}", "status": "completed", "conclusion": "success"}
    for i in range(1, 99)
]
print(json.dumps({"total_count": 102, "check_runs": runs}))
PY

python3 - <<'PY' > "${MP_FIXTURE_DIR}/p2.json"
import json
runs = [
    {"name": "dco-check", "status": "completed", "conclusion": "success"},
    {"name": "Backend Integration Tests", "status": "completed", "conclusion": "success"},
]
print(json.dumps({"total_count": 102, "check_runs": runs}))
PY

cat > "${MP_FIXTURE_DIR}/http.sh" <<SH
#!/usr/bin/env bash
case "\$1" in
  *"page=2"*) cat "${MP_FIXTURE_DIR}/p2.json" ;;
  *)          cat "${MP_FIXTURE_DIR}/p1.json" ;;
esac
SH
chmod +x "${MP_FIXTURE_DIR}/http.sh"

env GITHUB_TOKEN=test PROMOTION_GATE_HTTP="${MP_FIXTURE_DIR}/http.sh" \
  "${PROMO}" --repo org/repo --sha abc --token test "${REQ[@]}" \
  >/dev/null 2>&1 && ok "promotion: required checks split across 2 pages -> promotion allowed" \
  || bad "promotion: required checks split across 2 pages (exit $?)"

# Negative: a failing check on page 2 must still be detected as a failure
# (not misreported as "no check-run found").
python3 - "${MP_FIXTURE_DIR}/p2.json" <<'PY'
import json, sys
p = sys.argv[1]
d = json.load(open(p))
for r in d["check_runs"]:
    if r["name"] == "Backend Integration Tests":
        r["conclusion"] = "failure"
json.dump(d, open(p, "w"))
PY
env GITHUB_TOKEN=test PROMOTION_GATE_HTTP="${MP_FIXTURE_DIR}/http.sh" \
  "${PROMO}" --repo org/repo --sha abc --token test "${REQ[@]}" \
  >/dev/null 2>&1 && bad "promotion: page-2 failure blocked promotion (exit 0)" \
  || ok "promotion: page-2 failure blocks promotion"

# ---------- 4. Promotion gate bounded wait mode (--wait) ----------
# The promote workflow triggers via workflow_run as soon as Integration Tests
# completes, but other required checks (e.g. Security/CodeQL) may still be
# running on the same commit. With --wait the gate must distinguish "not
# finished yet" from "failed": pending checks are re-fetched until they settle
# or the timeout budget is exhausted; real failures fail immediately.

W_FIXTURE_DIR="$(mktemp -d)"
trap 'rm -rf "${W_FIXTURE_DIR}" "${MP_FIXTURE_DIR}" "${FIXTURE_DIR}"' EXIT

# Fixture shim returning a different payload per invocation (counter file),
# simulating check-runs appearing/completing between polls. Fixture selection
# is controlled per case via W_FIRST / W_REST environment variables.
cat > "${W_FIXTURE_DIR}/http-counter.sh" <<'SH'
#!/usr/bin/env bash
count_file="${W_COUNTER_FILE}"
n="$(cat "${count_file}" 2>/dev/null || echo 0)"
n=$((n + 1))
echo "${n}" > "${count_file}"
case "${n}" in
  1) cat "${W_FIRST}" ;;
  *) cat "${W_REST}" ;;
esac
SH
chmod +x "${W_FIXTURE_DIR}/http-counter.sh"

wait_run() {
  # wait_run <counter-file> <first-fixture> <rest-fixture> <extra gate args...>
  local counter="$1"; shift
  local first="$1"; shift
  local rest="$1"; shift
  rm -f "${counter}"
  env GITHUB_TOKEN=test PROMOTION_GATE_HTTP="${W_FIXTURE_DIR}/http-counter.sh" \
    W_COUNTER_FILE="${counter}" W_FIRST="${first}" W_REST="${rest}" \
    "${PROMO}" --repo org/repo --sha abc --token test "${REQ[@]}" \
    --wait --wait-interval 1 --wait-timeout 3 "$@" \
    >"${W_FIXTURE_DIR}/out.log" 2>&1
}

# (a) First fetch is missing a check-run, second fetch is complete+success:
# the gate must wait and then PASS.
if wait_run "${W_FIXTURE_DIR}/c-a" \
     "${FIXTURE_DIR}/missing.json" "${FIXTURE_DIR}/all_green.json"; then
  ok "wait: missing-then-green check-run -> promotion allowed"
else
  bad "wait: missing-then-green check-run (exit $?)"
fi
fetches="$(cat "${W_FIXTURE_DIR}/c-a")"
if [[ "${fetches}" -ge 2 ]]; then
  ok "wait: missing-then-green re-fetched check-runs (${fetches} fetches)"
else
  bad "wait: missing-then-green did not re-fetch (${fetches} fetches)"
fi

# (b) A completed-with-failure check must FAIL immediately, without further
# fetches (no waiting on real failures).
if wait_run "${W_FIXTURE_DIR}/c-b" \
     "${FIXTURE_DIR}/integration_failed.json" "${FIXTURE_DIR}/all_green.json"; then
  bad "wait: completed failure blocked promotion (exit 0)"
else
  ok "wait: completed failure -> moving alias blocked immediately"
fi
fetches="$(cat "${W_FIXTURE_DIR}/c-b")"
if [[ "${fetches}" -eq 1 ]]; then
  ok "wait: completed failure fetched exactly once (no waiting)"
else
  bad "wait: completed failure kept fetching (${fetches} fetches)"
fi

# (c) A never-completing check plus a tiny --wait-timeout must FAIL with a
# clear timeout message. Both flavors of "not finished yet": a check-run stuck
# in_progress and a check-run that never appears.
if wait_run "${W_FIXTURE_DIR}/c-c1" \
     "${FIXTURE_DIR}/integration_pending.json" "${FIXTURE_DIR}/integration_pending.json"; then
  bad "wait: never-completing in_progress check blocked promotion (exit 0)"
else
  ok "wait: never-completing in_progress check -> moving alias blocked"
fi
if grep -q "timed out waiting for Backend Integration Tests" "${W_FIXTURE_DIR}/out.log"; then
  ok "wait: in_progress timeout reports timed-out check name"
else
  bad "wait: in_progress timeout message missing check name"
fi

if wait_run "${W_FIXTURE_DIR}/c-c2" \
     "${FIXTURE_DIR}/missing.json" "${FIXTURE_DIR}/missing.json"; then
  bad "wait: never-appearing check blocked promotion (exit 0)"
else
  ok "wait: never-appearing check -> moving alias blocked"
fi
if grep -q "timed out waiting for Backend Integration Tests" "${W_FIXTURE_DIR}/out.log"; then
  ok "wait: never-appearing timeout reports timed-out check name"
else
  bad "wait: never-appearing timeout message missing check name"
fi

# (d) --wait-interval 0 must be clamped to a 1s floor. The wait loop clamps
# each sleep to the remaining budget, so a zero interval would leave `waited`
# stuck at 0 and the loop would poll unbounded without ever timing out
# (review reproduction: 92 fetches in 6s). The extra args below override
# wait_run's default --wait-interval/--wait-timeout (later flags win).
clamp_rc=0
wait_run "${W_FIXTURE_DIR}/c-d" \
  "${FIXTURE_DIR}/integration_pending.json" "${FIXTURE_DIR}/integration_pending.json" \
  --wait-interval 0 --wait-timeout 2 || clamp_rc=$?
fetches="$(cat "${W_FIXTURE_DIR}/c-d" 2>/dev/null || echo 0)"
if [[ "${clamp_rc}" -eq 1 ]] \
   && grep -q "timed out waiting for Backend Integration Tests" "${W_FIXTURE_DIR}/out.log" \
   && [[ "${fetches}" -le 3 ]]; then
  ok "wait: --wait-interval 0 clamped to 1s floor (timed out cleanly, ${fetches} fetches)"
else
  bad "wait: --wait-interval 0 not clamped (exit ${clamp_rc}, ${fetches} fetches)"
fi

echo ""
echo "gate-propagation tests: ${PASS} passed, ${FAIL} failed"
[[ "${FAIL}" -eq 0 ]]
