# Repository Integrity Hardening Program

This directory contains execution prompts for
[ADR-006](../../adr/ADR-006-repository-integrity-and-maintainability.md) and the
[implementation spec](../2026-09-26-repository-integrity-hardening-spec.md).

## Master implementation goal

> Make RustChat's repository state trustworthy enough that maintainers can use
> protected merge checks, post-merge health, release gates, GitHub protections,
> architecture boundaries, migration evidence, and canonical documentation as
> consistent sources of truth — while reducing maintainability debt incrementally
> and without a broad rewrite or expansion of product scope.

## Existing work alignment

This program does not replace existing issue ownership:

- #258 owns live GitHub protection/required-check remediation.
- #259 owns the v0.5.1 release.
- #88 and #89 remain product release blockers unless fixed independently.

The program should add implementation evidence to those work items rather than
creating competing umbrella issues.

## Execution authority

These are maintainer/architect campaign prompts. They do not override
`.governance/agent-contracts.yml`.

A bounded coding agent may change only paths already permitted by its contract.
Governance/workflow/settings work requires maintainer-supervised execution and
the existing human-review policy.

## Execution order

Run the prompts in order unless current repository evidence proves a later phase
is independently safe:

1. `P001-green-main-and-security.md`
2. `P002-governance-and-release-gates.md`
3. `P003-persistence-boundary.md`
4. `P004-module-decomposition.md`
5. `P005-docs-and-repo-hygiene.md`
6. `P006-migration-and-recovery-matrix.md`
7. `P007-buzz-compatibility-contract.md`
8. `P008-final-convergence.md`

Each prompt is intentionally narrower than the overall program. Do not combine
them into a single large implementation PR.

## Common rules for every implementation agent

Before editing:

1. read `AGENTS.md`;
2. read ADR-006, the implementation spec, and `CONTRACTS.md`;
3. inspect current `main` and relevant open issues/PRs;
4. revalidate the prompt's baseline facts;
5. identify the smallest affected runtime/repository scope;
6. identify the tests/evidence that will prove completion;
7. confirm the executing agent/session is authorized for every path it intends
   to edit.

During implementation:

- preserve behavior unless the prompt explicitly changes a contract;
- prefer mechanical enforcement over prose-only intent;
- do not weaken a failing check to achieve green CI;
- do not rewrite applied migration history;
- do not import Buzz internals;
- do not perform opportunistic unrelated refactors;
- keep exceptions explicit under RI-C10;
- do not activate a contract until its evidence exists.

At completion report:

- exact base and head SHA;
- files changed;
- contracts addressed and lifecycle changes;
- merge/promotion/release gates affected;
- tests/checks run and results;
- live GitHub evidence where required;
- remaining blockers;
- whether the phase is PASS, PARTIAL, or BLOCKED.

"Implemented" without evidence is not completion.
