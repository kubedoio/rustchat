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

Two stale references remain:

1. `.governance/protected-paths.yml` — an elevated-tier protected-path pattern
   for `backend/src/a2a/**` ("Agent-to-agent communication layer"). The pattern
   can never match a changed path again; it is dead policy.
2. `.governance/agent-contracts.yml` — `backend/src/a2a/**` listed as a
   prohibited path for the backend agent ("requires senior review"). Same
   property: the prohibition can never trigger.

Dead references in governance files are not harmless: agents and contributors
consult these files as the source of truth for review boundaries and path
policy, and a listed-but-nonexistent module misleads readers into believing an
a2a subsystem exists and carries special review requirements. It also inflates
the protected-path inventory that every governance review must reason about.

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

Remove the stale `backend/src/a2a/**` entries from
`.governance/protected-paths.yml` and `.governance/agent-contracts.yml`, and
record the removal decision here. No other governance semantics change.

## Consequences

- The protected-path and agent-boundary inventories describe only paths that
  exist; future governance reviews reason over live policy only.
- Any future re-introduction of an agent-to-agent communication layer would
  need its own governance entry (and likely an ADR) at that time — removal now
  sets no precedent about the concept, only about dead references.
- Historical documents mentioning a2a (audits, archives, dated plans) are
  point-in-time records and are intentionally not rewritten.
