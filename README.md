# 🟦 rustchat

**Self-hosted, enterprise-ready team collaboration platform built in Rust.**

[![Rust](https://img.shields.io/badge/rust-1.93+-orange.svg)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

## Overview

rustchat is a high-performance team messaging platform similar to Mattermost/Slack, designed for:

- 🔒 **Self-hosted deployments** — Your data stays on your infrastructure
- ⚡ **High performance** — Async Rust with Axum and Tokio
- 🔧 **DevOps/ChatOps** — Webhooks, slash commands, and bot integrations
- 🏢 **Enterprise-ready** — RBAC, SSO, audit logging, compliance features
- 📦 **Flexible storage** — S3-compatible backends (MinIO, Ceph RGW, AWS S3)

## Features

- Public & private channels
- Direct messages and group DMs
- Threads and reactions
- File uploads with S3 storage
- Real-time WebSocket events
- Full-text search
- Incoming/outgoing webhooks
- Slash commands
- Bot accounts

## Quick Start

### Prerequisites

- Rust 1.93+
- Docker & Docker Compose
- PostgreSQL 16+
- Redis 7+

### Development Setup

1. **Clone the repository**
   ```bash
   git clone https://github.com/rustchat/rustchat.git
   cd rustchat
   ```

2. **Start dependencies**
   ```bash
   docker compose up -d postgres redis minio
   ```

3. **Configure environment**
   ```bash
   cp .env.example .env
   # Edit .env with your settings
   ```

4. **Run the server**
   ```bash
   cd backend
   cargo run
   ```

5. **Verify**
   ```bash
   curl http://localhost:3000/api/v1/health/live
   # {"status":"ok","version":"0.1.0"}
   ```

## Project Structure

```
rustchat/
├── backend/           # Rust API server
│   ├── src/
│   │   ├── api/       # HTTP routes and handlers
│   │   ├── config/    # Configuration management
│   │   ├── db/        # Database connections
│   │   ├── error/     # Error types
│   │   └── telemetry/ # Logging and tracing
│   └── migrations/    # SQLx database migrations
├── docker/            # Docker build files
├── helm/              # Kubernetes Helm charts
└── docs/              # Documentation
```

## Documentation

Detailed guides are available in the [docs/](docs/) directory:

- 📖 **[User Guide](docs/user_guide.md)** — Getting started, messaging, and collaboration features.
- ⚙️ **[Admin Guide](docs/admin_guide.md)** — Installation, deployment, and system configuration.
- 🏗️ **[Architecture Overview](docs/architecture.md)** — Deep dive into the system design.
- 🚀 **[Running Environment](docs/running_environment.md)** — Step-by-step development setup.

## Configuration

rustchat is configured via environment variables with the `RUSTCHAT_` prefix:

| Variable | Description | Default |
|----------|-------------|---------|
| `RUSTCHAT_SERVER_HOST` | Server bind address | `0.0.0.0` |
| `RUSTCHAT_SERVER_PORT` | Server port | `3000` |
| `RUSTCHAT_DATABASE_URL` | PostgreSQL connection URL | — |
| `RUSTCHAT_REDIS_URL` | Redis connection URL | `redis://localhost:6379` |
| `RUSTCHAT_JWT_SECRET` | JWT signing secret | — |
| `RUSTCHAT_LOG_LEVEL` | Log level | `info` |

See [`.env.example`](.env.example) for all options.

## API

Base URL: `/api/v1`

### Health Checks

- `GET /health/live` — Liveness probe
- `GET /health/ready` — Readiness probe (checks DB)

### Authentication (coming soon)

- `POST /auth/register` — Register new user
- `POST /auth/login` — Login and get JWT
- `POST /auth/refresh` — Refresh token

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

MIT License — see [LICENSE](LICENSE) for details.
