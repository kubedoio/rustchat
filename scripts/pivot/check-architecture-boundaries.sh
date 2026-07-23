#!/usr/bin/env bash
set -euo pipefail

fail() {
  echo "::error::$*" >&2
  exit 1
}

required_docs=(
  "docs/adr/ADR-004-buzz-upstream-foundation.md"
  "docs/pivot/README.md"
  "docs/pivot/SPEC-001-repository-and-upstream-boundaries.md"
  "docs/pivot/SPEC-002-enterprise-control-plane.md"
  "docs/pivot/CONTRACTS.md"
  "docs/pivot/TEST-STRATEGY.md"
  "docs/pivot/PROMPT-LEDGER.md"
)

for file in "${required_docs[@]}"; do
  [[ -f "${file}" ]] || fail "Required pivot governance file is missing: ${file}"
done

# Buzz must remain a separately tracked upstream/fork. A copied source tree in
# this repository makes drift too easy and is therefore forbidden.
for path in buzz vendor/buzz upstream/buzz; do
  if git ls-files "${path}" "${path}/**" | grep -q .; then
    fail "Copied Buzz source is forbidden at ${path}; pin the Kubedo fork instead"
  fi
done

# A future core/buzz reference is allowed only as a git submodule/gitlink.
if git ls-files --stage core/buzz | grep -q .; then
  mode="$(git ls-files --stage core/buzz | awk 'NR == 1 {print $1}')"
  [[ "${mode}" == "160000" ]] || fail "core/buzz must be a gitlink/submodule, not copied files"
  [[ -f .gitmodules ]] || fail "core/buzz gitlink requires .gitmodules"
  grep -Eq 'github\.com[:/]kubedoio/buzz(\.git)?' .gitmodules \
    || fail "core/buzz must point to the Kubedo Buzz fork"
fi

# Enterprise services are contract consumers. Direct dependencies on private
# relay/database crates would couple RustChat to Buzz internals.
if [[ -d services ]]; then
  if grep -RInE --include='Cargo.toml' --include='*.rs' \
      '(buzz[-_]db|buzz[-_]relay)' services; then
    fail "Enterprise services may not depend directly on buzz-db or buzz-relay"
  fi
fi

# Releases must be reproducible. Mutable references are forbidden in the
# machine-readable version matrix.
if [[ -f distribution/versions.yaml ]]; then
  if grep -nE '(^|[[:space:]:])((main|latest))([[:space:]#]|$)' distribution/versions.yaml; then
    fail "distribution/versions.yaml contains a mutable main/latest reference"
  fi
  grep -Eq 'commit:[[:space:]]*[0-9a-f]{40}' distribution/versions.yaml \
    || fail "distribution/versions.yaml must pin a 40-character Buzz commit SHA"
fi

echo "RustChat Buzz pivot architecture boundaries: OK"
