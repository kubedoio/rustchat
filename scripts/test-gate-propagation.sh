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
    >/dev/null 2>&1 || actual=$?
  if [[ "${actual}" -eq "${expected}" ]]; then ok "${desc}"; else bad "${desc} (exit ${actual})"; fi
}

ALL_RESULTS=(DETECT_CHANGES_RESULT=success SECURITY_REGRESSION_GUARDS_RESULT=success \
  BACKEND_CHECK_RESULT=success FRONTEND_CHECK_RESULT=success \
  FRONTEND_E2E_RESULT=success PUSH_PROXY_CHECK_RESULT=success \
  DOCKER_VALIDATE_RESULT=success BUILD_RELEASE_RESULT=success)

run_agg_expect 0 "aggregate: all members success" "${ALL_RESULTS[@]}"

run_agg_expect 0 "aggregate: conditional jobs skipped is acceptable" \
  DETECT_CHANGES_RESULT=success SECURITY_REGRESSION_GUARDS_RESULT=success \
  BACKEND_CHECK_RESULT=skipped FRONTEND_CHECK_RESULT=skipped \
  FRONTEND_E2E_RESULT=skipped PUSH_PROXY_CHECK_RESULT=skipped \
  DOCKER_VALIDATE_RESULT=skipped BUILD_RELEASE_RESULT=skipped

run_agg_expect 1 "aggregate: one required backend failure blocks CI Complete" \
  DETECT_CHANGES_RESULT=success SECURITY_REGRESSION_GUARDS_RESULT=success \
  BACKEND_CHECK_RESULT=failure FRONTEND_CHECK_RESULT=success \
  FRONTEND_E2E_RESULT=success PUSH_PROXY_CHECK_RESULT=success \
  DOCKER_VALIDATE_RESULT=success BUILD_RELEASE_RESULT=success

run_agg_expect 1 "aggregate: missing required member is a wiring failure" \
  DETECT_CHANGES_RESULT=success SECURITY_REGRESSION_GUARDS_RESULT=success \
  BACKEND_CHECK_RESULT=success FRONTEND_CHECK_RESULT=success \
  FRONTEND_E2E_RESULT=success PUSH_PROXY_CHECK_RESULT=success \
  DOCKER_VALIDATE_RESULT=success

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

echo ""
echo "gate-propagation tests: ${PASS} passed, ${FAIL} failed"
[[ "${FAIL}" -eq 0 ]]
