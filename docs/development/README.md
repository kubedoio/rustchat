# Development Guide

This guide is for developers contributing to RustChat.

## Required Tools

| Tool | Version | Install | Check |
|------|---------|---------|-------|
| Rust | 1.95+ (MSRV, enforced by `rust-version` in `Cargo.toml`) | [rustup.rs](https://rustup.rs/) | `rustc --version` |
| Node.js | 24+ (required by `frontend/package.json` engines) | [nodejs.org](https://nodejs.org/) | `node --version` |
| Docker + Compose | 24.0+ / 2.20+ | [Docker Desktop](https://docs.docker.com/get-docker/) | `docker compose version` |
| sqlx-cli | latest | `cargo install sqlx-cli --no-default-features --features postgres` | `sqlx --version` |

> **Note:** The frontend requires Node.js 24+. If your system has an older version, use [nvm](https://github.com/nvm-sh/nvm) or [fnm](https://github.com/Schniz/fnm) to manage versions.

## Documentation Sections

### Getting Started
- [Local Setup](./local-setup.md) - Environment setup, daily commands, troubleshooting
- [Contributing Guidelines](./contributing.md) - PR process, issue templates
- [Agent Operating Model](./agent-model.md) - LLM agent workflows

### Development Practices
- [Code Style](./code-style.md) - Rust and TypeScript conventions
- [Testing](./testing.md) - Test layers and requirements
- [Ownership Map](./ownership.md) - Code ownership and review routing

### Architecture & Compatibility
- [Mattermost Compatibility](./compatibility.md) - API compatibility requirements
- [Target Operating Model](./operating-model.md) - Project goals and deferred items

### Release Process
- [Releasing](./releasing.md) - Version bumping and release checklist

## Project Structure

```
rustchat/
├── backend/            # Rust API server (Axum + SQLx)
├── frontend/           # Vue 3 + TypeScript SPA
├── push-proxy/         # Mobile push notification gateway
├── docs/               # Documentation
└── scripts/            # Utility scripts
```

## Key Technologies

**Backend:**
- Rust 1.95+ with Axum 0.8
- PostgreSQL 16+ (pgvector) with SQLx
- Redis 7+ for pub/sub and caching
- S3-compatible storage

**Frontend:**
- Vue 3.5 with Composition API
- TypeScript 5.9+
- Pinia for state management
- Vite for building
- Vitest for unit tests, Playwright for E2E

---

*For system architecture: See the [Architecture Guide](../architecture/README.md).*
