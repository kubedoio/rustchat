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
- **SPA Frontend:** A Vue 3 + Vite single-page application served via Nginx. It provides a responsive, desktop-like experience in the browser.
- **WebSocket Hub:** A real-time subsystem that manages persistent active connections for instant message delivery, typing indicators, and presence updates.
- **PostgreSQL:** The primary relational database for message history, user profiles, and channel metadata.
- **Redis (Optional/Recommended):** Used for caching frequently accessed data and providing pub/sub capabilities for real-time events across multiple backend instances.
- **File Storage:** Flexible storage for user-uploaded files. Supports Local File Systems and S3-compatible backends like MinIO, Ceph RGW, or AWS S3.
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
   This starts the backend, frontend, PostgreSQL, Redis, and MinIO.

### Deployment Targets

#### Kubernetes (Helm)
For enterprise production environments, we provide a Helm chart (see the `helm/` directory).
- **TLS:** Integrated with Cert-Manager for automatic SSL.
- **Auto-Scaling:** HPA (Horizontal Pod Autoscaler) for the backend and frontend.

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
| `RUSTCHAT_S3_ENDPOINT` | URL for your S3-compatible service. |
| `RUSTCHAT_S3_BUCKET` | The bucket name for file storage. |
| `RUSTCHAT_JWT_SECRET` | Secret key for signing session tokens. |
| `RUSTCHAT_SMTP_HOST` | Host for outgoing email notifications. |

---

## 5. Reverse Proxy Setup

We recommend using **Nginx**, **Traefik**, or **Caddy** for TLS termination and basic load balancing.

### Nginx Example Config
Crucially, ensure your proxy supports WebSockets:
```nginx
location /api/v1/ws {
    proxy_pass http://rustchat_backend;
    proxy_http_version 1.1;
    proxy_set_header Upgrade $http_upgrade;
    proxy_set_header Connection "Upgrade";
    proxy_set_header Host $host;
}
```

---

## 6. Maintenance & Backups

### Database Backups
Schedule daily `pg_dump` tasks for the PostgreSQL service. Store these backups off-site.

### File Storage Backups
If using MinIO or Ceph, leverage their built-in replication tools. For AWS S3, enable bucket versioning.

### Logs & Monitoring
RustChat outputs structured JSON logs. We recommend piping these into ELK (Elasticsearch, Logstash, Kibana) or Prometheus/Grafana for monitoring system health.
