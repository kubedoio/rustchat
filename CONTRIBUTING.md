# Contributing to RustChat

Thank you for contributing to RustChat.

All contributions must be signed off per the [Developer Certificate of Origin (DCO)](DCO.md). Every commit must include a `Signed-off-by` line:

```bash
git commit -s -m "feat: describe your change"
```

Pull requests will fail the required DCO check if any commit lacks this sign-off.

For the detailed GitHub workflow (fork, branch, PR, reviews), see [docs/contributor-workflow.md](docs/contributor-workflow.md).

## Prerequisites

- Rust `1.92+` (see `backend/Cargo.toml`)
- Node.js `24+` (see `frontend/package.json` engines)
- Docker + Docker Compose

## Development Setup

1. Clone your fork and enter the repository.
2. Copy environment defaults:
   ```bash
   cp .env.example .env
   ```
3. Set required secrets in `.env`:
   - `RUSTCHAT_JWT_SECRET`
   - `RUSTCHAT_ENCRYPTION_KEY`
   - `RUSTCHAT_S3_ACCESS_KEY`
   - `RUSTCHAT_S3_SECRET_KEY`
   - `RUSTFS_ACCESS_KEY`
   - `RUSTFS_SECRET_KEY`
4. Start local dependencies:
   ```bash
   docker compose up -d postgres redis rustfs
   ```
5. Run backend and frontend locally (separate terminals):
   ```bash
   cd backend && cargo run
   cd frontend && npm ci && npm run dev
   ```

Frontend package-management rules:
- use `npm` only in `frontend/`
- use `npm ci` for routine setup
- keep `frontend/package-lock.json` committed
- see [docs/frontend-dependency-policy.md](docs/frontend-dependency-policy.md) before adding or changing dependencies

For full containerized startup, use:

```bash
docker compose up -d --build
```

## Project Structure

| Path | Contents |
|------|----------|
| `backend/` | Rust API server (Axum 0.8 + Tokio) |
| `frontend/` | Svelte 5 + TypeScript SPA |
| `push-proxy/` | Mobile push notification gateway |
| `scripts/` | Smoke and utility scripts |
| `tools/mm-compat/` | Python Mattermost compatibility tooling |
| `docs/` | Project documentation |

## Code Quality Requirements

Before opening a PR, run these checks.

### Backend

Fast checks (run these before every PR):

```bash
cd backend
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
cargo check
cargo test --lib --no-fail-fast -- --nocapture
```

Full integration tests (requires Docker infrastructure):

```bash
# Start test services first: docker compose -f docker-compose.integration.yml up -d
cd backend
cargo test --no-fail-fast -- --nocapture
# Then: docker compose -f docker-compose.integration.yml down -v
```

Run the full integration tests when your change touches auth, permissions, database migrations, API contracts, or compat-sensitive paths.

### Frontend

```bash
cd frontend
npm ci --ignore-scripts
npm run check:dependency-policy
npm run apply:dependency-patches
npm run test:unit
npm run build
```

### Compatibility Smoke Tests

Run these when touching v4 API/websocket/compatibility-sensitive behavior:

```bash
./scripts/mm_compat_smoke.sh
./scripts/mm_mobile_smoke.sh
```

## Coding Guidelines

- Keep code and documentation in English.
- Keep functions focused and avoid unrelated refactors in the same PR.
- Add or update tests for bug fixes and behavior changes whenever feasible.
- Preserve API and websocket contract behavior unless the change explicitly targets it.

### Rust (Backend)

- **Formatting:** `cargo fmt` — enforced in CI
- **Linting:** `cargo clippy -- -D warnings` — enforced in CI
- **Naming:** Follow Rust RFC 430 (`snake_case` for fns/vars, `CamelCase` for types)
- **Error handling:** Use `thiserror` for error types, `Result<T, AppError>` pattern
- **Async:** Tokio runtime, prefer `async fn` for IO-bound work

### TypeScript/Svelte (Frontend)

