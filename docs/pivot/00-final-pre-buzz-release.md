# Final Pre-Buzz Release and Legacy Freeze

## Purpose

Publish the current standalone RustChat implementation one final time, preserve it as an immutable historical baseline, and prevent accidental continuation of the retired architecture.

The current backend manifest version is `0.5.1`. The proposed final standalone release is therefore:

- Tag: `v0.5.1`
- Title: `RustChat 0.5.1 — Final Standalone Release`
- Maintenance branch: `legacy/0.5`

The release owner must verify that no existing `v0.5.1` tag conflicts before executing the release.

## Scope allowed before release

Only the following changes are allowed:

- release-blocking bug fixes
- security fixes
- dependency corrections required for a reproducible build
- version and release metadata corrections
- documentation that accurately describes limitations
- CI or packaging fixes needed to publish the release

Not allowed:

- new product features
- new Mattermost compatibility endpoints
- new agent features
- major frontend changes
- schema redesign
- architectural refactoring unrelated to release correctness

## Release gates

### Source and version

- [ ] Backend, frontend, push proxy, images, and release metadata use one consistent version.
- [ ] Repository URLs and package metadata point to `kubedoio/rustchat`.
- [ ] The release commit SHA is recorded in the release notes.
- [ ] Generated lock files are committed and reproducible.

### Validation

- [ ] Backend formatting, Clippy, unit tests, migration tests, and security gates pass.
- [ ] Frontend formatting, type checking, unit tests, and production build pass.
- [ ] Push proxy tests and release build pass.
- [ ] Docker Compose starts from an empty data directory.
- [ ] A clean installation can create a user, team, channel, message, file upload, thread, and reaction.
- [ ] OIDC login is tested with Keycloak.
- [ ] WebSocket reconnect and resync behavior is tested.
- [ ] Backup and restore smoke test passes.
- [ ] Known production limitations are listed in the release notes.

### Supply chain and artifacts

- [ ] Images are built from the tagged commit.
- [ ] Image digests are recorded.
- [ ] SBOMs are produced when supported by the existing pipeline.
- [ ] License and notice files are present in artifacts.
- [ ] Release artifacts are retained independently from mutable `latest` tags.

## Release procedure

1. Create `release/0.5.1` from the approved release commit.
2. Apply release-only fixes through reviewed pull requests.
3. Run the complete release gate.
4. Merge the release branch.
5. Create signed tag `v0.5.1` on the exact approved commit.
6. Publish immutable artifacts and release notes.
7. Create `legacy/0.5` from the same tagged commit.
8. Protect `legacy/0.5` against force pushes and direct changes.
9. Update the default branch documentation to announce the Buzz pivot only after the final release exists.
10. Record artifact digests and the final commit in this document or the release record.

## Legacy maintenance policy

After the final release:

- `legacy/0.5` receives critical security fixes only.
- Every legacy fix requires a security or data-loss justification.
- No new features are accepted.
- No compatibility expansion is accepted.
- Legacy fixes must not be automatically copied into the Buzz-based architecture.
- The legacy line has no promise of long-term support until a formal support policy exists.

## Required release-note language

The release notes must clearly state:

> RustChat 0.5.1 is the final release of the original standalone RustChat architecture, including the Rust/Axum backend, Vue client, and Mattermost compatibility layer. Future RustChat development will use Buzz as the upstream collaboration core and will focus on enterprise identity, sovereign deployment, RustShare integration, lifecycle management, and operational support. This release remains available as an historical and evaluation baseline; it is not being presented as a mature enterprise product.

## Rollback

If the final release gate fails:

- do not tag the release
- do not create the legacy branch from an unverified commit
- fix only the failing release requirement
- rerun the complete gate

The Buzz pivot may proceed in a separate branch, but the project must not claim the legacy release is frozen until the signed tag and artifacts exist.
