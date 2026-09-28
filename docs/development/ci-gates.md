# CI Gates: Merge, Promotion, Release

Canonical definition of RustChat's three distinct repository-health states
(ADR-006 §1, RI-C01). This document is the source of truth for the required
check sets; `.github/workflows/*.yml` and `docs/github-protection.md` must match
it.

These three states must not be collapsed into one misleading "green" job.

## Why three states

1. **Mergeable** — the pull request satisfies the protected merge-gate set.
2. **Promotable main** — the merged commit has also satisfied the required
   post-merge health checks used for moving development aliases/artifacts.
3. **Releasable** — the exact release candidate satisfies the release-gate set,
   including release-only migration/recovery evidence.

A post-merge integration/recovery failure does not retroactively unmerge a PR;
it marks `main` **degraded / not promotable** and blocks moving aliases and
releases until repaired.

## Merge gate

Required before a PR may merge into `main`. Enforced by branch protection on
`main` (required reviews, CODEOWNERS, conversation resolution) plus these
stable required status-check contexts:

| Context | Workflow | Proven property |
|---|---|---|
| `CI Complete` | `ci.yml` (aggregate) | backend/frontend/push-proxy build, lint, unit, e2e, docker validate, release build, migration matrix (RI-C07) |
| `Security Complete` | `security.yml` (aggregate) | CodeQL, cargo-audit, cargo-deny, npm-audit (high), dependency-review |
| `dco-check` | `dco.yml` | every commit signed off per DCO |

## Promotion gate

Required before a moving development alias (`main`, `nightly`, `nightly-*`) or
a moving artifact may represent a merged `main` commit as **promoted/healthy**.

| Context | Reason |
|---|---|
| `CI Complete` | build/test integrity |
| `Security Complete` | dependency/security integrity |
| `dco-check` | provenance of the commit |
| `Backend Integration Tests` | post-merge integration health (nightly, on `main`) |

Enforcement: `.github/workflows/promote.yml` runs `scripts/promotion-gate.sh`
against the exact commit and refuses to advance any moving alias unless every
promotion-required check is completed and green. SHA-scoped diagnostic images
produced by `docker-publish.yml` on push are **not** promoted aliases.

## Release gate

Required before a stable or release-candidate artifact is published from the
exact candidate commit. Enforced by `.github/workflows/release.yml`:

| Context / evidence | Purpose |
|---|---|
| `CI Complete` | build/test integrity of the exact commit |
| `Security Complete` | dependency/security integrity of the exact commit |
| `dco-check` | provenance |
| `Backend Integration Tests` | post-merge integration health |
| version/changelog validation | VERSION/Cargo.toml/package.json/CHANGELOG agreement |
| migration upgrade matrix (RI-C07, P006) | empty→HEAD and latest-stable→HEAD readiness |
| Buzz deterministic compatibility (RI-C08, P007) | external-protocol compatibility record |

Release-only checks (migration matrix, Buzz compatibility, backup/restore) are
activated by their phases. The migration matrix (RI-C07) is now a required CI
job: `Migration Matrix` in `ci.yml` runs `scripts/migration-matrix.sh` on every
change to `backend/migrations/**`, proving empty→HEAD and
latest-published-stable→HEAD convergence with repository-critical column
readiness checks.

## Verification

```bash
# Deterministic negative tests for failure propagation (RI-C01/RI-C03):
scripts/test-gate-propagation.sh
```

## Change procedure

Any change to a gate set must update this document, the corresponding workflow,
and `docs/github-protection.md` in the same PR (RI-C06).
