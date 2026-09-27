#!/usr/bin/env bash
set -euo pipefail

# Promotion gate.
#
# A moving development alias (`main`, `nightly`, `nightly-*`) must only
# identify a merged `main` commit whose promotion-gate check set is green
# (RI-C03). This script queries GitHub check-runs for an exact commit and exits
# non-zero unless every required check name is COMPLETED and SUCCESS.
#
# Usage:
#   promotion-gate.sh --repo OWNER/REPO --sha SHA --token TOKEN \
#       --required "Check Name One" --required "Check Name Two" [...]
#
# Environment overrides for hermetic testing:
#   PROMOTION_GATE_API_BASE   base URL (default https://api.github.com)
#   PROMOTION_GATE_HTTP       executable returning check-run JSON for a query
#                             used to inject fixtures in unit tests

REPO=""
SHA=""
TOKEN="${GITHUB_TOKEN:-}"
REQUIRED=()
API_BASE="${PROMOTION_GATE_API_BASE:-https://api.github.com}"

while [[ "$#" -gt 0 ]]; do
  case "$1" in
    --repo) REPO="$2"; shift 2 ;;
    --sha) SHA="$2"; shift 2 ;;
    --token) TOKEN="$2"; shift 2 ;;
    --required) REQUIRED+=("$2"); shift 2 ;;
    *) echo "unknown arg: $1" >&2; exit 2 ;;
  esac
done

if [[ -z "${REPO}" || -z "${SHA}" || -z "${TOKEN}" ]]; then
  echo "promotion-gate.sh: --repo, --sha and --token are required" >&2
  exit 2
fi
if [[ "${#REQUIRED[@]}" -eq 0 ]]; then
  echo "promotion-gate.sh: at least one --required check is required" >&2
  exit 2
fi

# Fetch all check-run records for the commit (paginate).
CHECK_RUNS_JSON=""
PAGE=1
while :; do
  url="${API_BASE}/repos/${REPO}/commits/${SHA}/check-runs?per_page=100&page=${PAGE}"
  if [[ -n "${PROMOTION_GATE_HTTP:-}" ]]; then
    body="$("${PROMOTION_GATE_HTTP}" "${url}")"
  else
    body="$(curl -fsSL -H "Authorization: token ${TOKEN}" -H "Accept: application/vnd.github+json" "${url}")"
  fi
  CHECK_RUNS_JSON="${CHECK_RUNS_JSON}${body}"
  total="$(printf '%s' "${body}" | python3 -c 'import sys,json;print(json.load(sys.stdin).get("total_count",0))' 2>/dev/null || echo 0)"
  page_count="$(printf '%s' "${body}" | python3 -c 'import sys,json;print(len(json.load(sys.stdin).get("check_runs",[])))' 2>/dev/null || echo 0)"
  [[ "${page_count}" -lt 100 ]] && break
  PAGE=$((PAGE + 1))
done

FAILED=0
for name in "${REQUIRED[@]}"; do
  # Literal match on the check-run name (GitHub Actions jobs use the job name,
  # or job id when no name is set).
  found="$(printf '%s' "${CHECK_RUNS_JSON}" | python3 -c "
import sys, json
name = sys.argv[1]
data = json.load(sys.stdin)
for run in data.get('check_runs', []):
    if str(run.get('name', '')).lower() == name.lower():
        print(str(run.get('status', '')) + '|' + str(run.get('conclusion', '')))
        break
" "${name}")"
  if [[ -n "${found}" ]]; then
    state="${found%%|*}"
    concl="${found##*|}"
    if [[ "${state}" == "completed" && "${concl}" == "success" ]]; then
      echo "PASS promotion-required check: ${name}"
    else
      echo "FAIL promotion-required check: ${name} (state=${state}, conclusion=${concl})" >&2
      FAILED=1
    fi
  else
    echo "FAIL promotion-required check: ${name} (no check-run found for ${SHA})" >&2
    FAILED=1
  fi
done

if [[ "${FAILED}" -ne 0 ]]; then
  echo "Promotion gate NOT satisfied for ${SHA}." >&2
  exit 1
fi

echo "Promotion gate satisfied for ${SHA}."
