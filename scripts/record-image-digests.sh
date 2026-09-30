#!/usr/bin/env bash
# Record immutable digests for published RustChat release images.
#
# For each service image (backend, frontend, push-proxy) this resolves the
# registry manifest digest of the version tag and the per-platform child
# digests of the multi-arch index. With --enforce-aliases it also verifies
# that the rolling aliases (X.Y and latest) point at the same manifest as
# the version tag, failing on drift — the release-time counterpart of the
# moving-alias consistency the promotion pipeline enforces.
#
# The registry is accessed over its HTTP API directly (no docker daemon
# required). Public GHCR packages are readable anonymously; set
# REGISTRY_USER and REGISTRY_TOKEN to authenticate for private packages.
#
# Usage:
#   scripts/record-image-digests.sh --prefix ghcr.io/kubedoio/rustchat \
#       --version 0.5.1 \
#       [--services backend,frontend,push-proxy] \
#       [--output FILE] [--enforce-aliases]
#
# --enforce-aliases is meant for stable releases, where the build publishes
# X.Y.Z, X.Y, and latest from one manifest. Prereleases must not pass it:
# their aliases may legitimately point at the previous stable release.
set -euo pipefail

PREFIX=""
VERSION=""
SERVICES="backend,frontend,push-proxy"
OUTPUT=""
ENFORCE_ALIASES=0

usage() {
  cat >&2 <<'EOF'
Usage: record-image-digests.sh --prefix ghcr.io/owner/repo --version X.Y.Z
          [--services backend,frontend,push-proxy]
          [--output FILE] [--enforce-aliases]

Resolves the registry digest of the version tag per service image, records
per-platform child digests, and (with --enforce-aliases) fails unless the
X.Y and latest aliases point at the same manifest as the version tag.
EOF
  exit 2
}

# Normalize --opt=value into --opt value for the value-taking options.
normalized=()
for arg in "$@"; do
  case "$arg" in
    --prefix=*|--version=*|--services=*|--output=*)
      normalized+=("${arg%%=*}" "${arg#*=}") ;;
    *) normalized+=("$arg") ;;
  esac
done
set -- "${normalized[@]}"

while [ $# -gt 0 ]; do
  case "$1" in
    --prefix|--version|--services|--output)
      if [ $# -lt 2 ]; then
        echo "ERROR: $1 requires a value" >&2 ; usage
      fi
      case "$1" in
        --prefix) PREFIX="$2" ;;
        --version) VERSION="$2" ;;
        --services) SERVICES="$2" ;;
        --output) OUTPUT="$2" ;;
      esac
      shift 2 ;;
    --enforce-aliases) ENFORCE_ALIASES=1 ; shift ;;
    -h|--help) usage ;;
    *) echo "ERROR: unknown argument: $1" >&2 ; usage ;;
  esac
done

