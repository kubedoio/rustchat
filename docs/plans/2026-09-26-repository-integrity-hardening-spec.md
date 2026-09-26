# Repository Integrity Hardening — Implementation Specification

**Date:** 2026-09-26  
**Status:** Proposed  
**ADR:** [ADR-006](../adr/ADR-006-repository-integrity-and-maintainability.md)  
**Scope:** repository governance, CI/release integrity, backend layering, maintainability guards, documentation hygiene, migration evidence, external-integration compatibility

## 1. Goal

Make the RustChat repository itself a trustworthy production-engineering control surface.

At completion:

- `main` cannot be merged/promoted while required security or correctness gates are red;
- live GitHub protections match documented governance;
- releases and promoted container aliases come only from verified commits;
- new API-handler persistence leakage is mechanically prevented;
- existing oversized modules cannot silently grow without review;
- roadmap/current-state documentation is kept consistent with completed work;
- schema upgrades are proven from clean and supported historical states;
- the Buzz integration is checked against an explicit external compatibility baseline;
- the program finishes without a broad architecture rewrite.

## 2. Baseline

Initial audit baseline: `main@1fe4813811a832096d8ccac9effbf465ab7f877e`.

Observed facts at that baseline must be revalidated by each implementation phase before changing code. A prompt must not assume that a finding remains true if another PR has already fixed it.

Known audit findings included:

- security/dependency failures could exist on `main` independently of other green checks;
- the visible release-protection ruleset targeted version-like branches rather than tags;
- Docker branch/SHA publication from `main` was not coupled to the complete security result;
- direct SQL calls existed across many `backend/src/api/**` modules;
- several backend source modules exceeded roughly 1,300–2,400 lines;
- `ROADMAP.md` still listed runtime-bootstrap cleanup and the initial Buzz connector as unfinished after their implementation;
- documentation history contained overlapping categories and exact duplicates;
- release/upgrade evidence did not yet form a complete migration matrix.

## 3. Non-goals

This program must not:

- redesign RustChat into microservices;
- convert the backend into a large multi-crate workspace solely for cleanliness;
- rewrite all direct SQL at once;
- mechanically split every large file;
- change Mattermost compatibility behavior unless necessary to preserve behavior during a bounded refactor;
- expand the Buzz feature set;
- import Buzz internals;
- rewrite or squash already-applied database migrations;
- add unrelated product functionality.

## 4. Contracts

The normative contracts are in `.governance/repository-integrity-contracts.yml`.

Human-readable rationale and evidence rules are in `docs/plans/repository-integrity-hardening/CONTRACTS.md`.

A contract can be changed only in a PR that:

1. explicitly calls out the contract ID;
2. explains why the invariant changed rather than merely why the current implementation is inconvenient;
3. updates ADR/spec documentation when the architectural meaning changes;
4. receives the review level required by `.governance/risk-tiers.yml`.

## 5. Implementation phases

### Phase P001 — Restore authoritative green-main semantics

#### Required work

- Re-run all security/dependency checks on current `main`.
- Fix or explicitly and narrowly disposition real findings; do not suppress a failing advisory merely to make CI green.
- Rename misleading "production-readiness" checks if they are only static regression checks.
- Introduce a stable aggregate security/quality result suitable for branch protection.
- Ensure promoted `main` container aliases are not published from a commit that failed the required aggregate.

#### Acceptance

- all required security/dependency checks pass on the implementation head;
- an intentionally failing fixture/test proves the aggregate cannot remain green;
- image-promotion behavior is tested or dry-run verified;
- no advisory suppression is added without a documented, scoped justification and expiry/removal condition.

### Phase P002 — GitHub governance and release integrity

#### Required work

- Align default-branch protections with `GOVERNANCE.md`, `CODEOWNERS`, and `docs/github-protection.md`.
- Protect release tags as tags.
- Require a stable aggregate CI/security check and DCO.
- Verify review/codeowner/conversation-resolution policy.
- Document any administrator/emergency bypass.
- Make the release workflow independently verify the exact tagged commit before publishing.

#### Acceptance

Evidence must show:

- a failing required check blocks merge;
- direct/force push is rejected according to policy;
- ordinary contributors cannot overwrite/delete a release tag;
- the documented ruleset IDs/settings match live GitHub;
- a release cannot be created merely because version files match.

If the executing identity cannot change repository settings, the code/documentation work may be prepared, but the phase remains **BLOCKED**, not complete.

### Phase P003 — Persistence-boundary guard

#### Required work

- Inventory production direct SQL usage under `backend/src/api/**`.
- Store a deterministic baseline.
- Add a CI/test guard that rejects net-new direct handler persistence sites except an explicit, reviewed allowlist.
- Select one high-value touched path as a pilot and move its persistence/business access through the existing repository/service layer.
- Preserve API behavior and error mapping.

#### Acceptance

- baseline generation is deterministic;
- adding a synthetic new `sqlx::query*` in an API module makes the guard fail;
- existing debt does not require a mass rewrite;
- the pilot has focused unit/integration coverage;
- no circular dependency or "god repository" is introduced.

