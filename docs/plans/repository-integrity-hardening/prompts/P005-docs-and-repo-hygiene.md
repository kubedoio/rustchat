# P005 — Canonical Documentation and Repository Hygiene

## Mission

Make repository navigation and documentation trustworthy for humans and coding
agents, eliminate competing authority, and stop roadmap/current-state drift from
reappearing.

Primary contracts: **RI-C06, RI-C09, RI-C10**.

## Inventory first

Classify current documentation by purpose:

- current user/admin/operator/developer documentation;
- architecture decisions;
- maintainer-approved active implementation plans;
- analysis/tooling output required by agent contracts;
- historical/completed implementation material;
- audits;
- harness-specific contributor context;
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
- `docs/superpowers/**`;
- `AGENTS.md` and harness-specific instruction files.

Use Git blob hashes or content hashing to identify exact duplicates. Do not
delete similarly named documents merely because they look redundant.

## Required work

1. Correct current roadmap/current-state drift.
2. Enforce the authority model from ADR-006:
   - new ADRs -> `docs/adr/**`;
   - maintainer-approved current plans -> `docs/plans/**`;
   - current behavior -> audience-oriented docs;
   - completed/superseded history -> `docs/archive/**`.
3. Preserve analysis/tooling paths required by active agent contracts:
   - `previous-analyses/**`;
   - `docs/superpowers/**`.
   Do not move/delete them unless `.governance/agent-contracts.yml`,
   AGENTS/docs references, and the tooling workflow are intentionally updated in
   the same architectural change.
4. Legacy `docs/decisions/**` may keep backward-compatible redirect/index
   material; do not add new ADRs there.
5. Remove one side of exact duplicate documents only when authority/history is
   unambiguous; repair every reference.
6. Move root-level historical review material into the proper history area.
7. Keep `AGENTS.md` canonical.
   - A harness-specific file such as `CLAUDE.md` may contain bootstrap hints,
     but must link to AGENTS.md and must not define contradictory policy.
8. Add deterministic repository-hygiene checks for:
   - broken internal documentation links;
   - exact duplicates with competing active authority;
   - new ADRs outside `docs/adr/**`;
   - selected tracked generated/build/cache artifacts where detection is reliable.
9. Add/maintain a PR-template obligation to consider updates to:
   - ROADMAP;
   - repo-current-state;
   - ADRs;
   - compatibility documentation;
   - repository-integrity contracts when applicable.
10. Activate RI-C06 only when checks and authority documentation are in place.

## Scope discipline

Do not rewrite documentation prose for style across the whole repository.
Do not reorganize current audience-facing docs without a concrete navigation
problem.

This phase is about source-of-truth clarity, not aesthetic churn.

## Acceptance evidence

- before/after documentation authority map;
- duplicate report and exact files removed/moved;
- link/hygiene checker tests;
- all references to moved documents repaired;
- `ROADMAP.md` and `docs/repo-current-state.md` agree with current code;
- no active agent analysis workflow broken;
- no substantive policy divergence between AGENTS.md and harness-specific context.
