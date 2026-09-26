# Repository Integrity Hardening — Implementation Specification

**Date:** 2026-09-26  
**Status:** Proposed  
**ADR:** [ADR-006](../adr/ADR-006-repository-integrity-and-maintainability.md)  
**Scope:** repository governance, CI/release integrity, backend layering, maintainability guards, documentation hygiene, migration evidence, external-integration compatibility

## 1. Goal

Make the RustChat repository itself a trustworthy production-engineering control surface without turning repository policy into a second product.

At completion:

- protected merge checks accurately describe PR mergeability;
- post-merge health accurately controls moving development aliases/promotions;
- release gates accurately control release candidates;
- live GitHub protections match documented governance;
- new API-handler persistence leakage is mechanically detected;
- oversized modules cannot silently grow without review;
- roadmap/current-state documentation remains aligned with completed work;
- schema upgrades are proven from clean and supported historical states;
- the Buzz integration is checked against an explicit external compatibility record;
- the program finishes without a broad architecture rewrite.

## 2. Baseline and revalidation rule

Initial audit baseline: `main@1fe4813811a832096d8ccac9effbf465ab7f877e`.

The audit was a starting point, not permanent truth. Every implementation phase must revalidate current `main`, current open work, current checks, and current repository settings before making changes.

Known audit findings included:

- security/dependency failures could exist independently from other green checks;
- visible release-protection configuration targeted version-like branches rather than tags;
- moving Docker aliases from `main` were not coupled to the complete promotion/release evidence;
- direct SQL calls existed across many `backend/src/api/**` modules;
- several backend source modules exceeded roughly 1,300–2,400 lines;
- `ROADMAP.md` still listed runtime-bootstrap cleanup and the initial Buzz connector as unfinished after implementation;
- documentation history contained overlapping categories and exact duplicates;
- release/upgrade evidence did not yet form a complete migration matrix.

If current evidence differs, the implementation must follow current evidence and record the delta from this baseline.

## 3. Existing-policy and issue alignment

This program extends, rather than replaces:

- `GOVERNANCE.md`;
- `.governance/risk-tiers.yml`;
- `.governance/protected-paths.yml`;
- `.governance/agent-contracts.yml`;
- `.github/CODEOWNERS`;
- DCO requirements;
- ADR-005 for Buzz.

Canonical issue alignment:

- **#258** — live GitHub branch/tag protection and required checks; primarily P002.
- **#259** — v0.5.1 release; consumes P001/P002/P006/P008 evidence.
- **#88 / #89** — product defects that remain separate release blockers; this program must not mark them fixed unless their own behavior is actually corrected.

The program must not create duplicate issues for work already owned by those issues unless a separately scoped child issue is genuinely useful.

## 4. Contract lifecycle and gate model

The normative contract definitions live in
`.governance/repository-integrity-contracts.yml`.

Every contract has:

- an activation phase;
- lifecycle state (`planned`, `active`, or `superseded`);
- merge, promotion, and release applicability.

Merging ADR-006 does **not** mean all contracts are already satisfied.

### Merge gates

Checks/reviews required before a PR can merge. These should be stable branch-protection contexts, often via per-workflow aggregate jobs.

### Promotion gates

Checks required before a moving development alias/artifact represents a merged `main` commit as promoted/healthy. Post-merge integration failures may block promotion without retroactively invalidating the merge.

### Release gates

Checks/evidence required for the exact tagged/candidate commit. Release-only migration/recovery tests may live here rather than making every small PR expensive.

### Contract activation

When a phase has objective evidence, its implementation PR may change the relevant contract state from `planned` to `active`. That change is a governance change and requires existing architectural review.

Do not activate a contract based solely on implementation intent.

## 5. Non-goals

This program must not:

- redesign RustChat into microservices;
- convert the backend into a large multi-crate workspace solely for cleanliness;
- rewrite all direct SQL at once;
- mechanically split every large file;
- change Mattermost compatibility behavior unless necessary to preserve behavior during a bounded refactor;
- expand the Buzz feature set;
- import Buzz internals;
- rewrite or squash already-applied database migrations;
- duplicate #88/#89 product work;
- create one global mega-aggregate that couples unrelated workflows.

## 6. Execution authority

These prompts are maintainer/architect campaigns. They are **not** an override of
`.governance/agent-contracts.yml`.

