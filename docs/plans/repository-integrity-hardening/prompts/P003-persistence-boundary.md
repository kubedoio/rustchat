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
- architecture and governance rules from ADR-006.

## Required work

1. Build a deterministic inventory of production `sqlx::query*`,
   `query_as*`, `query_scalar*`, and equivalent direct DB call sites under
   `backend/src/api/**`.
   - Exclude tests/fixtures with explicit rules.
   - Store the reviewed baseline at
     `.governance/baselines/api-direct-sql.txt`.
2. Add a fast repository check that fails on net-new unapproved API-layer SQL
   call sites.
   - It must be deterministic across machines.
   - It must explain the offending file/line/symbol.
3. Add an explicit exception mechanism satisfying RI-C10.
4. Pick **one** high-value existing API path and move its persistence through a
   cohesive existing/new repository/service boundary.
   - Choose based on current coupling and testability, not largest LOC.
   - Preserve HTTP contract, authorization, error mapping, transaction semantics,
     and telemetry.
5. Add focused tests proving behavior before/after the extraction.

## Design rules

A valid extraction owns meaningful persistence behavior.

This is **not** valid:

```rust
fn run_query(pool: &PgPool) { /* same handler SQL moved one file away */ }
```

Prefer domain operations such as:

```text
handler -> service/use-case -> repository -> PostgreSQL
```

when business orchestration exists, or:

```text
handler -> repository -> PostgreSQL
```

for simple persistence reads/writes.

Do not invent an abstraction layer merely to satisfy the checker.

## Acceptance evidence

- committed deterministic baseline;
- guard test proving a synthetic new call site fails;
- one bounded pilot extraction with tests;
- before/after call-site count;
- normal backend fmt/clippy/unit/integration validation for touched behavior;
- no broad API rewrite.

If the best pilot expands beyond a reviewable PR, stop after the guard/baseline
and propose a smaller pilot.
