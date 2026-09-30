#!/usr/bin/env bash
set -euo pipefail

# Deterministic tests for scripts/record-image-digests.sh.
#
# The script talks to the registry HTTP API via curl; these tests run
# without network by putting a fixture-backed fake curl first on PATH.
# Covered:
#   1. Stable release, aliases consistent (enforced)  -> exit 0, full record
#   2. Alias drift under --enforce-aliases            -> exit 1
#   3. Version tag missing (HTTP 404)                 -> exit 1
#   4. Prerelease without --enforce-aliases           -> exit 0, aliases not touched
#   5. --output writes the record to a file
#   6. Registry credentials are forwarded to the token endpoint when set
#   7. Malformed/childless manifests and token-endpoint failures are fatal
#   8. Argument handling: missing values, boolean =value, service whitespace
#   9. Failure-branch specificity: 404 vs digest-less 200 vs drift vs
#      malformed/childless; recorded alias digests equal the version digest;
#      token requests carry -f; the anonymous-auth run itself succeeds
#  10. Transient-registry retry: converging tags resolve, exhausted retries
#      fail closed, auth errors fail immediately, settings are validated

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TARGET="${SCRIPT_DIR}/record-image-digests.sh"

# Keep retry behavior fast and deterministic in tests (the script defaults
# to 5 attempts / 5s delay for the live registry consistency window).
export RECORD_IMAGE_DIGESTS_ATTEMPTS=2
export RECORD_IMAGE_DIGESTS_RETRY_DELAY=0

PASS=0
FAIL=0
ok()  { echo "PASS: $1"; PASS=$((PASS + 1)); }
bad() { echo "FAIL: $1" >&2; FAIL=$((FAIL + 1)); }

WORK="$(mktemp -d)"
trap 'rm -rf "${WORK}"' EXIT
FAKE_BIN="${WORK}/bin"
FIXTURES="${WORK}/fixtures"
CURL_LOG="${WORK}/curl.log"
mkdir -p "${FAKE_BIN}" "${FIXTURES}"
: > "${CURL_LOG}"

# ---------- fake curl ----------
# Serves token and manifest requests from fixture files named
# "<repo with / replaced by __>__<ref>" with suffixes:
#   .json    manifest body (token.json for the token endpoint)
#   .digest  docker-content-digest for the response headers
#   .status  HTTP status (default 200; e.g. 404)
# Only the flags record-image-digests.sh uses are supported; -w supports
# the literal '%{http_code}' format only.
cat > "${FAKE_BIN}/curl" <<'FAKE'
#!/usr/bin/env bash
set -euo pipefail
FIXTURES="${FAKE_FIXTURES}"
hdr="" body="" wfmt="" url="" auth="" flag_f=0
while [ $# -gt 0 ]; do
  case "$1" in
    -D) hdr="$2"; shift 2 ;;
    -o) body="$2"; shift 2 ;;
    -w) wfmt="$2"; shift 2 ;;
    -u) auth="$2"; shift 2 ;;
    -H|--header) shift 2 ;;
    -s|-S|-f|-sS|-sSf)
      case "$1" in *f*) flag_f=1 ;; esac
      shift ;;
    -*) echo "fake curl: unhandled flag $1" >&2; exit 64 ;;
    *) url="$1"; shift ;;
  esac
