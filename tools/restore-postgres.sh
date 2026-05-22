#!/usr/bin/env sh
set -eu

COMPOSE_FILE="${COMPOSE_FILE:-docker-compose.prod.yml}"
BACKUP_FILE="${1:-}"

if [ -z "$BACKUP_FILE" ]; then
  echo "Usage: $0 path/to/rustchat-postgres.sql.gz" >&2
  exit 2
fi

if [ ! -f "$BACKUP_FILE" ]; then
  echo "Backup file not found: $BACKUP_FILE" >&2
  exit 2
fi

if [ "${RUSTCHAT_RESTORE_CONFIRM:-}" != "YES" ]; then
  echo "This will replace the rustchat PostgreSQL database."
  echo "Re-run with RUSTCHAT_RESTORE_CONFIRM=YES to continue." >&2
  exit 2
fi

echo "Stopping backend before restore"
docker compose -f "$COMPOSE_FILE" stop backend

restart_backend() {
  docker compose -f "$COMPOSE_FILE" up -d backend >/dev/null 2>&1 || true
}
trap restart_backend EXIT

echo "Recreating database"
docker compose -f "$COMPOSE_FILE" exec -T postgres dropdb -U rustchat --if-exists rustchat
docker compose -f "$COMPOSE_FILE" exec -T postgres createdb -U rustchat rustchat

echo "Restoring $BACKUP_FILE"
gunzip -c "$BACKUP_FILE" | docker compose -f "$COMPOSE_FILE" exec -T postgres psql -U rustchat -d rustchat

echo "Restore complete; restarting backend"