A bounded backend/frontend/compat agent may implement code inside its existing allowed paths, but may not edit prohibited governance paths merely because a prompt asks for it.

Where a phase requires both production-code changes and governance/workflow changes, either:

1. use a maintainer-supervised implementation session with the required human review; or
2. split the production-code and governance parts into reviewable companion PRs.

Human review requirements remain authoritative.

## 7. Implementation phases

### Phase P001 — Restore check integrity and promotion semantics

#### Required work

- Re-run current security/dependency checks on current `main`.
- Fix or narrowly disposition real findings; do not suppress a failing advisory merely to make CI green.
- Rename misleading checks if they prove a narrower property than their name implies.
- Define stable **merge**, **promotion**, and **release** gate sets.
- Prefer per-workflow aggregate checks (for example `CI Complete`, a stable security aggregate) over a cross-workflow mega-aggregate.
- Ensure moving `main` aliases/promotions cannot advance when promotion-required checks are red.
- Keep SHA-scoped diagnostic artifacts clearly non-promoted if they are produced before full promotion evidence.

#### Acceptance

- required merge security/dependency checks pass on the implementation head;
- a negative test proves a required merge failure blocks the merge aggregate/set;
- a negative test proves a required post-merge failure blocks moving-alias promotion;
- no advisory suppression is added without an RI-C10 exception.

### Phase P002 — GitHub governance and release integrity

#### Required work

- Align default-branch protections with `GOVERNANCE.md`, CODEOWNERS, DCO, and the gate sets from P001.
- Protect release versions as tags.
- Require the stable merge-gate contexts in GitHub.
- Verify review/codeowner/conversation-resolution policy.
- Document any break-glass bypass.
- Harden release provenance:
  - the tag must identify an approved/protected-main commit according to the documented release model;
  - the exact tagged commit must satisfy or rerun the release-gate set before publication;
  - version/changelog agreement alone is not sufficient.

#### Acceptance

Evidence must show:

- a failing required merge check blocks merge;
- ordinary direct/force push is rejected according to policy;
- ordinary contributors cannot overwrite/delete a release tag;
- documented rule/settings identifiers match live GitHub;
- an arbitrary tag with matching version files cannot bypass release verification.

If the executing identity cannot change repository settings, the phase remains **BLOCKED** until an administrator applies and verifies them.

### Phase P003 — Persistence-boundary tripwire

#### Required work

- Inventory production direct SQL usage under `backend/src/api/**`.
- Store a deterministic operational baseline under
  `tools/repository-integrity/baselines/api-direct-sql.txt`.
- The baseline must not depend on absolute line numbers.
- Add a fast check that rejects:
  - direct SQL in a newly created production API file;
  - an increase in normalized direct-SQL invocation count in an existing API file, unless explicitly excepted.
- Human review must still inspect material expansion of existing queries; the tripwire is not a semantic SQL verifier.
- Select one high-value touched path as a pilot and move persistence/business access through a cohesive repository/service boundary.
- Preserve API behavior and error mapping.

#### Acceptance

- baseline generation is deterministic;
- adding a synthetic new call site makes the guard fail;
- moving line numbers without changing call sites does not create false debt;
- existing debt does not require a mass rewrite;
- the pilot has focused unit/integration coverage.

### Phase P004 — Module-responsibility and growth control

#### Required work

- Record current oversized production modules under
  `tools/repository-integrity/baselines/large-production-modules.txt`.
- Implement the initial RI-C05 review tripwires.
- Thresholds are review triggers, not line-count targets.
- Decompose only modules where there is a clear responsibility boundary and meaningful review/test benefit.
- Start with the highest-value backend candidates, not a repository-wide split.

#### Acceptance

- public behavior remains unchanged;
- new modules map to cohesive responsibilities;
- tests remain discoverable and pass;
- line movement alone is not counted as success;
- the guard has an RI-C10 exception mechanism.

### Phase P005 — Documentation and repository hygiene

#### Required work

- Fix current roadmap/current-state drift.
- Reconcile canonical ADR/plan locations with existing analysis/agent-output paths.
- Do **not** move/delete `previous-analyses/**` or `docs/superpowers/**` while agent contracts still require them unless those contracts are updated in the same reviewed change.
- Collapse exact duplicate active/historical documents.
- Move root-level historical audit material to the proper documentation history area.
- Treat `AGENTS.md` as canonical agent policy; harness-specific compatibility files must not contradict it.
- Add lightweight docs/repository-hygiene checks for links, authority duplication, and selected tracked generated artifacts.
- Add a PR-template obligation to consider roadmap/current-state/ADR/compatibility updates.