- **Formatting:** Prettier with 2-space indent
- **Linting:** ESLint with TypeScript recommended rules
- **Naming:** `camelCase` for functions/variables, `PascalCase` for components/types
- **Components:** Svelte runes with `<script lang="ts">`
- **State:** Svelte stores in `svelte/stores/`

### General

- Prefer explicit over implicit
- Add comments only when logic is non-obvious
- Write tests that fail before implementing fixes

## Command UX Standard (Permanent)

- Primary command invocation is `Ctrl/Cmd+K` on desktop.
- Mobile/typed equivalent is `^k`.
- Do not implement `/` as the primary command trigger in the UI.
- New command features must integrate with the command menu flow, not slash-triggered UX.

## Pull Request Process

1. Create a branch from `main` (for example: `feature/my-change` or `fix/my-change`).
2. Make focused changes with clear commits.
3. Run the required checks listed above.
4. Open a PR with:
   - concise summary
   - verification steps/commands run
   - compatibility impact (if any)
   - dependency rationale if `frontend/package.json` or `frontend/package-lock.json` changed
5. Address review feedback and keep history clean.

All PRs must pass the required CI checks (see [Required Status Checks](#required-status-checks)).

### Required Status Checks

All PRs must pass:

- **CI** — Rust formatting, clippy, unit tests; frontend install, policy check, tests, build; Docker validation
- **Security** — CodeQL, cargo audit, cargo deny, npm audit, dependency review
- **DCO** — Developer Certificate of Origin sign-off verification
- **MM-Mobile Compatibility** — Mattermost API contract analysis

**Integration tests** (full backend test suite with live DB/Redis/S3) run automatically on every push to `main` and on the nightly schedule. They are not required on PRs to keep the feedback loop fast, but contributors should run them locally before merging compatibility-sensitive or auth-related changes.

## Commit Messages

Use Conventional Commit style:

```text
feat: add user registration endpoint
fix: correct JWT expiry calculation
docs: update API documentation
test: add channel permission tests
refactor: simplify message rendering
```

## Compatibility-Sensitive Changes

If your change affects API v4 contracts, mobile/desktop client compatibility, websocket events, or calls behavior:

1. Analyze upstream behavior first in `../mattermost` and `../mattermost-mobile`.
2. Document your findings in the PR description so reviewers can verify the analysis.
3. For significant compatibility investigations, maintainers may create an analysis artifact in `docs/internal/compat-analysis/`.

## Security Notes

- Never commit secrets, credentials, or private keys.
- For production hardening guidance, see:
  - [`docs/security-deployment-guide.md`](docs/security-deployment-guide.md)
  - [`docs/security-zero-trust-guide.md`](docs/security-zero-trust-guide.md)

## Good First Issues

Issues labeled [`good-first-issue`](https://github.com/rustchatio/rustchat/labels/good-first-issue) are small, well-defined, and do not touch risky behavior (auth, permissions, payment-like flows). They are the best place to start if you are new to the project.

If no `good-first-issue` issues are currently open, look for `help-wanted`, documentation improvements, or small test coverage improvements.

## Issue Labels

We use the following labels to organize work:

| Label | Meaning |
|-------|---------|
| `good-first-issue` | Small, well-defined tasks for new contributors |
| `help-wanted` | Valid issues where maintainer bandwidth is limited |
| `type/bug` | Something is broken |
| `type/feature` | New functionality request |
| `type/docs` | Documentation improvement |
| `area/backend` | Rust backend code |
| `area/frontend` | Svelte/TypeScript frontend code |
| `area/ci` | CI/CD, workflows, automation |
| `area/docs` | Documentation content |
| `risk/low` | Safe change with limited blast radius |
| `risk/medium` | Moderate risk; review carefully |
| `risk/high` | High risk; requires extra scrutiny and testing |
| `release-blocker` | Must be resolved before the next release |

## Questions

- **General questions**: [GitHub Discussions](https://github.com/rustchatio/rustchat/discussions)
- **Bug reports / feature requests**: Use the [issue templates](https://github.com/rustchatio/rustchat/issues/new/choose)
- **Security issues**: See [SECURITY.md](SECURITY.md) — do not open public issues for vulnerabilities
