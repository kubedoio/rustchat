# API Persistence Boundary (RI-C04)

Direct production SQL inside API handlers is baseline debt: it must not grow
silently. New business persistence belongs behind a cohesive
repository/service boundary (e.g. `backend/src/repositories/**`), and any
exception must be explicit, tested, and reviewed.

This page is the source of truth for the RI-C04 guard.

## The guard

`tools/repository-integrity/check_api_direct_sql.py` inventories production
direct-SQL usage under `backend/src/api/**` and, in `--check` mode (the
default, used by CI), rejects:

* direct SQL appearing in a **newly created** production API file; and
* an **increased** normalized direct-SQL count in an existing API file,

unless the affected path is listed in the RI-C04 exception register.

The normalized count is the number of direct-SQL invocation tokens
(`sqlx::query`, `sqlx::query_as`, `sqlx::query_scalar`, `sqlx::query_with`,
`sqlx::query_file`, `sqlx::raw_sql`, `sqlx::QueryBuilder`) left after comments
are stripped. It is independent of line numbers, so reformatting or moving
code does not create false debt.

## Files

| Path | Purpose |
|------|---------|
| `tools/repository-integrity/check_api_direct_sql.py` | guard + baseline generator |
| `tools/repository-integrity/baselines/api-direct-sql.txt` | deterministic per-file normalized inventory |
| `tools/repository-integrity/exceptions/api-direct-sql.txt` | RI-C04 exception register |
| `tools/repository-integrity/test_api_direct_sql.py` | guard acceptance tests |

## Commands

```bash
# Enforce the tripwires and baseline freshness (matches CI).
python3 tools/repository-integrity/check_api_direct_sql.py

# Regenerate the baseline after an intentional, reviewed change.
python3 tools/repository-integrity/check_api_direct_sql.py --update

# Show current per-file counts.
python3 tools/repository-integrity/check_api_direct_sql.py --print

# Run the guard acceptance tests.
python3 tools/repository-integrity/test_api_direct_sql.py
```

## Keeping it green

The committed baseline must equal the current deterministic generation. If you
intentionally add direct SQL to an existing API file, you must:

1. register the path in the exception register with a decision note,
   focused tests, and reviewer justification; and
2. regenerate the baseline with `--update` so the manifest reflects reality.

New production API files must take their persistence through a repository or
service boundary and must not contain direct SQL.

## Pilot (P003)

`backend/src/api/site.rs` was the first path moved behind the boundary: its
two inline `server_config` reads now go through `SystemRepository
::get_server_config`, with the site→public mapping extracted into a pure,
unit-tested `build_public_config`. Behavior and error mapping are unchanged.
