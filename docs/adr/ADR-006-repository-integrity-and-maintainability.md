# ADR-006: Repository Integrity and Incremental Maintainability

**Date:** 2026-09-26  
**Status:** Proposed — becomes Accepted when PR #269 is approved and merged  
**Risk tier:** architectural

## Context

RustChat has recently completed two important architectural corrections:

1. runtime composition moved out of HTTP routing into a supervised bootstrap layer; and
2. Buzz was retained as an optional external integration rather than becoming a RustChat runtime dependency.

Those decisions reduced structural risk, but the repository audit on 2026-09-26 found a different class of problems:

- security/dependency failures can exist independently from other green checks, so a single green job does not necessarily describe repository health;
- repository protection and release-tag enforcement are not yet aligned with the governance documents;
- moving container aliases from `main` are not coupled to the complete promotion/release evidence;
- direct SQL access is still widespread in API handler modules despite an existing repository/service layer;
- several backend modules are already large enough to create review and ownership pressure;
- roadmap and current-state documentation drifted immediately after recently completed work;
- migration history is large enough that upgrade behavior needs explicit release evidence;
- historical plans, audits, analysis output, decision notes, and archives overlap;
- Buzz changes quickly, so the integration needs an explicit externally verified compatibility record.

The repository is not in need of another broad rewrite. The primary risk is now **entropy**: new code can continue to bypass intended boundaries, documentation can become stale, and merge/release signals can disagree with the actual state of the branch.

## Decision

RustChat will adopt repository integrity as a first-class engineering and release property.

### 1. Separate merge integrity, promotable-main health, and release integrity

RustChat has three distinct states:

1. **Mergeable** — the pull request satisfies the protected merge-gate set.
2. **Promotable main** — the merged commit has also satisfied required post-merge health checks used for moving development aliases/artifacts.
3. **Releasable** — the exact release candidate satisfies the release-gate set, including release-only migration/recovery evidence.

These states must not be collapsed into one misleading "green" job.

A stable **required-check set** is authoritative. Per-workflow aggregate jobs may be used to keep branch protection manageable, but unrelated workflows do not need to be coupled into one mega-aggregate.

A post-merge integration/recovery failure may not retroactively make a merged PR "unmerged"; instead it marks `main` degraded and blocks moving promotion aliases and releases until repaired.

### 2. GitHub enforcement must match documented governance

The live repository settings are part of the system.

The default branch must enforce the documented review and status-check policy. Release versions must be protected as **tags**. Any break-glass bypass must be narrow, documented, and auditable.

Repository documentation must describe **verified** settings, not merely desired settings.

### 3. Preserve the current product architecture; do not perform a broad rewrite

RustChat remains a focused application with:

- Axum backend;
- Vue frontend;
- PostgreSQL, Redis, and S3-compatible storage;
- a separate push proxy;
- optional external integrations.

This ADR does not authorize a conversion into a large Rust workspace, microservice decomposition, Buzz-derived architecture, or a generic plugin platform.

### 4. Stop new persistence leakage from API handlers

Existing direct `sqlx::query*` usage in `backend/src/api/**` is treated as baseline technical debt, not as the preferred architecture.

New business persistence should flow through a cohesive repository or service boundary. Mechanical checks are a **tripwire**, not proof of architecture: they must detect new API files containing direct SQL and net growth of normalized direct-SQL call sites, while human review remains responsible for materially expanding existing queries.

When an existing handler is substantially modified, extraction should be performed only when it reduces responsibility without creating a large unrelated refactor.

Exceptions require a written decision note explaining why handler-local SQL is the least risky option.

### 5. Control module growth by responsibility, not arbitrary fragmentation

Large modules are not rejected purely by line count, but source growth must remain reviewable.

The repository will:

- record a baseline of oversized production modules outside the protected governance-policy tree;
- use line-count/growth thresholds as review tripwires rather than architectural goals;
- require explanation for material growth or a newly monolithic production module;
- decompose by domain responsibility and invariant, not by mechanical line slicing.

Large-scale "split every file" campaigns are explicitly out of scope.

### 6. Documentation must have clear sources of truth without breaking analysis workflows

Canonical current decision and plan locations are:

- architecture decisions: `docs/adr/**`;
- maintainer-approved active implementation plans/specifications: `docs/plans/**`;
- product/runtime documentation: the existing audience-oriented `docs/**` hierarchy;
- completed or obsolete implementation history: `docs/archive/**`.

Existing analysis/tooling output locations such as `previous-analyses/**` and `docs/superpowers/**` may continue to exist where current agent contracts require them. They are **not** alternate canonical ADR locations and must not silently become product architecture sources of truth.

Legacy compatibility directories such as `docs/decisions/**` may retain redirect/index material, but new architectural decisions belong in `docs/adr/**`.

A change that completes or invalidates a roadmap/current-state item must update the corresponding canonical document in the same PR.

