# Repository Integrity Contracts

This document explains the machine-readable contracts in
`.governance/repository-integrity-contracts.yml`.

The YAML file is normative for IDs and required evidence. This document provides
the implementation meaning.

## RI-C01 — Authoritative main

A commit on `main` is not "green" because one convenient aggregate succeeded.
All gates designated as required by governance must be represented in the
authoritative result.

A security/dependency failure may not be hidden behind a separately green
"production readiness" check.

**Failure example:** `npm Audit` is red but the merge/release aggregate is green.

## RI-C02 — Protected default branch and release tags

The repository's live GitHub settings must match the written policy.

The contract is not satisfied by committing a desired ruleset document. Evidence
must come from the actual GitHub configuration or a reproducible administrator
verification.

Release protection must target `refs/tags/v*` (or the documented equivalent),
not version-looking branches.

## RI-C03 — Verified artifact promotion

Building an image is not the same as promoting it.

SHA-scoped diagnostic artifacts may be produced during CI, but a moving branch
alias, stable alias, or release artifact must not claim a commit that failed the
required release gates.

Implementations may use workflow dependencies, a promotion workflow, or explicit
commit-status verification. The invariant matters more than the mechanism.

## RI-C04 — API persistence boundary

Direct SQL currently exists in API handlers. This contract does not require a
mass rewrite.

Instead:

1. generate and review a baseline of existing direct SQL call sites;
2. fail when a PR adds an unapproved new call site;
3. move persistence behind repositories/services when a touched domain benefits
   from the extraction;
4. require explicit review for justified exceptions.

Do not satisfy this contract by wrapping raw SQL in a meaningless one-line helper.

## RI-C05 — Module growth control

Line counts are indicators, not architecture.

The guard exists to prevent already-large modules from silently becoming much
larger and to force discussion when a new production module becomes monolithic.

A good decomposition owns a coherent capability or invariant. A bad
decomposition merely creates `part1.rs`, `part2.rs`, and forwarding glue.

The thresholds in the YAML are review triggers. An exception is valid when a
single cohesive module is genuinely safer than artificial fragmentation.

## RI-C06 — Canonical documentation and drift

Use:

- `docs/adr/**` for architectural decisions;
- `docs/plans/**` for active specifications/plans;
- existing audience-oriented docs for current behavior;
- `docs/archive/**` for completed historical implementation material;
- `AGENTS.md` as canonical agent/contributor policy.

A roadmap/current-state claim that a completed feature is still pending is a
repository defect.

## RI-C07 — Migration upgrade proof

For every release candidate, prove both a clean installation and an upgrade from
the latest published stable schema state.

The fixture must represent a real supported historical state. Do not recreate
history by editing old migrations.

A migration test passes only when the migrated application reaches the expected
readiness state, not merely when `sqlx migrate run` exits zero.

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
- Buzz private runtime APIs.

The compatibility record must identify the exact upstream revision tested.

## RI-C09 — Bounded hardening PRs

Repository cleanup is particularly vulnerable to giant "while we are here"
changes.

Each hardening PR must have one primary invariant and evidence. If the change
starts redesigning unrelated product behavior, split or stop.

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
