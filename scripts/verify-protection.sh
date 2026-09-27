#!/usr/bin/env bash
set -euo pipefail

# Verify live GitHub protection matches documented governance (RI-C02).
#
# Usage: GITHUB_TOKEN=... verify-protection.sh [OWNER/REPO]
#   default repo: github.repository or kubedoio/rustchat
#
# This is a reproducible inspection procedure; it requires a token with admin
# read access to repository settings.

REPO="${1:-${GITHUB_REPOSITORY:-kubedoio/rustchat}}"
TOKEN="${GITHUB_TOKEN:?GITHUB_TOKEN is required}"
API="https://api.github.com"

FAIL=0

require() {
  local desc="$1"
  if eval "$2"; then
    echo "PASS: ${desc}"
  else
    echo "FAIL: ${desc}" >&2
    FAIL=1
  fi
}

BRANCH_PROTECTION="$(curl -fsSL -H "Authorization: token ${TOKEN}" -H "Accept: application/vnd.github+json" "${API}/repos/${REPO}/branches/main/protection")"

contexts="$(printf '%s' "${BRANCH_PROTECTION}" | python3 -c 'import sys,json;print(sorted(json.load(sys.stdin).get("required_status_checks",{}).get("contexts",[])))')"
required_checks="['CI Complete', 'Security Complete', 'dco-check']"

require "main requires the merge-gate status contexts" \
  "[[ \"${contexts}\" == \"${required_checks}\" ]]"

require "main enforces admins" \
  "[[ \"$(printf '%s' "${BRANCH_PROTECTION}" | python3 -c 'import sys,json;print(json.load(sys.stdin).get("enforce_admins",{}).get("enabled"))')\" == \"True\" ]]"

require "main blocks force pushes for ordinary contributors" \
  "[[ \"$(printf '%s' "${BRANCH_PROTECTION}" | python3 -c 'import sys,json;print(json.load(sys.stdin).get("allow_force_pushes",{}).get("enabled"))')\" == \"False\" ]]"

require "main blocks branch deletion" \
  "[[ \"$(printf '%s' "${BRANCH_PROTECTION}" | python3 -c 'import sys,json;print(json.load(sys.stdin).get("allow_deletions",{}).get("enabled"))')\" == \"False\" ]]"

require "main requires conversation resolution" \
  "[[ \"$(printf '%s' "${BRANCH_PROTECTION}" | python3 -c 'import sys,json;print(json.load(sys.stdin).get("required_conversation_resolution",{}).get("enabled"))')\" == \"True\" ]]"

RULESETS="$(curl -fsSL -H "Authorization: token ${TOKEN}" -H "Accept: application/vnd.github+json" "${API}/repos/${REPO}/rulesets")"

# Find an active "Protect release tags" ruleset targeting tags with refs/tags/v*.
tag_ruleset_target="$(printf '%s' "${RULESETS}" | python3 -c '
import sys, json
for r in json.load(sys.stdin):
    if r.get("name") == "Protect release tags" and r.get("target") == "tag":
        print(r.get("id"))
        break
')"

if [[ -n "${tag_ruleset_target}" ]]; then
  require "release-tag ruleset targets tags (id ${tag_ruleset_target})" "true"
  detail="$(curl -fsSL -H "Authorization: token ${TOKEN}" "${API}/repos/${REPO}/rulesets/${tag_ruleset_target}")"
  require "release-tag ruleset includes refs/tags/v*" \
    "printf '%s' \"\${detail}\" | python3 -c 'import sys,json; inc=json.load(sys.stdin).get(\"conditions\",{}).get(\"ref_name\",{}).get(\"include\",[]); sys.exit(0 if \"refs/tags/v*\" in inc else 1)'"
else
  require "release-tag ruleset targets tags" "false"
fi

if [[ "${FAIL}" -ne 0 ]]; then
  echo "Live protection verification FAILED."
  exit 1
fi

echo "Live protection verification passed for ${REPO}."
exit 0