#### Acceptance

- no exact duplicate document remains with competing active authority;
- moved files have repaired references;
- current roadmap/current-state accurately represent completed work;
- existing agent analysis workflows are not broken;
- deterministic docs checks run in CI.

### Phase P006 — Migration, backup, and recovery evidence

#### Required work

Build a reproducible matrix for:

1. empty database → HEAD;
2. latest published stable schema state → HEAD;
3. application readiness after migration;
4. backup → restore → application sanity where current recovery tooling supports it.

If no immutable historical database snapshot exists, construct the first stable-schema fixture from the exact published tag/migration history, record provenance/checksums, and state clearly that this proves schema upgrade—not recovery of an unknown real-world backup.

#### Acceptance

- CI or a release-gate workflow exercises required paths;
- failures preserve useful evidence;
- readiness assertions match actual embedded migrations;
- existing migration files are not rewritten;
- fixtures/logs contain no secrets.

### Phase P007 — Buzz external compatibility record

#### Required work

- Document the exact external Buzz surface RustChat relies on.
- Record the exact upstream revision most recently verified against.
- Treat that SHA as evidence, **not** a runtime/source dependency pin.
- Add deterministic connector/protocol tests that do not require Buzz HEAD.
- Add optional scheduled/manual compatibility probing against a selected newer upstream revision.
- Upstream drift must report incompatibility without breaking unrelated RustChat development unless the RustChat-owned contract itself is broken.

#### Acceptance

- RustChat contains no Buzz source/runtime dependency;
- connector tests are deterministic;
- loop-prevention/idempotency/security behavior remains covered;
- compatibility output identifies the exact upstream revision tested.

### Phase P008 — Final convergence

#### Required work

Re-audit the repository against every **active** contract and all planned contracts expected for the release.

No new feature work is allowed.

Also inspect external v0.5.1 blockers (#88, #89, #258, #259 and any newer release blockers). P008 can determine repository-integrity readiness; it must not claim product release readiness while separate release blockers remain open/unresolved.

#### Acceptance

Produce a single evidence report containing:

- contract ID and lifecycle state;
- implementation location;
- automated test/check;
- live/manual evidence where required;
- any exception, owner, and removal condition;
- unresolved repository-integrity blockers;
- unresolved external release blockers.

The phase passes repository-integrity convergence only when no contract is falsely reported as satisfied.

## 8. PR strategy

Do not implement this spec as one giant PR.

Preferred sequence:

1. P001 check/security/promotion integrity;
2. P002 governance/release;
3. P003 persistence tripwire + one pilot extraction;
4. P004 growth tripwire + targeted decomposition;
5. P005 docs/repository hygiene;
6. P006 migration/recovery;
7. P007 Buzz compatibility;
8. P008 convergence/evidence.

P001 and code-only portions of P002 may overlap only if review remains understandable. P003–P007 should remain independently reviewable.

Architectural-tier PRs are governed by human judgment and the two-reviewer policy. Standard/elevated hard-size guidance must not be mechanically applied in contradiction with `risk-tiers.yml`.

## 9. Validation requirements

Every implementation PR must run the normal checks for all touched areas plus the smallest focused tests proving its new contract.

At final convergence, validate the current commands in `AGENTS.md` and current workflows rather than copying stale commands from this specification.

At minimum the evidence set should cover:

- backend formatting/lint/unit/integration validation;
- push-proxy formatting/lint/tests/dependency policy;
- frontend install/build/unit/E2E/dependency policy;
- merge/security gate checks;
- repository integrity guards;
- migration matrix;
- docs/link/hygiene checks;
- Buzz deterministic compatibility tests;
- release-candidate validation.

## 10. Stop conditions

An implementation agent must stop and report rather than "solve around" the contract when:

- current repository state materially contradicts the prompt baseline;
- a required GitHub setting cannot be changed with available permissions;
- fixing a phase requires an unrelated product redesign;
- a security finding has no safe verified remediation;
- a migration test would require modifying applied migration history;
- a Buzz change would require importing upstream internals;
- a requested file move would break an active agent contract without an aligned governance change;
- the work cannot fit into a bounded reviewable PR.

A stopped phase is evidence of a real blocker, not a failure to complete the assignment.
