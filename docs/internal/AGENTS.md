# Repository Guidelines

This document defines how AI agents and contributors should work in `rustchat`.

## Project Overview

Rustchat is a self-hosted team collaboration platform built with:
- **Backend**: Rust using Axum + Tokio + SQLx (PostgreSQL)
- **Frontend**: Svelte 5 + TypeScript + Vite
- **Push Proxy**: Rust service for mobile push notifications (FCM/APNS)
- **Compatibility**: Mattermost API v4 compatibility layer for external clients

### Repository Structure

```
rustchat/
├── backend/              # Rust API server (Axum + SQLx)
│   ├── src/
│   │   ├── api/          # HTTP handlers (v1 native + v4 compatibility)
│   │   ├── auth/         # Authentication & JWT
│   │   ├── config/       # Environment configuration
│   │   ├── db/           # Database connection & migrations
│   │   ├── error/        # Error types
│   │   ├── jobs/         # Background job workers
│   │   ├── mattermost_compat/  # MM compatibility utilities
│   │   ├── middleware/   # Axum middleware
│   │   ├── models/       # Data models (User, Channel, Post, etc.)
│   │   ├── realtime/     # WebSocket hub & cluster broadcast
│   │   ├── services/     # Business logic layer
│   │   ├── storage/      # S3 file storage
│   │   └── telemetry/    # Logging & tracing
│   ├── migrations/       # SQLx database migrations
│   └── tests/            # Integration tests
├── frontend/             # Svelte 5 + TypeScript SPA
│   ├── src/
│   │   ├── api/          # API client functions
│   │   ├── components/   # Svelte components
│   │   ├── composables/  # Shared utilities
│   │   ├── core/         # Shared entities, errors, websocket
│   │   ├── features/     # Feature-based modules (auth, calls, messages, etc.)
│   │   ├── router/       # Custom Svelte router
│   │   ├── stores/       # Svelte stores
│   │   ├── types/        # TypeScript type definitions
│   │   └── views/        # Page-level components
│   └── e2e/              # Playwright E2E tests
├── push-proxy/           # Push notification proxy (FCM/APNS)
├── scripts/              # Utility & smoke test scripts
├── tools/                # Compatibility analysis tools
├── docs/                 # Architecture & operational documentation
└── docker/               # Dockerfile definitions
```

## Technology Stack

### Backend
- **Framework**: Axum 0.8 with Tower middleware
- **Runtime**: Tokio async runtime
- **Database**: PostgreSQL 16+ with SQLx (compile-time checked queries)
- **Cache**: Redis 7+
- **Storage**: S3-compatible (RustFS)
- **Search**: Meilisearch (optional, via profile)
- **WebSocket**: Native Axum WebSocket with Redis pub/sub clustering
- **WebRTC**: SFU for voice/video calls (single-node or Redis-backed cluster)
- **Auth**: Argon2 password hashing, JWT tokens
- **Metrics**: Prometheus

### Frontend
- **Framework**: Svelte 5+ (runes)
- **Language**: TypeScript 5.9+
- **Build Tool**: Vite 7+
- **State**: Svelte stores
- **Styling**: Tailwind CSS 4+
- **Icons**: Lucide Svelte
- **Utilities**: Svelte utilities (svelte-use, etc.), date-fns, axios
- **E2E Testing**: Playwright

### Push Proxy
- **Framework**: Axum 0.7
- **Push Services**: Firebase Cloud Messaging (Android), APNS (iOS VoIP)
- **Auth**: JWT-based APNS authentication

## Build, Test, and Verification Commands

### Backend

```bash
cd backend

# Format code
cargo fmt --all

# Lint (must pass before PR)
cargo clippy --all-targets --all-features -- -D warnings

# Type check
cargo check

# Run tests (requires database)
cargo test --no-fail-fast -- --nocapture
```

If tests require local dependencies:
```bash
docker compose up -d postgres redis rustfs
```