done
if [ -n "${auth}" ]; then printf -- '-u %s\n' "${auth}" >> "${FAKE_CURL_LOG}"; fi
case "${url}" in
  */token?scope=*)
    # The real script must pass -f so token failures are fatal.
    if [ "${flag_f}" -ne 1 ]; then
      echo "fake curl: token request arrived without -f" >&2; exit 64
    fi
    repo="${url##*scope=repository:}"; repo="${repo%%:*}"
    name="${repo//\//__}"
    if [ -f "${FIXTURES}/${name}__token.hardfail" ]; then exit 22; fi
    if [ -f "${FIXTURES}/${name}__token.errors" ]; then
      cat "${FIXTURES}/${name}__token.errors"
    else
      cat "${FIXTURES}/token.json"
    fi
    ;;
  */v2/*/manifests/*)
    path="${url#*/v2/}"
    repo="${path%%/manifests/*}"
    ref="${path##*/}"
    name="${repo//\//__}__${ref}"
    status="$(cat "${FIXTURES}/${name}.status" 2>/dev/null || echo 200)"
    # A .retry fixture makes the endpoint return 404 that many times before
    # serving normally (models registry convergence on fresh tags).
    if [ -f "${FIXTURES}/${name}.retry" ]; then
      left="$(cat "${FIXTURES}/${name}.retry")"
      if [ "${left}" -gt 0 ]; then
        echo "$((left - 1))" > "${FIXTURES}/${name}.retry"
        status=404
      fi
    fi
    digest="$(cat "${FIXTURES}/${name}.digest" 2>/dev/null || true)"
    if [ -n "${hdr}" ]; then
      {
        printf 'HTTP/1.1 %s\r\n' "${status}"
        printf 'Content-Type: application/vnd.oci.image.index.v1+json\r\n'
        if [ -n "${digest}" ]; then
          printf 'Docker-Content-Digest: %s\r\n' "${digest}"
        fi
        printf '\r\n'
      } > "${hdr}"
    fi
    if [ -n "${body}" ]; then
      if [ -f "${FIXTURES}/${name}.json" ]; then
        cat "${FIXTURES}/${name}.json" > "${body}"
      else
        : > "${body}"
      fi
    fi
    if [ -n "${wfmt}" ]; then printf '%s' "${status}"; fi
    ;;
  *) echo "fake curl: unexpected url ${url}" >&2; exit 64 ;;
esac
FAKE
chmod +x "${FAKE_BIN}/curl"

run_target() {
  PATH="${FAKE_BIN}:$PATH" \
    FAKE_FIXTURES="${FIXTURES}" FAKE_CURL_LOG="${CURL_LOG}" \
    bash "${TARGET}" "$@"
}

run_expect() {
  # run_expect <expected-exit> <desc> <args...>
  local expected="$1"; shift
  local desc="$1"; shift
  local actual=0
  run_target "$@" >/dev/null 2>&1 || actual=$?
  if [ "${actual}" -eq "${expected}" ]; then ok "${desc}"
  else bad "${desc} (exit ${actual}, expected ${expected})"; fi
}

# ---------- fixtures ----------
# Multi-arch index with a real platform pair plus an attestation manifest
# (unknown/unknown), which must be filtered from the recorded platforms.
index_json() {
  printf '%s' '{"schemaVersion":2,"mediaType":"application/vnd.oci.image.index.v1+json","manifests":[
    {"mediaType":"application/vnd.oci.image.manifest.v1+json","digest":"sha256:aaaa111111111111111111111111111111111111111111111111111111111111","size":528,"platform":{"os":"linux","architecture":"amd64"}},
    {"mediaType":"application/vnd.oci.image.manifest.v1+json","digest":"sha256:bbbb222222222222222222222222222222222222222222222222222222222222","size":528,"platform":{"os":"linux","architecture":"arm64"}},
    {"mediaType":"application/vnd.oci.image.manifest.v1+json","digest":"sha256:cccc333333333333333333333333333333333333333333333333333333333333","size":528,"platform":{"os":"unknown","architecture":"unknown"}}]}'
}
digest_for() { printf 'sha256:%s' "$(printf '%s' "$1" | sha256sum | cut -d' ' -f1)"; }
printf '{"token":"fake-token"}' > "${FIXTURES}/token.json"

for svc in backend frontend; do
  name="acme__rustchat-${svc}"
  index_json > "${FIXTURES}/${name}__0.5.1.json"
  index_json > "${FIXTURES}/${name}__0.5.1-rc.1.json"
  # Stable aliases point at the same manifest as the version tag.
  for ref in 0.5.1 0.5 latest; do
    digest_for "index-${svc}" > "${FIXTURES}/${name}__${ref}.digest"
  done
  # Prerelease tag has its own manifest; latest (above) stays on stable.
  digest_for "rc-${svc}" > "${FIXTURES}/${name}__0.5.1-rc.1.digest"
done
# Missing-tag case and a drifted latest digest used by the drift case.
printf '404' > "${FIXTURES}/acme__rustchat-backend__9.9.9.status"
digest_for "drifted" > "${FIXTURES}/drifted.digest"

PREFIX_ARGS=(--prefix "ghcr.io/acme/rustchat" "--services=backend,frontend")

