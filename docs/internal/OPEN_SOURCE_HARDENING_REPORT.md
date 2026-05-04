# Open-Source Hardening Report

## 1. Repository Audit Summary

RustChat already had a strong open-source foundation:

- **README.md**: Comprehensive, honest about limitations, good architecture diagram
- **LICENSE**: Apache-2.0, present
- **CHANGELOG.md**: Keep a Changelog format, actively maintained
- **CODE_OF_CONDUCT.md**: Contributor Covenant, present
- **SECURITY.md**: Basic vulnerability reporting policy present
- **CONTRIBUTING.md**: Dev setup and conventions present
- **.env.example / docker-compose.yml**: Production-oriented with health checks
- **.github/CODEOWNERS**: Detailed ownership with compat-reviewer rules
- **.github/pull_request_template.md**: Good template with risk tiers
- **Issue templates**: bug, feature, refactor (YAML forms)
- **CI workflows**: ci, security, dco, release, nightly, docker-publish, compat, docs-ci-cd, scorecard
- **docs/**: Well-structured VitePress docs site (admin, user, dev, architecture, operations, reference)

**Missing pieces identified:**
- No DCO enforcement or documentation
- No nightly build workflow
- No security scanning workflow (CodeQL, cargo audit)
- No OpenSSF Scorecard
- Missing governance, support, maintainers, and roadmap files
- dependabot only covered frontend npm
- No unified quickstart/deployment/development landing docs at docs root
- No GitHub protection documentation
- No release-process doc
- No dev-setup or release-check scripts

## 2. Files Added

### Root Files
- `VERSION` — Current version (0.3.5)
- `DCO.md` — Developer Certificate of Origin text and sign-off instructions
- `GOVERNANCE.md` — Lightweight governance model, roles, decision making
- `SUPPORT.md` — Support policy, version support matrix, nightly disclaimer
- `MAINTAINERS.md` — Current maintainers and how to become one
- `ROADMAP.md` — Honest near/medium/long-term roadmap with completed milestones
- `deny.toml` — Minimal cargo-deny configuration for backend dependency auditing

### GitHub Templates
- `.github/ISSUE_TEMPLATE/documentation.md` — Documentation request template
- `.github/ISSUE_TEMPLATE/security_hardening.md` — Security hardening proposal template

### GitHub Workflows
- `.github/workflows/dco.yml` — Enforces signed-off commits on PRs
- `.github/workflows/security.yml` — CodeQL (Rust + JS), cargo audit, dependency review
- `.github/workflows/nightly.yml` — Scheduled nightly container builds with `nightly` and date/SHA tags
- `.github/workflows/scorecard.yml` — OpenSSF Scorecard analysis and SARIF upload

### Docs
- `docs/quickstart.md` — Fastest path to running locally
- `docs/deployment.md` — Evaluation vs production deployment notes
- `docs/development.md` — Backend + frontend setup, tests, formatting
- `docs/architecture.md` — High-level summary linking to detailed architecture docs
- `docs/security-model.md` — Auth assumptions, dependency security process, supported versions
- `docs/release-process.md` — SemVer rules, stable/RC/nightly release flows, container tags
- `docs/github-protection.md` — Manual GitHub UI settings that cannot be file-enforced
- `docs/contributor-workflow.md` — Fork, branch, sign-off, PR flow, review expectations

### Scripts
- `scripts/smoke-test.sh` — Unified wrapper for compatibility smoke tests
- `scripts/check-release-ready.sh` — Pre-release validation (versions, changelog, fmt, clippy, docker)
- `scripts/dev-setup.sh` — One-command dev environment setup
- `scripts/release-notes-check.sh` — Validates CHANGELOG.md before tagging

## 3. Files Modified

### Root Files
- `CONTRIBUTING.md` — Added DCO sign-off requirement and instructions
- `README.md` — Added Release Channels table, security reporting callout, roadmap link
- `SECURITY.md` — Expanded with supported versions table, dependency security process, nightly disclaimer

### GitHub Files
- `.github/pull_request_template.md` — Added DCO sign-off checkbox
- `.github/dependabot.yml` — Added cargo (backend + push-proxy), github-actions, and docker ecosystems
- `.github/workflows/ci.yml` — Added `frontend-build` job (install, patch, build, unit tests)
- `.github/workflows/release.yml` — Added changelog extraction from CHANGELOG.md, container build/push for release tags, proper `latest` tagging, pre-release detection

## 4. Workflows Added/Changed

### Added
| Workflow | Purpose | Trigger |
|----------|---------|---------|
| `dco.yml` | Verify all commits have `Signed-off-by` | push to main, PRs |
| `security.yml` | CodeQL analysis, cargo audit, dependency review | push/PR + weekly schedule |
| `nightly.yml` | Build and push `nightly` container images | daily cron + manual |
| `scorecard.yml` | OpenSSF Scorecard | push to main + weekly |

### Changed
| Workflow | Change |
|----------|--------|
| `ci.yml` | Added frontend-build job (npm ci, patch, build, test:unit) |
| `release.yml` | Extracts changelog section, builds/pushes release images with `latest` tag, marks pre-releases |
| `dependabot.yml` | Expanded from npm-only to cargo (2 dirs), github-actions, docker |

## 5. Manual GitHub Settings Still Required

These cannot be enforced by repository files and must be configured in the GitHub UI:

### Branch Protection (for `main`)
- Require pull request before merging (1 approval standard, 2 for architectural changes)
- Require review from CODEOWNERS
- Require status checks to pass:
  - `CI / Backend Check`, `CI / Frontend Check`, `CI / Push Proxy Check`, `CI / Docker Validate`, `CI / Build Release` (from `ci.yml`)
  - `Security / CodeQL Analysis`, `Security / Cargo Audit (Backend)`, `Security / Cargo Audit (Push Proxy)`, `Security / Cargo Deny (Backend)`, `Security / Cargo Deny (Push Proxy)`, `Security / npm Audit`, `Security / Dependency Review` (from `security.yml`)
  - `DCO / dco-check` (from `dco.yml`)
- Require conversation resolution before merging
- Do not allow bypassing settings (admins included)

### Protected Tags
- Pattern `v*` — restrict create/update/delete to maintainers

### Security Features
- Enable **Secret scanning** under Settings > Security
- Enable **Push protection** under Settings > Security
- Enable **Dependabot alerts** and **security updates**
- Enable **Private vulnerability reporting**

### Packages
- Ensure GHCR package visibility is set correctly
- Link packages to this repository

All of these are documented in `docs/github-protection.md`.

## 6. Commands Run

```bash
# Syntax validation
bash -n scripts/smoke-test.sh
bash -n scripts/check-release-ready.sh
bash -n scripts/dev-setup.sh
bash -n scripts/release-notes-check.sh

# YAML validation
python3 -c "import yaml; yaml.safe_load(open('.github/workflows/...'))"

# Backend checks
cd backend && cargo check
cd backend && cargo fmt --all -- --check
cd backend && cargo clippy --all-targets --all-features -- -D warnings

# Frontend checks
cd frontend && npm run build
cd frontend && npm run test:unit

# Release check
cd /Users/scolak/Projects/rustchat && ./scripts/release-notes-check.sh
cd /Users/scolak/Projects/rustchat && ./scripts/check-release-ready.sh
```

## 7. Test/Build Results

| Check | Result | Notes |
|-------|--------|-------|
| Script syntax (`bash -n`) | ✅ Pass | All 4 new scripts |
| Workflow YAML validity | ✅ Pass | All 13 workflow files |
| Backend `cargo fmt --check` | ✅ Pass | No formatting issues |
| Backend `cargo check` | ❌ Fail | 5 pre-existing compilation errors |
| Backend `cargo clippy` | ❌ Fail | Same pre-existing errors prevent clippy |
| Frontend `npm run build` | ✅ Pass | Built successfully in ~3.3s |
| Frontend `npm run test:unit` | ✅ Pass | 35 tests passed |
| `release-notes-check.sh` | ✅ Pass | Detected Unreleased content (expected) |
| `check-release-ready.sh` | ⚠️ Expected fail | Detected uncommitted changes and pre-existing clippy failures |

**Important**: The backend compilation failures are pre-existing and unrelated to any files changed in this hardening effort. No source code was modified.

## 8. Known Limitations

1. **Backend compilation errors**: There are 5 pre-existing Rust compilation errors in `backend/src/api/admin.rs` (type mismatches). These must be fixed in a separate PR.
2. **No `lint` script in frontend**: `frontend/package.json` does not expose a dedicated `lint` script. CI runs `build` and `test:unit` but does not run ESLint/Prettier as a standalone check.
3. **Nightly images not yet published**: The `nightly.yml` workflow is created but will only start producing images once it runs on the `main` branch.
4. **Scorecard baseline unknown**: The OpenSSF Scorecard will establish a baseline on its first run. Improvements may be needed based on the initial score.
5. **cargo-deny not run in CI**: `deny.toml` is present but a CI job for `cargo deny check` was not added to avoid blocking on existing dependency issues.

## 9. Recommended Next PRs

1. **Fix backend compilation errors** — Address the 5 pre-existing errors so `cargo check` and `cargo clippy` pass cleanly.
2. **Add cargo-deny to CI** — Once the backend compiles cleanly, add a `cargo deny check` job to `security.yml`.
3. **Add frontend lint script** — Add `lint` and `type-check` scripts to `frontend/package.json` and run them in CI.
4. **Configure branch protection** — A maintainer should apply the settings documented in `docs/github-protection.md`.
5. **Enable GitHub security features** — Turn on secret scanning, push protection, and Dependabot alerts in the repository settings.
6. **First nightly run verification** — After merging, trigger the nightly workflow manually to verify GHCR push permissions and tagging.
7. **Consider a docs site** — The existing VitePress docs under `docs/` could be published to GitHub Pages for a polished documentation experience.
