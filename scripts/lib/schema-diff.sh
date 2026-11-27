#!/usr/bin/env bash
# Schema-dump normalization and diff diagnostics shared by
# scripts/migration-matrix.sh and scripts/test-migration-matrix-diagnostics.sh.
#
# This file is sourced, does not run anything, and expects the caller to
# define fail() (print a message and exit non-zero) before calling into it.

# schema_dump <database> <output-file>
#
# Writes a normalized `pg_dump --schema-only` of <database> to <output-file>,
# or calls fail() with the pg_dump error.
#
# Normalization rules (kept deliberately narrow so real schema differences can
# never be filtered away):
#   * --no-owner / --no-privileges: ownership and ACL text is environment
#     metadata, not schema shape, and is not what the matrix proves.
#   * LC_ALL=C: deterministic ordering and byte-wise comparison.
#   * \restrict / \unrestrict: PostgreSQL 18 wraps dumps in these psql
#     meta-commands using a fresh random token per dump (a dump-injection
#     mitigation). The token differs between any two dumps and carries no
#     schema meaning.
#   * SET transaction_timeout = 0;: a session GUC introduced in PostgreSQL 17
#     that only newer pg_dump versions emit. It is a client header rather than
#     schema, and dropping it keeps the comparison stable across runner image
#     upgrades. Other SET lines are left alone: they are identical for the two
#     dumps produced by one pg_dump binary and can carry schema-relevant
#     search_path/defaults.
schema_dump() {
  local db="$1"
  local out="$2"
  local raw="${out}.raw"
  local err="${out}.err"
  local detail

  # pg_dump writes straight to a file. Piping it into grep/sed would let
  # pipefail (or a downstream SIGPIPE) turn a dump failure into an opaque
  # script exit instead of the explicit fail() below.
  if ! LC_ALL=C pg_dump --schema-only --no-owner --no-privileges "${db}" \
      >"${raw}" 2>"${err}"; then
    detail="$(sed -n '1,20p' "${err}" 2>/dev/null || true)"
    rm -f "${raw}" "${err}"
    fail "pg_dump --schema-only failed for database ${db}${detail:+: ${detail}}"
  fi

  # sed reads the whole file and has no early-exit behaviour; unlike
  # `grep -v`, it also exits 0 when every line is filtered out.
  if ! LC_ALL=C sed -E \
      -e '/^\\(un)?restrict([[:space:]]|$)/d' \
      -e '/^SET transaction_timeout = 0;$/d' \
      "${raw}" >"${out}"; then
    rm -f "${raw}" "${err}"
    fail "failed to normalize pg_dump output for database ${db}"
  fi

  rm -f "${raw}" "${err}"
}

# assert_schemas_identical <left-dump> <right-dump> <diff-file> <label>
#
# Returns 0 when the dumps are identical. Otherwise prints a bounded head of
# the unified diff on stderr and calls fail() with <label>.
#
# The diff is written to a file first. Piping `diff` into `head` under
# `set -o pipefail` makes head close the pipe early; diff then dies with
# SIGPIPE (or reports "Broken pipe" and exits 2), pipefail propagates that as
# the script's exit status, and set -e exits before the real failure is
# reported. Reading a bounded range from the file with sed has no producer to
# kill.
assert_schemas_identical() {
  local left="$1"
  local right="$2"
  local diff_file="$3"
  local label="$4"
  local rc=0
  local detail

  # diff exits 0 (same), 1 (different) or 2 (trouble: unreadable input, I/O
  # error). Only 1 is a schema-difference diagnosis; 2 is a tool failure and
  # must not be reported as "schemas differ".
  LC_ALL=C diff -u "${left}" "${right}" >"${diff_file}" 2>"${diff_file}.err" || rc=$?

  case "${rc}" in
    0)
      rm -f "${diff_file}" "${diff_file}.err"
      return 0
      ;;
    1)
      # Bounded read from a regular file: no pipeline, so pipefail cannot
      # replace the diagnostic with a broken-pipe exit.
      sed -n '1,80p' "${diff_file}" >&2
      if [[ -n "$(sed -n '81p' "${diff_file}")" ]]; then
        echo "... (unified diff truncated to the first 80 lines)" >&2
      fi
      fail "${label}: schemas differ"
      ;;
    *)
      detail="$(sed -n '1,20p' "${diff_file}.err" 2>/dev/null || true)"
      fail "${label}: diff -u failed with exit ${rc}${detail:+: ${detail}}"
      ;;
  esac
}
