# P003 — Freeze Handler-Level Persistence Debt

## Mission

Stop RustChat's API layer from accumulating new direct database access while
avoiding a risky mass rewrite.

Primary contracts: **RI-C04, RI-C09, RI-C10**.

## Read first

- backend architecture documentation;
- `backend/src/api/**`;
- `backend/src/repositories/**`;
- `backend/src/services/**`;
- existing repository/service tests;
- ADR-006 and the repository-integrity contracts.

## Required work

1. Build a deterministic inventory of production `sqlx::query*`,
   `query_as*`, `query_scalar*`, and equivalent direct DB invocation sites
   under `backend/src/api/**`.
   - Exclude tests/fixtures with explicit rules.
   - Do not make absolute line numbers normative.
   - Record a normalized per-file inventory at
     `tools/repository-integrity/baselines/api-direct-sql.txt`.
2. Add a fast tripwire that fails on:
   - direct SQL in a **new production API file**;
   - increased normalized direct-SQL invocation count in an existing production
     API file unless explicitly excepted.
3. Print actionable offending paths and counts/fingerprints.
4. Keep human review responsible for semantic expansion of an existing query;
   the tripwire is not a SQL semantic analyzer.
5. Add an RI-C10 exception mechanism.
6. Pick **one** high-value existing API path and move its persistence through a
   cohesive existing/new repository/service boundary.
   - Choose based on current coupling and testability, not largest LOC.
   - Preserve HTTP contract, authorization, error mapping, transaction semantics,
     and telemetry.
7. Add focused tests proving behavior before/after extraction.
8. Activate RI-C04 only when the tripwire and evidence are merged.

## Design rules

A valid extraction owns meaningful persistence behavior.

This is **not** valid:

```rust
fn run_query(pool: &PgPool) { /* same handler SQL moved one file away */ }
```

Prefer:

```text
handler -> service/use-case -> repository -> PostgreSQL
```

when business orchestration exists, or:

```text
handler -> repository -> PostgreSQL
```

for simple persistence reads/writes.

Do not invent abstraction merely to satisfy the checker.

## Scope/authority note

A bounded backend agent may implement the production-code extraction only within
its allowed paths. Guard/baseline tooling outside those paths requires a
maintainer-supervised companion change; this prompt does not override
`.governance/agent-contracts.yml`.

## Acceptance evidence

- committed deterministic baseline;
- guard test proving a synthetic new file/call site fails;
- harmless line movement does not create a false regression;
- one bounded pilot extraction with focused tests;
- before/after normalized call-site inventory;
- normal backend validation;
- no broad API rewrite.

If the best pilot expands beyond a reviewable PR, stop after the tripwire/baseline
and propose a smaller pilot.
