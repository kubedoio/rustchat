# Testing Model

**Last updated:** 2026-05-30

---

## Overview

rustchat uses several test layers. Backend library tests can run without live infrastructure, while backend integration tests require PostgreSQL, Redis, and S3-compatible storage. The frontend has unit/component tests, E2E tests, and build-time type checking.

| Layer | Location | Scope | Infrastructure required |
|---|---|---|---|
| Backend unit | `backend/src/` | Pure logic, mappers, auth helpers, config, realtime helpers | None |
| Backend integration | `backend/tests/` | HTTP API, services, DB queries | PostgreSQL + Redis + S3 |
| Compat contract | `backend/compat/tests/` | Mattermost v4 response shape validation utilities | Called from integration tests |
| Frontend unit/component | `frontend/src/**/*.test.ts` | Stores, components, API behavior | None |
| Frontend E2E | `frontend/e2e/` | Playwright browser automation | Running full stack |
| Frontend build | `frontend` | TypeScript compilation + Vite build | None |

---

## 1. Backend Unit Tests

**Location:** `backend/src/`
**Framework:** Rust `#[test]` and `tokio::test`

These tests cover pure logic and small async units that do not need the integration stack.

```bash
cd backend
cargo test --lib
```

---

## 2. Backend Integration Tests

**Location:** `backend/tests/`
**Count:** ~170 tests (as of 2026-03-22)
**Framework:** Rust `tokio::test`

These tests start real HTTP servers against a real database and assert on full request/response cycles. They are not mocked.

**Required environment:**
```bash
export RUSTCHAT_TEST_DATABASE_URL=postgres://user:pass@localhost/rustchat_test
export RUSTCHAT_TEST_REDIS_URL=redis://localhost:6379
export RUSTCHAT_TEST_S3_ENDPOINT=http://localhost:9000
export RUSTCHAT_TEST_S3_ACCESS_KEY=testaccesskey
export RUSTCHAT_TEST_S3_SECRET_KEY=testsecretkey
export RUSTCHAT_TEST_S3_BUCKET=rustchat-test
```

**Run all integration tests:**
```bash
cd backend
cargo test --no-fail-fast
```

**Run a single test file:**
```bash
cd backend
cargo test --test channels_test
```

**Run with output:**
```bash
cd backend
cargo test -- --nocapture 2>&1 | head -100
```

**Note:** `cargo test --lib` runs only library unit tests. The integration test suite under `backend/tests/` is separate and requires infrastructure.

---

## 3. Compat Contract Validation

**Location:** `backend/compat/tests/contract_validation.rs`
**Purpose:** Validation utility functions that check Mattermost v4 JSON response shapes

This file contains helper functions (e.g., `validate_user_response`, `validate_channel_response`) that validate response field presence and format. They are **not** runnable Cargo test binaries — they are utility functions intended to be called from integration tests.

**The actual compat CI check is the `compat.yml` workflow**, which runs an OpenAPI diff between rustchat's API spec and the Mattermost v4 spec:

```bash
# compat.yml runs this automatically on PRs
# To run the OpenAPI diff locally, see tools/mm-compat/
```

Contract schemas: `backend/compat/contracts/` — JSON/YAML schema definitions per entity type.

When the compat surface changes, check whether the OpenAPI diff in `compat.yml` still passes. Do not merge compat surface changes without reviewing the compat.yml output. See `docs/compatibility-scope.md` for the full compat surface definition.

---

## 4. Frontend Unit and Component Tests

**Location:** `frontend/src/**/*.test.ts`
**Framework:** Vitest

```bash
cd frontend
npm run test:unit
```

---

## 5. Frontend E2E Tests

**Location:** `frontend/e2e/`
**Framework:** Playwright (snapshot-based)

Playwright tests capture visual snapshots and assert against them. They run against a full running stack.

**Run:**
```bash
cd frontend
npx playwright test
```

**Update snapshots** (when intentional UI changes are made):
```bash
cd frontend
npx playwright test --update-snapshots
```

After updating snapshots, commit the new baseline files in `frontend/e2e/`.

---

## 6. Frontend Build Check

The frontend CI (`ci.yml`) runs `npm run build` as a build-time type check.

```bash
cd frontend
npm run build
```

Expected: exits 0 with no TypeScript errors.

---

## 5. CI Gates

| Workflow | What it runs | Required to merge | Triggers |
|---|---|---|---|
| `ci.yml` | `cargo fmt`, `cargo clippy`, `cargo test --lib`, `cargo build`, frontend install/build/test, Docker validation | Yes | PR + push to `main` |
| `security.yml` | CodeQL, cargo audit, cargo deny, npm audit, dependency review | Yes | PR + push to `main` + weekly |
| `compat.yml` | OpenAPI diff report against Mattermost v4 spec | Informational (not blocking) | PR + push to `main` |
| `dco.yml` | DCO sign-off verification | Yes | PR + push to `main` |
| `integration.yml` | Full backend integration tests (~170 tests) with live DB/Redis/S3 | No (post-merge + nightly) | Push to `main` + nightly schedule + manual |
| `docker-publish.yml` | Multi-arch Docker build + push to GHCR | N/A | Push to `main` and tag push |
| `release.yml` | Changelog generation + GitHub Release + container images | N/A | `v*` tag push only |

**Why integration tests are not required on PRs:** The full integration test suite takes several minutes and requires live infrastructure. PRs run `cargo test --lib` (unit tests) for fast feedback. Integration tests run automatically after merge to `main` and on the nightly schedule to catch regressions without slowing down the contribution flow.

---

## 6. Test Requirements by Risk Tier

Per `.governance/risk-tiers.yml`:

| Tier | Tests required | Notes |
|---|---|---|
| `standard` | Recommended | Typos, UI polish, CI improvements |
| `elevated` | **Mandatory** | Auth, permissions, API behavior, schema, compat paths |
| `architectural` | **Mandatory** | Architecture changes, storage model, protocol changes |

For elevated changes to the compat surface: compat contract tests must pass. For auth changes: backend integration tests covering the auth paths must pass.

---

## 7. Running the Full Test Suite Locally

### Fast checks (no infrastructure needed)

```bash
# Backend unit tests
cd backend && cargo test --lib

# Backend formatting + linting
cd backend && cargo fmt --check && cargo clippy --all-targets --all-features -- -D warnings

# Frontend build check
cd frontend && npm ci --ignore-scripts && npm run apply:dependency-patches && npm run build
```

### Integration tests (requires Docker)

```bash
# 1. Start test infrastructure
docker compose -f docker-compose.integration.yml up -d

# 2. Wait for services (or use your own wait tool)
# PostgreSQL on localhost:55432
# Redis on localhost:56379
# S3 (RustFS) on localhost:59000

# 3. Run backend integration tests
cd backend
export RUSTCHAT_TEST_DATABASE_URL=postgres://rustchat:rustchat@127.0.0.1:55432/rustchat
export RUSTCHAT_TEST_REDIS_URL=redis://127.0.0.1:56379/
export RUSTCHAT_TEST_S3_ENDPOINT=http://127.0.0.1:59000
export RUSTCHAT_TEST_S3_ACCESS_KEY=testaccesskey
export RUSTCHAT_TEST_S3_SECRET_KEY=testsecretkey
export RUSTCHAT_TEST_S3_BUCKET=test-bucket
cargo test --no-fail-fast

# 4. Clean up
docker compose -f docker-compose.integration.yml down -v
```

### E2E tests (requires full stack running)

```bash
# Start the full application first:
#   docker compose up -d --build
# Then run Playwright:
cd frontend
npx playwright test
```

For environment setup details see `docs/development/development.md`.
