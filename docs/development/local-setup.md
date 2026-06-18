# Running the RustChat Environment

> **Note:** For the comprehensive development guide (tools, commands, troubleshooting), see [Development Guide](../development.md). This page covers Docker-based setup specifically.

RustChat is containerized using Docker Compose for easy setup and development. The environment includes:

- **Backend**: Rust (Axum) API
- **Frontend**: Vue 3 + Vite (Served via Nginx)
- **Postgres**: Database
- **Redis**: Caching
- **RustFS**: S3-compatible object storage
- **Meilisearch**: (Optional) Full-text search engine

## Prerequisites

- [Docker](https://docs.docker.com/get-docker/) installed.
- [Docker Compose](https://docs.docker.com/compose/install/) installed (usually included with Docker Desktop).

## Quick Start

1.  **Build and Start Services:**
    Run the following command in the project root to build the backend and frontend images and start all services:
    ```bash
    docker compose up --build -d
    ```

    *The `-d` flag runs containers in detached mode (background).*

2.  **Verify Services:**
    Check the status of the containers:
    ```bash
    docker compose ps
    ```
    All services (`backend`, `frontend`, `postgres`, `redis`, `rustfs`) should be `Up`.

3.  **Access the Application:**

    - **Frontend:** [http://localhost:8080](http://localhost:8080)
    - **Backend API:** [http://localhost:3000](http://localhost:3000)
    - **RustFS Console:** [http://localhost:9001](http://localhost:9001) (use your `RUSTFS_ACCESS_KEY` / `RUSTFS_SECRET_KEY`)
    - **Meilisearch:** [http://localhost:7700](http://localhost:7700) (if enabled)

## Development Mode

If you are actively developing code:

### Backend Development
You can run the backend locally while keeping infrastructure services (DB, Redis, RustFS) in Docker.
1.  Stop the `backend` container if running: `docker compose stop backend`
2.  Run cargo locally:
    ```bash
    cd backend
    cargo run
    ```
    *Note: Ensure your local `.env` file points to localhost ports for DB/Redis/RustFS.*

### Frontend Development
1.  Stop the `frontend` container if running: `docker compose stop frontend`
2.  Run npm locally:
    ```bash
    cd frontend
    npm run dev
    ```
    *Access at [http://localhost:5173](http://localhost:5173).*

## Security Modes (Dev vs Prod)

RustChat changes behavior based on `RUSTCHAT_ENVIRONMENT`. The default is `production`.

- `production` (default): CORS is deny-by-default unless `RUSTCHAT_CORS_ALLOWED_ORIGINS` is explicitly set.
- `development`: CORS is still deny-by-default; set `RUSTCHAT_ALLOW_DEV_CORS=true` to enable permissive CORS for local development only.

For local development, either:

- Set `RUSTCHAT_ALLOW_DEV_CORS=true` (never enable this in production), or
- Configure `RUSTCHAT_CORS_ALLOWED_ORIGINS` with the exact origins your frontend uses, for example `http://localhost:8080,http://localhost:5173`.

Recommended production settings:

- Set `RUSTCHAT_ENVIRONMENT=production`
- Set `RUSTCHAT_CORS_ALLOWED_ORIGINS` to your exact frontend origins (comma-separated)
- Use strong secrets for `RUSTCHAT_JWT_SECRET` and `RUSTCHAT_ENCRYPTION_KEY`
- Terminate TLS at the reverse proxy/load balancer (HTTPS at the edge)
- Use encrypted SSO client secrets (stored via Admin UI/API)
- Set TURN credentials explicitly if `TURN_SERVER_ENABLED=true`
- Query-token compatibility is removed; URL/header OAuth token delivery and query-string WebSocket tokens are rejected at startup
- If `RUSTCHAT_SITE_URL` is set in production, it must use `https://`; `RUSTCHAT_CORS_ALLOWED_ORIGINS` entries must also be `https://` only.

## Troubleshooting

- **Database Connection Errors:** Ensure the `postgres` container is healthy (`docker compose ps`).
- **S3 Upload Failures:** The backend creates the upload bucket automatically on startup. If uploads still fail, check that the RustFS service is healthy and that the `RUSTFS_ACCESS_KEY` and `RUSTFS_SECRET_KEY` in `.env` match the `RUSTCHAT_S3_ACCESS_KEY` and `RUSTCHAT_S3_SECRET_KEY`.
- **Rebuild:** If you change dependencies or Dockerfiles, force a rebuild:
    ```bash
    docker compose up --build -d
    ```
