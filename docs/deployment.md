# Deployment Guide

## Evaluation Deployment

The fastest way to run RustChat for evaluation or small teams is single-host Docker Compose.

### What You Get

| Component | Resource | Purpose |
|-----------|----------|---------|
| Backend | 1 CPU, 512 MB | API server, WebSocket hub, business logic |
| Frontend | 0.1 CPU, 64 MB | Nginx serving the Vue.js SPA |
| PostgreSQL | 0.5 CPU, 256 MB | Primary data store |
| Redis | 0.1 CPU, 64 MB | Pub/sub, sessions, rate limiting |
| RustFS (S3) | 0.1 CPU, 128 MB | File uploads and storage |

**Minimum host:** 2 GB RAM, 1 vCPU, 10 GB disk.
**Recommended:** 4 GB RAM, 2 vCPUs, 20 GB disk.

### Quick Start

```bash
cp .env.example .env
# Set secrets (see Required Secrets below)
docker compose up -d --build
```

For a production-oriented single-host deployment, use the standalone production Compose file instead:

```bash
cp .env.example .env
# Set all required production secrets and public URLs.
docker compose -f docker-compose.prod.yml up -d --build
```

The production Compose file does not publish PostgreSQL, Redis, or RustFS management ports to the host. Only the web frontend and calls media ports are exposed by default.

### Generate Required Secrets

```bash
# Add these to your .env file
RUSTCHAT_JWT_SECRET=$(openssl rand -hex 32)
RUSTCHAT_ENCRYPTION_KEY=$(openssl rand -hex 32)
RUSTCHAT_S3_ACCESS_KEY=$(openssl rand -hex 16)
RUSTCHAT_S3_SECRET_KEY=$(openssl rand -hex 32)
RUSTFS_ACCESS_KEY="${RUSTCHAT_S3_ACCESS_KEY}"
RUSTFS_SECRET_KEY="${RUSTCHAT_S3_SECRET_KEY}"
```

### Verify the Deployment

```bash
# All services healthy?
docker compose ps

# Production Compose
docker compose -f docker-compose.prod.yml ps

# Backend responding?
curl -s http://localhost:3000/api/v1/health/live | jq .

# Web UI accessible?
curl -s -o /dev/null -w "%{http_code}" http://localhost:8080
# Expected: 200
```

---

## Production-Oriented Deployment

Before running RustChat in production:

Use `docker-compose.prod.yml` as the production baseline for single-host deployments. The default `docker-compose.yml` remains optimized for local development and exposes internal services for debugging.

### 1. Environment Hardening Checklist

```bash
# .env production baseline
RUSTCHAT_ENVIRONMENT=production
RUSTCHAT_SITE_URL=https://chat.example.com
RUSTCHAT_CORS_ALLOWED_ORIGINS=https://chat.example.com

# Cryptographic secrets (generate fresh, min 32 chars)
RUSTCHAT_JWT_SECRET=$(openssl rand -hex 32)
RUSTCHAT_ENCRYPTION_KEY=$(openssl rand -hex 32)

# Security hardening
RUSTCHAT_SECURITY_OAUTH_TOKEN_DELIVERY=cookie
RUSTCHAT_SECURITY_RATE_LIMIT_ENABLED=true
RUSTCHAT_SECURITY_RATE_LIMIT_AUTH_PER_MINUTE=10
RUSTCHAT_SECURITY_RATE_LIMIT_WS_PER_MINUTE=30
```

| Setting | Development | Production | Why |
|---------|-------------|------------|-----|
| `RUSTCHAT_ENVIRONMENT` | `development` (explicit override) | `production` | Enables strict validation; default is `production` |
| `RUSTCHAT_SITE_URL` | `http://localhost:8080` | `https://...` | Required for OAuth callbacks |
| `CORS_ALLOWED_ORIGINS` | Permissive | Exact HTTPS domains | Prevents cross-origin attacks |
| `OAUTH_TOKEN_DELIVERY` | `cookie` | `cookie` | Secure one-time OAuth token exchange |
| `RATE_LIMIT_ENABLED` | `true` | `true` | Brute-force protection |

### 2. Reverse Proxy

Place a reverse proxy in front of RustChat for TLS termination and static asset serving.

#### Nginx

```nginx
server {
    listen 443 ssl http2;
    server_name chat.example.com;

    ssl_certificate /path/to/cert.pem;
    ssl_certificate_key /path/to/key.pem;

    location / {
        proxy_pass http://rustchat-backend:3000;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host $host;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```