For deterministic integration tests:
```bash
docker compose -f docker-compose.integration.yml up -d
export RUSTCHAT_TEST_DATABASE_URL=postgres://rustchat:rustchat@127.0.0.1:55432/rustchat
export RUSTCHAT_TEST_REDIS_URL=redis://127.0.0.1:56379/
export RUSTCHAT_TEST_S3_ENDPOINT=http://127.0.0.1:59000
export RUSTCHAT_TEST_S3_ACCESS_KEY=testaccesskey
export RUSTCHAT_TEST_S3_SECRET_KEY=testsecretkey
```

### Frontend

```bash
cd frontend

# Install dependencies
npm ci

# Development server
npm run dev

# Production build
npm run build

# Preview production build
npm run preview

# E2E tests
npm run test:e2e

# Settings parity tests (visual regression)
npm run test:e2e:settings-parity
```

### Compatibility Smoke Checks

```bash
# Requires running backend
BASE=http://127.0.0.1:3000 ./scripts/mm_compat_smoke.sh
BASE=http://127.0.0.1:3000 ./scripts/mm_mobile_smoke.sh
```

### Full Stack Local Run

```bash
# 1. Configure environment
cp .env.example .env
# Edit .env and set required secrets

# 2. Start all services
docker compose up -d --build
```

## Code Style Guidelines

### Rust Backend

1. **Error Handling**: All public functions must return `Result` or `Option`. Use `thiserror` for error types.
2. **Async**: Prefer idiomatic Tokio + Axum patterns. Use `async-trait` where necessary.
3. **Handler Structure**: Keep handlers thin; place logic in services/repositories.
4. **API Contracts**: Preserve Mattermost API response signatures for v4 compatibility surfaces.
5. **Formatting**: Use `cargo fmt` before committing.
6. **Linting**: All code must pass `cargo clippy --all-targets --all-features -- -D warnings`.

### Frontend TypeScript/Svelte

1. **Feature Organization**: Code is organized by feature (`src/features/*`), not by type.
2. **Architecture Pattern**:
   - Repository: Data access layer
   - Service: Business logic & orchestration
   - Store: State management (Svelte stores)
   - Handler: WebSocket event handlers
3. **Type Safety**: Use branded types for IDs. Avoid `any`.
4. **Imports**: Use `@/` path alias for project imports.
5. **Svelte runes ($state, $derived, $effect)**: Use `<script lang="ts">` for components.

### Naming Conventions

- **Rust**: `snake_case` for functions/variables, `PascalCase` for types/structs, `SCREAMING_SNAKE_CASE` for constants
- **TypeScript**: `camelCase` for functions/variables, `PascalCase` for types/interfaces/components
- **Files**: Match export name (e.g., `userService.ts` exports `userService`)

### Documentation

- Keep code, comments, docs, commit messages, and PR text in English.
- Respond to users in the same language they use.

## Testing Instructions

### Unit Tests

Backend unit tests are colocated with source files using `#[cfg(test)]` modules.

### Integration Tests

Integration tests are in `backend/tests/`. They require:
- PostgreSQL database
- Redis instance
- S3-compatible storage

Run with: `cargo test --no-fail-fast -- --nocapture`

### E2E Tests

Frontend E2E tests use Playwright:
```bash
cd frontend
npm run test:e2e
```

### Compatibility Testing

For changes affecting Mattermost compatibility:
1. Run smoke tests: `./scripts/mm_compat_smoke.sh`
2. Run mobile smoke tests: `./scripts/mm_mobile_smoke.sh`
3. Verify WebSocket event contracts
4. Check API response schemas match expected formats

### Test Data

The backend creates an admin user on first startup if configured:
- Set `RUSTCHAT_ADMIN_USER` and `RUSTCHAT_ADMIN_PASSWORD` in `.env`
- User is created with `system_admin` role

## Security Considerations

### Required Secrets (Production)

The following must be set in `.env` for production:

```bash
RUSTCHAT_JWT_SECRET              # Min 32 chars, random, high entropy
RUSTCHAT_JWT_ISSUER              # JWT issuer claim
RUSTCHAT_JWT_AUDIENCE            # JWT audience claim
RUSTCHAT_ENCRYPTION_KEY          # 32-byte key for sensitive data
RUSTCHAT_S3_ACCESS_KEY           # S3 credentials
RUSTCHAT_S3_SECRET_KEY
RUSTFS_ACCESS_KEY                # RustFS credentials
RUSTFS_SECRET_KEY
```

