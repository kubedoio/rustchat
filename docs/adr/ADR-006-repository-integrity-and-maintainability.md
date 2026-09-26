# ADR-006: Repository Integrity and Incremental Maintainability

**Date:** 2026-09-26  
**Status:** Proposed  
**Risk tier:** architectural

## Context

RustChat has recently completed two important architectural corrections:

1. runtime composition moved out of HTTP routing into a supervised bootstrap layer; and
2. Buzz was retained as an optional external integration rather than becoming a RustChat runtime dependency.

Those decisions reduced structural risk, but the repository audit on 2026-09-26 found a different class of problems:

- the current `main` branch can contain red dependency/security checks while other production-readiness checks remain green;
- repository protection and release-tag enforcement are not yet aligned with the governance documents;
- container publication from `main` is not coupled to the complete security/CI result;
- direct SQL access is still widespread in API handler modules despite an existing repository/service layer;
- several backend modules are already large enough to create review and ownership pressure;
- roadmap and current-state documentation drifted immediately after recently completed work;
- migration history is large enough that upgrade behavior needs explicit release evidence;
- historical plans, audits, internal notes, and archives still overlap;
- Buzz changes quickly, so the integration needs a pinned, externally verifiable compatibility contract.

The repository is not in need of another broad rewrite. The primary risk is now **entropy**: new code can continue to bypass intended boundaries, documentation can become stale, and release/governance signals can disagree with the actual state of the branch.

## Decision

RustChat will adopt repository integrity as a first-class engineering and release property.

### 1. A green authoritative branch is a hard invariant

`main` must not be considered healthy, releasable, or promotable while a required CI, security, dependency, DCO, or integration gate is failing.

A "production-readiness" or aggregate check must not report success while a required security gate is red. Stable and promoted container aliases must only be produced from a commit that has satisfied the release contract.

### 2. GitHub enforcement must match documented governance

The live repository settings are part of the system.

The default branch must enforce the documented review and status-check policy. Release tags must be protected as tags. Any bypass must be narrow, documented, and auditable.

Repository documentation must describe **verified** settings, not desired settings.

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

New or materially expanded business persistence must flow through a repository or service boundary. A baseline guard will prevent the number of handler-layer SQL call sites from increasing.

When an existing handler is substantially modified, extraction should be performed only when it reduces responsibility without creating a large unrelated refactor.

Exceptions require a written decision note explaining why handler-local SQL is the least risky option.

### 5. Control module growth by responsibility, not arbitrary fragmentation

Large modules are not rejected purely by line count, but source growth must remain reviewable.

The repository will:

- record a baseline of oversized production modules;
- prevent unexplained growth of those modules;
- reject new production modules that become monolithic without a documented exception;
- decompose by domain responsibility and invariant, not by mechanical line slicing.

Large-scale "split every file" campaigns are explicitly out of scope.

### 6. Documentation must have clear sources of truth

Canonical documentation classes are:

- architecture decisions: `docs/adr/**`;
- active implementation plans/specifications: `docs/plans/**`;
- product/runtime documentation: the existing audience-oriented `docs/**` hierarchy;
- completed or obsolete implementation history: `docs/archive/**`.

New parallel categories for the same purpose should not be created.

A change that completes or invalidates a roadmap/current-state item must update the corresponding canonical document in the same PR.

Exact duplicate historical documents should not remain in multiple active locations.

### 7. Migration compatibility is a release contract

Every release candidate must prove, at minimum:

- empty database → current schema;
- latest published stable database snapshot → current schema;
- application startup/readiness after migration.

Backup/restore verification is required for release-readiness evidence once the release process declares production support for that path.

Applied migrations must never be rewritten to make history look cleaner.

### 8. External integrations remain externally bounded

ADR-005 remains authoritative for Buzz.

RustChat must not import Buzz source crates, schema, internal database layout, or runtime components. Compatibility must be verified against the documented external protocol and a pinned upstream revision.

Upstream Buzz HEAD is not a merge-time dependency. Periodic compatibility verification may report drift without making unrelated RustChat work non-deterministic.

### 9. Improvements must be delivered as bounded, evidence-producing PRs

The repository-integrity program is split into independent changes:

1. restore green `main` and correct security aggregation;
2. enforce repository/release governance;
3. prevent new handler-layer persistence leakage;
4. decompose only the highest-value oversized modules;
5. consolidate documentation/repository hygiene;
6. add migration and recovery evidence;
7. formalize the Buzz compatibility baseline;
8. perform final convergence and release-readiness review.

Each phase must be reviewable and independently revertible.

## Consequences

### Positive

- Repository state becomes a trustworthy signal rather than an approximation.
- Security failures cannot coexist with a misleading green release signal.
- Architectural debt stops increasing before a costly rewrite becomes necessary.
- Large modules can be reduced where there is demonstrated value.
- LLM and human contributors receive explicit mechanical boundaries.
- Documentation drift becomes detectable.
- Upgrade safety and integration compatibility become evidence-based.
- RustChat remains substantially simpler than Buzz while borrowing Buzz's stronger enforcement discipline.

### Negative

- Some PRs will require small additional repository/service extraction work.
- Release work may block on governance or dependency problems that previously could be bypassed.
- Baseline/guard scripts add maintenance responsibility.
- Repository settings become part of acceptance and therefore require administrator participation.
- A few historical documentation paths may move, requiring link updates.

## Rejected Alternatives

### Broad clean-architecture rewrite

Rejected because it would create large review surfaces and high regression risk without solving the immediate governance and release-integrity failures.

### Adopt Buzz's monorepo/crate structure

Rejected because RustChat's smaller architecture is an advantage. Buzz is an external integration target, not a template for repository scale.

### Enforce a universal small-file limit

Rejected because file length alone does not identify poor responsibility boundaries. Baseline growth control plus targeted decomposition is safer.

### Ignore red security checks until release time

Rejected because a default branch that knowingly carries failed security policy is not an authoritative integration branch.

### Track these rules only in prose

Rejected because the audit showed that documented intent can diverge from live repository behavior. The critical invariants must have machine-checkable contracts and evidence.

## Follow-up

Implementation is defined by:

- `docs/plans/2026-09-26-repository-integrity-hardening-spec.md`
- `.governance/repository-integrity-contracts.yml`
- `docs/plans/repository-integrity-hardening/CONTRACTS.md`
- `docs/plans/repository-integrity-hardening/prompts/**`
