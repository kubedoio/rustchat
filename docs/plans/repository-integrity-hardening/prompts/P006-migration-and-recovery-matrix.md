# P006 — Migration, Backup, and Recovery Matrix

## Mission

Turn database evolution from "migrations exist" into release evidence that
supported installations can reach the current version safely.

Primary contracts: **RI-C07, RI-C03, RI-C09, RI-C10**.

Canonical release work item: **#259**.

## Revalidate supported history

Determine from repository/release evidence:

- latest published stable RustChat release;
- exact tag/commit for that release;
- schema/migration state corresponding to that release;
- whether an immutable historical database snapshot actually exists;
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

### B. Latest published stable schema → HEAD

Preferred evidence is an immutable sanitized database fixture from the exact
published release.

If that does not exist:

- check out/build the exact published tag or use its exact migration history;
- deterministically construct the historical schema fixture;
- record source tag/commit, generation command, and checksum;
- clearly label this as **schema-upgrade evidence**, not proof that an arbitrary
  real-world production backup restores correctly.

Then:

- apply current migrations exactly as a real upgrade would;
- start the backend/readiness path;
- verify representative critical data survives if the fixture contains data.

### C. Backup → restore → sanity

Where current tooling/documentation supports it:

- create representative data;
- take a documented backup;
- restore into a clean target;
- run migration/readiness if appropriate;
- verify representative users/teams/channels/posts/file metadata included in the
  recovery contract.

If backup/restore tooling is not yet production-supported, report that as a
separate release gap rather than faking evidence.

## Fixture rules

- fixtures contain no real user secrets;
- fixture provenance/version is documented;
- fixture generation is reproducible or the fixture has an immutable checksum;
- never edit old migrations to make a fixture pass;
- preserve failure logs/evidence.

## CI/release integration

Use the cheapest reliable placement:

- fast empty→HEAD checks may run on relevant PRs;
- stable-schema upgrade may run post-merge/integration or in release gates;
- expensive backup/restore proof may be release-candidate or scheduled when justified.

RI-C07 is release-required. Do not silently make every small PR run the complete
recovery matrix without evidence that the cost is warranted.

## Acceptance evidence

Provide:

| Path | Fixture/source | Command/workflow | Result | Evidence |
|---|---|---|---|---|
| empty→HEAD | ... | ... | PASS | ... |
| stable-schema→HEAD | ... | ... | PASS | ... |
| restore sanity | ... | ... | PASS/BLOCKED | ... |

A migration command exiting zero without application readiness is insufficient.

Activate RI-C07 only when the required release evidence is real and reproducible.
