# Quick Start

Get RustChat running locally in under 5 minutes.

## Prerequisites

| Tool | Minimum Version | How to Check |
|------|-----------------|--------------|
| Docker | 24.0+ | `docker --version` |
| Docker Compose | 2.20+ | `docker compose version` |
| Git | 2.30+ | `git --version` |
| RAM | 2 GB | — |

> **macOS / Windows:** Docker Desktop includes Docker Compose.
> **Linux:** Install `docker-ce` and `docker-compose-plugin` via your package manager.

## One-Command Start

The fastest way to evaluate RustChat:

```bash
# 1. Clone and enter the repository
git clone https://github.com/rustchatio/rustchat.git
cd rustchat

# 2. Run the automated setup (creates .env, starts dependencies, installs tools)
./scripts/dev-setup.sh

# 3. Start all services
docker compose up -d --build
```

That's it. The stack builds and starts in the background.

### What the setup script does

`dev-setup.sh` automates the boring parts:
- Creates `.env` from `.env.example` if it doesn't exist
- Starts PostgreSQL, Redis, and RustFS (S3-compatible storage)
- Waits for PostgreSQL to be ready
- Runs database migrations (if `sqlx-cli` is installed)
- Installs frontend dependencies (if Node.js is available)

### Manual steps (if you prefer)

```bash
cp .env.example .env
# Edit .env and set required secrets (see below)
docker compose up -d --build
```

## Required Secrets

Before first launch, edit `.env` and set these secrets:

```bash
# Generate with: openssl rand -hex 32
RUSTCHAT_JWT_SECRET=replace-me-with-a-long-random-secret
RUSTCHAT_ENCRYPTION_KEY=replace-me-with-a-long-random-32-byte-key

# S3 credentials (can be any random strings for local evaluation)
RUSTCHAT_S3_ACCESS_KEY=rustchat-local-access
RUSTCHAT_S3_SECRET_KEY=rustchat-local-secret
RUSTFS_ACCESS_KEY=rustchat-local-access
RUSTFS_SECRET_KEY=rustchat-local-secret

# First admin user (created automatically on first startup)
RUSTCHAT_ADMIN_USER=admin@rustchat.local
RUSTCHAT_ADMIN_PASSWORD=changeme-strong-password
```

> **Evaluation only:** The example values above are safe for local testing. For production, generate cryptographically random secrets.

## Access the Application

| Service | URL | Description |
|---------|-----|-------------|
| **Web UI** | http://localhost:8080 | Main application |
| **API** | http://localhost:3000 | REST API and WebSocket |
| **API Health** | http://localhost:3000/api/v1/health/live | Liveness probe |
| **RustFS Console** | http://localhost:9001 | S3-compatible storage UI |

### First Login

After the backend starts (wait for `docker compose logs -f backend` to show "Server running"), log in with:

- **Email:** The value you set for `RUSTCHAT_ADMIN_USER`
- **Password:** The value you set for `RUSTCHAT_ADMIN_PASSWORD`

## Check Service Status

```bash
# View all containers
docker compose ps

# Follow backend logs
docker compose logs -f backend

# Follow all logs
docker compose logs -f
```

## Enable Search (Optional)

Search requires Meilisearch, which is behind a Docker Compose profile:

```bash
docker compose --profile search up -d
```

Access Meilisearch at http://localhost:7700.

## Stop the Environment

```bash
# Stop all containers (keeps data)
docker compose down

# Stop and remove all data (full reset)
docker compose down -v
```

| Command | Data Preserved? | Use When |
|---------|-----------------|----------|
| `docker compose down` | ✅ Yes | Daily shutdown |
| `docker compose down -v` | ❌ No | Clean slate / troubleshooting |
| `docker compose restart backend` | ✅ Yes | Config change, code rebuild |

## Common First-Run Issues

### "Backend container keeps restarting"

Check logs: `docker compose logs backend`

- **Missing secrets:** Ensure all `RUSTCHAT_JWT_SECRET`, `RUSTCHAT_ENCRYPTION_KEY`, and S3 credentials are set in `.env`
- **Port conflict:** Something is already using port 3000, 8080, 5432, 6379, or 9000. Stop the conflicting service or edit `docker-compose.yml` ports.

### "Cannot log in — user not found"

The admin user is created **only on first startup**. If you started without `RUSTCHAT_ADMIN_USER` set:

1. Stop: `docker compose down -v`
2. Set `RUSTCHAT_ADMIN_USER` and `RUSTCHAT_ADMIN_PASSWORD` in `.env`
3. Start: `docker compose up -d --build`

### "File uploads fail"

The backend creates the upload bucket automatically on startup. If uploads fail:

1. Check that RustFS is healthy: `docker compose ps rustfs`
2. Ensure the RustFS credentials in `.env` match between `RUSTFS_ACCESS_KEY`/`RUSTFS_SECRET_KEY` and `RUSTCHAT_S3_ACCESS_KEY`/`RUSTCHAT_S3_SECRET_KEY`

## Next Steps

- [Deployment Guide](./deployment.md) — Evaluation vs production deployment
- [Development Guide](./development.md) — Running from source
- [Architecture Overview](./architecture.md) — How the system works
- [Admin Configuration](./admin/configuration.md) — All environment variables
