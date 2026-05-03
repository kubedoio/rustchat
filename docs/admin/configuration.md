# Configuration Reference

Complete reference for RustChat configuration options.

## Environment Variables

All configuration is done via environment variables with the `RUSTCHAT_` prefix.

### Core Settings

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `RUSTCHAT_ENVIRONMENT` | No | `development` | Runtime mode: `development` or `production` |
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
| `RUSTCHAT_S3_PUBLIC_URL` | No | - | Public URL for file access |

> **Private bucket by design**
>
> RustChat stores uploaded files in a **private** S3/RustFS bucket by default. Uploaded objects are not
> accessible via direct object-store URLs — attempting to fetch them without authentication will return a
> `403 Forbidden` response. This is intentional: all file access is proxied through the RustChat backend,
> which validates the requester's session before serving the object. This design keeps files scoped to
> their workspace and prevents unauthorised access.
>
> **Do not change the bucket to public.** Making the bucket public removes this protection and exposes all
> uploaded files — including attachments in private channels — to anyone who knows or guesses an object URL.
>
> If you need to serve files via a CDN or reverse-proxy, configure it to authenticate with your RustChat
> backend rather than bypassing it by pointing directly at the object store.

### CORS

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `RUSTCHAT_CORS_ALLOWED_ORIGINS` | Production | - | Comma-separated allowed origins |

Example:
```bash
RUSTCHAT_CORS_ALLOWED_ORIGINS=https://chat.example.com,https://app.example.com
```

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
| `RUSTCHAT_SECURITY_WS_ALLOW_QUERY_TOKEN` | No | `true` | Allow WebSocket auth via query param |
| `RUSTCHAT_SECURITY_OAUTH_TOKEN_DELIVERY` | No | `header` | OAuth token delivery: `header` or `cookie` |
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
RUSTCHAT_SECURITY_WS_ALLOW_QUERY_TOKEN=false
RUSTCHAT_SECURITY_OAUTH_TOKEN_DELIVERY=cookie
RUSTCHAT_SECURITY_RATE_LIMIT_ENABLED=true
```

## Docker Compose Example

See [Installation Guide](./installation.md) for complete Docker Compose configuration.

## Configuration Validation

The backend validates configuration at startup and will fail fast with clear error messages if required variables are missing or invalid.

---

*For security best practices: See [Security Guide](./security.md)*
