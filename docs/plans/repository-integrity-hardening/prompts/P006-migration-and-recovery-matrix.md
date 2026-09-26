# P006 — Migration, Backup, and Recovery Matrix

## Mission

Turn database evolution from "migrations exist" into release evidence that
supported installations can reach the current version safely.

Primary contracts: **RI-C07, RI-C03, RI-C10**.

## Revalidate supported history

Determine from repository/release evidence:

- latest published stable RustChat release;
- schema/migration state corresponding to that release;
- current embedded migration head;
- currently documented backup/restore procedure.

Do not invent a historical schema snapshot from memory.

## Required work

Create a reproducible test/release matrix covering:

### A. Empty database → HEAD

- create a clean PostgreSQL instance/database;
- apply all current migrations;
- start the backend or execute the same readiness path the release uses;
- assert no pending/failed migration state.

### B. Latest published stable → HEAD

- restore or construct an immutable, verified fixture representing the latest
  published stable schema;
- apply current migrations exactly as a real upgrade would;
- start the backend/readiness path;
- verify representative critical data survives.

### C. Backup → restore → sanity

Where supported by current tooling:

- create representative data;
- take a documented backup;
- restore into a clean target;
- run migration/readiness if appropriate;
- verify representative users/teams/channels/posts/files metadata required by
  the backup contract.

## Fixture rules

- fixtures must contain no real user secrets;
- fixture provenance/version must be documented;
- fixture generation must be reproducible or the fixture must have an immutable
  checksum;
- never edit old migrations to make a fixture pass;
- preserve failure logs/evidence.

## CI/release integration

Choose the cheapest reliable placement:

- fast empty→HEAD checks may run on every relevant PR;
- stable-snapshot upgrade may run on integration/release workflows;
- expensive backup/restore proof may be release-candidate or scheduled if that
  is justified.

The final release gate must consume the required evidence.

## Acceptance evidence

Provide a table:

| Path | Fixture/source | Command/workflow | Result | Evidence |
|---|---|---|---|---|
| empty→HEAD | ... | ... | PASS | ... |
| stable→HEAD | ... | ... | PASS | ... |
| restore sanity | ... | ... | PASS/BLOCKED | ... |

A migration command exiting zero without application readiness is insufficient.
