#!/usr/bin/env bash
set -euo pipefail

# One-command development environment setup for RustChat.
# Usage: ./scripts/dev-setup.sh [options]
#
# Options:
#   --check-only    Verify prerequisites without modifying anything
#   --verbose       Show all commands being executed
#   --help          Show this help message
#
# What this script does:
#   1. Checks prerequisites (Docker, Docker Compose, Rust, Node.js)
#   2. Creates .env from .env.example if missing
#   3. Starts infrastructure services (postgres, redis, rustfs)
#   4. Runs database migrations
#   5. Installs frontend dependencies

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

cd "${PROJECT_ROOT}"

# Parse arguments
CHECK_ONLY=false
VERBOSE=false

for arg in "$@"; do
  case "$arg" in
    --check-only)
      CHECK_ONLY=true
      ;;
    --verbose)
      VERBOSE=true
      set -x
      ;;
    --help)
      echo "Usage: $(basename "$0") [options]"
      echo ""
      echo "Options:"
      echo "  --check-only    Verify prerequisites without modifying anything"
      echo "  --verbose       Show all commands being executed"
      echo "  --help          Show this help message"
      echo ""
      echo "Example:"
      echo "  $(basename "$0")              # Full setup"
      echo "  $(basename "$0") --check-only # Verify only"
      exit 0
      ;;
    *)
      echo "Unknown option: $arg"
      echo "Run '$(basename "$0") --help' for usage."
      exit 1
      ;;
  esac
done

echo "=== RustChat Dev Setup ==="
echo ""

# Check prerequisites
echo "--- Checking Prerequisites ---"

command -v docker >/dev/null 2>&1 || { echo "ERROR: Docker is required. Install from https://docs.docker.com/get-docker/"; exit 1; }
command -v docker compose >/dev/null 2>&1 || { echo "ERROR: Docker Compose is required. Install from https://docs.docker.com/compose/install/"; exit 1; }

if command -v rustc >/dev/null 2>&1; then
  RUST_VERSION=$(rustc --version)
  echo "Rust: ${RUST_VERSION}"
  # Check minimum version (1.95+)
  RUST_NUM=$(echo "$RUST_VERSION" | sed 's/rustc //' | cut -d. -f1,2 | tr -d '.')
  if [[ "$RUST_NUM" -lt "195" ]]; then
    echo "WARNING: Rust 1.95+ required. Current: ${RUST_VERSION}"
    echo "  Update with: rustup update"
  fi
else
  echo "WARNING: Rust not found. Install from https://rustup.rs/"
fi

if command -v node >/dev/null 2>&1; then
  NODE_VERSION=$(node --version)
  echo "Node.js: ${NODE_VERSION}"
  # Check minimum version (24+)
  NODE_MAJOR=$(echo "$NODE_VERSION" | sed 's/v//' | cut -d. -f1)
  if [[ "$NODE_MAJOR" -lt "24" ]]; then
    echo "WARNING: Node.js 24+ required. Current: ${NODE_VERSION}"
    echo "  Update with: nvm use 24  (or install from https://nodejs.org/)"
  fi
else
  echo "WARNING: Node.js not found. Install from https://nodejs.org/"
fi

echo "Docker: $(docker --version)"
echo "Docker Compose: $(docker compose version)"
echo ""

if [[ "$CHECK_ONLY" == true ]]; then
  echo "=== Check complete (no changes made) ==="
  exit 0
fi

# Environment file
echo "--- Environment Configuration ---"
if [[ -f .env ]]; then
  echo ".env already exists. Leaving it untouched."
  echo "  (Delete it and re-run if you want a fresh .env from .env.example)"
else
  cp .env.example .env
  echo "Created .env from .env.example"
  echo ""
  echo "⚠️  IMPORTANT: Edit .env and set required secrets before starting services:"
  echo ""
  echo "  RUSTCHAT_JWT_SECRET"
  echo "  RUSTCHAT_ENCRYPTION_KEY"
  echo "  RUSTCHAT_S3_ACCESS_KEY / RUSTCHAT_S3_SECRET_KEY"
  echo "  RUSTFS_ACCESS_KEY / RUSTFS_SECRET_KEY"
  echo "  RUSTCHAT_ADMIN_USER / RUSTCHAT_ADMIN_PASSWORD"
  echo ""
  echo "  Generate secrets with: openssl rand -hex 32"
fi
echo ""

# Start dependencies
echo "--- Starting Dependencies ---"
docker compose up -d postgres redis rustfs
echo ""

# Wait for Postgres
echo "--- Waiting for PostgreSQL ---"
for i in {1..30}; do
  if docker compose exec -T postgres pg_isready -U rustchat >/dev/null 2>&1; then
    echo "PostgreSQL is ready"
    break
  fi
  if [[ $i -eq 30 ]]; then
    echo "WARNING: PostgreSQL did not become ready within 30 seconds"
    echo "  Check: docker compose logs postgres"
  fi
  sleep 1
done
echo ""

# Backend setup
echo "--- Backend Setup ---"
if command -v cargo >/dev/null 2>&1; then
  cd backend

  # Check for sqlx-cli
  if command -v sqlx >/dev/null 2>&1; then
    echo "Running migrations..."
    if sqlx migrate run 2>/dev/null; then
      echo "Migrations: OK"
    else
      echo "WARNING: Migrations failed. Ensure DATABASE_URL is set in .env"
      echo "  Try: export DATABASE_URL=postgres://rustchat:rustchat@localhost:5432/rustchat"
      echo "  Then: cd backend && sqlx migrate run"
    fi
  else
    echo "WARNING: sqlx-cli not found."
    echo "  Install with: cargo install sqlx-cli --no-default-features --features postgres"
  fi

  if cargo check 2>/dev/null; then
    echo "Backend check: OK"
  else
    echo "Backend check: failed (may need dependencies running or sqlx query data)"
    echo "  Try: cargo sqlx prepare"
  fi
  cd "${PROJECT_ROOT}"
else
  echo "Skipping backend setup (Rust not installed)"
fi
echo ""

# Frontend setup
echo "--- Frontend Setup ---"
if command -v npm >/dev/null 2>&1; then
  cd frontend
  if [[ ! -d node_modules ]]; then
    npm ci --ignore-scripts
    npm run apply:dependency-patches
  else
    echo "node_modules already exists. Skipping npm ci."
  fi
  if npm run build 2>/dev/null; then
    echo "Frontend build: OK"
  else
    echo "Frontend build: failed (check node version and patches)"
    echo "  Try: rm -rf node_modules && npm ci --ignore-scripts && npm run apply:dependency-patches"
  fi
  cd "${PROJECT_ROOT}"
else
  echo "Skipping frontend setup (Node.js not installed)"
fi
echo ""

echo "=== Setup Complete ==="
echo ""
echo "Next steps:"
echo "  1. Edit .env and set required secrets (if you haven't already)"
echo "  2. Start backend:  cd backend && cargo run"
echo "  3. Start frontend: cd frontend && npm run dev"
echo "  4. Or run everything in Docker: docker compose up -d --build"
echo ""
echo "Quick verification:"
echo "  docker compose ps"
echo "  curl -s http://localhost:3000/api/v1/health/live | jq ."
