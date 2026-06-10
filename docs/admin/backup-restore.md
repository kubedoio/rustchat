# Backup and Restore

This guide covers backing up and restoring RustChat data.

## What to Back Up

1. **PostgreSQL Database** - All application data
2. **S3-compatible File Storage (RustFS)** - User uploads
3. **Configuration** - Environment variables, secrets

## Database Backup

### Automated Daily Backups

Create a backup script:

```bash
#!/bin/bash
# /opt/rustchat/scripts/backup.sh

BACKUP_DIR="/backups/rustchat"
DATE=$(date +%Y%m%d_%H%M%S)
DB_NAME="rustchat"
DB_USER="rustchat"
DB_HOST="localhost"

# Create backup directory
mkdir -p $BACKUP_DIR

# Backup database
pg_dump -h $DB_HOST -U $DB_USER -d $DB_NAME \
    -F custom -f "$BACKUP_DIR/rustchat_db_$DATE.dump"

# Compress
gzip "$BACKUP_DIR/rustchat_db_$DATE.dump"

# Keep only last 7 days
find $BACKUP_DIR -name "rustchat_db_*.dump.gz" -mtime +7 -delete

echo "Backup completed: rustchat_db_$DATE.dump.gz"
```

Add to crontab:
```bash
# Daily at 2 AM
0 2 * * * /opt/rustchat/scripts/backup.sh >> /var/log/rustchat-backup.log 2>&1
```

### Docker Compose Backup

```bash
# Create backup
docker compose exec postgres pg_dump -U rustchat rustchat > backup_$(date +%Y%m%d).sql

# Compress
gzip backup_*.sql
```

## File Storage Backup

### S3-compatible Backup (RustFS)

If using RustFS or another S3-compatible store:

```bash
# Mirror bucket to backup location
# Using rclone with an S3 remote configured for RustFS
rclone sync s3:rustchat-files /backups/rustchat-files
```

### AWS S3 Backup

```bash
# Enable versioning (recommended)
aws s3api put-bucket-versioning \
    --bucket rustchat-files \
    --versioning-configuration Status=Enabled

# Cross-region replication for disaster recovery
# Configure replication rules in S3 console
```

## Restore from Backup

### Database Restore

```bash
# Stop application
docker compose stop backend

# Restore database
docker compose exec -T postgres psql -U rustchat < backup_20240101.sql

# Or with pg_restore for custom format
docker compose exec postgres pg_restore \
    -U rustchat -d rustchat --clean --if-exists \
    /backups/rustchat_db_20240101.dump

# Start application
docker compose start backend
```

### File Storage Restore

```bash
# Restore from backup
# Using rclone with an S3 remote configured for RustFS
rclone sync /backups/rustchat-files s3:rustchat-files
```

## Disaster Recovery

### Full System Restore

1. **Prepare new server**
   - Install Docker, Docker Compose
   - Restore configuration files

2. **Restore database**
   ```bash
   # Start just the database
   docker compose up -d postgres
   
   # Wait for it to be ready
   sleep 10
   
   # Restore backup
   docker compose exec -T postgres psql -U rustchat < backup.sql
   ```

3. **Restore file storage**
   ```bash
   mc mirror /backups/rustchat-files rustchat/rustchat-files
   ```

4. **Start full stack**
   ```bash
   docker compose up -d
   ```

## Backup Verification

Test your backups regularly:

```bash
# Test restore to temporary database
docker run --rm -d --name test-postgres -e POSTGRES_PASSWORD=test pgvector/pgvector:pg16
sleep 5
docker exec -i test-postgres psql -U postgres -c "CREATE DATABASE rustchat_test;"
docker exec -i test-postgres psql -U postgres rustchat_test < backup.sql

# Verify
docker exec test-postgres psql -U postgres rustchat_test -c "SELECT COUNT(*) FROM users;"

# Cleanup
docker stop test-postgres
docker rm test-postgres
```

## Backup Checklist

- [ ] Daily automated database backups
- [ ] Regular file storage backups
- [ ] Encrypted backup storage
- [ ] Off-site backup copies
- [ ] Backup verification tests
- [ ] Documented restore procedures
- [ ] Tested disaster recovery plan

---

*For monitoring: See [Operations Guide](../operations/runbook.md)*
