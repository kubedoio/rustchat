#!/usr/bin/env bash
set -euo pipefail

# Regression test for the failure-diagnostic path in
# scripts/migration-matrix.sh (the 2026-11-20 "Broken pipe" nightly).
#
# Root cause under test: `diff -u ... | head -40 >&2` under `set -o pipefail`.
# head exits after 40 lines, diff dies with SIGPIPE (or reports "Broken pipe"
# and exits 2), pipefail propagates that status and set -e aborts before
# fail() can report the schema difference.
#
# The fix lives in scripts/lib/schema-diff.sh (sourced by the matrix script):
# dumps go to files with explicit failure handling, and the unified diff is
# written to a file and bounded with sed -n, with no producer to kill.
#
# This test is self-contained: a fake pg_dump serves fixtures from disk, so no
# PostgreSQL server, psql, git history or tags are needed.
#
# Covered:
#   1. differing schemas -> exit 1 (not 2/141), unified diff printed, clear FAIL
#   2. identical schemas with per-dump PG18 \restrict tokens and a differing
#      SET transaction_timeout header -> exit 0, no diagnostic noise
#   3. pg_dump failure -> exit 1 with pg_dump's error, not an opaque pipe exit
#   4. large schema diff -> exit 1, output bounded to 80 lines, no broken pipe
#   5. the harness keeps strict mode active: a mutated `diff | head` surfaces
#      its raw 141/2 status instead of being masked as fail()'s exit 1
#   6. the matrix script and shared lib have no diagnostic `... | head` /
#      `... | grep` paths

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
LIB="${SCRIPT_DIR}/lib/schema-diff.sh"
TARGET="${SCRIPT_DIR}/migration-matrix.sh"

PASS=0
FAIL=0
ok()  { echo "PASS: $1"; PASS=$((PASS + 1)); }
bad() { echo "FAIL: $1" >&2; FAIL=$((FAIL + 1)); }

[[ -f "${LIB}" ]] || { echo "FAIL: missing ${LIB}" >&2; exit 1; }
[[ -f "${TARGET}" ]] || { echo "FAIL: missing ${TARGET}" >&2; exit 1; }

WORK="$(mktemp -d)"
trap 'rm -rf "${WORK}"' EXIT

FAKE_BIN="${WORK}/bin"
FIXTURES="${WORK}/fixtures"
mkdir -p "${FAKE_BIN}" "${FIXTURES}"

# ---------- fake pg_dump ----------
# The last argument is the database name (schema_dump passes flags first).
cat > "${FAKE_BIN}/pg_dump" <<'FAKE'
#!/usr/bin/env bash
set -euo pipefail
expected=(--schema-only --no-owner --no-privileges)
if [[ "$#" -ne 4 \
    || "$1" != "${expected[0]}" \
    || "$2" != "${expected[1]}" \
    || "$3" != "${expected[2]}" ]]; then
  echo "fake pg_dump: unexpected arguments (expected: ${expected[*]} <db>): $*" >&2
  exit 1
fi
db="$4"
if [[ "${db}" == "dump_fails" ]]; then
  echo 'pg_dump: error: connection to database "dump_fails" failed: Connection refused' >&2
  exit 1
fi
fixture="${FAKE_SCHEMA_FIXTURES}/${db}.sql"
if [[ ! -f "${fixture}" ]]; then
  echo "fake pg_dump: no fixture for database ${db}" >&2
  exit 1
fi
cat "${fixture}"
FAKE
chmod +x "${FAKE_BIN}/pg_dump"
export PATH="${FAKE_BIN}:${PATH}"
export FAKE_SCHEMA_FIXTURES="${FIXTURES}"

# ---------- fixtures ----------
# same_*: identical schema. The restrict tokens differ (as PG18 emits a random
# token per dump) and only one side carries the PG17+ transaction_timeout
# header, so a green result proves both normalizations.
cat > "${FIXTURES}/same_a.sql" <<'SQL'
\restrict aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa

-- Dumped from database version 18.0
SET statement_timeout = 0;
SET transaction_timeout = 0;
SET client_encoding = 'UTF8';

CREATE TABLE public.posts (
    id uuid NOT NULL,
    message text
);

\unrestrict aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa
SQL
cat > "${FIXTURES}/same_b.sql" <<'SQL'
\restrict bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb

-- Dumped from database version 18.0
SET statement_timeout = 0;
SET client_encoding = 'UTF8';

CREATE TABLE public.posts (
    id uuid NOT NULL,
    message text
);

\unrestrict bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb
SQL

