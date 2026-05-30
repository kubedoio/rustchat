#!/usr/bin/env sh
set -eu

COMPOSE_FILE="${COMPOSE_FILE:-docker-compose.prod.yml}"
BACKUP_DIR="${BACKUP_DIR:-backups}"
TIMESTAMP="$(date +%Y%m%d-%H%M%S)"
OUTPUT="${1:-${BACKUP_DIR}/rustchat-postgres-${TIMESTAMP}.sql.gz}"

mkdir -p "$(dirname "$OUTPUT")"

echo "Creating PostgreSQL backup with ${COMPOSE_FILE}"
docker compose -f "$COMPOSE_FILE" exec -T postgres pg_dump -U rustchat rustchat | gzip > "$OUTPUT"

echo "Backup written to $OUTPUT"
