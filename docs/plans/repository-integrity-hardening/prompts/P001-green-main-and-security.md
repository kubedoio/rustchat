# P001 — Restore Check Integrity and Promotion Semantics

## Mission

Make repository health signals truthful without creating one cross-workflow
mega-aggregate.

Primary contracts: **RI-C01, RI-C03, RI-C09, RI-C10**.

## Start by revalidating reality

Do not assume the 2026-09-26 audit findings are still current.

Inspect current `main`:

- latest commit and all check runs;
- `.github/workflows/security.yml`;
- `.github/workflows/ci.yml`;
- `.github/workflows/integration.yml`;
- `.github/workflows/dco.yml`;
- compatibility workflow(s);
- `.github/workflows/docker-publish.yml`;
- `.github/workflows/docker-build-push.yml`;
- `.github/workflows/release.yml`;
- current release/process documentation.

Record exactly which checks are red now, which are PR-blocking today, which are
post-merge/nightly, and which are release-only.

## Required work

1. Fix every currently actionable required dependency/security failure.
   - Prefer upstream patched versions.
   - Keep lockfiles deterministic.
   - If an advisory is not applicable, prove that with dependency path and
     runtime exposure; any ignore must be narrow and include a removal condition.
2. Correct misleading check naming.
   - If `P0 Production-Readiness Gates` is only a static regression script,
     rename it to describe what it actually proves.
3. Define three explicit gate sets:
   - **merge** — protected PR checks/reviews;
   - **promotion** — post-merge evidence required before moving development
     aliases/artifacts;
   - **release** — exact-candidate release evidence.
4. Prefer stable **per-workflow aggregate checks** where conditional jobs make
   branch protection difficult.
   - Do not force DCO, CI, security, compatibility, and post-merge integration
     into one workflow solely to create one context.
5. Preserve the current intentional distinction that expensive full integration
   may be post-merge/nightly unless maintainers explicitly decide otherwise.
   - A failing required post-merge health check must mark `main` not promotable
     and block release, even if it was not a PR merge gate.
6. Change moving `main` alias publication so it cannot advance when the
   promotion gate is unsatisfied.
   - SHA-scoped diagnostic artifacts are acceptable if clearly non-promoted.
7. Add regression tests/scripts for failure propagation.
8. When evidence is complete, activate RI-C01/RI-C03/RI-C09/RI-C10 only as
   appropriate; contract activation is a reviewed governance change.

## Hard constraints

- Do not lower audit severity solely to become green.
- Do not add broad permanent advisory ignores.
- Do not remove CodeQL, cargo-audit, cargo-deny, npm audit, DCO, compatibility,
  or integration coverage to simplify the model.
- Do not make every PR run expensive release-only recovery tests without a
  measured reason.
- Do not introduce unrelated dependency upgrades.
- Do not publish a release.

## Evidence required

Provide:

- before/after gate matrix with `merge / promotion / release` columns;
- dependency path for each fixed advisory;
- negative test proving a required merge failure blocks the merge state;
- negative test proving a required post-merge failure blocks moving-alias promotion;
- workflow evidence showing release still requires its own candidate validation;
- complete relevant local/CI validation.

If a finding cannot be safely fixed, stop with **BLOCKED** and explain the exact
dependency/upstream condition. Do not mask it.