# diff_*: a real schema difference that must be printed.
cat > "${FIXTURES}/diff_a.sql" <<'SQL'
CREATE TABLE public.posts (
    id uuid NOT NULL,
    message text
);
SQL
cat > "${FIXTURES}/diff_b.sql" <<'SQL'
CREATE TABLE public.posts (
    id uuid NOT NULL,
    message text,
    client_msg_id text NOT NULL
);
SQL

# big_*: thousands of differing lines so a `diff | head` pipeline would get
# SIGPIPE/broken-pipe long before diff finishes writing.
{
  echo "CREATE TABLE public.big ("
  for i in $(seq 1 3000); do echo "    col_${i}_from_empty_path text,"; done
  echo "    marker text"
  echo ");"
} > "${FIXTURES}/big_a.sql"
{
  echo "CREATE TABLE public.big ("
  for i in $(seq 1 3000); do echo "    col_${i}_from_upgrade_path text,"; done
  echo "    marker text"
  echo ");"
} > "${FIXTURES}/big_b.sql"

# ---------- driver ----------
# The flow runs as a separate process, not as a shell function. Invoking a
# function (or subshell) in a `||` list suppresses `set -e` for the whole call,
# which is exactly the defect this harness must not have: a reintroduced
# `diff | head` would exit 1 via fail() instead of exposing its raw 141/2.
DRIVER="${WORK}/run-schema-flow.sh"
cat > "${DRIVER}" <<'DRIVER'
#!/usr/bin/env bash
set -euo pipefail

fail() { echo "migration-matrix: FAIL: $*" >&2; exit 1; }
# shellcheck source=lib/schema-diff.sh
source "${SCHEMA_DIFF_LIB}"
schema_dump "$1" "${FLOW_WORK}/left.sql"
schema_dump "$2" "${FLOW_WORK}/right.sql"
assert_schemas_identical "${FLOW_WORK}/left.sql" "${FLOW_WORK}/right.sql" \
  "${FLOW_WORK}/schema.diff" "$3"
DRIVER
chmod +x "${DRIVER}"
export SCHEMA_DIFF_LIB="${LIB}"
export FLOW_WORK="${WORK}"

CASE_OUT="${WORK}/case.out"
CASE_ERR="${WORK}/case.err"
CASE_RC=0

# Run an arbitrary strict-mode script as a child and capture its raw status.
run_script_case() {
  local script="$1"
  shift
  CASE_RC=0
  bash "${script}" "$@" >"${CASE_OUT}" 2>"${CASE_ERR}" || CASE_RC=$?
}

run_case() {
  run_script_case "${DRIVER}" "$@"
}

expect_rc() {
  local expected="$1"
  local desc="$2"
  if [[ "${CASE_RC}" -eq "${expected}" ]]; then
    ok "${desc} (exit ${expected})"
  else
    bad "${desc} (expected exit ${expected}, got ${CASE_RC})"
  fi
}

expect_err_contains() {
  local needle="$1"
  local desc="$2"
  if grep -Fq -- "${needle}" "${CASE_ERR}"; then
    ok "${desc}"
  else
    bad "${desc} (stderr missing: ${needle})"
  fi
}

expect_no_broken_pipe() {
  if grep -Fq "Broken pipe" "${CASE_ERR}"; then
    bad "$1 (stderr contains a broken-pipe error)"
  else
    ok "$1 (no broken pipe)"
  fi
}

# ---------- 1. schema difference is printed and exits 1 ----------
run_case diff_a diff_b "migrated-readiness"
expect_rc 1 "differing schemas exit 1"
expect_err_contains "migration-matrix: FAIL: migrated-readiness: schemas differ" \
  "differing schemas produce a clear FAIL message"
expect_err_contains "@@" "differing schemas print a unified diff hunk"
expect_err_contains "+    client_msg_id text NOT NULL" \
  "differing schemas print the added column"
expect_no_broken_pipe "differing schemas"

# ---------- 2. noise is normalized, identical schemas pass ----------
run_case same_a same_b "migrated-readiness"
expect_rc 0 "identical schemas with PG18 dump noise exit 0"
if [[ ! -s "${CASE_ERR}" ]]; then
  ok "identical schemas produce no diagnostic output"
else
  bad "identical schemas produced stderr: $(sed -n '1,3p' "${CASE_ERR}")"
fi

# ---------- 3. pg_dump failure is explicit and exits 1 ----------
run_case dump_fails same_a "migrated-readiness"
expect_rc 1 "pg_dump failure exits 1"
expect_err_contains "pg_dump --schema-only failed for database dump_fails" \
  "pg_dump failure names the database"
