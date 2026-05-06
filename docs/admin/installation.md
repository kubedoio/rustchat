# RustChat Admin Guide

This guide is intended for system administrators, DevOps engineers, and IT staff responsible for deploying, configuring, and maintaining RustChat.

---

## 1. Introduction

### Admin Responsibilities
As a RustChat administrator, you are responsible for:
- **System Availability:** Ensuring the backend, frontend, and database services are running smoothly.
- **Security:** Managing SSL/TLS certificates, firewall rules, and authentication settings.
- **Scaling:** Monitoring resource usage and scaling services to meet user demand.
- **Data Integrity:** Configuring regular backups for the PostgreSQL database and S3 file storage.
- **User Management:** Onboarding users, managing roles, and overseeing system-wide settings.

### Supported Environments
RustChat is designed to be highly portable:
- **Bare Metal:** High-performance direct deployment on Linux servers.
- **Docker Compose:** Ideal for small teams or development/staging environments.
- **Kubernetes:** Best for high availability and enterprise-scale deployments using Helm.

---

## 2. Architecture Overview

RustChat follows a modern, decoupled architecture designed for speed and horizontal scalability.

- **Rust Backend:** A high-performance REST API built with Axum and Tokio. It handles business logic, database interactions, and secure authentication.
- **SPA Frontend:** A Svelte 5 + Vite single-page application served via Nginx. It provides a responsive, desktop-like experience in the browser.
- **WebSocket Hub:** A real-time subsystem that manages persistent active connections for instant message delivery, typing indicators, and presence updates.
- **PostgreSQL:** The primary relational database for message history, user profiles, and channel metadata.
- **Redis (Optional/Recommended):** Used for caching frequently accessed data and providing pub/sub capabilities for real-time events across multiple backend instances.
- **File Storage:** Flexible storage for user-uploaded files. Supports Local File Systems and S3-compatible backends like RustFS, Ceph RGW, or AWS S3.
- **Load Balancer:** Required for TLS termination and routing WebSocket traffic with sticky sessions (if scaling multiple backend pods).

---

## 3. Installation & Deployment

### Quick Start with Docker Compose
For a quick local or small-scale setup:

1. **Clone the Repo:**
   ```bash
   git clone https://github.com/rustchat/rustchat.git
   cd rustchat
   ```
2. **Configure Environment:**
   Copy `.env.example` to `.env` and adjust the variables.
3. **Launch Services:**
   ```bash
   docker compose up -d
   ```
   This starts the backend, frontend, PostgreSQL, Redis, and RustFS.

### Deployment Targets

#### Kubernetes
For enterprise production environments, deploy RustChat with your standard Kubernetes manifests and ingress stack.
- **TLS:** Terminate TLS at ingress (for example with Cert-Manager managed certificates).
- **Auto-Scaling:** Configure HPA for backend and frontend workloads.
- **Note:** This repository does not currently ship an official `helm/` chart directory.

#### Bare Metal / Systemd
1. Compile the backend: `cargo build --release`.
2. Install the binary as a Systemd service.
3. Serve the frontend `dist` folder using Nginx.

---

## 4. Configuration

RustChat is configured primarily via environment variables with the `RUSTCHAT_` prefix.

| Variable | Description |
|----------|-------------|
| `RUSTCHAT_DATABASE_URL` | PostgreSQL connection string. |
| `RUSTCHAT_REDIS_URL` | Redis connection string. |
| `RUSTCHAT_ENVIRONMENT` | Runtime mode (`development` or `production`). |
| `RUSTCHAT_CORS_ALLOWED_ORIGINS` | Comma-separated browser origin allowlist for CORS. |
| `RUSTCHAT_S3_ENDPOINT` | URL for your S3-compatible service. |
| `RUSTCHAT_S3_BUCKET` | The bucket name for file storage. |
| `RUSTCHAT_JWT_SECRET` | Secret key for signing session tokens. |
| `RUSTCHAT_ENCRYPTION_KEY` | Key used to encrypt/decrypt stored sensitive config values (for example OAuth client secrets). |
| `RUSTCHAT_SMTP_HOST` | Host for outgoing email notifications. |

Production guidance:
- Set `RUSTCHAT_ENVIRONMENT=production`.
- Set `RUSTCHAT_CORS_ALLOWED_ORIGINS` explicitly (no wildcards).
- Use long random values for `RUSTCHAT_JWT_SECRET` and `RUSTCHAT_ENCRYPTION_KEY`.

---

## 5. Reverse Proxy Setup

We recommend using **Nginx**, **Traefik**, or **Caddy** for TLS termination and basic load balancing.

### How It Works

```
┌─────────────────┐     HTTPS      ┌─────────────────┐     HTTP       ┌─────────────────┐
│   Web Browser   │───────────────▶│  Reverse Proxy  │───────────────▶│  RustChat API   │
│                 │                │  (TLS Termination)               │   (Port 3000)   │
└─────────────────┘                └─────────────────┘                └─────────────────┘
       https://chat.example.com         proxy_pass http://rustchat:3000
```

### CORS Configuration Behind Proxy

When running behind a reverse proxy, configure RustChat with the **public HTTPS URLs**:

```bash
# .env - Production behind reverse proxy
RUSTCHAT_ENVIRONMENT=production
RUSTCHAT_SITE_URL=https://chat.example.com
RUSTCHAT_CORS_ALLOWED_ORIGINS=https://chat.example.com
```

**Important:** Even though internal communication uses HTTP, you must configure `https://` origins for CORS because:
- Browsers see only the public HTTPS URL
- CORS validation happens against the `Origin` header sent by browsers
- Production mode requires `https://` for security

### Nginx Example Config

Crucially, ensure your proxy supports WebSockets:
```nginx
server {
    listen 443 ssl http2;
    server_name chat.example.com;
    
    ssl_certificate /path/to/cert.pem;
    ssl_certificate_key /path/to/key.pem;
    
    location / {
        proxy_pass http://rustchat_backend;  # Internal HTTP backend
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host $host;
        
        # Forward client IP for rate limiting
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Real-IP $remote_addr;
    }
}
```

---

## 6. Maintenance & Backups

### Database Backups
Schedule daily `pg_dump` tasks for the PostgreSQL service. Store these backups off-site.

### File Storage Backups
If using RustFS or Ceph, leverage their built-in replication tools. For AWS S3, enable bucket versioning.

### Logs & Monitoring
RustChat outputs structured JSON logs. We recommend piping these into ELK (Elasticsearch, Logstash, Kibana) or Prometheus/Grafana for monitoring system health.
