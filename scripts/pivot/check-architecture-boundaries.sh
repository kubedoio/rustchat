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
  "docs/pivot/LLM-ALIGNMENT.md"
)

for file in "${required_docs[@]}"; do
  [[ -f "${file}" ]] || fail "Required pivot governance file is missing: ${file}"
  # Existence is not enough: a hollowed-out document must not pass. Each
  # governance file must keep meaningful content and its level-1 title.
  lines="$(wc -l < "${file}")"
  [[ "${lines}" -ge 20 ]] || fail "Governance file looks hollowed out (${lines} lines): ${file}"
  grep -qE '^# ' "${file}" || fail "Governance file lost its title header: ${file}"
done

# Buzz must remain a separately tracked upstream/fork. A copied source tree in
# this repository makes drift too easy and is therefore forbidden. Check the
# known locations AND detect copied Buzz crates anywhere in the tree by their
# Cargo package names, so renaming the directory does not bypass the guard.
for path in buzz vendor/buzz upstream/buzz; do
  if git ls-files "${path}" "${path}/**" | grep -q .; then
    fail "Copied Buzz source is forbidden at ${path}; pin the Kubedo fork instead"
  fi
done

while IFS= read -r manifest; do
  case "${manifest}" in
    core/buzz/*) continue ;; # submodule content, checked separately below
  esac
  if grep -qE '^name = "buzz-' "${manifest}"; then
    fail "Copied Buzz crate detected at ${manifest}; pin the Kubedo fork instead of tracking Buzz source"
  fi
done < <(git ls-files '*Cargo.toml' 'Cargo.toml')

# A future core/buzz reference is allowed only as a git submodule/gitlink.
if git ls-files --stage core/buzz | grep -q .; then
  mode="$(git ls-files --stage core/buzz | awk 'NR == 1 {print $1}')"
  [[ "${mode}" == "160000" ]] || fail "core/buzz must be a gitlink/submodule, not copied files"
  [[ -f .gitmodules ]] || fail "core/buzz gitlink requires .gitmodules"
  # The core/buzz submodule entry itself (not just any submodule) must point
  # at the Kubedo fork.
  url="$(git config -f .gitmodules --get submodule.core/buzz.url || true)"
  [[ "${url}" =~ github\.com[:/]kubedoio/buzz(\.git)?$ ]] \
    || fail "core/buzz submodule must point to the Kubedo Buzz fork, got: ${url:-<missing>}"
fi

# Enterprise services are contract consumers. Direct dependencies on private
# relay/database crates would couple RustChat to Buzz internals. Scan every
# tracked Cargo.toml (including the workspace root, which member crates can
# inherit via `workspace = true`) and all Rust sources outside core/buzz.
while IFS= read -r src; do
  case "${src}" in
    core/buzz/*) continue ;;
  esac
  if grep -qE '(buzz[-_]db|buzz[-_]relay)' "${src}"; then
    fail "Direct buzz-db/buzz-relay reference forbidden outside core/buzz: ${src}"
  fi
done < <(git ls-files '*Cargo.toml' 'Cargo.toml' '*.rs')

# Releases must be reproducible. Mutable references are forbidden in the
# machine-readable version matrix.
if [[ -f distribution/versions.yaml ]]; then
  if grep -nE '(^|[[:space:]:])((main|master|latest|edge|dev))([[:space:]#]|$)' distribution/versions.yaml; then
    fail "distribution/versions.yaml contains a mutable branch/tag reference"
  fi
  grep -Eq 'commit:[[:space:]]*[0-9a-f]{40}' distribution/versions.yaml \
    || fail "distribution/versions.yaml must pin a 40-character Buzz commit SHA"
fi

echo "RustChat Buzz pivot architecture boundaries: OK"