# ---------- 1. stable happy path (enforced) ----------
run_expect 0 "stable: aliases consistent, enforcement passes" \
  "${PREFIX_ARGS[@]}" --version 0.5.1 --enforce-aliases

OUT="$(run_target "${PREFIX_ARGS[@]}" --version 0.5.1 --enforce-aliases 2>/dev/null || true)"
if echo "${OUT}" | grep -q '^# ghcr.io/acme/rustchat-backend$' \
  && echo "${OUT}" | grep -q 'version tag 0.5.1: ghcr.io/acme/rustchat-backend@' \
  && echo "${OUT}" | grep -q 'linux/amd64: sha256:aaaa1111' \
  && echo "${OUT}" | grep -q 'linux/arm64: sha256:bbbb2222' \
  && echo "${OUT}" | grep -q 'alias latest:' \
  && echo "${OUT}" | grep -q '^# ghcr.io/acme/rustchat-frontend$'; then
  ok "stable: record contains image, digest, platforms, aliases"
else
  bad "stable: record content incomplete"
fi
if ! echo "${OUT}" | grep -q 'unknown/unknown'; then
  ok "stable: attestation manifests are filtered out"
else
  bad "stable: attestation manifest leaked into record"
fi
# Recorded alias digests must equal the version-tag digest (not merely exist).
# The extractions rely on backend being processed before frontend (services
# list order), so head -1 selects the backend's lines.
vd="$(echo "${OUT}" | awk '/version tag 0.5.1: ghcr.io\/acme\/rustchat-backend@/{print $NF}' | sed 's/.*@//' | head -1)"
ld="$(echo "${OUT}" | awk '/alias latest:/{print $NF}' | head -1)"
if [ -n "${vd}" ] && [ "${vd}" = "${ld}" ]; then
  ok "stable: recorded alias digest equals version digest"
else
  bad "stable: alias digest mismatch in record (version=${vd} latest=${ld})"
fi

# ---------- 2. alias drift ----------
cp "${FIXTURES}/acme__rustchat-backend__latest.digest" "${WORK}/latest.orig"
cp "${FIXTURES}/drifted.digest" "${FIXTURES}/acme__rustchat-backend__latest.digest"
run_expect 1 "stable: alias drift fails under --enforce-aliases" \
  "${PREFIX_ARGS[@]}" --version 0.5.1 --enforce-aliases
ERRDRIFT="$(run_target "${PREFIX_ARGS[@]}" --version 0.5.1 --enforce-aliases 2>&1 >/dev/null || true)"
if echo "${ERRDRIFT}" | grep -q 'alias drift on ghcr.io/acme/rustchat-backend'; then
  ok "stable: drift reported via the alias-drift error branch"
else
  bad "stable: wrong failure branch for drift (got: ${ERRDRIFT})"
fi
mv "${WORK}/latest.orig" "${FIXTURES}/acme__rustchat-backend__latest.digest"

# ---------- 3. missing version tag ----------
run_expect 1 "missing: version tag 404 fails" \
  "${PREFIX_ARGS[@]}" --version 9.9.9
ERR404="$(run_target "${PREFIX_ARGS[@]}" --version 9.9.9 2>&1 >/dev/null || true)"
if echo "${ERR404}" | grep -q 'not resolved (HTTP 404)'; then
  ok "missing: 404 reported via the not-resolved error branch"
else
  bad "missing: wrong failure branch for 404 (got: ${ERR404})"
fi

# 200 response without a docker-content-digest header hits the
# no-digest-header branch specifically.
index_json > "${FIXTURES}/acme__rustchat-backend__0.8.0.json"
ERRDH="$(run_target --prefix ghcr.io/acme/rustchat --services backend \
  --version 0.8.0 2>&1 >/dev/null || true)"
if echo "${ERRDH}" | grep -q 'no digest header'; then
  ok "missing: digest-less 200 reported via the no-digest-header branch"
else
  bad "missing: wrong failure branch for digest-less 200 (got: ${ERRDH})"
fi

# ---------- 4. prerelease without enforcement ----------
run_expect 0 "prerelease: aliases not enforced" \
  "${PREFIX_ARGS[@]}" --version 0.5.1-rc.1