### Security Configuration

Production hardening (set `RUSTCHAT_ENVIRONMENT=production`):

```bash
# Disable insecure WebSocket token transport
RUSTCHAT_SECURITY_WS_ALLOW_QUERY_TOKEN=false

# Use secure OAuth token delivery
RUSTCHAT_SECURITY_OAUTH_TOKEN_DELIVERY=cookie

# Enable rate limiting
RUSTCHAT_SECURITY_RATE_LIMIT_ENABLED=true
RUSTCHAT_SECURITY_RATE_LIMIT_AUTH_PER_MINUTE=10
RUSTCHAT_SECURITY_RATE_LIMIT_WS_PER_MINUTE=30

# Enforce HTTPS origins only
RUSTCHAT_CORS_ALLOWED_ORIGINS=https://your-domain.com
RUSTCHAT_SITE_URL=https://your-domain.com
```

### Never Commit

- Secrets or credential files
- Private keys (APNS, Firebase)
- `.env` files (use `.env.example` as template)
- Generated test data with real credentials

## Development Workflow

### Plan-First Workflow (Mandatory)

For feature or behavior changes:

1. **Requirement Phase**:
   - Research current code and compatibility context
   - Draft `docs/internal/SPEC.md` with scope, contract impact, and verification

2. **Validation Gate**:
   - Ask: "Does this plan meet your expectations? Please approve or provide feedback."
   - Do not write implementation code until explicit user approval

3. **Verification Phase**:
   - Update `docs/internal/task_plan.md`
   - Mark task readiness for testing
   - Provide at least one concrete manual verification command

### Compatibility-First Workflow (Mandatory)

For any change affecting API v4 routes, websocket events, pagination, error semantics, or calls behavior:

1. **Phase 1 - Analysis**:
   - Analyze upstream behavior in `../mattermost` and `../mattermost-mobile`
   - Create analysis folder: `previous-analyses/YYYY-MM-DD-<topic>/`
   - Use templates from `previous-analyses/_TEMPLATE/`

2. **Phase 2 - Contract Capture**:
   - Document HTTP status codes, response shapes, headers
   - Document websocket event names and payload fields
   - Document pagination, ordering, filtering defaults

3. **Phase 3 - Implementation**:
   - Implement only after every gap is explicit and testable
   - Never treat "close enough" as compatible

### Commit Messages

Use Conventional Commits:

```text
feat: add user registration endpoint
fix: correct JWT expiry calculation
docs: update API documentation
test: add channel permission tests
refactor: simplify message rendering
```

## Skills System

The repository uses a skill-based workflow system in `.agents/skills/`:

- `mattermost-analysis-first`: Required before compatibility-sensitive implementation
- `mattermost-api-parity`: Required for API error/payload contract work
- `mattermost-mobile-compatibility`: Required for mobile-consumed behavior
- `mm-endpoint-contract-parity`: Required for API v4 method/path/status/schema parity
- `mm-websocket-calls-parity`: Required for websocket/calls events
- `mm-mobile-journey-parity`: Required for end-to-end mobile journey validation
- `production-readiness-gate`: Required when evaluating release readiness
- `user-validation`: Required for plan-first delivery workflow
- `guidelines`: Required for code quality sanity checks

Trigger the appropriate skills based on task category.

## Command Invocation Policy (Permanent)

- Primary command invocation is `Ctrl/Cmd+K` (desktop) and `^k` token (mobile/typed)
- Do not introduce `/`-triggered slash command UX as the primary path
- Command discovery must be command-menu based and keyboard-first
- Keep backward compatibility paths internal-only

## Important Reminders

- Do exactly what was requested: no speculative features
- Prefer editing existing files over creating new ones
- If you see unrelated issues, note them separately; do not silently fix them
- Make MINIMAL changes to achieve the goal
- Every bug fix should include or update a regression test when feasible
- Surface assumptions explicitly before implementing
- If uncertain, ask rather than guessing
