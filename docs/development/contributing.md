# Contributor Model

> **Purpose:** Define how human contributors participate in the rustchat project.
> **Last Updated:** 2026-03-24

---

## Getting Started

### Prerequisites

- **Rust** 1.80+ with `cargo`
- **Node.js** 24+ with `npm` for frontend development. Any Node.js version below 24 will fail the frontend install/build because `frontend/package.json` requires `>=24.0.0`; upgrade via the [Node.js downloads](https://nodejs.org/en/download) page or a version manager such as [nvm](https://github.com/nvm-sh/nvm).
- **PostgreSQL** 16+
- **Redis** 7+
- **S3-compatible storage** (RustFS/MinIO for local dev)

### Repository Structure

| Path | Contents |
|------|----------|
| `backend/` | Rust API server (Axum 0.8 + Tokio) |
| `frontend/` | Vue 3.5 + TypeScript SPA |
| `push-proxy/` | Mobile push notification gateway |
| `tools/mm-compat/` | Python Mattermost compatibility tooling |
| `docs/` | Project documentation |
| `.governance/` | LLM operating model policies |

### Quick Start

See [Running Environment](../running_environment.md) for full Docker-based local development setup.

---

## Contribution Rules

1. **All changes go through pull requests** — no direct pushes to `main`.
2. **Every PR must answer:**
   - What problem is solved?
   - Which area is affected?
   - Does behavior change?
   - What tests are added?
3. **Do not mix unrelated domains** in one PR (e.g., auth + UI + CI changes should be separate).
4. **Risky changes require tests** before code movement — see [Testing Model](testing.md).
5. **Architecture changes require an ADR** — add to `docs/adr/` following the template.

---

## Issue Intake

Use the appropriate issue template:

| Template | Use When |
|----------|----------|
| **Bug** | Something is broken |
| **Feature** | New functionality request |
| **Refactor** | Internal improvement with no behavior change |
| **Test Gap** | Missing test coverage |
| **Security Concern** | Potential vulnerability |
| **Documentation Gap** | Missing or outdated docs |

---

## Pull Request Process

1. Create a branch from `main`
2. Implement the smallest safe change
3. Add/update tests (see [Testing Model](testing.md))
4. Update docs if structure changed
5. Fill in the PR template completely
6. Request review from the appropriate code owner (see [Ownership Map](ownership.md))
7. Address review comments
8. Merge only after all checks pass and review is approved

### Required Status Checks

All PRs must pass:

- `backend-ci` — Rust tests, clippy, formatting
- `frontend-ci` — TypeScript/Vue tests, linting
- `MM-Mobile Compatibility` — Mattermost API contract tests

---

## Good First Issues

Issues marked `good-first-issue` meet these criteria:

- Scope is small and well-defined
- Acceptance criteria are clear
- An owner is available for review
- Relevant files are known
- No risky behavior is involved

Look for these labels in the issue tracker.

---

## Coding Conventions

### Rust (Backend)

- **Formatting:** `cargo fmt` — enforced in CI
- **Linting:** `cargo clippy -- -D warnings` — enforced in CI
- **Naming:** Follow Rust RFC 430 (`snake_case` for fns/vars, `CamelCase` for types)
- **Error handling:** Use `thiserror` for error types, `Result<T, AppError>` pattern
- **Async:** Tokio runtime, prefer `async fn` for IO-bound work

### TypeScript/Vue (Frontend)

- **Formatting:** Prettier with 2-space indent
- **Linting:** ESLint with Vue 3 recommended rules
- **Naming:** `camelCase` for functions/variables, `PascalCase` for components/types
- **Components:** Composition API with `<script setup>`
- **State:** Pinia stores in `features/[domain]/stores/`

### General

- Keep functions small and focused
- Prefer explicit over implicit
- Add comments only when logic is non-obvious
- Write tests that fail before implementing fixes

---

## Testing Instructions

### Backend Tests

```bash
cd backend
# Run all tests (requires Postgres, Redis, S3 running)
cargo test

# Run specific test
cargo test test_name -- --exact

# Run integration tests only
cargo test --test integration

# Run contract validation tests
cargo test contract
```

### Frontend Tests

```bash
cd frontend
# Unit tests
npm run test

# E2E tests (requires backend running)
npx playwright test

# Update E2E snapshots
npx playwright test --update-snapshots
```

### Compatibility Tests

```bash
cd backend
# Validate Mattermost API contract
cargo test --test mattermost_contract_tests
```

### Pre-commit Checklist

```bash
# Backend
cd backend && cargo fmt --check && cargo clippy -- -D warnings && cargo test

# Frontend
cd frontend && npm run lint && npm run type-check
```

See [Testing Model](testing.md) for complete testing strategy.

---

## Where to Get Help

- **Architecture questions:** See [Architecture Overview](../architecture/overview.md)
- **Agent/LLM workflow:** See [Agent Operating Model](agent-model.md)
- **Mattermost compatibility:** See [Compatibility Scope](../compatibility-scope.md)
- **Code ownership:** See [Ownership Map](ownership.md)
