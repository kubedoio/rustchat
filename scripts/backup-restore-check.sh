#!/usr/bin/env bash
set -euo pipefail

# Backup/restore evidence check (P006 recovery evidence / ROADMAP
# "Backup & restore").
#
# Proves, from a real database, that the documented backup path (pg_dump
# custom format) restores into a FRESH database with:
#   1. identical schema objects (tables, columns, indexes, constraints,
#      sequences, views, triggers, enums, extensions, functions),
#   2. identical sqlx migration bookkeeping (_sqlx_migrations),
#   3. deterministic seed data intact (per-table row counts and content
#      checksums),
#   4. referential integrity of the restored rows.
#
# The procedure mirrors what an operator runs in production (pg_dump -Fc,
# pg_restore into a new database; see docs/operations/runbook.md), so a
# green run is standing evidence that the "Backup + restore smoke test"
# release gate holds for the current schema and seed-representative data.
#
# Usage: scripts/backup-restore-check.sh
#
# Environment:
#   PGHOST / PGPORT / PGUSER / PGPASSWORD
#                         Postgres connection (defaults: localhost:5432,
#                         postgres). PGHOST may be a unix socket directory
#                         for peer auth.
#   MIGRATIONS_DIR        migrations directory (default: backend/migrations)
#
# Safety: never touches DATABASE_URL or any existing database. Two scratch
# databases are created and dropped; the dump file lives in a mktemp dir.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
MIGRATIONS_DIR="${MIGRATIONS_DIR:-${REPO_ROOT}/backend/migrations}"
# A relative MIGRATIONS_DIR is resolved against the repo root (same rationale
# as scripts/migration-matrix.sh).
if [[ "${MIGRATIONS_DIR}" != /* ]]; then
  MIGRATIONS_DIR="${REPO_ROOT}/${MIGRATIONS_DIR}"
fi

PGHOST="${PGHOST:-localhost}"
PGPORT="${PGPORT:-5432}"
PGUSER="${PGUSER:-postgres}"
export PGHOST PGPORT PGUSER
if [[ -n "${PGPASSWORD:-}" ]]; then export PGPASSWORD; fi

PSQL=(psql -v ON_ERROR_STOP=1 -q)
STAMP="$$"
SRC_DB="rc_backup_src_${STAMP}"
DST_DB="rc_backup_restore_${STAMP}"
WORK_DIR="$(mktemp -d)"
DUMP_FILE="${WORK_DIR}/rustchat_backup.dump"
trap 'rm -rf "${WORK_DIR}"; ${PSQL[@]} postgres -c "DROP DATABASE IF EXISTS \"${SRC_DB}\";" >/dev/null 2>&1 || true; ${PSQL[@]} postgres -c "DROP DATABASE IF EXISTS \"${DST_DB}\";" >/dev/null 2>&1 || true' EXIT

fail() { echo "backup-restore-check: FAIL: $*" >&2; exit 1; }
note() { echo "backup-restore-check: $*"; }

[[ -d "${MIGRATIONS_DIR}" ]] || fail "migrations dir not found: ${MIGRATIONS_DIR}"

# Guard: migration filenames must match the sqlx <version>_<description>.sql
# convention (digits + [A-Za-z0-9_]). Names outside it would be word-split by
# the apply loops or inject into the bookkeeping INSERT below, so they fail
# loudly here; this also fails loudly when the directory contains no
# migrations at all (a non-matching glob is passed through literally).
for f in "${MIGRATIONS_DIR}"/*.sql; do
  [[ "$(basename "$f")" =~ ^[0-9]+_[A-Za-z0-9_]+\.sql$ ]] \
    || fail "migration filename does not match the sqlx <version>_<description>.sql convention: $(basename "$f")"
done

# Tables seeded below; row counts and content checksums are compared
# verbatim between the source and the restored database.
SEED_TABLES=(
  organizations
  users
  teams
  team_members
  channels
  channel_members
  posts
  reactions
)

# Collect a normalized inventory of everything that must survive the round
# trip, compared as TEXT so source and restored databases can be diffed.
#
# Structural facts are taken from the catalog SEMANTICALLY (names, types,
# key-column sets, FK actions, function signatures) rather than deparsed
# expression text: pg_get_constraintdef / pg_dump output can legitimately
# differ in text between a database and its own restore (semantically
# equivalent expression trees re-deparse differently). CHECK-constraint and
# column-default EXPRESSION TEXT is therefore not compared — that class of
# difference is inherited from the dump itself, not introduced by restore.
# What operators need proven is that the same objects with the same shape
# exist after restore — which these facts capture.
collect_facts() {
  local db="$1"
  local t v
  {
    echo "== tables"
    ${PSQL[@]} "$db" -tAc "SELECT tablename FROM pg_tables WHERE schemaname='public' ORDER BY 1"
    echo "== columns"
    ${PSQL[@]} "$db" -tAc "SELECT table_name||':'||column_name||':'||udt_name||':'||is_nullable FROM information_schema.columns WHERE table_schema='public' ORDER BY 1"
    echo "== indexes"
    ${PSQL[@]} "$db" -tAc "
      SELECT t.relname||':'||ix.relname||':'||i.indisunique||':'||i.indisprimary||':'||
             (SELECT string_agg(a.attname,',' ORDER BY x.ord) FROM unnest(i.indkey) WITH ORDINALITY x(attnum,ord)
              JOIN pg_attribute a ON a.attrelid=i.indrelid AND a.attnum=x.attnum)
      FROM pg_index i
      JOIN pg_class ix ON ix.oid=i.indexrelid
      JOIN pg_class t ON t.oid=i.indrelid
      JOIN pg_namespace n ON n.oid=t.relnamespace
      WHERE n.nspname='public' ORDER BY 1"
    echo "== constraints"
    ${PSQL[@]} "$db" -tAc "
      SELECT c.conname||':'||c.contype::text||':'||c.conrelid::regclass::text||':'||
             COALESCE((SELECT string_agg(a.attname,',' ORDER BY x.ord) FROM unnest(c.conkey) WITH ORDINALITY x(attnum,ord)
                       JOIN pg_attribute a ON a.attrelid=c.conrelid AND a.attnum=x.attnum),'')||':'||
             COALESCE(c.confrelid::regclass::text,'')||':'||
             COALESCE((SELECT string_agg(a.attname,',' ORDER BY x.ord) FROM unnest(c.confkey) WITH ORDINALITY x(attnum,ord)
                       JOIN pg_attribute a ON a.attrelid=c.confrelid AND a.attnum=x.attnum),'')||':'||
             c.confdeltype::text||':'||c.confupdtype::text||':'||c.confmatchtype::text
      FROM pg_constraint c
      WHERE c.connamespace='public'::regnamespace ORDER BY 1"
    echo "== sequences"
    ${PSQL[@]} "$db" -tAc "SELECT sequencename FROM pg_sequences WHERE schemaname='public' ORDER BY 1"
    echo "== views"
    ${PSQL[@]} "$db" -tAc "SELECT viewname FROM pg_views WHERE schemaname='public' ORDER BY 1"
    echo "== triggers"
    ${PSQL[@]} "$db" -tAc "SELECT tgname||':'||c.relname FROM pg_trigger t JOIN pg_class c ON c.oid=t.tgrelid JOIN pg_namespace n ON n.oid=c.relnamespace WHERE n.nspname='public' AND NOT t.tgisinternal ORDER BY 1"
    echo "== enums"
    ${PSQL[@]} "$db" -tAc "SELECT t.typname||':'||string_agg(e.enumlabel,',' ORDER BY e.enumsortorder) FROM pg_type t JOIN pg_enum e ON e.enumtypid=t.oid JOIN pg_namespace n ON n.oid=t.typnamespace WHERE n.nspname='public' GROUP BY t.typname ORDER BY 1"
    echo "== extensions"
    ${PSQL[@]} "$db" -tAc "SELECT extname||':'||extversion FROM pg_extension ORDER BY 1"
    echo "== functions"
    ${PSQL[@]} "$db" -tAc "SELECT DISTINCT proname||':'||oidvectortypes(p.proargtypes) FROM pg_proc p JOIN pg_namespace n ON n.oid=p.pronamespace WHERE n.nspname='public' ORDER BY 1"
    echo "== migrations"
    ${PSQL[@]} "$db" -tAc "SELECT version||':'||encode(checksum,'hex') FROM _sqlx_migrations ORDER BY version"
    echo "== row counts"
    for t in "${SEED_TABLES[@]}"; do
      # Captured with explicit failure propagation: a command substitution
      # inside echo would mask a failed query (e.g. after a future column
      # rename) on BOTH databases and silently compare two empty values.
      v="$(${PSQL[@]} "$db" -tAc "SELECT count(*) FROM ${t}")" \
        || fail "row-count fact query failed for ${t} (${db})"
      echo "${t}=${v}"
    done
    echo "== content checksums"
    for t in "${SEED_TABLES[@]}"; do
      v="$(${PSQL[@]} "$db" -tAc "SELECT coalesce(md5(string_agg(md5(t::text),'' ORDER BY md5(t::text))),'empty') FROM ${t} t")" \
        || fail "content-checksum fact query failed for ${t} (${db})"
      echo "${t}=${v}"
    done
    echo "== fk integrity (dangling child rows)"
    ${PSQL[@]} "$db" -tAc "
      SELECT fk||'='||dangling FROM (VALUES
        ('team_members->teams',      (SELECT count(*) FROM team_members c LEFT JOIN teams p ON p.id=c.team_id WHERE p.id IS NULL)),
        ('team_members->users',      (SELECT count(*) FROM team_members c LEFT JOIN users p ON p.id=c.user_id WHERE p.id IS NULL)),
        ('channels->teams',          (SELECT count(*) FROM channels c LEFT JOIN teams p ON p.id=c.team_id WHERE p.id IS NULL)),
        ('channel_members->channels',(SELECT count(*) FROM channel_members c LEFT JOIN channels p ON p.id=c.channel_id WHERE p.id IS NULL)),
        ('channel_members->users',   (SELECT count(*) FROM channel_members c LEFT JOIN users p ON p.id=c.user_id WHERE p.id IS NULL)),
        ('posts->channels',          (SELECT count(*) FROM posts c LEFT JOIN channels p ON p.id=c.channel_id WHERE p.id IS NULL)),
        ('posts->users',             (SELECT count(*) FROM posts c LEFT JOIN users p ON p.id=c.user_id WHERE p.id IS NULL)),
        ('reactions->posts',         (SELECT count(*) FROM reactions c LEFT JOIN posts p ON p.id=c.post_id WHERE p.id IS NULL))
      ) AS f(fk, dangling) ORDER BY 1"
  }
}

# ---------------------------------------------------------------------------
# Step 1: fresh database at HEAD, seeded with deterministic data.
# ---------------------------------------------------------------------------
# TEMPLATE template0 matches the documented operator procedure
# (createdb -T template0 in docs/operations/runbook.md): a pristine base,
# immune to local template1 customization.
${PSQL[@]} postgres -c "CREATE DATABASE \"${SRC_DB}\" TEMPLATE template0;" >/dev/null
MIGRATION_COUNT=0
for f in $(ls "${MIGRATIONS_DIR}"/*.sql | sort); do
  ${PSQL[@]} "${SRC_DB}" -f "$f" >/dev/null \
    || fail "seed database: migration failed: $(basename "$f")"
  MIGRATION_COUNT=$((MIGRATION_COUNT + 1))
done
note "applied ${MIGRATION_COUNT} migrations to the source database"

# sqlx migration bookkeeping, recorded the way `sqlx migrate run` records it
# (version = filename prefix, description = filename suffix, checksum =
# sha384 of the file content). installed_on/execution_time take defaults —
# only version + checksum are load-bearing for this check (a restore must
# preserve migration state so the app knows where it stands). Applied via
# psql here, so the table is created and populated explicitly.
${PSQL[@]} "${SRC_DB}" >/dev/null <<'SQL'
CREATE TABLE _sqlx_migrations (
    version BIGINT NOT NULL PRIMARY KEY,
    description TEXT NOT NULL,
    installed_on TIMESTAMPTZ NOT NULL DEFAULT now(),
    success BOOLEAN NOT NULL,
    checksum BYTEA NOT NULL,
    execution_time BIGINT NOT NULL DEFAULT 0
);
SQL
for f in $(ls "${MIGRATIONS_DIR}"/*.sql | sort); do
  base="$(basename "$f" .sql)"
  version="${base%%_*}"
  description="$(echo "${base#*_}" | tr '_' ' ')"
  checksum="$(sha384sum "$f" | cut -d' ' -f1)"
  ${PSQL[@]} "${SRC_DB}" -c "INSERT INTO _sqlx_migrations (version, description, success, checksum) VALUES (${version}, '${description}', TRUE, decode('${checksum}', 'hex'));" >/dev/null \
    || fail "failed to record sqlx migration bookkeeping for ${base}"
done

# Deterministic seed data across the core collaboration tables (fixed UUIDs
# and timestamps so every run is reproducible). NOT NULL columns added by
# later migrations all carry defaults, so explicit columns suffice.
${PSQL[@]} "${SRC_DB}" >/dev/null <<'SQL'
INSERT INTO organizations (id, name) VALUES
  ('11111111-1111-1111-1111-111111111111', 'Backup Check Org');
INSERT INTO users (id, username, email, password_hash, role) VALUES
  ('22222222-2222-2222-2222-222222222221', 'brcheck_alice', 'alice@brcheck.test', 'seed-hash-1', 'admin'),
  ('22222222-2222-2222-2222-222222222222', 'brcheck_bob',   'bob@brcheck.test',   'seed-hash-2', 'member');
INSERT INTO teams (id, org_id, name) VALUES
  ('33333333-3333-3333-3333-333333333333', '11111111-1111-1111-1111-111111111111', 'brcheck-team');
INSERT INTO team_members (team_id, user_id, role) VALUES
  ('33333333-3333-3333-3333-333333333333', '22222222-2222-2222-2222-222222222221', 'admin'),
  ('33333333-3333-3333-3333-333333333333', '22222222-2222-2222-2222-222222222222', 'member');
INSERT INTO channels (id, team_id, type, name, creator_id) VALUES
  ('44444444-4444-4444-4444-444444444444', '33333333-3333-3333-3333-333333333333', 'public', 'brcheck-channel', '22222222-2222-2222-2222-222222222221');
INSERT INTO channel_members (channel_id, user_id, role) VALUES
  ('44444444-4444-4444-4444-444444444444', '22222222-2222-2222-2222-222222222221', 'admin'),
  ('44444444-4444-4444-4444-444444444444', '22222222-2222-2222-2222-222222222222', 'member');
INSERT INTO posts (id, channel_id, user_id, message, created_at) VALUES
  ('55555555-5555-5555-5555-555555555551', '44444444-4444-4444-4444-444444444444', '22222222-2222-2222-2222-222222222221', 'backup check root', '2026-01-01T00:00:00Z'),
  ('55555555-5555-5555-5555-555555555552', '44444444-4444-4444-4444-444444444444', '22222222-2222-2222-2222-222222222222', 'backup check reply', '2026-01-01T00:00:01Z');
UPDATE posts SET root_post_id = '55555555-5555-5555-5555-555555555551' WHERE id = '55555555-5555-5555-5555-555555555552';
INSERT INTO reactions (post_id, user_id, emoji_name) VALUES
  ('55555555-5555-5555-5555-555555555551', '22222222-2222-2222-2222-222222222222', 'thumbsup');
SQL
note "seeded ${#SEED_TABLES[@]} core tables with deterministic data"

collect_facts "${SRC_DB}" > "${WORK_DIR}/facts_source.txt"

# ---------------------------------------------------------------------------
# Step 2: the documented operator procedure — custom-format dump, restore
# into a fresh database (docs/operations/runbook.md).
# ---------------------------------------------------------------------------
pg_dump -Fc -d "${SRC_DB}" -f "${DUMP_FILE}" \
  || fail "pg_dump failed"
note "dumped source database ($(du -h "${DUMP_FILE}" | cut -f1), custom format)"

${PSQL[@]} postgres -c "CREATE DATABASE \"${DST_DB}\" TEMPLATE template0;" >/dev/null
pg_restore --no-owner --no-privileges -d "${DST_DB}" "${DUMP_FILE}" \
  || fail "pg_restore into a fresh database failed"
note "restored dump into a fresh database"

# ---------------------------------------------------------------------------
# Step 3: compare — structure, migration bookkeeping, data, integrity.
# ---------------------------------------------------------------------------
# Semantic structural facts, migration bookkeeping, row counts, content
# checksums and referential integrity are all captured by collect_facts; the
# source and restored inventories must match verbatim.
collect_facts "${DST_DB}" > "${WORK_DIR}/facts_restored.txt"
if ! diff -u "${WORK_DIR}/facts_source.txt" "${WORK_DIR}/facts_restored.txt" > "${WORK_DIR}/facts.diff"; then
  sed -n '1,60p' "${WORK_DIR}/facts.diff" >&2
  fail "restored database differs from the source (see diff above)"
fi
note "PASS schema identical: tables, columns, indexes, constraints, sequences, views, triggers, enums, extensions, functions"
note "PASS migration bookkeeping identical: ${MIGRATION_COUNT} _sqlx_migrations rows with matching checksums"
note "PASS data identical: row counts and content checksums for ${#SEED_TABLES[@]} seeded tables"
note "PASS referential integrity: no dangling child rows after restore"

note "ALL CHECKS PASSED (dump -> fresh-database restore -> verified equivalence)"
