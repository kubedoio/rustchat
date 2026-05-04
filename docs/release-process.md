# Release Process

This document describes how RustChat releases are made, versioned, and published.

## Versioning Philosophy

RustChat follows [Semantic Versioning 2.0.0](https://semver.org/spec/v2.0.0.html):

| Component | Meaning | Example |
|-----------|---------|---------|
| `MAJOR` | Incompatible API or behavior changes | `1.0.0` |
| `MINOR` | New features, backward compatible | `0.4.0` |
| `PATCH` | Bug fixes and security patches | `0.3.6` |

### Pre-1.0 Compatibility

Before `1.0.0`, minor version bumps **may include breaking changes**. Patch versions are safe to upgrade within the same minor line. Always review `CHANGELOG.md` before upgrading.

After `1.0.0`, SemVer guarantees strict backward compatibility within major versions.

## Release Channels

### Stable Releases

Stable releases are created from Git tags matching `v*.*.*` (e.g., `v0.3.6`). They are the recommended choice for production use.

**Container tags published:**

| Tag | Example | Behavior |
|-----|---------|----------|
| `vX.Y.Z` | `v0.3.6` | Immutable — points to exactly this release |
| `vX.Y` | `v0.3` | Rolling minor — receives latest patch in this minor line |
| `latest` | `latest` | Always points to the most recent stable release |

**What happens when a stable tag is pushed:**
1. The `Release` workflow triggers
2. Validation checks run (VERSION file, Cargo.toml, package.json, CHANGELOG.md)
3. GitHub Release is created with changelog content
4. Multi-arch container images (`linux/amd64`, `linux/arm64`) are built and pushed to GHCR

### Release Candidates

Pre-release tags with a suffix trigger release candidate builds:

- Tag format: `v0.4.0-rc.1`, `v0.4.0-rc.2`
- Container images are published with the exact RC tag
- GitHub Release is marked as **pre-release**
- The `latest` tag is **not** updated

RCs are useful for testing significant changes before declaring them stable.

### Nightly Builds

Nightly images are built automatically from the `main` branch.

| Tag | Example | Behavior |
|-----|---------|----------|
| `nightly` | `nightly` | Rolling — overwritten every night |
| `nightly-YYYYMMDD` | `nightly-20260427` | Date-stamped — know exactly when it was built |
| `nightly-SHA` | `nightly-a1b2c3d` | Commit-stamped — know exactly which commit |

**Important:** Nightly images are **not stable**. They are built from the latest `main` commit and may contain incomplete features or regressions. Use them only for testing and feedback.

Nightly builds **do not** create GitHub Releases and **do not** update the `latest` tag.

## Maintainer Release Checklist

Before cutting a release, run the readiness check:

```bash
./scripts/check-release-ready.sh
```

This validates formatting, clippy, version consistency, and working tree cleanliness.

Then follow this checklist:

- [ ] All CI checks pass on `main`
- [ ] `CHANGELOG.md` is updated with release notes
- [ ] `[Unreleased]` section is empty (all items moved to the versioned section)
- [ ] Version bumped in:
  - [ ] `VERSION`
  - [ ] `backend/Cargo.toml`
  - [ ] `frontend/package.json`
- [ ] `scripts/check-release-ready.sh` passes
- [ ] `scripts/release-notes-check.sh` passes
- [ ] No open security advisories blocking the release
- [ ] Container images build locally: `docker compose build`
- [ ] Commit the version bump: `git add -A && git commit -s -m "chore(release): bump version to X.Y.Z"`
- [ ] Tag: `git tag -s vX.Y.Z -m "Release vX.Y.Z"`
- [ ] Push: `git push origin main && git push origin vX.Y.Z`
- [ ] Wait for the `Release` workflow to complete
- [ ] Verify the GitHub Release and container images are published
- [ ] Announce in GitHub Discussions (optional)

## Example: Creating v0.4.0

```bash
# 1. Ensure you're on main and everything is clean
git checkout main
git pull origin main

# 2. Run readiness checks
./scripts/check-release-ready.sh
./scripts/release-notes-check.sh 0.4.0

# 3. Update version in all files
# Edit VERSION, backend/Cargo.toml, frontend/package.json

# 4. Update CHANGELOG.md — move [Unreleased] items to [0.4.0]

# 5. Commit and tag
git add -A
git commit -s -m "chore(release): bump version to 0.4.0"
git tag -s v0.4.0 -m "Release v0.4.0"

# 6. Push
git push origin main
git push origin v0.4.0

# 7. The Release workflow triggers automatically
# Verify at: https://github.com/rustchatio/rustchat/actions
```

## Verifying Published Images

After a release completes, verify the images:

```bash
# List available tags for the backend image
skopeo list-tags docker://ghcr.io/rustchatio/rustchat-backend

# Pull and inspect a specific version
docker pull ghcr.io/rustchatio/rustchat-backend:v0.3.6
docker pull ghcr.io/rustchatio/rustchat-frontend:v0.3.6

# Verify multi-arch support
docker manifest inspect ghcr.io/rustchatio/rustchat-backend:v0.3.6
```

## Rollback Procedure

If a stable release has a critical issue:

1. **Immediate:** Pin your deployments to the previous stable tag (`vX.Y.Z-1`) instead of `latest` or `vX.Y`
2. **Fix:** Open a PR with the fix against `main`
3. **Patch release:** After merge, cut a new patch release (`vX.Y.Z+1`) following the normal checklist
4. **Communication:** Post in GitHub Discussions about the issue and the fix

**Why we don't delete/re-tag:** Docker image tags and GitHub Releases should be treated as immutable. Re-tagging breaks caches and reproducibility. Always issue a patch release instead.

## Changelog Rules

- Follow [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
- Add changes under `[Unreleased]` during development
- Move items to a versioned section at release time
- Categorize as:
  - `Added` — New features
  - `Changed` — Changes to existing functionality
  - `Deprecated` — Soon-to-be-removed features
  - `Removed` — Removed features
  - `Fixed` — Bug fixes
  - `Security` — Vulnerability fixes

## GitHub Settings Required for Releases

The following settings must be enabled in the GitHub UI for releases to work:

1. **Actions permissions:**
   - Settings > Actions > General > Workflow permissions
   - Select "Read and write permissions" (needed for `GITHUB_TOKEN` to create releases)

2. **Packages permissions:**
   - Settings > Packages
   - Ensure packages are visible and linked to the repository
   - The `packages: write` permission in workflows enables GHCR publishing

3. **Protected tags:**
   - Settings > Rules > Tags
   - Pattern: `v*`
   - Restrict create/update/delete to maintainers
   - Prevents accidental or malicious tag manipulation

## Container Image Retention

RustChat publishes images to GitHub Container Registry (GHCR). Over time, untagged or branch-ref images accumulate.

### Recommended Retention Policy

| Image Pattern | Retention | Reason |
|---------------|-----------|--------|
| `v*.*.*` (stable) | Keep forever | Immutable release artifacts |
| `v*.*` (rolling minor) | Keep last 10 | Rolling tags |
| `latest` | Keep forever | Convenience tag |
| `nightly*` | Last 14 days | Testing builds |
| Branch / PR / SHA refs | Last 7 days | CI artifacts |

### How to Clean Up

Repository maintainers can clean up old images manually:

```bash
# List all tags for the backend image
skopeo list-tags docker://ghcr.io/rustchatio/rustchat-backend

# Delete a specific untagged manifest (requires GHCR delete permission)
# Note: GitHub Packages does not support deleting via CLI for multi-arch images easily.
# Use the GitHub web UI: Settings > Packages > rustchat-backend > Manage versions
```

To automate retention, consider enabling GitHub's built-in package retention or a scheduled workflow that prunes old `nightly-*` and branch-ref tags.

### Setting Up Automated Retention

1. Go to **Settings > Packages** in the repository
2. For each package (`rustchat-backend`, `rustchat-frontend`, `rustchat-push-proxy`):
   - Open **Package settings**
   - Set **Package versioning** retention rules if available
3. Alternatively, use a scheduled GitHub Actions workflow with `actions/delete-package-versions` to prune old nightly and branch tags.

---

## Automation Notes

- **Nightly:** Runs automatically at 02:00 UTC daily. Can also be triggered manually with a reason.
- **Stable:** Triggered only by pushing a `v*.*.*` tag. No automatic stable releases.
- **Dependabot** may open PRs for security fixes. These should be merged and released as patch versions promptly.