OUT_RC="$(run_target "${PREFIX_ARGS[@]}" --version 0.5.1-rc.1 2>/dev/null || true)"
if echo "${OUT_RC}" | grep -q 'version tag 0.5.1-rc.1: ghcr.io/acme/rustchat-backend@' \
  && ! echo "${OUT_RC}" | grep -q 'alias latest:'; then
  ok "prerelease: records version digest, does not touch aliases"
else
  bad "prerelease: record wrong (aliases touched or version digest missing)"
fi

# ---------- 5. --output mode ----------
OFILE="${WORK}/digests.txt"
run_expect 0 "output: writes record to file" \
  "${PREFIX_ARGS[@]}" --version 0.5.1 --enforce-aliases --output "${OFILE}"
if grep -q 'version tag 0.5.1: ghcr.io/acme/rustchat-backend@' "${OFILE}"; then
  ok "output: file contains the digest record"
else
  bad "output: file missing digest record"
fi

# ---------- 6. credential forwarding ----------
: > "${CURL_LOG}"
PATH="${FAKE_BIN}:$PATH" FAKE_FIXTURES="${FIXTURES}" FAKE_CURL_LOG="${CURL_LOG}" \
  REGISTRY_USER=ci REGISTRY_TOKEN=sekrit \
  bash "${TARGET}" --prefix ghcr.io/acme/rustchat --services backend \
  --version 0.5.1 >/dev/null 2>&1 || true
if grep -q -- '-u ci:sekrit' "${CURL_LOG}"; then
  ok "auth: credentials forwarded to token endpoint when set"
else
  bad "auth: credentials not forwarded"
fi

: > "${CURL_LOG}"
# Assert the run itself succeeded before trusting the absence-based log
# check below (a failed target would otherwise vacuously pass it).
anon_rc=0
run_target --prefix ghcr.io/acme/rustchat --services backend \
  --version 0.5.1 >/dev/null 2>&1 || anon_rc=$?
if [ "${anon_rc}" -eq 0 ] && ! grep -q -- '-u ' "${CURL_LOG}"; then
  ok "auth: anonymous token request when no credentials set"
else
  bad "auth: anonymous run failed (exit ${anon_rc}) or credentials sent without env set"
fi

# ---------- 7. malformed / childless manifests and token failures ----------
# A record that silently omits platform digests would defeat the evidence
# contract, so parse failures and childless indexes must be fatal.
printf '{not json' > "${FIXTURES}/acme__rustchat-backend__0.6.0.json"
digest_for "malformed" > "${FIXTURES}/acme__rustchat-backend__0.6.0.digest"
run_expect 1 "malformed: unparseable manifest body fails" \
  --prefix ghcr.io/acme/rustchat --services backend --version 0.6.0
ERRMF="$(run_target --prefix ghcr.io/acme/rustchat --services backend \
  --version 0.6.0 2>&1 >/dev/null || true)"
if echo "${ERRMF}" | grep -q 'not a usable multi-arch index'; then
  ok "malformed: reported via the unusable-index error branch"
else
  bad "malformed: wrong failure branch (got: ${ERRMF})"
fi

printf '{"schemaVersion":2,"mediaType":"application/vnd.oci.image.index.v1+json"}' \
  > "${FIXTURES}/acme__rustchat-backend__0.7.0.json"
digest_for "childless" > "${FIXTURES}/acme__rustchat-backend__0.7.0.digest"
run_expect 1 "childless: index without platform children fails" \
  --prefix ghcr.io/acme/rustchat --services backend --version 0.7.0
ERRCL="$(run_target --prefix ghcr.io/acme/rustchat --services backend \
  --version 0.7.0 2>&1 >/dev/null || true)"
if echo "${ERRCL}" | grep -q 'not a usable multi-arch index'; then
  ok "childless: reported via the unusable-index error branch"
else
  bad "childless: wrong failure branch (got: ${ERRCL})"
fi

# Token endpoint returning HTTP 200 with an errors body yields an empty
# token (jq null handling) rather than a literal "null" bearer token.
printf '{"errors":[{"code":"DENIED"}]}' > "${FIXTURES}/acme__rustchat-backend__token.errors"
ERR="$(run_target --prefix ghcr.io/acme/rustchat --services backend \
  --version 0.5.1 2>&1 >/dev/null || true)"
if echo "${ERR}" | grep -q 'no registry token obtained'; then
  ok "token: errors body rejected with clear message"
else
  bad "token: errors body not handled (got: ${ERR})"