### 7. Migration compatibility is a release contract

Every release candidate must prove, at minimum:

- empty database → current schema;
- latest published stable schema state → current schema;
- application startup/readiness after migration.

If no immutable database snapshot exists for the latest stable release, the first implementation may construct a deterministic fixture from the **exact published tag and its migration history**, record provenance/checksums, and clearly distinguish that schema-upgrade fixture from a real production backup.

Backup/restore verification becomes release-required once RustChat claims that path as supported production recovery evidence.

Applied migrations must never be rewritten to make history look cleaner.

### 8. External integrations remain externally bounded

ADR-005 remains authoritative for Buzz.

RustChat must not import Buzz source crates, schema, internal database layout, or runtime components. Compatibility must be verified against the documented external protocol.

The compatibility record stores the exact Buzz revision most recently **verified against**; this is evidence, not a RustChat runtime/source dependency pin. RustChat should model the external protocol it needs and may be compatible with more than one upstream commit.

Buzz HEAD is not a merge-time dependency. Periodic compatibility verification may report drift without making unrelated RustChat work non-deterministic.

### 9. Contract rollout is explicit

The contracts introduced with this ADR begin in **planned** state.

Merging this ADR does not pretend the repository already satisfies them. Each implementation phase activates only the contracts for which objective evidence exists. Activation is itself a governance change and requires the review level defined by existing governance policy.

### 10. Improvements must be delivered as bounded, evidence-producing PRs

The repository-integrity program is split into independent changes:

1. restore security/check integrity and promotion semantics;
2. enforce repository/release governance;
3. prevent new handler-layer persistence leakage;
4. control and selectively reduce oversized modules;
5. consolidate documentation/repository hygiene;
6. add migration and recovery evidence;
7. formalize the Buzz compatibility record;
8. perform final convergence and release-readiness evidence review.

Each phase must be reviewable and independently revertible.

## Alignment with existing governance

This ADR does not replace `GOVERNANCE.md`, `.governance/risk-tiers.yml`, CODEOWNERS, DCO, or agent boundary contracts.

Where existing documents conflict, the implementation program must reconcile them explicitly rather than silently choosing one. In particular:

- architectural changes retain the existing two-approval requirement;
- architectural changes are governed by human review rather than standard/elevated hard PR-size limits;
- `docs/adr/**` is the canonical ADR location;
- repository-integrity prompts are maintainer/architect campaigns, not authorization for a bounded backend/frontend agent to edit prohibited governance paths;
- issue #258 remains the canonical live-work item for GitHub protection;
- issue #259 remains the canonical release work item;
- product defects #88 and #89 remain separate release blockers and are not implicitly fixed by this program.

## Consequences

### Positive

- Mergeability, post-merge health, and release readiness become distinct and honest signals.
- Security failures cannot hide behind an unrelated green job.
- Architectural debt stops increasing before a costly rewrite becomes necessary.
- Large modules can be reduced where there is demonstrated value.
- LLM and human contributors receive explicit mechanical boundaries without turning heuristics into architecture.
- Documentation drift becomes detectable.
- Upgrade safety and external-integration compatibility become evidence-based.

### Negative

- Some PRs will require small additional repository/service extraction work.
- Release work may block on governance or dependency problems that previously could be bypassed.
- Baseline/guard scripts add maintenance responsibility.
- Repository settings become part of acceptance and therefore require administrator participation.
- A few historical documentation paths may move, requiring link updates.

## Rejected Alternatives

### One global mega-aggregate check

Rejected. Stable per-workflow aggregate checks plus an authoritative required-check set preserve failure isolation and avoid unnecessary cross-workflow coupling.

### Broad clean-architecture rewrite

Rejected because it would create large review surfaces and high regression risk without solving the immediate governance and release-integrity failures.

### Adopt Buzz's repository structure

Rejected. Buzz is an external integration target, not a template for RustChat repository scale.

### Enforce a universal small-file limit

Rejected because file length alone does not identify poor responsibility boundaries. Growth tripwires plus targeted decomposition are safer.

### Ignore red security checks until release time

Rejected because required merge security failures must block merge, and post-merge release blockers must block promotion/release.

### Treat the compatibility-tested Buzz SHA as a dependency pin

Rejected. The SHA is verification evidence only; RustChat depends on an external protocol contract, not on Buzz source identity.

### Track these rules only in prose

Rejected because the audit showed that documented intent can diverge from live repository behavior. Critical invariants need machine-readable contracts and objective evidence.

## Follow-up

Implementation is defined by:

- `docs/plans/2026-09-26-repository-integrity-hardening-spec.md`
- `.governance/repository-integrity-contracts.yml`
- `docs/plans/repository-integrity-hardening/CONTRACTS.md`
- `docs/plans/repository-integrity-hardening/prompts/**`
