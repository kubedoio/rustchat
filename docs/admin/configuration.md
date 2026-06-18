# Configuration Reference

Complete reference for RustChat configuration options.

## Environment Variables

All configuration is done via environment variables with the `RUSTCHAT_` prefix.

### Core Settings

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `RUSTCHAT_ENVIRONMENT` | No | `production` | Runtime mode: `development` or `production` |
| `RUSTCHAT_SITE_URL` | Yes | - | Public URL of your RustChat instance |
| `RUSTCHAT_JWT_SECRET` | Yes | - | Secret key for JWT signing (min 32 chars) |
| `RUSTCHAT_JWT_ISSUER` | Yes | - | JWT issuer claim |
| `RUSTCHAT_JWT_AUDIENCE` | Yes | - | JWT audience claim |
| `RUSTCHAT_ENCRYPTION_KEY` | Yes | - | 32-byte key for sensitive data encryption |

### Database

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `RUSTCHAT_DATABASE_URL` | Yes | - | PostgreSQL connection string |

Example:
```bash
RUSTCHAT_DATABASE_URL=postgres://user:password@localhost:5432/rustchat
```

### Redis

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `RUSTCHAT_REDIS_URL` | Yes | - | Redis connection string |

Example:
```bash
RUSTCHAT_REDIS_URL=redis://localhost:6379/
```

### File Storage (S3-compatible)

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `RUSTCHAT_S3_ENDPOINT` | Yes | - | S3 endpoint URL |
| `RUSTCHAT_S3_BUCKET` | Yes | - | S3 bucket name |
| `RUSTCHAT_S3_ACCESS_KEY` | Yes | - | S3 access key |
| `RUSTCHAT_S3_SECRET_KEY` | Yes | - | S3 secret key |
| `RUSTCHAT_S3_REGION` | No | `us-east-1` | S3 region |
| `RUSTCHAT_S3_PUBLIC_ENDPOINT` | No | - | Public endpoint used when file URLs must be rewritten for clients |

### CORS

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `RUSTCHAT_CORS_ALLOWED_ORIGINS` | Production | - | Comma-separated allowed origins |
| `RUSTCHAT_ALLOW_DEV_CORS` | No | `false` | **Danger:** enables permissive CORS (`*`). Only for local development. |

Example:
```bash
RUSTCHAT_CORS_ALLOWED_ORIGINS=https://chat.example.com,https://app.example.com
```

Cross-origin requests are denied by default. For local development you must either set `RUSTCHAT_ALLOW_DEV_CORS=true` (never in production) or configure `RUSTCHAT_CORS_ALLOWED_ORIGINS` with the exact origins your frontend uses (e.g. `http://localhost:8080,http://localhost:5173`).

### Email (SMTP)

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `RUSTCHAT_SMTP_HOST` | No | - | SMTP server hostname |
| `RUSTCHAT_SMTP_PORT` | No | `587` | SMTP server port |
| `RUSTCHAT_SMTP_USER` | No | - | SMTP username |
| `RUSTCHAT_SMTP_PASSWORD` | No | - | SMTP password |
| `RUSTCHAT_SMTP_FROM` | No | - | From address for emails |
| `RUSTCHAT_SMTP_TLS` | No | `true` | Use TLS for SMTP |

### Push Notifications

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `RUSTCHAT_PUSH_PROXY_URL` | No | - | URL of push proxy service |

### Security

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `RUSTCHAT_SECURITY_OAUTH_TOKEN_DELIVERY` | No | `cookie` | OAuth token delivery. Only `cookie` is accepted. |
| `RUSTCHAT_SECURITY_RATE_LIMIT_ENABLED` | No | `true` | Enable rate limiting |
| `RUSTCHAT_SECURITY_RATE_LIMIT_AUTH_PER_MINUTE` | No | `10` | Auth endpoint rate limit |
| `RUSTCHAT_SECURITY_RATE_LIMIT_WS_PER_MINUTE` | No | `30` | WebSocket rate limit |

### Logging

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `RUST_LOG` | No | `info` | Log level (error, warn, info, debug, trace) |
| `RUSTCHAT_LOG_FORMAT` | No | `json` | Log format: `json` or `pretty` |

## Production Configuration

For production deployments, ensure:

```bash
# Environment
RUSTCHAT_ENVIRONMENT=production

# Security
RUSTCHAT_JWT_SECRET=<long-random-string-min-32-chars>
RUSTCHAT_ENCRYPTION_KEY=<32-byte-key>
RUSTCHAT_CORS_ALLOWED_ORIGINS=https://your-domain.com

# Security hardening
RUSTCHAT_SECURITY_OAUTH_TOKEN_DELIVERY=cookie
RUSTCHAT_SECURITY_RATE_LIMIT_ENABLED=true
```

## Docker Compose Example

See [Installation Guide](./installation.md) for complete Docker Compose configuration.

## Configuration Validation

The backend validates configuration at startup and will fail fast with clear error messages if required variables are missing or invalid.

---

*For security best practices: See [Security Guide](./security.md)*
