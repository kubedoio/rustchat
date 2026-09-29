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
#       --required "Check Name One" --required "Check Name Two" [...] \
#       [--wait] [--wait-interval SECONDS] [--wait-timeout SECONDS]
#
# Without --wait, the check-runs are fetched ONCE: a required check that is
# missing or not yet completed is treated as a failure. This is the original
# behavior and is kept byte-for-byte for existing callers.
#
# With --wait (opt-in bounded wait), the script distinguishes "not finished
# yet" from "failed", because callers such as the promote workflow trigger as
# soon as ONE required workflow completes while others (e.g. Security/CodeQL)
# may still be running on the same commit:
#   - a required check completed with a non-success conclusion -> FAIL now;
#   - a required check missing or not yet completed -> sleep --wait-interval
#     seconds, re-fetch, and re-evaluate, until --wait-timeout seconds have
#     elapsed, then FAIL with a "timed out waiting for <name>" message;
#   - all required checks completed+success -> PASS.
# Waiting never weakens the gate: aliases still advance only when the full
# required set is green for the exact commit (RI-C03).
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
WAIT=0
WAIT_INTERVAL=60
WAIT_TIMEOUT=1800

while [[ "$#" -gt 0 ]]; do
  case "$1" in
    --repo) REPO="$2"; shift 2 ;;
    --sha) SHA="$2"; shift 2 ;;
    --token) TOKEN="$2"; shift 2 ;;
    --required) REQUIRED+=("$2"); shift 2 ;;
    --wait) WAIT=1; shift ;;
    --wait-interval) WAIT_INTERVAL="$2"; shift 2 ;;
    --wait-timeout) WAIT_TIMEOUT="$2"; shift 2 ;;
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

PAGE_DIR="$(mktemp -d)"
trap 'rm -rf "${PAGE_DIR}"' EXIT
RUNS_FILE="${PAGE_DIR}/runs.jsonl"

# Fetch all check-run records for the commit (paginate). Each page body is a
# separate JSON document, so pages are merged into a single JSON-lines file of
# check_runs rather than concatenated (concatenation breaks json.load once
# there are 2+ pages, i.e. >100 check-runs on a release commit). The wait loop
# re-fetches through this same function so fixtures stay injectable.
fetch_check_runs() {
  local page=1
  local url body page_count
  : > "${RUNS_FILE}"
  while :; do
    url="${API_BASE}/repos/${REPO}/commits/${SHA}/check-runs?per_page=100&page=${page}"
    if [[ -n "${PROMOTION_GATE_HTTP:-}" ]]; then
      body="$("${PROMOTION_GATE_HTTP}" "${url}")"
    else
      body="$(curl -fsSL -H "Authorization: token ${TOKEN}" -H "Accept: application/vnd.github+json" "${url}")"
    fi
    printf '%s' "${body}" | python3 -c '
import sys, json
runs = json.load(sys.stdin).get("check_runs", [])
with open(sys.argv[1], "a") as f:
    for run in runs:
        f.write(json.dumps(run) + "\n")
' "${RUNS_FILE}"
    page_count="$(printf '%s' "${body}" | python3 -c 'import sys,json;print(len(json.load(sys.stdin).get("check_runs",[])))' 2>/dev/null || echo 0)"
    [[ "${page_count}" -lt 100 ]] && break
    page=$((page + 1))
  done
}

# Evaluate the required checks against the fetched check-runs.
#
# Returns:
#   0  every required check is completed with success
#   1  at least one required check completed with a non-success conclusion,
#      or (without --wait) is missing / not yet completed: treated as failure
#   2  with --wait only: at least one required check is missing or not yet
#      completed and none completed with a non-success conclusion
#
# Pending check names are collected in PENDING so the caller can report which
# checks timed out.
PENDING=()
evaluate_required_checks() {
  local name found state concl definitive=0 pending=0
  PENDING=()
  for name in "${REQUIRED[@]}"; do
    # Literal match on the check-run name (GitHub Actions jobs use the job name,
    # or job id when no name is set). Search the merged JSON-lines file so runs
    # beyond the first page are not dropped.
    found="$(python3 -c "
import sys, json
name = sys.argv[1]
with open(sys.argv[2]) as f:
    for line in f:
        line = line.strip()
        if not line:
            continue
        run = json.loads(line)
        if str(run.get('name', '')).lower() == name.lower():
            print(str(run.get('status', '')) + '|' + str(run.get('conclusion', '')))
            break
" "${name}" "${RUNS_FILE}")"
    if [[ -n "${found}" ]]; then
      state="${found%%|*}"
      concl="${found##*|}"
      if [[ "${state}" == "completed" && "${concl}" == "success" ]]; then
        echo "PASS promotion-required check: ${name}"
      elif [[ "${state}" == "completed" ]]; then
        echo "FAIL promotion-required check: ${name} (state=${state}, conclusion=${concl})" >&2
        definitive=1
      elif [[ "${WAIT}" -eq 1 ]]; then
        echo "WAIT promotion-required check: ${name} (state=${state}, not yet completed)" >&2
        PENDING+=("${name}")
        pending=1
      else
        echo "FAIL promotion-required check: ${name} (state=${state}, conclusion=${concl})" >&2
        definitive=1
      fi
    elif [[ "${WAIT}" -eq 1 ]]; then
      echo "WAIT promotion-required check: ${name} (no check-run found yet for ${SHA})" >&2
      PENDING+=("${name}")
      pending=1
    else
      echo "FAIL promotion-required check: ${name} (no check-run found for ${SHA})" >&2
      definitive=1
    fi
  done
  if [[ "${definitive}" -ne 0 ]]; then
    return 1
  fi
  if [[ "${pending}" -ne 0 ]]; then
    return 2
  fi
  return 0
}

if [[ "${WAIT}" -eq 1 ]]; then
  # Bounded wait: re-fetch and re-evaluate until every required check is
  # completed (success or failure) or the timeout budget is exhausted. Real
  # failures fail immediately; only genuinely pending checks are waited on.
  waited=0
  while :; do
    fetch_check_runs
    verdict=0
    evaluate_required_checks || verdict=$?
    if [[ "${verdict}" -eq 0 ]]; then
      echo "Promotion gate satisfied for ${SHA}."
      exit 0
    fi
    if [[ "${verdict}" -eq 1 ]]; then
      echo "Promotion gate NOT satisfied for ${SHA}." >&2
      exit 1
    fi
    # verdict 2: required checks still pending.
    if [[ "${waited}" -ge "${WAIT_TIMEOUT}" ]]; then
      for name in "${PENDING[@]}"; do
        echo "FAIL promotion-required check: ${name} (timed out waiting for ${name} after ${WAIT_TIMEOUT}s on ${SHA})" >&2
      done
      echo "Promotion gate NOT satisfied for ${SHA}." >&2
      exit 1
    fi
    sleep_for="${WAIT_INTERVAL}"
    remaining=$((WAIT_TIMEOUT - waited))
    if [[ "${sleep_for}" -gt "${remaining}" ]]; then
      sleep_for="${remaining}"
    fi
    echo "Waiting ${sleep_for}s for pending promotion-gate checks (${waited}s elapsed of ${WAIT_TIMEOUT}s budget)..." >&2
    sleep "${sleep_for}"
    waited=$((waited + sleep_for))
  done
fi

# Default (no --wait): single fetch, missing/pending treated as failure.
fetch_check_runs
if evaluate_required_checks; then
  echo "Promotion gate satisfied for ${SHA}."
else
  echo "Promotion gate NOT satisfied for ${SHA}." >&2
  exit 1
fi
