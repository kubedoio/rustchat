# P002 — Enforce GitHub Governance and Release Provenance

## Mission

Make live GitHub behavior match RustChat's documented governance and make the
release path independently verify the exact commit it publishes.

Primary contracts: **RI-C02, RI-C03, RI-C09, RI-C10**.

Canonical live-work issue: **#258**. Add evidence there rather than creating a
competing governance umbrella.

## Read first

- `GOVERNANCE.md`
- `.github/CODEOWNERS`
- `.governance/risk-tiers.yml`
- `.governance/pr-size-limits.yml`
- `docs/github-protection.md`
- `docs/release-process.md`
- `.github/workflows/release.yml`
- the gate-set evidence from P001

Inspect the current GitHub default-branch protection and repository rulesets.
Do not infer settings from documentation.

## Required work

1. Define/apply the exact default-branch policy:
   - PR required;
   - required reviewers and CODEOWNERS behavior per governance;
   - required **merge-gate** contexts from P001;
   - conversation resolution;
   - no force-push/deletion for ordinary contributors;
   - documented break-glass policy.
2. Correct release protection so release versions are protected **tags**.
   - ordinary contributors must not update/delete them;
   - tag creation authority and any break-glass override must be explicit.
3. Apply live settings if the executing identity has permission.
4. Update `docs/github-protection.md` with verified settings, rule identifiers,
   and a reproducible inspection procedure.
5. Harden release provenance so a tag with matching version files cannot publish
   an arbitrary commit.
   - Define the allowed relationship between the tagged commit and protected
     `main` (for example, exact/reachable protected-main release commit).
   - Verify or rerun the **release-gate** set for the exact tag/candidate.
   - Keep artifact publication coupled to successful release validation.
6. Reconcile any stale check names in protection docs with actual stable contexts.
7. Add repository-side tests/lints for workflow/protection assumptions where
   practical.
8. Activate RI-C02 only after live evidence exists.

## Required negative proofs

Demonstrate safely:

- a failing required merge check blocks merge;
- an ordinary direct/force push is rejected as documented;
- an ordinary contributor cannot overwrite/delete a release tag;
- an arbitrary tag with matching version metadata cannot bypass release gates.

If destructive proof is unsafe, use the strongest non-destructive API/settings
evidence and explicitly state the limitation.

## Permission stop condition

If you cannot change GitHub repository settings, commit only independently
correct code/docs changes and report:

`BLOCKED: administrator action required`

with exact settings/actions required.

Do not mark RI-C02 active without live evidence.