fi
rm "${FIXTURES}/acme__rustchat-backend__token.errors"

# Hard token-endpoint failure (curl non-zero) exits 1, not curl's exit code.
printf '%s' "" > "${FIXTURES}/acme__rustchat-backend__token.hardfail"
ERR="$(run_target --prefix ghcr.io/acme/rustchat --services backend \
  --version 0.5.1 2>&1 >/dev/null || true)"
if echo "${ERR}" | grep -q 'failed to obtain a registry token'; then
  ok "token: hard failure reported with clear message"
else
  bad "token: hard failure not handled (got: ${ERR})"
fi
rm "${FIXTURES}/acme__rustchat-backend__token.hardfail"

# ---------- 8. argument handling ----------
run_expect 2 "args: missing option value is a usage error" \
  --prefix ghcr.io/acme/rustchat --version
run_expect 2 "args: boolean flag with =value is a usage error" \
  --prefix ghcr.io/acme/rustchat --version 0.5.1 --enforce-aliases=1
run_expect 0 "args: whitespace in --services is tolerated" \
  --prefix ghcr.io/acme/rustchat "--services=backend, frontend" --version 0.5.1

# ---------- 9. transient-registry retry ----------
# A freshly-published tag may 404 briefly while the registry converges; the
# script must retry transient statuses (404/408/425/429/5xx) and succeed
# once the tag appears, while still failing when it never does.
index_json > "${FIXTURES}/acme__rustchat-backend__0.4.2.json"
digest_for "converging" > "${FIXTURES}/acme__rustchat-backend__0.4.2.digest"
printf '2' > "${FIXTURES}/acme__rustchat-backend__0.4.2.retry"
export RECORD_IMAGE_DIGESTS_ATTEMPTS=3
OUTRETRY="$(run_target \
  --prefix ghcr.io/acme/rustchat --services backend --version 0.4.2 2>/dev/null || true)"
export RECORD_IMAGE_DIGESTS_ATTEMPTS=2
if echo "${OUTRETRY}" | grep -q 'version tag 0.4.2: ghcr.io/acme/rustchat-backend@'; then
  ok "retry: transient 404s retried until the tag resolves"
else
  bad "retry: converging tag not resolved (got: ${OUTRETRY})"
fi
if [ "$(cat "${FIXTURES}/acme__rustchat-backend__0.4.2.retry")" = "0" ]; then
  ok "retry: exactly the transient attempts were consumed"
else
  bad "retry: unexpected retry counter state"
fi

# Exhausted retries still fail closed with the attempts-annotated message.
ERR404R="$(run_target "${PREFIX_ARGS[@]}" --version 9.9.9 2>&1 >/dev/null || true)"
if echo "${ERR404R}" | grep -q 'not resolved (HTTP 404) after 2 attempt(s)'; then
  ok "retry: exhausted retries report the attempts in the error"
else
  bad "retry: exhausted-retry message wrong (got: ${ERR404R})"
fi

# Non-transient statuses fail immediately, without retries.
printf '401' > "${FIXTURES}/acme__rustchat-backend__0.4.3.status"
ERR401="$(run_target --prefix ghcr.io/acme/rustchat --services backend \
  --version 0.4.3 2>&1 >/dev/null || true)"
if echo "${ERR401}" | grep -q 'not resolved (HTTP 401)$'; then
  ok "retry: auth errors fail immediately without attempts annotation"
else
  bad "retry: 401 handling wrong (got: ${ERR401})"
fi
rm -f "${FIXTURES}/acme__rustchat-backend__0.4.3.status"

# Invalid retry settings are usage errors.
export RECORD_IMAGE_DIGESTS_ATTEMPTS=x
ERRBAD="$(run_target --prefix ghcr.io/acme/rustchat --services backend \
  --version 0.5.1 2>&1 >/dev/null || true)"
export RECORD_IMAGE_DIGESTS_ATTEMPTS=2
if echo "${ERRBAD}" | grep -q 'retry settings must be non-negative integers'; then
  ok "retry: non-numeric attempts rejected"
else
  bad "retry: non-numeric attempts not rejected (got: ${ERRBAD})"
fi

echo
echo "image-digest tests: ${PASS} passed, ${FAIL} failed"
[ "${FAIL}" -eq 0 ]
