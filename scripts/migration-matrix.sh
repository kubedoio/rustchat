#!/usr/bin/env bash
set -euo pipefail

# Migration matrix (RI-C07 / ADR-006 P006).
#
# A release candidate must prove, from a real database, that:
#   1. empty -> HEAD applies cleanly (fresh installs),
#   2. latest published stable -> HEAD applies cleanly (upgrades), and
#   3. both paths converge to the same schema (migrated readiness).
#
# This script creates two scratch databases, applies the matrix, diffs the
# resulting schemas, spot-checks historically divergent columns, and drops the
# scratch databases. It never touches DATABASE_URL or any existing database.
#
# Usage: scripts/migration-matrix.sh
#
# Environment:
#   MIGRATION_MATRIX_TAG  base tag for the upgrade path (default: v0.4.1, the
#                         latest published stable release)
#   PGHOST / PGPORT / PGUSER / PGPASSWORD
#                         Postgres superuser connection (defaults:
#                         localhost:5432, postgres)
#   MIGRATIONS_DIR        migrations directory (default: backend/migrations)
#
# sqlx semantics note: migrations are applied here with psql in filename order
# (exactly what sqlx::migrate! does). The upgrade path additionally verifies
# via git that no migration present in the base tag was modified or deleted
# afterwards — sqlx would reject such a checksum change — and applies only the
# added migrations, which is what `sqlx migrate run` would do on a real install.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
MIGRATIONS_DIR="${MIGRATIONS_DIR:-${REPO_ROOT}/backend/migrations}"
# A relative MIGRATIONS_DIR is resolved against the repo root: realpath
# would otherwise resolve it against the caller's CWD, failing loudly from
# any other directory (or, if a same-named directory existed there, silently
# guarding the wrong one).
if [[ "${MIGRATIONS_DIR}" != /* ]]; then
  MIGRATIONS_DIR="${REPO_ROOT}/${MIGRATIONS_DIR}"
fi
# Repo-relative pathspec for git commands: git resolves pathspecs relative to
# the repo root, so an absolute MIGRATIONS_DIR must not be passed verbatim
# (it would match nothing and silently disable the append-only guard).
MIGRATIONS_RELPATH="$(realpath --relative-to="${REPO_ROOT}" "${MIGRATIONS_DIR}")"
BASE_TAG="${MIGRATION_MATRIX_TAG:-v0.4.1}"
# Commit of ${BASE_TAG} (v0.4.1). Used when the tag ref is absent — e.g.
# fork PR checkouts that do not inherit upstream tags created after the
# fork. The commit is an ancestor of main, so shallow-free checkouts of
# the PR merge ref still contain it.
BASE_TAG_FALLBACK_SHA="8fa18602f4e4f1b3205489326c477b2e1e11d2a9"

PGHOST="${PGHOST:-localhost}"
PGPORT="${PGPORT:-5432}"
PGUSER="${PGUSER:-postgres}"
export PGHOST PGPORT PGUSER
if [[ -n "${PGPASSWORD:-}" ]]; then export PGPASSWORD; fi

PSQL=(psql -v ON_ERROR_STOP=1 -q)
STAMP="$$"
EMPTY_DB="rc_matrix_empty_${STAMP}"
UPGRADE_DB="rc_matrix_upgrade_${STAMP}"
WORK_DIR="$(mktemp -d)"
trap 'rm -rf "${WORK_DIR}"; ${PSQL[@]} postgres -c "DROP DATABASE IF EXISTS \"${EMPTY_DB}\";" >/dev/null 2>&1 || true; ${PSQL[@]} postgres -c "DROP DATABASE IF EXISTS \"${UPGRADE_DB}\";" >/dev/null 2>&1 || true' EXIT

fail() { echo "migration-matrix: FAIL: $*" >&2; exit 1; }
note() { echo "migration-matrix: $*"; }

[[ -d "${MIGRATIONS_DIR}" ]] || fail "migrations dir not found: ${MIGRATIONS_DIR}"
if git -C "${REPO_ROOT}" rev-parse --verify --quiet "${BASE_TAG}^{commit}" >/dev/null; then
  BASE_REF="${BASE_TAG}"
else
  # The tag ref may be absent on fork PR checkouts (tags are not part of
  # normal fetch refspecs). Fall back to the pinned commit of the same
  # release; identical tree, so the matrix result is unchanged.
  if git -C "${REPO_ROOT}" cat-file -e "${BASE_TAG_FALLBACK_SHA}^{commit}" 2>/dev/null; then
    BASE_REF="${BASE_TAG_FALLBACK_SHA}"
    note "base tag ${BASE_TAG} not found; using pinned commit ${BASE_REF} (same tree)"
  else
    fail "base tag not found: ${BASE_TAG} (set MIGRATION_MATRIX_TAG)"
  fi
fi

note "base ref for upgrade path: ${BASE_REF}"

# ---------------------------------------------------------------------------
# Guard: migrations must be append-only since the base tag (RI-C07 prohibits
# rewriting previously applied migrations; sqlx checksums would reject it).
# ---------------------------------------------------------------------------
CHANGE_STAT="$(git -C "${REPO_ROOT}" diff --name-status "${BASE_REF}" HEAD -- "${MIGRATIONS_RELPATH}" | awk '{print $1}' | sort -u | tr -d '\n')"
if [[ -n "${CHANGE_STAT}" && "${CHANGE_STAT}" != "A" ]]; then
  fail "migrations were modified or deleted since ${BASE_REF} (only additions are allowed): $(git -C "${REPO_ROOT}" diff --name-status "${BASE_REF}" HEAD -- "${MIGRATIONS_RELPATH}" | grep -v '^A' | head -5)"
fi
note "PASS append-only history: no migration modified/deleted since ${BASE_REF}"

# ---------------------------------------------------------------------------
# Path 1: empty -> HEAD (fresh install).
# ---------------------------------------------------------------------------
${PSQL[@]} postgres -c "CREATE DATABASE \"${EMPTY_DB}\";" >/dev/null
for f in $(ls "${MIGRATIONS_DIR}"/*.sql | sort); do
  ${PSQL[@]} "${EMPTY_DB}" -f "$f" >/dev/null \
    || fail "empty->HEAD: migration failed: $(basename "$f")"
done
note "PASS empty-to-head: all $(ls "${MIGRATIONS_DIR}"/*.sql | wc -l) migrations applied cleanly"

# ---------------------------------------------------------------------------
# Path 2: latest published stable -> HEAD (upgrade).
# ---------------------------------------------------------------------------
TAG_DIR="${WORK_DIR}/tag_migrations"
mkdir -p "${TAG_DIR}"
git -C "${REPO_ROOT}" archive "${BASE_REF}" "${MIGRATIONS_RELPATH}" \
  | tar -x --strip-components=2 -C "${TAG_DIR}"

${PSQL[@]} postgres -c "CREATE DATABASE \"${UPGRADE_DB}\";" >/dev/null
for f in $(ls "${TAG_DIR}"/*.sql | sort); do
  ${PSQL[@]} "${UPGRADE_DB}" -f "$f" >/dev/null \
    || fail "stable->HEAD: base-tag migration failed: $(basename "$f")"
done
BASE_COUNT=$(ls "${TAG_DIR}"/*.sql | wc -l)
note "stable schema applied: ${BASE_COUNT} migrations from ${BASE_TAG}"

# Apply only migrations added since the base tag (what sqlx migrate run does).
NEW_COUNT=0
for f in $(ls "${MIGRATIONS_DIR}"/*.sql | sort); do
  base="$(basename "$f")"
  if [[ ! -f "${TAG_DIR}/${base}" ]]; then
    ${PSQL[@]} "${UPGRADE_DB}" -f "$f" >/dev/null \
      || fail "stable->HEAD: upgrade migration failed: ${base}"
    NEW_COUNT=$((NEW_COUNT + 1))
  fi
done
note "PASS latest-stable-schema-to-head: ${NEW_COUNT} new migrations applied cleanly"

# ---------------------------------------------------------------------------
# Path 3: migrated readiness — both paths converge and expose the columns the
# repositories actually query (spot checks on historically divergent tables).
# ---------------------------------------------------------------------------
pg_dump --schema-only "${EMPTY_DB}" | grep -vE '^\\(un)?restrict ' > "${WORK_DIR}/schema_empty.sql"
pg_dump --schema-only "${UPGRADE_DB}" | grep -vE '^\\(un)?restrict ' > "${WORK_DIR}/schema_upgrade.sql"
if ! diff -q "${WORK_DIR}/schema_empty.sql" "${WORK_DIR}/schema_upgrade.sql" >/dev/null; then
  diff "${WORK_DIR}/schema_empty.sql" "${WORK_DIR}/schema_upgrade.sql" | head -40 >&2
  fail "migrated-readiness: schemas from empty->HEAD and ${BASE_TAG}->HEAD differ"
fi
note "PASS converged schema: empty->HEAD and ${BASE_TAG}->HEAD are identical"

# Readiness spot checks: table:column pairs the repositories depend on. These
# tables were created by divergent historical migrations; the compat
# migrations must converge them on both paths.
READINESS_CHECKS=(
  "channel_bookmarks:owner_id"
  "channel_bookmarks:display_name"
  "channel_bookmarks:link_url"
  "channel_bookmarks:file_id"
  "channel_bookmarks:image_url"
  "channel_bookmarks:type"
  "reactions:create_at"
  "scheduled_posts:processed_at"
  "posts:client_msg_id"
)
for db in "${EMPTY_DB}" "${UPGRADE_DB}"; do
  for check in "${READINESS_CHECKS[@]}"; do
    table="${check%%:*}"
    column="${check##*:}"
    found="$(${PSQL[@]} "$db" -tAc \
      "SELECT 1 FROM information_schema.columns WHERE table_schema='public' AND table_name='${table}' AND column_name='${column}';")"
    [[ "${found}" == "1" ]] \
      || fail "migrated-readiness: ${db} is missing ${table}.${column} (repository queries would break)"
  done
done
note "PASS migrated-readiness: repository-critical columns present on both paths"

note "ALL CHECKS PASSED (empty-to-head, ${BASE_REF}-to-head, converged readiness)"