expect_err_contains "Connection refused" "pg_dump failure includes pg_dump's error"
expect_no_broken_pipe "pg_dump failure"

# ---------- 4. large diff is bounded without a broken pipe ----------
run_case big_a big_b "migrated-readiness"
expect_rc 1 "large schema difference exits 1"
BODY_LINES="$(grep -cE '^[@+-]' "${CASE_ERR}" || true)"
if [[ "${BODY_LINES}" -ge 70 && "${BODY_LINES}" -le 80 ]]; then
  ok "large schema difference is bounded to 80 diff lines (${BODY_LINES} printed)"
else
  bad "large schema difference printed ${BODY_LINES} diff lines (expected 70..80)"
fi
expect_err_contains "unified diff truncated to the first 80 lines" \
  "large schema difference reports truncation"
expect_no_broken_pipe "large schema difference"

# ---------- 5. the harness keeps strict mode active (mutation control) ----------
# Control: the old function-in-`||` shape swallows errexit, so the SIGPIPE from
# `diff | head` is ignored and fail() runs, turning the raw 141/2 into exit 1.
# This is the defect the harness itself previously had.
run_old_style_mutation() {
  (
    set -euo pipefail
    fail() { echo "migration-matrix: FAIL: schemas differ" >&2; exit 1; }
    diff -u "$1" "$2" | head -40 >&2
    fail "schemas differ"
  )
}

OLD_STYLE_RC=0
run_old_style_mutation "${FIXTURES}/big_a.sql" "${FIXTURES}/big_b.sql" \
  >"${CASE_OUT}" 2>"${CASE_ERR}" || OLD_STYLE_RC=$?
if [[ "${OLD_STYLE_RC}" -eq 1 ]] \
    && grep -Fq "migration-matrix: FAIL: schemas differ" "${CASE_ERR}"; then
  ok "control: function-in-|| harness masks SIGPIPE as fail()'s exit 1"
else
  bad "control: expected masked exit 1 from function-in-|| harness, got ${OLD_STYLE_RC}"
fi

# The real harness runs each flow as a child process, so strict mode is active
# inside it. The same mutated pipeline must surface the raw 141/2 failure and
# must never reach fail(); otherwise a reintroduced `diff | head` would pass.
MUTATION_DRIVER="${WORK}/mutated-diff-pipe.sh"
cat > "${MUTATION_DRIVER}" <<'MUT'
#!/usr/bin/env bash
set -euo pipefail

fail() { echo "migration-matrix: FAIL: schemas differ" >&2; exit 1; }
diff -u "$1" "$2" | head -40 >&2
fail "schemas differ"
MUT
chmod +x "${MUTATION_DRIVER}"

run_script_case "${MUTATION_DRIVER}" "${FIXTURES}/big_a.sql" "${FIXTURES}/big_b.sql"
if [[ "${CASE_RC}" -eq 141 || "${CASE_RC}" -eq 2 ]]; then
  ok "mutated diff|head surfaces raw pipe failure (exit ${CASE_RC}), not fail()'s 1"
else
  bad "mutated diff|head returned ${CASE_RC}; expected 141 (SIGPIPE) or 2 (pipe error)"
fi
if grep -Fq "migration-matrix: FAIL: schemas differ" "${CASE_ERR}"; then
  bad "mutated diff|head reached fail(); strict mode was masked in the child"
else
  ok "mutated diff|head never reaches fail()"
fi

# ---------- 6. static guards against reintroducing pipe diagnostics ----------
# Strip comments so prose about the old pattern does not count as code.
# Both files on the diagnostic path are scanned: a `| head` added to the shared
# library is just as dangerous as one added to the matrix script.
scan_no_pipe_diagnostics() {
  local file="$1"
  local code
  code="$(sed 's/#.*$//' "${file}")"
  if printf '%s\n' "${code}" \
      | grep -Eq '\|[[:space:]]*(head|grep)([[:space:]]|$)'; then
    bad "$(basename "${file}") still pipes into head/grep"
  else
    ok "$(basename "${file}") has no producer | head/grep pipelines"
  fi
}

scan_no_pipe_diagnostics "${TARGET}"
scan_no_pipe_diagnostics "${LIB}"

if grep -Fq 'source "${SCRIPT_DIR}/lib/schema-diff.sh"' "${TARGET}"; then
  ok "migration-matrix.sh uses the tested schema-diff helper"
else
  bad "migration-matrix.sh does not source lib/schema-diff.sh"
fi

echo "test-migration-matrix-diagnostics: ${PASS} passed, ${FAIL} failed"
[[ "${FAIL}" -eq 0 ]] || exit 1