### Phase P004 — Module-responsibility and growth control

#### Required work

- Record current oversized production modules.
- Add a growth-budget guard that detects new monolithic production files and unexplained growth of baseline-large modules.
- Decompose only modules where there is a clear responsibility boundary and meaningful review/test benefit.
- Start with the most valuable backend candidates, not a repository-wide split.

Initial candidates from the audit include:

- `backend/src/repositories/post_repository.rs`;
- `backend/src/repositories/admin_repository.rs`;
- `backend/src/repositories/channel_repository.rs`;
- `backend/src/repositories/user_repository.rs`;
- `backend/src/api/v4/websocket/connection.rs`;
- `backend/src/services/unreads.rs`.

#### Acceptance

- public behavior remains unchanged;
- new modules map to cohesive responsibilities;
- tests remain discoverable and pass;
- line movement alone is not counted as success;
- the guard has a documented exception mechanism.

### Phase P005 — Documentation and repository hygiene

#### Required work

- Fix already-known roadmap/current-state drift.
- Collapse exact duplicate active/historical documents.
- Establish canonical categories from ADR-006.
- Move root-level historical audit material to the proper documentation area.
- Treat `AGENTS.md` as canonical contributor/agent policy; compatibility instruction files must not silently diverge.
- Add a lightweight docs-drift/repository-hygiene check for links, canonical-path duplication, and roadmap/current-state obligations.

#### Acceptance

- no exact duplicate document remains in two active categories without an explicit reason;
- moved files have updated references;
- current roadmap accurately marks completed runtime-bootstrap and initial Buzz-bridge work;
- no generated/build/cache artifacts are newly tracked;
- docs checks run in CI with deterministic output.

### Phase P006 — Migration, backup, and recovery evidence

#### Required work

Build an automated or reproducible matrix for:

1. empty database → HEAD;
2. latest published stable snapshot → HEAD;
3. readiness/startup after migration;
4. backup → restore → application sanity check where tooling is supported.

Tests must use immutable fixtures or a documented fixture-generation procedure.

#### Acceptance

- CI or a release-gate workflow exercises the matrix;
- failures preserve useful evidence;
- migration state/readiness assertions match actual embedded migrations;
- existing migration files are not rewritten;
- secrets are not embedded in fixtures/logs.

### Phase P007 — Buzz external compatibility contract

#### Required work

- Document the exact external Buzz surface RustChat relies on.
- Pin the verified upstream revision in the compatibility record.
- Add deterministic connector/protocol tests that do not require Buzz HEAD.
- Add an optional scheduled/manual compatibility probe against a newer upstream revision.
- Upstream drift must report incompatibility without breaking unrelated RustChat development unless the pinned contract itself is broken.

#### Acceptance

- RustChat contains no Buzz source/runtime dependency;
- the connector can be tested against deterministic fixtures/mocks;
- loop-prevention/idempotency/security behavior remains covered;
- the compatibility report identifies the exact Buzz revision tested.

### Phase P008 — Final convergence

#### Required work

Re-audit the repository against every contract.

No new feature work is allowed.

#### Acceptance

Produce a single evidence report containing:

- contract ID;
- implementation location;
- automated test/check;
- latest passing run or command evidence;
- any exception, owner, and expiry/removal condition;
- unresolved blockers.

The phase passes only when no contract is falsely reported as satisfied.

## 6. PR strategy

Do not implement this spec as one giant PR.

Preferred sequence:

1. P001 security/green-main;
2. P002 governance/release;
3. P003 persistence guard + one pilot extraction;
4. P004 growth guard + targeted decomposition;
5. P005 docs/repository hygiene;
6. P006 migration/recovery;
7. P007 Buzz compatibility;
8. P008 convergence/evidence.

P001 and code-only portions of P002 may overlap only if review remains understandable. P003–P007 should remain independently reviewable.

## 7. Validation requirements

Every implementation PR must run the normal checks for all touched areas plus the smallest focused tests proving its new contract.

At final convergence, at minimum validate:

### Backend

```bash
cd backend
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --lib
```

and the documented integration suite with PostgreSQL, Redis, and S3-compatible storage.

### Push proxy

```bash
cd push-proxy
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo audit
cargo deny check
```

### Frontend

```bash
cd frontend
npm ci --ignore-scripts
npm run apply:dependency-patches
npm run build
npm run test:unit
npm audit --audit-level=high
```

### Repository

- governance/contract guard;
- documentation/link/hygiene guard;
- migration matrix;
- release dry-run or release-candidate verification;
- Buzz compatibility tests.

## 8. Stop conditions

An implementation agent must stop and report rather than "solve around" the contract when:

- current repository state materially contradicts the prompt baseline;
- a required GitHub setting cannot be changed with available permissions;
- fixing a phase requires an unrelated product redesign;
- a security finding has no safe verified remediation;
- a migration test would require modifying applied migration history;
- a Buzz change would require importing upstream internals;
- the work cannot fit into a bounded reviewable PR.

A stopped phase is evidence of a real blocker, not a failure to complete the assignment.
