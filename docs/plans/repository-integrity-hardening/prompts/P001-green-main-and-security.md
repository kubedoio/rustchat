# P001 — Restore Green Main and Security Integrity

## Mission

Make the current RustChat integration branch authoritative again: required
security/dependency failures must be fixed or truthfully dispositioned, and no
misleading aggregate or promoted `main` artifact may hide a required failure.

Primary contracts: **RI-C01, RI-C03, RI-C10**.

## Start by revalidating reality

Do not assume the 2026-09-26 audit findings are still current.

Inspect current `main`:

- latest commit and all check runs;
- `.github/workflows/security.yml`;
- `.github/workflows/ci.yml`;
- `.github/workflows/docker-publish.yml`;
- `.github/workflows/docker-build-push.yml`;
- `.github/workflows/release.yml`;
- dependency manifests/lockfiles for every currently failing job.

Record exactly which checks are red now and why.

## Required work

1. Fix every currently actionable required dependency/security failure.
   - Prefer upstream patched versions.
   - Keep lockfiles deterministic.
   - If an advisory is not applicable, prove that with dependency path and
     runtime exposure; any ignore must be narrow and include a removal condition.
2. Correct misleading check naming.
   - If `P0 Production-Readiness Gates` is only a static regression script,
     rename it to describe what it actually proves.
3. Create a stable authoritative aggregate suitable for branch protection.
   - It must fail if any required CI/security/DCO/integration component fails.
   - Path-conditional jobs must have deliberate skipped semantics.
4. Change `main` image publication so a moving/promoted alias is never
   published from a commit that failed the authoritative gate.
   - SHA-only diagnostic artifacts are acceptable if clearly non-promoted.
5. Add regression tests/scripts that prove failure propagation.

## Hard constraints

- Do not lower audit severity solely to become green.
- Do not add broad permanent advisory ignores.
- Do not remove CodeQL, cargo-audit, cargo-deny, npm audit, DCO, or integration
  coverage to simplify aggregation.
- Do not introduce unrelated dependency upgrades.
- Do not publish a release.

## Evidence required

Provide:

- before/after check matrix;
- dependency path for each fixed advisory;
- test proving a simulated required failure makes the aggregate fail;
- workflow evidence showing promotion is gated;
- complete relevant local/CI validation.

If a finding cannot be safely fixed, stop with **BLOCKED** and explain the exact
dependency/upstream condition. Do not mask it.
