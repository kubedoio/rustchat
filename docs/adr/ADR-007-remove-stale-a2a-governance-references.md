# ADR-007: Remove Stale a2a Governance References

**Date:** 2026-09-29  
**Status:** Proposed  
**Acceptance condition:** Maintainer sponsorship of this PR (the architectural-tier review required by `.governance/risk-tiers.yml`)  
**Risk tier:** architectural (touches `.governance/**`; the change itself is a dead-reference removal with no behavioral effect)

## Context

The `backend/src/a2a/` module ("agent-to-agent communication layer") was removed
from the codebase during the pre-0.5 refactoring; it does not exist on `main`
and has not for many releases. The Tier-1 governance-doc sweep
(`docs/plans/2026-09-28-gap-closure-implementation-plan.md`, PR-1) already
removed the stale `a2a` references from `AGENTS.md`, but deliberately left
`.governance/` untouched because both files carrying the references are
protected paths (architectural tier). That deferral is gap-closure item **M7(a)**.

Two stale references remain in the governance source of truth:

1. `.governance/protected-paths.yml` — an elevated-tier protected-path pattern
   for `backend/src/a2a/**` ("Agent-to-agent communication layer"). The pattern
   can never match a changed path again; it is dead policy.
2. `.governance/agent-contracts.yml` — `backend/src/a2a/**` listed as a
   prohibited path for the backend agent ("requires senior review"). Same
   property: the prohibition can never trigger.

Stale references also persist in every file derived from or describing those
governance sources, and in current-state docs that list a2a as an existing
module:

- `.github/CODEOWNERS` — dead owner entry `/backend/src/a2a/`
- `docs/development/agent-model.md` — self-described human-readable summary of
  `agent-contracts.yml`; still shows the a2a prohibition
- `docs/development/ownership.md` — compiled from `protected-paths.yml`; still
  shows the a2a row
- `docs/architecture/backend.md`, `docs/architecture/overview.md` — current-state
  architecture docs listing `a2a/` as an existing module/service
- `docs/admin/ai-agents.md` — admin guidance describing agent-to-agent (a2a)
  communication as an available capability

Dead references are not harmless: agents and contributors consult these files
as the source of truth for review boundaries, ownership, and architecture, and
a listed-but-nonexistent module misleads readers into believing an a2a
subsystem exists and carries special review requirements. Removing the
governance entries without their derived renditions would trade one stale
source of truth for a new one, so this decision covers all of them together.

## Options considered

1. **Keep the entries as historical documentation.** Rejected: governance files
   are live policy, not history — git history already records that a2a existed.
   Historical context belongs in dated docs (audits, plans, archives), which
   still mention a2a and remain untouched.
2. **Remove the two entries (this ADR).** The references are dead, their
   removal cannot change any runtime or review behavior (nothing can match a
   path that does not exist), and it aligns the governance source of truth
   with the repository.
3. **Replace with a comment noting the module's removal.** Rejected as noise:
   the ADR itself (linked from the PR) is the durable record of this decision.

## Decision

Remove the stale `backend/src/a2a/**` references from
`.governance/protected-paths.yml`, `.governance/agent-contracts.yml`,
`.github/CODEOWNERS`, and the derived/current-state docs listed above
(`agent-model.md`, `ownership.md`, `backend.md`, `overview.md`,
`ai-agents.md`), and record the removal decision here. No other governance
semantics change. Historical point-in-time records (dated plans, audits,
archives, internal notes) that mention a2a are intentionally left untouched.

## Consequences

- The protected-path, agent-boundary, ownership, and CODEOWNERS inventories
  describe only paths that exist; future governance reviews reason over live
  policy only, and the derived docs agree with their sources.
- Any future re-introduction of an agent-to-agent communication layer would
  need its own governance entry (and likely an ADR) at that time — removal now
  sets no precedent about the concept, only about dead references.
- Historical documents mentioning a2a (audits, archives, dated plans,
  `docs/internal/`) are point-in-time records and are intentionally not
  rewritten. After this change, remaining repo `a2a` hits are Cargo.lock
  checksum hex and those historical records only.
