# RustChat Tools

This directory contains utility scripts for RustChat deployment and maintenance.

## Available Tools

### `generate-env.sh` - Environment Generator

Generates a secure `.env` file with cryptographically strong random secrets.

**Usage:**
```bash
./tools/generate-env.sh
```

**Features:**
- Generates 48-byte base64 secrets for JWT and encryption keys
- Generates S3 access and secret keys shared between backend (`RUSTCHAT_S3_*`) and bundled RustFS (`RUSTFS_*`)
- Generates TURN server credentials
- Warns before overwriting existing `.env` file
- Creates automatic backup when overwriting
- Sets secure file permissions (600)

**Generated Secrets:**
| Variable | Size | Format |
|----------|------|--------|
| `RUSTCHAT_JWT_SECRET` | 48 bytes | base64 |
| `RUSTCHAT_ENCRYPTION_KEY` | 48 bytes | base64 |
| `RUSTCHAT_S3_ACCESS_KEY` | 16 bytes | hex |
| `RUSTCHAT_S3_SECRET_KEY` | 48 bytes | base64 |
| `RUSTFS_ACCESS_KEY` | 16 bytes | hex (same value as `RUSTCHAT_S3_ACCESS_KEY`) |
| `RUSTFS_SECRET_KEY` | 48 bytes | base64 (same value as `RUSTCHAT_S3_SECRET_KEY`) |
| `TURN_SERVER_USERNAME` | 16 bytes | hex |
| `TURN_SERVER_CREDENTIAL` | 48 bytes | base64 |

**Example:**
```bash
# First time setup
cd /path/to/rustchat
./tools/generate-env.sh
# Type "yes" when prompted

# Review and edit
nano .env

# Start RustChat
docker-compose up -d
```

### `backup-postgres.sh` - PostgreSQL Backup

Creates a compressed PostgreSQL dump from the Compose-managed `postgres` service.

**Usage:**
```bash
./tools/backup-postgres.sh
# or choose a path
./tools/backup-postgres.sh backups/rustchat.sql.gz
```

By default the script uses `docker-compose.prod.yml`. Override with `COMPOSE_FILE=docker-compose.yml` for local development.

### `restore-postgres.sh` - PostgreSQL Restore

Restores a compressed PostgreSQL dump into the Compose-managed `postgres` service. This replaces the existing database, so it requires an explicit confirmation environment variable.

**Usage:**
```bash
RUSTCHAT_RESTORE_CONFIRM=YES ./tools/restore-postgres.sh backups/rustchat.sql.gz
```

## Security Notes

- Never commit `.env` files to version control
- Keep backups of your `.env` file in a secure location
- Rotate secrets periodically (e.g., every 90 days)
- Use different secrets for different environments (dev/staging/prod)
