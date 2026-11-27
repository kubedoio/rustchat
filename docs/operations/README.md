# Operations Guide

This section contains operational runbooks and procedures for running RustChat in production.

## Documentation Sections

### Runbooks
- [Operations Runbook](./runbook.md) - Common operational tasks and procedures
- [Docs Site Deployment](./docs-site-deployment.md) - CI/CD deployment for `docs.rustchat.io`

### Topics Covered

#### Incident Response
- Service outage response
- Database connectivity issues
- Redis failures
- File storage problems

#### Maintenance
- Database backups
- Log rotation
- Certificate renewal
- Version upgrades

#### Monitoring
- Health check endpoints
- Key metrics to watch
- Alert thresholds
- Log analysis

#### Debugging
- Common issues and solutions
- Log locations
- Debug mode
- Tracing and observability

## Quick Reference

### Health Checks
```bash
# Backend liveness (process is up)
curl http://localhost:3000/api/v1/health/live

# Backend readiness (database, Redis, S3, migrations)
curl http://localhost:3000/api/v1/health/ready

# Full stack (through the frontend/Nginx proxy)
curl http://localhost:8080/api/v4/system/ping
```

### Log Locations
```
# Docker Compose
docker compose logs -f backend
docker compose logs -f frontend

# Structured JSON logs (backend)
# Configure RUST_LOG for verbosity
```

---

*For deployment: See [Admin Guide](../admin/README.md).*
