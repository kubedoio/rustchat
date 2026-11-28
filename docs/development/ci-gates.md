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
| `CI Complete` | `ci.yml` (aggregate) | backend/frontend/push-proxy build, lint, unit, e2e, docker validate, release build, migration matrix (RI-C07), backup/restore evidence (P006 recovery) |
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
| `Backend Integration Tests` | post-merge integration health (nightly, on `main`); also runs on PRs touching `backend/**` (advisory for merge, M5) |

Enforcement: `.github/workflows/promote.yml` runs `scripts/promotion-gate.sh`
against the exact commit and refuses to advance any moving alias unless every
promotion-required check is completed and green. SHA-scoped diagnostic images
produced by `docker-publish.yml` on push are **not** promoted aliases.

Promotions also serialize through a workflow-level concurrency group, and a
`workflow_run` promotion whose target is no longer the head of `main` exits
successfully without moving any alias (the newer commit is responsible for the
aliases); explicit `workflow_dispatch` backfills are exempt from the tip check
but publish only the immutable `nightly-<sha>` tag, never the shared `main` /
`nightly` aliases. The release/promotion alias policy is guarded statically by
`scripts/check-alias-policy.sh`.

### Workflow lint (PR level)

Every pull request runs the `Workflow Lint` job in `ci.yml`, which is part of
the required `CI Complete` check. It runs three guards against the workflow
sources:

| Step | Guard |
|---|---|
| `actionlint -no-color` | GitHub Actions syntax/semantics (`go install ...@v1.7.12`, immutable via the Go module proxy/sumdb) |
| `scripts/check-alias-policy.sh` | allowlisted `release.yml` / `promote.yml` tag sets plus a repo-wide scan for ungated or dynamic moving release aliases in `tags:` / `flavor:` inputs |
| `scripts/test-alias-policy.sh` | positive/negative regression cases for the policy checker itself |

The Release workflow's `validate` job re-runs `scripts/check-alias-policy.sh`
against the exact tagged commit, so a bypass that lands through a direct push
to a tag still fails the release gate.

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
| alias policy (`scripts/check-alias-policy.sh`) | prereleases cannot move `X.Y`/`latest`; promotions serialize |
| migration upgrade matrix (RI-C07, P006) | empty→HEAD and latest-stable→HEAD readiness |
| Buzz deterministic compatibility (RI-C08, P007) | **Planned — not yet enforced in the release gate.** No Buzz compatibility job exists in `release.yml` or `ci.yml` at this revision. |

Migration/recovery evidence (RI-C07, P006) is continuously proven by jobs in
`ci.yml` that feed the required `CI Complete` check. The migration matrix
(RI-C07) is now a required CI job: `Migration Matrix` in `ci.yml` runs
`scripts/migration-matrix.sh` on every
change to `backend/migrations/**`, proving empty→HEAD and
latest-published-stable→HEAD convergence with repository-critical column
readiness checks.

Backup/restore recovery evidence (P006) is likewise continuously proven: the
`Backup Restore` job in `ci.yml` runs `scripts/backup-restore-check.sh` on
every change to `backend/migrations/**`, the script, or the CI workflow. It
applies all
migrations, seeds representative data, takes a custom-format `pg_dump`,
restores it into a fresh database, and asserts that schema objects, sqlx
migration bookkeeping, row counts, content checksums, and referential
integrity are identical after the round trip — the same procedure documented
for operators in `docs/operations/runbook.md`.

Buzz deterministic compatibility (RI-C08, P007) is **planned but not yet
enforced**: at this revision neither `release.yml` nor `ci.yml` contains a Buzz
compatibility job, so the release gate above can pass without one. Activating
RI-C08 requires a future workflow change plus matching updates to this document
and `docs/github-protection.md` under the change procedure below.

## Verification

```bash
# Deterministic negative tests for failure propagation (RI-C01/RI-C03):
scripts/test-gate-propagation.sh

# Database-free migration-matrix diagnostic regression test (exit-code contract):
scripts/test-migration-matrix-diagnostics.sh
```

This harness runs in CI as part of the `Security Regression Guards` job on
every run, so a regression in the aggregate or promotion-gate logic fails the
`CI Complete` required check.

The same job runs `scripts/test-migration-matrix-diagnostics.sh`, a
database-free regression test for the migration matrix's schema-diff
diagnostics. A fake `pg_dump` serving fixtures proves that schema differences
exit 1 with a bounded unified diff, `pg_dump` failures are reported explicitly,
PostgreSQL 18 `\restrict`/`transaction_timeout` dump noise is normalized, and
the old `diff | head` pipeline would surface its raw SIGPIPE/pipe status
(141/2) rather than being masked as a generic `fail()` exit 1. Static checks
cover both `scripts/migration-matrix.sh` and `scripts/lib/schema-diff.sh`, and
the `migrations` path filter runs the DB-backed `Migration Matrix` job when
either file changes.

## Change procedure

Any change to a gate set must update this document, the corresponding workflow,
and `docs/github-protection.md` in the same PR (RI-C06).
