# P005 — Canonical Documentation and Repository Hygiene

## Mission

Make repository navigation and documentation trustworthy for humans and coding
agents, eliminate active duplication, and stop roadmap/current-state drift from
reappearing.

Primary contracts: **RI-C06, RI-C09, RI-C10**.

## Inventory first

Classify current documentation by purpose:

- current user/admin/operator/developer documentation;
- architecture decisions;
- active implementation plans;
- historical/completed implementation material;
- audits;
- agent/contributor policy;
- generated/build content.

Inspect at least:

- repository-root Markdown files;
- `docs/internal/**`;
- `docs/audits/**`;
- `docs/plans/**`;
- `docs/decision-notes/**`;
- `docs/decisions/**`;
- `docs/roadmap/**`;
- `docs/archive/**`;
- `previous-analyses/**`;
- `AGENTS.md` and compatibility instruction files.

Use Git blob hashes or content hashing to identify exact duplicates. Do not
delete similarly named documents merely because they look redundant.

## Required work

1. Correct current roadmap/current-state drift.
2. Keep the canonical categories defined by ADR-006.
3. Move completed historical plans/reviews out of active-looking locations when
   their status is unambiguous.
4. Remove one side of exact duplicate documents and repair all references.
5. Move root-level historical review material into the appropriate docs history
   location.
6. Keep `AGENTS.md` canonical.
   - If `CLAUDE.md` or another harness-specific file contains independent
     policy, replace it with a minimal compatibility pointer or generated sync
     mechanism only after verifying no unique required instruction is lost.
7. Add deterministic repository-hygiene checks for:
   - broken internal documentation links;
   - exact duplicates across active/history categories;
   - forbidden new parallel decision/plan categories;
   - tracked generated/build/cache artifacts where patterns are reliable.
8. Add a PR-template or equivalent contributor obligation to update roadmap,
   current-state, ADR, and compatibility docs when a change invalidates them.

## Scope discipline

Do not rewrite documentation prose for style across the whole repository.
Do not reorganize current audience-facing docs without a concrete navigation
problem.

This phase is about source-of-truth clarity, not aesthetic churn.

## Acceptance evidence

- before/after documentation map;
- duplicate report and exact files removed/moved;
- link checker/hygiene checker tests;
- all references to moved documents repaired;
- `ROADMAP.md` and `docs/repo-current-state.md` agree with current code;
- no substantive agent policy lost during instruction-file consolidation.
