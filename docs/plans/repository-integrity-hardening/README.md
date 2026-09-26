# Repository Integrity Hardening Program

This directory contains execution prompts for
[ADR-006](../../adr/ADR-006-repository-integrity-and-maintainability.md) and the
[implementation spec](../2026-09-26-repository-integrity-hardening-spec.md).

## Master implementation goal

> Make RustChat's repository state trustworthy enough that a maintainer can use
> `main`, CI, GitHub protections, release artifacts, architecture boundaries,
> migration evidence, and canonical documentation as consistent sources of
> truth — while reducing maintainability debt incrementally and without a broad
> rewrite or expansion of product scope.

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
6. identify the tests/evidence that will prove completion.

During implementation:

- preserve behavior unless the prompt explicitly changes a contract;
- prefer mechanical enforcement over prose-only intent;
- do not weaken a failing check to achieve green CI;
- do not rewrite applied migration history;
- do not import Buzz internals;
- do not perform opportunistic unrelated refactors;
- keep exceptions explicit under RI-C10.

At completion report:

- exact base and head SHA;
- files changed;
- contracts addressed;
- tests/checks run and results;
- live GitHub evidence where required;
- remaining blockers;
- whether the phase is PASS, PARTIAL, or BLOCKED.

"Implemented" without evidence is not completion.
