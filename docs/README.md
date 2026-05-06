# RustChat Documentation

Complete documentation for RustChat — self-hosted team collaboration platform.

---

## Quick Navigation

| If you're a... | Start here |
|----------------|------------|
| **End User** looking to use RustChat | [User Guide](./user/README.md) |
| **Administrator** deploying or managing RustChat | [Admin Guide](./admin/README.md) |
| **Developer** contributing code | [Development Guide](./development/README.md) |
| **Operator** handling incidents | [Operations](./operations/README.md) |

---

## Documentation Structure

### 📖 [User Documentation](./user/README.md)
End-user guides for using RustChat features.
- [Quick Start](./user/quick-start.md) — Get chatting in minutes
- [Feature Guide](./user/features.md) — Messaging, channels, search, notifications
- [Troubleshooting](./user/troubleshooting.md) — Common issues and solutions

### 🔧 [Admin Documentation](./admin/README.md)
Guides for system administrators and operators.
- [Installation](./admin/installation.md) — Docker Compose deployment
- [Configuration](./admin/configuration.md) — Environment variables and settings
- [Security](./admin/security.md) — Security hardening guide
- [SSO Setup](./admin/sso.md) — Single Sign-On configuration
- [Email Setup](./admin/email.md) — SMTP for notifications
- [Push Notifications](./admin/push-notifications.md) — Mobile push setup
- [Scaling](./admin/scaling.md) — Horizontal scaling and HA
- [Backup and Restore](./admin/backup-restore.md) — Data protection

### 🏗️ [Architecture](./architecture/README.md)
System architecture and design documentation.
- [Overview](./architecture/overview.md) — High-level system design
- [Backend](./architecture/backend.md) — Rust API server architecture
- [Frontend](./architecture/frontend.md) — Vue.js SPA architecture
- [Data Model](./architecture/data-model.md) — Database schema and entities
- [Calls Deployment](./architecture/calls-deployment.md) — Voice/video call architecture
- [WebSocket](./architecture/websocket.md) — Real-time communication

### 👨‍💻 [Development](./development/README.md)
Resources for contributors and developers.
- [Local Setup](./development/local-setup.md) — Development environment
- [Contributing](./development/contributing.md) — How to contribute
- [Code Style](./development/code-style.md) — Rust and TypeScript conventions
- [Testing](./development/testing.md) — Test layers and requirements
- [Compatibility](./development/compatibility.md) — Mattermost API compatibility
- [Agent Model](./development/agent-model.md) — LLM agent workflows
- [Releasing](./development/releasing.md) — Release process

### 🚨 [Operations](./operations/README.md)
Operational runbooks and procedures.
- [Runbook](./operations/runbook.md) — Incident response and common tasks

### 📚 [Reference](./reference/README.md)
Reference documentation.
- [Compatibility Matrix](./reference/compatibility-matrix.md) — API coverage

---

## Key Topics

### Getting Started
1. [Install RustChat](./admin/installation.md) — Deploy with Docker Compose
2. [Configure SSO](./admin/sso.md) — Set up OAuth login
3. [Invite Users](./user/quick-start.md) — Get your team connected

### Understanding the System
1. [Architecture Overview](./architecture/overview.md) — How it all fits together
2. [Data Model](./architecture/data-model.md) — Entities and relationships
3. [Mattermost Compatibility](./development/compatibility.md) — Mobile app support

### Running in Production
1. [Security Hardening](./admin/security.md) — Production security
2. [Scaling Guide](./admin/scaling.md) — Handle growth
3. [Backup Strategy](./admin/backup-restore.md) — Protect your data

---

## External Resources

- [Main Repository](https://github.com/rustchatio/rustchat) — Source code and issues
- [CHANGELOG](https://github.com/rustchatio/rustchat/blob/main/CHANGELOG.md) — Release history
- [LICENSE](https://github.com/rustchatio/rustchat/blob/main/LICENSE) — Apache-2.0 License

---

## Contributing to Documentation

Documentation improvements are welcome! When contributing:

1. Keep language clear and concise
2. Use examples for complex procedures
3. Test commands before including them
4. Update the table of contents when adding pages

See [Development Guide](./development/README.md) for contribution workflow.
