# P002 — Enforce GitHub Governance and Release Gates

## Mission

Make live GitHub behavior match RustChat's documented governance and make the
release path independently verify the exact commit it publishes.

Primary contracts: **RI-C02, RI-C03, RI-C10**.

## Read first

- `GOVERNANCE.md`
- `.github/CODEOWNERS`
- `.governance/risk-tiers.yml`
- `docs/github-protection.md`
- `.github/workflows/release.yml`
- the outputs/evidence from P001

Inspect the current GitHub default-branch protection and repository rulesets.
Do not infer settings from documentation.

## Required work

1. Define the exact required default-branch policy:
   - PR required;
   - required reviewers and CODEOWNERS behavior per governance;
   - required authoritative CI/security/DCO checks;
   - conversation-resolution policy;
   - no force-push/deletion for ordinary contributors;
   - documented bypass policy.
2. Correct release-ref protection so release versions are protected **tags**.
3. Apply live settings if the executing identity has permission.
4. Update `docs/github-protection.md` with verified settings, identifiers, and
   a reproducible inspection command/procedure.
5. Harden `release.yml` so version-file agreement is not sufficient:
   - verify the exact tagged commit satisfies required release gates, or rerun
     equivalent release gates in the tag workflow;
   - keep artifact publication coupled to validation;
   - preserve immutable tag/release semantics.
6. Add repository-side tests/lints for the workflow contract where practical.

## Required negative proofs

Demonstrate, in a safe test PR/repository-settings verification:

- a failing required check blocks merge;
- an ordinary direct/force push is rejected as documented;
- an ordinary contributor cannot overwrite/delete a release tag.

If destructive proof is unsafe, use the strongest non-destructive API/settings
evidence and explicitly state the limitation.

## Permission stop condition

If you cannot change GitHub repository settings, commit only the code/docs
changes that are independently correct and report:

`BLOCKED: administrator action required`

with the exact settings to apply.

Do not mark RI-C02 complete without live evidence.
