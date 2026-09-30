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

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TARGET="${SCRIPT_DIR}/record-image-digests.sh"

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
hdr="" body="" wfmt="" url="" auth=""
while [ $# -gt 0 ]; do
  case "$1" in
    -D) hdr="$2"; shift 2 ;;
    -o) body="$2"; shift 2 ;;
    -w) wfmt="$2"; shift 2 ;;
    -u) auth="$2"; shift 2 ;;
    -H|--header) shift 2 ;;
    -s|-S|-f|-sS|-sSf) shift ;;
    -*) echo "fake curl: unhandled flag $1" >&2; exit 64 ;;
    *) url="$1"; shift ;;
  esac
done
if [ -n "${auth}" ]; then printf -- '-u %s\n' "${auth}" >> "${FAKE_CURL_LOG}"; fi
case "${url}" in
  */token?scope=*)
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

OUT="$(run_target "${PREFIX_ARGS[@]}" --version 0.5.1 --enforce-aliases 2>/dev/null)"
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

# ---------- 2. alias drift ----------
cp "${FIXTURES}/acme__rustchat-backend__latest.digest" "${WORK}/latest.orig"
cp "${FIXTURES}/drifted.digest" "${FIXTURES}/acme__rustchat-backend__latest.digest"
run_expect 1 "stable: alias drift fails under --enforce-aliases" \
  "${PREFIX_ARGS[@]}" --version 0.5.1 --enforce-aliases
mv "${WORK}/latest.orig" "${FIXTURES}/acme__rustchat-backend__latest.digest"

# ---------- 3. missing version tag ----------
run_expect 1 "missing: version tag 404 fails" \
  "${PREFIX_ARGS[@]}" --version 9.9.9

# ---------- 4. prerelease without enforcement ----------
run_expect 0 "prerelease: aliases not enforced" \
  "${PREFIX_ARGS[@]}" --version 0.5.1-rc.1

OUT_RC="$(run_target "${PREFIX_ARGS[@]}" --version 0.5.1-rc.1 2>/dev/null)"
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
  --version 0.5.1 >/dev/null 2>&1
if grep -q -- '-u ci:sekrit' "${CURL_LOG}"; then
  ok "auth: credentials forwarded to token endpoint when set"
else
  bad "auth: credentials not forwarded"
fi

: > "${CURL_LOG}"
run_target --prefix ghcr.io/acme/rustchat --services backend \
  --version 0.5.1 >/dev/null 2>&1
if ! grep -q -- '-u ' "${CURL_LOG}"; then
  ok "auth: anonymous token request when no credentials set"
else
  bad "auth: credentials sent without env set"
fi

# ---------- 7. malformed / childless manifests and token failures ----------
# A record that silently omits platform digests would defeat the evidence
# contract, so parse failures and childless indexes must be fatal.
printf '{not json' > "${FIXTURES}/acme__rustchat-backend__0.6.0.json"
digest_for "malformed" > "${FIXTURES}/acme__rustchat-backend__0.6.0.digest"
run_expect 1 "malformed: unparseable manifest body fails" \
  --prefix ghcr.io/acme/rustchat --services backend --version 0.6.0

printf '{"schemaVersion":2,"mediaType":"application/vnd.oci.image.index.v1+json"}' \
  > "${FIXTURES}/acme__rustchat-backend__0.7.0.json"
digest_for "childless" > "${FIXTURES}/acme__rustchat-backend__0.7.0.digest"
run_expect 1 "childless: index without platform children fails" \
  --prefix ghcr.io/acme/rustchat --services backend --version 0.7.0

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

echo
echo "image-digest tests: ${PASS} passed, ${FAIL} failed"
[ "${FAIL}" -eq 0 ]
