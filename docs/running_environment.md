# Running the RustMatch Environment

RustMatch is containerized using Docker Compose for easy setup and development. The environment includes:

- **Backend**: Rust (Axum) API
- **Frontend**: Vue 3 + Vite (Served via Nginx)
- **Postgres**: Database
- **Redis**: Caching
- **MinIO**: S3-compatible object storage
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
    All services (`backend`, `frontend`, `postgres`, `redis`, `minio`, `createbuckets`) should be `Up` (or `Exited (0)` for `createbuckets`).

3.  **Access the Application:**

    - **Frontend:** [http://localhost:8080](http://localhost:8080)
    - **Backend API:** [http://localhost:3000](http://localhost:3000)
    - **MinIO Console:** [http://localhost:9001](http://localhost:9001) (User: `minioadmin`, Pass: `minioadmin`)
    - **Meilisearch:** [http://localhost:7700](http://localhost:7700) (if enabled)

## Development Mode

If you are actively developing code:

### Backend Development
You can run the backend locally while keeping infrastructure services (DB, Redis, MinIO) in Docker.
1.  Stop the `backend` container if running: `docker compose stop backend`
2.  Run cargo locally:
    ```bash
    cd backend
    cargo run
    ```
    *Note: Ensure your local `.env` file points to localhost ports for DB/Redis/MinIO.*

### Frontend Development
1.  Stop the `frontend` container if running: `docker compose stop frontend`
2.  Run npm locally:
    ```bash
    cd frontend
    npm run dev
    ```
    *Access at [http://localhost:5173](http://localhost:5173).*

## Troubleshooting

- **Database Connection Errors:** Ensure the `postgres` container is healthy (`docker compose ps`).
- **S3 Upload Failures:** Ensure the `createbuckets` service ran successfully. You can manually create the bucket in MinIO Console if needed.
- **Rebuild:** If you change dependencies or Dockerfiles, force a rebuild:
    ```bash
    docker compose up --build -d
    ```