#### Caddy

```caddy
chat.example.com {
    reverse_proxy rustchat-backend:3000 {
        header_up Host {host}
        header_up X-Real-IP {remote}
        header_up X-Forwarded-For {remote}
        header_up X-Forwarded-Proto {scheme}
    }
}
```

#### Traefik

```yaml
# docker-compose.yml labels for Traefik
labels:
  - "traefik.enable=true"
  - "traefik.http.routers.rustchat.rule=Host(`chat.example.com`)"
  - "traefik.http.routers.rustchat.tls=true"
  - "traefik.http.routers.rustchat.tls.certresolver=letsencrypt"
  - "traefik.http.services.rustchat.loadbalancer.server.port=3000"
```

**Critical:** WebSocket upgrade headers must be forwarded for real-time messaging to work.

### 3. TLS / Certificates

Use Let's Encrypt (via Caddy or Traefik) or bring your own certificates. RustChat itself runs on HTTP internally — TLS is terminated at the reverse proxy.

---

## Environment Variables

The most critical variables for deployment:

| Variable | Required | Description |
|----------|----------|-------------|
| `RUSTCHAT_JWT_SECRET` | Yes | Secret for JWT signing (min 32 chars) |
| `RUSTCHAT_ENCRYPTION_KEY` | Yes | 32-byte key for sensitive data |
| `RUSTCHAT_SITE_URL` | Yes | Public HTTPS URL |
| `RUSTCHAT_CORS_ALLOWED_ORIGINS` | Yes (prod) | Comma-separated allowed origins |
| `RUSTCHAT_DATABASE_URL` | Yes | PostgreSQL connection string |
| `RUSTCHAT_REDIS_URL` | Yes | Redis connection string |
| `RUSTCHAT_S3_ENDPOINT` | Yes | S3-compatible storage endpoint |
| `RUSTCHAT_S3_BUCKET` | Yes | S3 bucket name |
| `RUSTCHAT_S3_ACCESS_KEY` | Yes | S3 access key |
| `RUSTCHAT_S3_SECRET_KEY` | Yes | S3 secret key |
| `RUSTCHAT_ADMIN_USER` | No | Auto-created admin email |
| `RUSTCHAT_ADMIN_PASSWORD` | No | Auto-created admin password |

For the complete reference, see [Admin Configuration](./admin/configuration.md).

---

## Backup and Restore

### PostgreSQL

**Backup:**
```bash
# Production Compose helper
./tools/backup-postgres.sh

# Optional output path
./tools/backup-postgres.sh backups/rustchat.sql.gz
```

**Restore:**
```bash
RUSTCHAT_RESTORE_CONFIRM=YES ./tools/restore-postgres.sh backups/rustchat.sql.gz
```

### S3 / File Storage

Mirror or version your S3 bucket using your storage provider's tools:

```bash
# Example with rclone
rclone sync rustfs:rustchat-uploads backup:rustchat-uploads-backup

# Or using an S3-compatible client
mc mirror rustchat/rustchat-uploads /backup/rustchat-files
```

### Redis

Redis is used as a cache and pub/sub broker. Data loss is acceptable — it rebuilds from PostgreSQL on restart. No backup required.

---

## Scaling

### Horizontal Scaling

Run multiple backend containers behind a load balancer. Redis pub/sub handles cross-instance WebSocket fan-out.

```yaml
# docker-compose.yml snippet
services:
  backend:
    deploy:
      replicas: 3
```

**Requirements for horizontal scaling:**
- Shared PostgreSQL (managed service recommended)
- Shared Redis (or Redis Cluster)
- Shared S3-compatible storage
- Load balancer with sticky sessions for WebSocket (or use Redis pub/sub for stateless fan-out)

### Database

Use a managed PostgreSQL service (AWS RDS, Google Cloud SQL, Azure Database) for high availability.

### File Storage

Use a scalable S3-compatible object store (AWS S3, Cloudflare R2, RustFS cluster).

### Calls

> **Limitation:** The WebRTC media plane is instance-local. A distributed SFU mesh is not yet implemented. For calls to work correctly in a multi-backend setup, clients should connect to the same backend instance or use an external TURN server.

---

## Further Reading

- [Admin Security Guide](./admin/security.md) — Full hardening checklist
- [Admin Configuration](./admin/configuration.md) — Complete environment variable reference
- [Admin Scaling Guide](./admin/scaling.md) — Advanced scaling strategies
- [Reverse Proxy Guide](./admin/reverse-proxy.md) — Detailed proxy configuration
