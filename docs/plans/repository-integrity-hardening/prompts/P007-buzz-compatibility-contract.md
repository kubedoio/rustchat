# P007 — Formalize the Buzz External Compatibility Record

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
- remote ID constraints.

Do not use Buzz database layout or internal Rust modules as the contract.

## Required work

1. Create a RustChat-owned machine-readable or tightly structured compatibility
   manifest describing the **external protocol surface** RustChat requires.
2. Record the exact upstream Buzz commit most recently tested against.
   - Name it `verified_against`, `tested_upstream_sha`, or equivalent.
   - Do **not** call it a source/runtime dependency pin.
3. Record when/how it was verified and, where useful, external protocol/version
   identifiers independent from repository SHA.
4. Add deterministic local tests/fixtures for every RustChat-required contract
   element.
5. Add a manual or scheduled upstream verification job/script that can test a
   selected newer Buzz revision/environment.
6. Ensure latest-upstream probing reports drift without making unrelated
   RustChat PRs non-deterministic.
7. Document the compatibility update procedure:
   - detect external drift;
   - classify breaking/non-breaking;
   - update connector/fixtures if needed;
   - update `verified_against` only after evidence passes.
8. Activate RI-C08 only after deterministic local contract tests and the first
   explicit compatibility record exist.

## Hard boundaries

Do not:

- add Buzz crates to RustChat;
- vendor Buzz;
- copy Buzz database schema;
- rely on private/internal endpoints;
- change RustChat core posting semantics to satisfy Buzz;
- add inbound/federation features in this phase;
- claim the recorded Buzz SHA is the only compatible version unless tested
  evidence establishes such a constraint.

## Acceptance evidence

- compatibility manifest/record with exact tested upstream revision;
- deterministic connector contract tests;
- proof that disabled/unavailable Buzz cannot break core RustChat posting;
- proof of idempotency/loop-prevention behavior promised by the bridge;
- sample upstream compatibility report identifying the revision/environment tested.

If current Buzz changed incompatibly, report the exact **external contract**
break. Do not reach into Buzz internals as a workaround.
