# Repository Integrity Contracts

This document explains the machine-readable contracts in
`.governance/repository-integrity-contracts.yml`.

The YAML file is normative for contract IDs, lifecycle state, gate applicability,
and required evidence. This document provides implementation meaning.

## Lifecycle

Contracts begin in `planned` state.

A phase may change its contracts to `active` only after objective evidence exists.
Contract activation is itself a governance change and requires the repository's
existing architectural review policy.

This avoids the false claim that merging the ADR instantly makes the repository
compliant with work that has not yet been implemented.

## Gate model

### Merge

The protected check/review set required before a PR may merge.

### Promotion

The evidence required before a moving development alias/artifact may represent a
merged `main` commit as promoted/healthy.

A post-merge integration failure may therefore leave the PR legitimately merged
while making `main` temporarily **degraded / not promotable**.

### Release

Evidence required before a stable or release-candidate artifact is published
from the exact candidate commit.

Release-only upgrade/recovery checks do not have to make every small PR expensive.

## RI-C01 — Authoritative repository state

Do not create one mega-workflow merely to obtain a single green check.

The authoritative state is a documented **set of stable required contexts**.
Per-workflow aggregate jobs are encouraged when they simplify conditional jobs.

**Failure example:** `npm Audit` is required but red while all protected/required
contexts still appear green.

## RI-C02 — Protected default branch and release tags

The repository's live GitHub settings must match written policy.

The contract is not satisfied by committing a desired ruleset document. Evidence
must come from actual GitHub configuration or reproducible administrator
verification.

Release protection must target tags, not version-looking branches.

## RI-C03 — Verified artifact promotion

Building an image is not the same as promoting it.

An immutable/SHA-scoped diagnostic image may exist before all promotion checks,
provided it is not a moving alias users are expected to treat as healthy.

Moving `main`/nightly/stable aliases and release artifacts must follow their
applicable gate sets.

## RI-C04 — API persistence boundary

Direct SQL currently exists in API handlers. This contract does not require a
mass rewrite.

The machine guard is intentionally a tripwire:

1. generate a normalized per-file baseline of direct SQL invocations;
2. reject direct SQL in new production API files;
3. reject count growth in an existing file without an exception;
4. keep human review responsible for semantic expansion of existing queries;
5. move persistence behind repositories/services when a touched domain benefits
   from extraction.

Absolute line numbers must not be normative, because harmless edits move them.

Do not satisfy this contract by wrapping raw SQL in a meaningless one-line helper.

## RI-C05 — Module growth control

Line counts are indicators, not architecture.

The initial thresholds are review tripwires. They may evolve based on repository
evidence without turning "fewer lines" into the architectural goal.

A good decomposition owns a coherent capability or invariant. A bad
decomposition merely creates `part1.rs`, `part2.rs`, and forwarding glue.

Operational size baselines live outside `.governance/**` so ordinary debt
reduction does not itself become an architectural policy edit.

## RI-C06 — Canonical documentation and drift

Use:

- `docs/adr/**` for new architectural decisions;
- `docs/plans/**` for maintainer-approved active specifications/plans;
- audience-oriented docs for current behavior;
- `docs/archive/**` for completed historical implementation material;
- `AGENTS.md` as canonical agent/contributor policy.

Existing `previous-analyses/**` and `docs/superpowers/**` paths are recognized
analysis/tooling outputs because current agent contracts depend on them. They are
not alternate architectural sources of truth.

Legacy `docs/decisions/**` may remain as redirects/indexes; new ADRs belong in
`docs/adr/**`.

## RI-C07 — Migration upgrade proof

For every release candidate, prove both a clean installation and an upgrade from
the latest published stable schema state.

If no immutable database snapshot exists, use a deterministic fixture generated
from the exact published tag/migration history, record its provenance/checksum,
and describe what it does and does not prove.

A migration test passes only when the migrated application reaches expected
readiness, not merely when `sqlx migrate run` exits zero.

## RI-C08 — External integration isolation

ADR-005 remains authoritative.

RustChat's Buzz integration can know:

- documented HTTP/WebSocket protocol;
- event formats/tags required by the connector;
- authentication/signing contract;
- RustChat-owned mappings and outbox state.

It must not know:

- Buzz database tables;
- Buzz internal Rust modules/crates;
- Buzz deployment topology;
- private runtime APIs.

The recorded Buzz commit is the revision most recently tested against. It is
verification evidence, **not** a RustChat source dependency or compatibility
range by itself.

## RI-C09 — Bounded hardening PRs

Repository cleanup is particularly vulnerable to giant "while we are here"
changes.

Each implementation PR must have one primary invariant and evidence. Architectural
program/spec PRs may be larger where needed for coherent review; existing
architectural two-reviewer/human-judgment rules apply rather than standard change
size ceilings.

## RI-C10 — Exception accountability

Guards need escape hatches because static rules cannot understand every valid
design.

Every exception must be explicit and removable. At minimum record:

- contract ID;
- exact affected scope;
- technical rationale;
- responsible owner/reviewer;
- condition under which the exception can be deleted.

"Existing code does it" is not a valid exception rationale.
