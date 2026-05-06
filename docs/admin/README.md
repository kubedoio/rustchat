# Administrator Guide

This guide is for system administrators, DevOps engineers, and IT staff responsible for deploying, configuring, and maintaining RustChat.

## Quick Start

1. [Installation](./installation.md) - Deploy RustChat using Docker Compose
2. [Configuration](./configuration.md) - Environment variables and settings
3. [Security](./security.md) - Harden your deployment

## Documentation Sections

### Deployment
- [Installation Guide](./installation.md) - Docker Compose setup
- [Reverse Proxy Setup](./reverse-proxy.md) - Nginx, Traefik, Caddy configuration
- [Scaling](./scaling.md) - Horizontal scaling and high availability

### Configuration
- [Configuration Reference](./configuration.md) - All environment variables
- [Email Setup](./email.md) - SMTP configuration for notifications
- [SSO Configuration](./sso.md) - Single Sign-On setup (OAuth/SAML)
- [Push Notifications](./push-notifications.md) - Mobile push setup

### Security
- [Security Deployment Guide](./security.md) - Security hardening
- HTTPS/TLS configuration
- CORS and origin settings
- Rate limiting

### Operations
- [Backup and Restore](./backup-restore.md)
- [Monitoring and Logging](../operations/runbook.md)
- Database maintenance
- File storage management

## Architecture Overview

Before deploying, review the [System Architecture](../architecture/overview.md) to understand:
- Backend components (Rust/Axum)
- Frontend (Vue.js SPA)
- Database (PostgreSQL)
- Cache and pub/sub (Redis)
- File storage (S3-compatible)

---

*For end users: See the [User Guide](../user/README.md) for using RustChat.*
