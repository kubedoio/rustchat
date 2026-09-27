#!/usr/bin/env bash
set -euo pipefail

# Status-check aggregate.
#
# Branch protection requires stable single contexts per workflow regardless of
# which conditional jobs are present. This script centralises the "did every
# required dependent job pass?" decision so it is deterministic and
# unit-testable (see scripts/test-gate-propagation.sh).
#
# Usage: ci-aggregate.sh <ENV_VAR_NAME> [<ENV_VAR_NAME> ...]
#
# Each argument names an environment variable that holds a GitHub Actions job
# result. Agreement: a result is acceptable when it is `success` or `skipped`.
# Any other terminal result (`failure`, `cancelled`, `timed_out`,
# `startup_failure`) is a failed dependent and must make the aggregate fail —
# a required failure must never be masked by another green job (RI-C01, RI-C04
# gate semantics: required failures propagate to the relevant state).

if [[ "$#" -eq 0 ]]; then
  echo "usage: $0 <ENV_VAR> [<ENV_VAR> ...]" >&2
  exit 2
fi

FAILURES=0

for name in "$@"; do
  result="${!name:-}"
  echo "${name}: ${result:-unset}"
  case "${result}" in
    success|skipped) ;; # ok
    failure|cancelled|timed_out|startup_failure)
      echo "${name} failed (${result})" >&2
      FAILURES=$((FAILURES + 1))
      ;;
    "")
      # Missing/unset variable indicates a dependency wiring error.
      echo "${name} result missing/unset" >&2
      FAILURES=$((FAILURES + 1))
      ;;
    *)
      echo "${name} has unexpected result: ${result}" >&2
      FAILURES=$((FAILURES + 1))
      ;;
  esac
done

if [[ "${FAILURES}" -eq 0 ]]; then
  echo "All aggregate members passed or were skipped."
  exit 0
fi

echo "${FAILURES} aggregate member(s) failed." >&2
exit 1
