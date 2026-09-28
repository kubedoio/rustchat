# GitHub Protection Settings

This repository uses files to enforce as much as possible, but several settings
must be configured manually in the GitHub UI (or via the API).

## Verified Settings

> **⚠ 2026-09-28 re-verification FAILED — issue #258 remains open.**
> Running `scripts/verify-protection.sh` on 2026-09-28 showed that live
> `main` does **not** currently enforce the merge gate:
>
> - `required_status_checks` is **empty** — `CI Complete`, `Security Complete`,
>   and `dco-check` are *not* required contexts.
> - `required_pull_request_reviews` is **null** — no approving-review or
>   CODEOWNERS requirement is enforced.
>
> What *is* live and verified on 2026-09-28: `enforce_admins` on; force pushes
> and deletion blocked; conversation resolution required; the release-tag
> ruleset (below). The configuration below is therefore the **intended**
> state, not the current state, until a maintainer applies the required
> checks and review policy on `main` (tracked in #258).

The following reflects the **verified, live** configuration as last fully
verified — **2026-09-27**; see the re-verification warning above for the
current delta (RI-C02). Reproduce with:

```bash
gh api repos/kubedoio/rustchat/branches/main/protection
gh api repos/kubedoio/rustchat/rulesets
```

### `main` branch protection (verified 2026-09-27; merge gate found missing 2026-09-28)

- **Require a pull request before merging** — 1 approving review, CODEOWNERS review for affected paths, dismiss stale approvals on new commits.
- **Require status checks** — required contexts `CI Complete`, `Security Complete`, `dco-check`; require branches to be up to date (`strict`).
- **Require conversation resolution** — enabled.
- **No bypass** — `enforce_admins` on; force push and deletion blocked for ordinary contributors.

### Release tags (verified 2026-09-28)

The repository ruleset **"Protect release tags"** (active, id 14126218) targets
**`refs/tags/v*`** and blocks creation, update, deletion, and non-fast-forward
for ordinary contributors. Release versions are protected as **tags**, not
version-looking branches.

## Branch Protection

For the `main` branch, enable:

- [ ] **Require a pull request before merging**
  - [ ] Require approvals: minimum 1
  - [ ] Dismiss stale PR approvals when new commits are pushed
  - [ ] Require review from CODEOWNERS for affected paths
  - Note: Architectural changes (as defined in `GOVERNANCE.md`) require 2 approvals in practice, enforced by CODEOWNERS rules and maintainer discretion

- [ ] **Require status checks to pass before merging**
  - [ ] Require branches to be up to date before merging
  - Required checks are the stable **merge gate** set defined in
    [`docs/development/ci-gates.md`](development/ci-gates.md):
    - `CI Complete` (from `ci.yml`)
    - `Security Complete` (from `security.yml`)
    - `dco-check` (from `dco.yml`)
  - **Note:** These are per-workflow aggregate contexts, not individual jobs.
    Individual CI/Security jobs may be conditional/skipped; the aggregate
    propagates any required failure (RI-C01).

- [ ] **Require conversation resolution before merging**

- [ ] **Do not allow bypassing the above settings** — admins must not be able to push directly to `main` or merge without checks

## Rulesets (Recommended Alternative to Branch Protection)

If using GitHub rulesets instead of classic branch protection:

- Target branch: `main`
- Restrict creations, updates, and deletions
- Require pull requests with CODEOWNERS review
- Require status checks (list above)
- Block force pushes

## Protected Tags

Protect version tags to prevent accidental deletion or overwrite:

- Pattern: `v*` — **verified** via the "Protect release tags" ruleset targeting `refs/tags/v*` (create/update/delete/non-fast-forward blocked for ordinary contributors)
- Restrict create, update, and delete to maintainers

## CODEOWNERS

`CODEOWNERS` is already in place. Ensure the file is accurate and reviewers are active.

## DCO Requirement

The `.github/workflows/dco.yml` workflow enforces signed-off commits. In branch protection, mark the `DCO` check as **required**. Without this, unsigned commits can be merged even if the DCO workflow fails.

## Secret Scanning and Push Protection

Enable in the repository settings:

- [ ] **Secret scanning** — Detects accidentally committed secrets
- [ ] **Push protection** — Blocks pushes that contain secrets

These cannot be enabled via repository files and must be turned on in the GitHub UI under **Settings > Security > Code security and analysis**.

## Dependabot

[Dependabot configuration](../.github/dependabot.yml) is in place. Enable in the GitHub UI:

- [ ] **Dependabot alerts** — Notifications for vulnerable dependencies
- [ ] **Dependabot security updates** — Automatic PRs for security fixes

## Package Permissions

For container images published to GHCR:

- [ ] Ensure `GITHUB_TOKEN` has `packages: write` permission in workflows
- [ ] Configure package visibility (public or internal) under **Packages** settings
- [ ] Link packages to this repository

## Security Advisories

- [ ] Enable private vulnerability reporting under **Settings > Security > Reporting**
- [ ] Designate maintainers as security contacts