[ -n "$PREFIX" ] || { echo "ERROR: --prefix is required" >&2 ; usage ; }
[ -n "$VERSION" ] || { echo "ERROR: --version is required" >&2 ; usage ; }
case "$PREFIX" in
  */*) : ;;
  *) echo "ERROR: --prefix must be a full image prefix like ghcr.io/owner/repo" >&2 ; exit 2 ;;
esac
# Refs and service names flow into registry URLs; restrict them to a safe
# charset so nothing needs URL encoding.
case "$VERSION" in
  *[!A-Za-z0-9._-]*)
    echo "ERROR: --version must contain only [A-Za-z0-9._-]: ${VERSION}" >&2 ; exit 2 ;;
esac
SERVICES="${SERVICES// /}"

REGISTRY="https://${PREFIX%%/*}"
MINOR_VERSION="${VERSION%.*}"
command -v jq >/dev/null 2>&1 || { echo "ERROR: jq is required" >&2 ; exit 1 ; }

WORK="$(mktemp -d)"
trap 'rm -rf "${WORK}"' EXIT

# Exchange (optionally authenticated) for a pull-scoped registry token.
# Anonymous access works for public packages.
registry_token() {
  local repo="$1" auth_args=()
  if [ -n "${REGISTRY_USER:-}" ] && [ -n "${REGISTRY_TOKEN:-}" ]; then
    auth_args=(-u "${REGISTRY_USER}:${REGISTRY_TOKEN}")
  fi
  curl -sSf "${auth_args[@]}" \
    "${REGISTRY}/token?scope=repository:${repo}:pull" \
    | jq -r '(.token // .access_token) // empty'
}

# Fetch a manifest by reference into the globals MANIFEST_BODY and
# MANIFEST_DIGEST. Accepts both the OCI index and Docker manifest-list
# media types. Fails (exits) with a clear message if the reference does
# not resolve.
fetch_manifest() {
  local repo="$1" ref="$2" hdr body status
  hdr="${WORK}/manifest.hdr" body="${WORK}/manifest.json"
  if ! status=$(curl -sS -D "$hdr" -o "$body" -w '%{http_code}' \
      -H "Authorization: Bearer ${TOKEN}" \
      -H "Accept: application/vnd.oci.image.index.v1+json" \
      -H "Accept: application/vnd.docker.distribution.manifest.list.v2+json" \
      "${REGISTRY}/v2/${repo}/manifests/${ref}" 2>/dev/null); then
    echo "ERROR: registry request failed for ${repo}:${ref}" >&2
    exit 1
  fi
  if [ "$status" != "200" ]; then
    echo "ERROR: manifest ${repo}:${ref} not resolved (HTTP ${status})" >&2
    exit 1
  fi
  MANIFEST_BODY="$(cat "$body")"
  MANIFEST_DIGEST="$(tr -d '\r' < "$hdr" | awk 'tolower($1)=="docker-content-digest:" {print $2}')"
  if [ -z "$MANIFEST_DIGEST" ]; then
    echo "ERROR: registry returned no digest header for ${repo}:${ref}" >&2
    exit 1
  fi
}

# Child digests of the multi-arch index, skipping attestation manifests
# (platform "unknown/unknown"). A jq failure here must be fatal: a
# complete-looking record with silently missing platform digests defeats
# the evidence contract, so callers assert the output is non-empty.
platform_lines() {
  jq -e -r '[.manifests[]?
    | select((.platform.os? // "unknown") != "unknown")
    | "  \(.platform.os)/\(.platform.architecture): \(.digest)"] | .[]' <<<"$MANIFEST_BODY"
}

out() {
  if [ -n "$OUTPUT" ]; then
    printf '%s\n' "$*" >>"$OUTPUT"
  else
    printf '%s\n' "$*"
  fi
}

[ -z "$OUTPUT" ] || : >"$OUTPUT"

IFS=',' read -r -a SERVICE_LIST <<<"$SERVICES"
for service in "${SERVICE_LIST[@]}"; do
  image="${PREFIX}-${service}"
  repo="${image#*/}"
  if ! TOKEN="$(registry_token "$repo")"; then
    echo "ERROR: failed to obtain a registry token for ${repo}" >&2
    exit 1
  fi
  if [ -z "$TOKEN" ]; then
    echo "ERROR: no registry token obtained for ${repo}" >&2
    exit 1
  fi

  fetch_manifest "$repo" "$VERSION"
  out "# ${image}"
  out "  version tag ${VERSION}: ${image}@${MANIFEST_DIGEST}"
  # Release images are multi-arch by construction; a missing, malformed, or
  # childless platform list is exactly what this evidence job must catch
  # (jq -e makes an empty result fatal).
  if ! children="$(platform_lines)"; then
    echo "ERROR: manifest for ${repo}:${VERSION} is not a usable multi-arch index (parse failure or no platform children)" >&2
    exit 1
  fi
  while IFS= read -r line; do
    out "$line"
  done <<<"$children"

  if [ "$ENFORCE_ALIASES" -eq 1 ]; then
    version_digest="$MANIFEST_DIGEST"
    for alias in "$MINOR_VERSION" latest; do
      [ "$alias" != "$VERSION" ] || continue
      fetch_manifest "$repo" "$alias"
      out "  alias ${alias}: ${MANIFEST_DIGEST}"
      if [ "$MANIFEST_DIGEST" != "$version_digest" ]; then
        echo "ERROR: alias drift on ${image}: ${alias}=${MANIFEST_DIGEST} != ${VERSION}=${version_digest}" >&2
        exit 1
      fi
    done
  fi
  out ""
done
