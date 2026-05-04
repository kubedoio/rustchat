# SPEC: Open-Source Hardening & Repository Polish

## Scope
Polish the RustChat repository so it presents as a serious, supported open-source product without overclaiming production readiness.

## Non-Destructive Rules
- Inspect before modifying; preserve existing project info
- Improve weak files rather than blindly replacing
- Do not delete existing source code, workflows, or docs
- No fake badges; no secrets; placeholders clearly marked

## Audit Summary

### Already Strong
- `README.md` — Comprehensive, honest about limitations
- `LICENSE` — Apache-2.0, present
- `CHANGELOG.md` — Keep a Changelog format, maintained
- `CODE_OF_CONDUCT.md` — Contributor Covenant, present
- `SECURITY.md` — Basic policy present
- `CONTRIBUTING.md` — Dev setup present, lacks DCO
- `.env.example` — Thorough
- `docker-compose.yml` — Production-oriented with health checks
- `.github/CODEOWNERS` — Detailed ownership
- `.github/pull_request_template.md` — Good template
- `.github/ISSUE_TEMPLATE/{bug,feature,refactor}.yml` — Present
- `.github/workflows/{ci,backend-ci,frontend-ci,release,docker-publish,compat,docs-ci-cd,frontend-dependency-review}.yml` — Present
- `docs/` — Well-structured VitePress docs site with admin, user, dev, architecture, operations, reference sections
- `scripts/` — Docker build, version bump, smoke tests present

### Missing / Weak
- Root: `ROADMAP.md`, `GOVERNANCE.md`, `SUPPORT.md`, `MAINTAINERS.md`, `DCO.md`, `VERSION`
- GitHub: `dco.yml`, `security.yml` (CodeQL + cargo audit), `nightly.yml`, `scorecard.yml`
- GitHub: issue templates for `documentation.md`, `security_hardening.md`
- GitHub: `dependabot.yml` missing cargo, github-actions, docker
- GitHub: PR template missing DCO checkbox
- Docs: `quickstart.md`, `deployment.md`, `development.md`, `architecture.md`, `security-model.md`, `release-process.md`, `github-protection.md`, `contributor-workflow.md`
- Scripts: `smoke-test.sh`, `check-release-ready.sh`, `dev-setup.sh`, `release-notes-check.sh`
- Cargo: no `deny.toml`

## Implementation Plan

### Phase 1: Root Files
1. Create `VERSION` (0.3.5)
2. Create `DCO.md` (Developer Certificate of Origin)
3. Create `GOVERNANCE.md` (lightweight governance model)
4. Create `SUPPORT.md` (support policy and channels)
5. Create `MAINTAINERS.md` (current maintainers)
6. Create `ROADMAP.md` (honest near-term roadmap)
7. Update `CONTRIBUTING.md` — add DCO sign-off section
8. Update `README.md` — add release channels, roadmap link, security reporting prominence
9. Update `SECURITY.md` — expand with supported versions, dependency process

### Phase 2: GitHub Infrastructure
1. Update `.github/pull_request_template.md` — add DCO checkbox
2. Create `.github/ISSUE_TEMPLATE/documentation.md`
3. Create `.github/ISSUE_TEMPLATE/security_hardening.md`
4. Update `.github/dependabot.yml` — add cargo, github-actions, docker
5. Create `.github/workflows/dco.yml`
6. Create `.github/workflows/security.yml` (CodeQL Rust + cargo audit)
7. Create `.github/workflows/nightly.yml` (nightly builds, container tags)
8. Create `.github/workflows/scorecard.yml` (OpenSSF Scorecard)
9. Improve `.github/workflows/release.yml` — extract changelog, generate release notes
10. Improve unified `.github/workflows/ci.yml` — ensure frontend build/lint/test coverage

### Phase 3: Documentation
Create targeted docs that complement the existing VitePress structure:
1. `docs/quickstart.md` — fastest path to running
2. `docs/deployment.md` — evaluation vs production deployment
3. `docs/development.md` — backend + frontend setup, tests, formatting
4. `docs/architecture.md` — high-level summary linking to architecture/
5. `docs/security-model.md` — auth assumptions, dependency process, vuln reporting, supported versions
6. `docs/release-process.md` — nightly, RC, stable, container tags, changelog rules
7. `docs/github-protection.md` — manual settings required (branch protection, rulesets, secret scanning)
8. `docs/contributor-workflow.md` — fork, branch, sign-off, PR flow

### Phase 4: Scripts & Tooling
1. Create `scripts/smoke-test.sh` — unified smoke test wrapper
2. Create `scripts/check-release-ready.sh` — pre-release validation
3. Create `scripts/dev-setup.sh` — one-command dev environment setup
4. Create `scripts/release-notes-check.sh` — validate CHANGELOG before release
5. Create `deny.toml` — minimal cargo-deny config for backend

### Phase 5: Validation
1. Validate all new workflow YAML syntax
2. Run `cargo check` in backend
3. Run `cargo fmt --check` in backend
4. Run frontend `npm run build` if node_modules available
5. Document any build failures honestly

## Verification
- [ ] Newcomer can understand what RustChat is from README
- [ ] Contributor can set up dev env and sign commits from CONTRIBUTING.md
- [ ] Maintainer can cut releases from docs/release-process.md
- [ ] PRs are protected by backend checks, frontend checks, DCO, security checks
- [ ] Manual GitHub settings are documented in docs/github-protection.md
- [ ] Existing code still builds
