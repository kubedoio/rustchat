# P007 — Formalize the Buzz External Compatibility Contract

## Mission

Keep the Buzz bridge useful without allowing Buzz's rapidly changing repository
or internals to become RustChat architecture.

Primary contracts: **RI-C08, RI-C09, RI-C10**. ADR-005 remains authoritative.

## Inspect current integration

Read:

- `docs/adr/ADR-005-rustchat-core-and-buzz-integration.md`;
- `docs/integrations/buzz.md`;
- `backend/src/integrations/buzz/**`;
- integration outbox code and migrations;
- existing Buzz integration tests.

Identify exactly which external Buzz behaviors RustChat relies on:

- endpoint/transport;
- authentication/signing;
- event kind/tag/content shape;
- success/error semantics;
- idempotency/deduplication expectations;
- loop-prevention markers;
- any remote ID constraints.

Do not use Buzz database or internal Rust code as the contract.

## Required work

1. Create a machine-readable or tightly structured compatibility manifest owned
   by RustChat.
2. Record the exact upstream Buzz revision most recently verified.
3. Add deterministic local tests/fixtures for every RustChat-required contract
   element.
4. Add a manual or scheduled upstream verification job/script that can test a
   selected newer Buzz revision/environment.
5. Ensure the scheduled/latest-upstream probe reports drift without making
   unrelated RustChat PRs non-deterministic.
6. Document the upgrade procedure:
   - detect upstream drift;
   - classify breaking/non-breaking;
   - update connector/fixtures;
   - move the pinned verified revision only after evidence passes.

## Hard boundaries

Do not:

- add Buzz crates to RustChat;
- vendor Buzz;
- copy Buzz database schema;
- rely on private/internal endpoints;
- change RustChat core posting semantics to satisfy Buzz;
- add inbound/federation features in this phase.

## Acceptance evidence

- manifest/record with pinned revision;
- deterministic connector contract tests;
- proof that disabled/unavailable Buzz cannot break core RustChat posting;
- proof of idempotency/loop-prevention behavior already promised by the bridge;
- sample upstream compatibility report showing the revision tested.

If current Buzz has changed incompatibly, report the exact external contract
break. Do not reach into Buzz internals as a workaround.
