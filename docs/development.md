# Development Guide

This guide covers running RustChat from source for daily development.

## Required Tools

| Tool | Version | Install | Check |
|------|---------|---------|-------|
| Rust | 1.92+ | [rustup.rs](https://rustup.rs/) | `rustc --version` |
| Node.js | 24+ | [nodejs.org](https://nodejs.org/) | `node --version` |
| Docker + Compose | 24.0+ / 2.20+ | [Docker Desktop](https://docs.docker.com/get-docker/) | `docker compose version` |
| sqlx-cli | latest | `cargo install sqlx-cli --no-default-features --features postgres` | `sqlx --version` |

> **Note:** The frontend requires Node.js 24+. If your system has an older version, use [nvm](https://github.com/nvm-sh/nvm) or [fnm](https://github.com/Schniz/fnm) to manage versions.

## Quick Setup

```bash
# 1. Clone and enter
git clone https://github.com/rustchatio/rustchat.git
cd rustchat

# 2. Run automated setup
./scripts/dev-setup.sh

# 3. Edit .env and set secrets
# (see .env.example for required variables)
```

The setup script starts PostgreSQL, Redis, and RustFS in Docker, then prepares the backend and frontend for local development.

---

## Backend Development

### Start Dependencies

```bash
# Start only infrastructure services (no backend/frontend containers)
docker compose up -d postgres redis rustfs
```

### Database Setup

```bash
cd backend

# Install sqlx-cli if you haven't already
cargo install sqlx-cli --no-default-features --features postgres

# Run migrations
sqlx migrate run

# Verify (optional)
sqlx migrate info
```

### Daily Commands

```bash
cd backend

# Type check (fast)
cargo check

# Run library tests (no external services needed)
cargo test --lib

# Build release binary
cargo build --release

# Run the server (uses .env for configuration)
cargo run
```

The backend will be available at http://localhost:3000.

### Formatting and Linting

These are enforced in CI. Run them before committing:

```bash
cd backend

# Format code
cargo fmt --all

# Check formatting without writing
cargo fmt --all -- --check

# Run clippy (treat warnings as errors)
cargo clippy --all-targets --all-features -- -D warnings
```

---

## Frontend Development

### Install Dependencies

```bash
cd frontend
npm ci --ignore-scripts
npm run apply:dependency-patches
```

> **Why `--ignore-scripts`?** The project policy blocks install scripts in CI for security. Patches are applied explicitly via the separate command.

### Daily Commands

```bash
cd frontend

# Development server with hot reload
npm run dev

# Production build (with type checking)
npm run build

# Preview production build locally
npm run preview
```

The dev server will be available at http://localhost:5173.

### Tests

```bash
cd frontend

# Unit tests (vitest)
npm run test:unit

# E2E tests (requires full stack running)
npm run test:e2e

# Update E2E snapshots after intentional UI changes
npx playwright test --update-snapshots
```

### Dependency Policy

- Use `npm` only in `frontend/`
- Keep `frontend/package-lock.json` committed
- Run `npm ci --ignore-scripts` in CI
- See [Frontend Dependency Policy](frontend-dependency-policy.md) before adding dependencies

---

## Running the Full Stack Locally

For active development, run three processes in separate terminals:

**Terminal 1 — Backend:**
```bash
cd backend && cargo run
```

**Terminal 2 — Frontend:**
```bash
cd frontend && npm run dev
```

**Terminal 3 — Push Proxy (optional):**
```bash
cd push-proxy && cargo run
```

**Access points:**
| Service | URL | Notes |
|---------|-----|-------|
| Frontend (dev) | http://localhost:5173 | Hot reload, Vite dev server |
| Frontend (prod build) | http://localhost:8080 | Nginx serving `dist/` |
| Backend API | http://localhost:3000 | Direct API access |
| Push Proxy | http://localhost:3001 | Mobile push notifications |

> **Important:** When running the backend locally (not in Docker), ensure `.env` points to `localhost` for database and Redis:
> ```bash
> RUSTCHAT_DATABASE_URL=postgres://rustchat:rustchat@localhost:5432/rustchat
> RUSTCHAT_REDIS_URL=redis://localhost:6379
> RUSTCHAT_S3_ENDPOINT=http://localhost:9000
> ```

---

## Integration Tests

Integration tests require all infrastructure services running:

```bash
# 1. Start test infrastructure
docker compose -f docker-compose.integration.yml up -d

# 2. Set test environment variables
export RUSTCHAT_TEST_DATABASE_URL=postgres://rustchat:rustchat@127.0.0.1:55432/rustchat
export RUSTCHAT_TEST_REDIS_URL=redis://127.0.0.1:56379/
export RUSTCHAT_TEST_S3_ENDPOINT=http://127.0.0.1:59000
export RUSTCHAT_TEST_S3_ACCESS_KEY=testaccesskey
export RUSTCHAT_TEST_S3_SECRET_KEY=testsecretkey

# 3. Run all integration tests
cd backend && cargo test --no-fail-fast -- --nocapture

# 4. Run a single test file
cargo test --test channels_test
```

---

## Pre-Commit Checklist

Run these before opening a pull request:

```bash
# Backend
cd backend && cargo fmt --check && cargo clippy --all-targets --all-features -- -D warnings && cargo test --lib

# Frontend
cd frontend && npm run build && npm run test:unit

# Smoke test (validates Docker configs and runs compat checks)
./scripts/smoke-test.sh
```

---

## Common Troubleshooting

### "Failed to connect to database"

```bash
# Check if PostgreSQL is running
docker compose ps postgres

# Check if it's ready
docker compose exec postgres pg_isready -U rustchat

# Restart if needed
docker compose restart postgres
```

### "sqlx query validation failed" / "failed to find sqlx-data.json"

```bash
# Ensure DATABASE_URL is set and the database is running
export DATABASE_URL=postgres://rustchat:rustchat@localhost:5432/rustchat

# For offline builds (CI), prepare query data
cd backend && cargo sqlx prepare
```

### "Port 3000 already in use"

```bash
# Find the process
lsof -i :3000

# Or use a different port
RUSTCHAT_SERVER_PORT=3001 cargo run
```

### "Node version mismatch"

```bash
# Check version
node --version

# If older than 24, switch versions
nvm use 24
# or
fnm use 24
```

### "Frontend build fails with TypeScript errors"

```bash
# Ensure patches are applied
cd frontend && npm run apply:dependency-patches

# Clear node_modules and reinstall
rm -rf node_modules package-lock.json
npm ci --ignore-scripts
npm run apply:dependency-patches
npm run build
```

### "WebSocket connection fails in dev"

Ensure the backend is running and CORS allows the dev server origin:

```bash
# In .env
RUSTCHAT_CORS_ALLOWED_ORIGINS=http://localhost:5173,http://localhost:8080
```

### "Push proxy fails to start"

Push proxy is optional for local development. If you don't need mobile push notifications, you can skip it. If you do need it:

```bash
cd push-proxy
RUSTCHAT_PUSH_PORT=3001 cargo run
```

---

## IDE Setup

### VS Code

Recommended extensions:
- **Rust:** `rust-lang.rust-analyzer` — enable `cargo check` on save
- **Vue:** Vue.volar (official Vue 3 + TypeScript support)
- **Tailwind:** bradlc.vscode-tailwindcss
- **Docker:** ms-azuretools.vscode-docker

Settings for `settings.json`:
```json
{
  "rust-analyzer.cargo.features": "all",
  "rust-analyzer.check.command": "clippy",
  "editor.formatOnSave": true
}
```

### Vim / Neovim

- Rust: `rust-analyzer` via `nvim-lspconfig`
- Vue: `volar` language server
- See [docs/development/local-setup.md](development/local-setup.md) for additional editor-specific tips.

---

## Further Reading

- [Contributor Workflow](contributor-workflow.md) — Fork, branch, PR process
- [Local Setup Tips](development/local-setup.md) — Additional IDE and environment tips
- [Testing Model](development/testing.md) — Full test strategy and CI gates
- [Code Style](development/code-style.md) — Rust and TypeScript conventions
- [Architecture Overview](architecture.md) — System design
