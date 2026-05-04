# GitHub Protection Settings

This repository uses files to enforce as much as possible, but several settings must be configured manually in the GitHub UI.

## Branch Protection

For the `main` branch, enable:

- [ ] **Require a pull request before merging**
  - [ ] Require approvals: minimum 1
  - [ ] Dismiss stale PR approvals when new commits are pushed
  - [ ] Require review from CODEOWNERS for affected paths
  - Note: Architectural changes (as defined in [GOVERNANCE.md](../GOVERNANCE.md)) require 2 approvals in practice, enforced by CODEOWNERS rules and maintainer discretion

- [ ] **Require status checks to pass before merging**
  - [ ] Require branches to be up to date before merging
  - Required checks (from `ci.yml`):
    - `backend-check`
    - `frontend-check`
    - `push-proxy-check`
    - `docker-validate`
    - `build-release`
  - Required checks (from `security.yml`):
    - `codeql`
    - `cargo-audit-backend`
    - `cargo-audit-push-proxy`
    - `cargo-deny`
    - `npm-audit`
    - `dependency-review`
  - Required checks (from `dco.yml`):
    - `dco-check`
  - **Note:** The `cargo-deny` and `codeql` checks may be slow. They can be marked as "not required" initially while the project stabilizes.

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

- Pattern: `v*`
- Restrict create, update, and delete to maintainers

## CODEOWNERS

[CODEOWNERS](../.github/CODEOWNERS) is already in place. Ensure the file is accurate and reviewers are active.

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
