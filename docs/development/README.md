# Development Guide

This guide is for developers contributing to RustChat.

## Quick Start

1. [Local Development Setup](./local-setup.md) - Get the dev environment running
2. [Contributing Guidelines](./contributing.md) - How to contribute code
3. [Code Style](./code-style.md) - Coding conventions

## Documentation Sections

### Getting Started
- [Local Development Setup](./local-setup.md) - Docker-based development
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
- Rust 1.80+ with Axum 0.8
- PostgreSQL 16+ with SQLx
- Redis 7+ for pub/sub and caching
- S3-compatible storage

**Frontend:**
- Vue 3.5 with Composition API
- TypeScript 5.9+
- Pinia for state management
- Vite for building

---

*For system architecture: See the [Architecture Guide](../architecture/README.md).*
