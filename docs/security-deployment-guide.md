# Security Deployment Guide

This guide covers production deployment security hardening for self-hosted RustChat instances. Follow these steps before exposing RustChat to the internet.

## Target Audience

System administrators and operators deploying RustChat in production environments.

---

## TLS/HTTPS Configuration

RustChat does not terminate TLS itself. Run it behind a reverse proxy (nginx, Caddy, Traefik, or cloud load balancer).

### Reverse Proxy Setup

1. **Enforce HTTPS at the edge.** The proxy handles TLS; RustChat runs internally on HTTP.
2. **Set `RUSTCHAT_SITE_URL` to the HTTPS URL.** Even though RustChat runs on HTTP internally, this URL is what browsers see.
   ```bash
   RUSTCHAT_SITE_URL=https://chat.example.com
   ```
3. **Forward real client IPs.** Configure your proxy to pass `X-Forwarded-For` and `X-Forwarded-Proto` headers so RustChat can enforce origin checks correctly.
4. **Disable direct HTTP access.** Firewall rules should block external traffic to the RustChat port (default `3000`). Only the reverse proxy should reach it.

### Recommended nginx Snippet

```nginx
server {
    listen 443 ssl http2;
    server_name chat.example.com;

    ssl_certificate /path/to/cert.pem;
    ssl_certificate_key /path/to/key.pem;
    ssl_protocols TLSv1.3;

    location / {
        proxy_pass http://127.0.0.1:3000;
        proxy_http_version 1.1;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
    }
}
```

---

## Secret Management

### Required Secrets

| Variable | Minimum Length | Purpose |
|----------|---------------|---------|
| `RUSTCHAT_JWT_SECRET` | 32 chars | Signs all JWT tokens |
| `RUSTCHAT_ENCRYPTION_KEY` | 32 bytes | Encrypts sensitive at-rest data |
| `RUSTCHAT_S3_SECRET_KEY` | Strong random | S3-compatible storage authentication |
| `RUSTFS_SECRET_KEY` | Strong random | RustFS root credentials |
| `PUSH_PROXY_AUTH_KEY` | 32 chars | Backend-to-push-proxy authentication |

### Generating Secrets

```bash
# JWT secret (high entropy)
openssl rand -base64 48

# 32-byte encryption key (must be exactly 32 bytes for AES-256-GCM)
openssl rand -base64 32

# S3 access/secret keys
openssl rand -hex 16  # access key
openssl rand -base64 32  # secret key
```

### Secret Rotation

- **JWT Secret:** Rotating this invalidates all active sessions. Plan rotation during maintenance windows. There is no built-in key versioning; a hard cutover is required.
- **Encryption Key:** Rotating this requires re-encrypting existing data. Back up the database before attempting rotation.
- **S3/RustFS Keys:** Rotate via your object storage admin console, then update `.env` and restart RustChat.

### Startup Validation

RustChat validates secrets on startup:

**In Production (`RUSTCHAT_ENVIRONMENT=production`):**
- Fails to start if secrets are weak
- Checks minimum length (32 chars)
- Checks entropy (no repeated patterns)
- Checks for common weak patterns

**In Development:**
- Logs warnings for weak secrets
- Allows startup with weak secrets

### Storage Best Practices

- Never commit `.env` or secret files.
- Use a secrets manager (HashiCorp Vault, AWS Secrets Manager, 1Password Secrets Automation) or Docker secrets in swarm mode.
- Set file permissions on `.env` to `0600`.

---

## Environment-Based Security Constraints

Set `RUSTCHAT_ENVIRONMENT=production` to enable strict validation:

```bash
RUSTCHAT_ENVIRONMENT=production
```

Effects in production mode:
- HTTP origins in `RUSTCHAT_CORS_ALLOWED_ORIGINS` are rejected (HTTPS only).
- Stricter request validation on authentication flows.
- Error responses may omit internal details.

---

## Rate Limiting Configuration

Enable rate limiting to protect against brute-force attacks:

```bash
RUSTCHAT_SECURITY_RATE_LIMIT_ENABLED=true
RUSTCHAT_SECURITY_RATE_LIMIT_AUTH_PER_MINUTE=10
RUSTCHAT_SECURITY_RATE_LIMIT_WS_PER_MINUTE=30
```

| Endpoint Class | Default | Guidance |
|----------------|---------|----------|
| Auth (login, register) | 10/min | Lower this if you do not expect high registration volume. |
| WebSocket connections | 30/min | Tuned for mobile clients that may reconnect frequently. |

If you run multiple backend instances behind a load balancer, ensure the rate limiter uses a shared Redis store (RustChat's existing Redis connection is used for this).

### Rate Limit Responses

When rate limited, the API returns:

```http
HTTP/1.1 429 Too Many Requests
X-RateLimit-Limit: 10
X-RateLimit-Remaining: 0
X-RateLimit-Reset: 1704067200
Retry-After: 45

{
  "error": {
    "code": "RATE_LIMIT_EXCEEDED",
    "message": "Too many requests. Please try again later.",
    "retry_after": 45
  }
}
```

---

## CORS and Origin Enforcement

Restrict CORS to exactly the origins that serve your RustChat frontend:

```bash
# Production: HTTPS only, comma-separated, no spaces
RUSTCHAT_CORS_ALLOWED_ORIGINS=https://chat.example.com,https://app.example.com
```

- Never use `*` in production.
- Do not include `http://` origins.
- If you embed RustChat in an iframe or mobile app WebView, add those exact origins.

---

## S3 Access Key Security

1. **Do not reuse default/demo credentials.** Generate unique keys per deployment.
2. **Scope bucket policies narrowly.** RustChat needs `PutObject`, `GetObject`, and `DeleteObject` only on its upload bucket.
3. **Enable bucket versioning or object locking** if regulatory requirements apply.
4. **Use separate credentials for RustFS and RustChat S3 access** when possible.
5. **Enable TLS on S3 endpoints.** If using RustFS, run it with a valid TLS certificate or access it through an internal TLS-terminating proxy.

---

## Database Security (PostgreSQL)

### Connection Encryption

1. **Enable SSL/TLS on PostgreSQL.** Set `ssl = on` in `postgresql.conf` and provide server certificates.
2. **Use `sslmode=require` or `verify-full` in the connection string:**
   ```bash
   RUSTCHAT_DATABASE_URL=postgres://rustchat:PASSWORD@db.example.com:5432/rustchat?sslmode=require
   ```
3. **Client certificate verification** (optional, high-security environments):
   ```bash
   RUSTCHAT_DATABASE_URL=postgres://rustchat@db.example.com:5432/rustchat?sslmode=verify-full&sslcert=/certs/client.crt&sslkey=/certs/client.key
   ```

### Database Hardening

- Create a dedicated PostgreSQL user with minimal privileges (no superuser, no createdb).
- Disable external network access to PostgreSQL; restrict to the RustChat backend subnet.
- Enable PostgreSQL logging for connection attempts and slow queries.
- Run `pg_hba.conf` with `md5` or `scram-sha-256` authentication, never `trust`.

---

## Redis Security

### Authentication

If your Redis instance supports AUTH, include the password in the URL:

```bash
RUSTCHAT_REDIS_URL=redis://:PASSWORD@redis.example.com:6379/0
```

### TLS

For Redis 6+ with TLS:

```bash
RUSTCHAT_REDIS_URL=rediss://:PASSWORD@redis.example.com:6379/0
```

Ensure your Redis server has `tls-port` and `tls-cert-file` configured.

### Network Hardening

- Bind Redis to a private interface or Unix socket.
- Disable the `FLUSHALL` and `CONFIG` commands via `rename-command` if they are not needed.
- Enable Redis ACLs to restrict the RustChat user to the minimum command set.

---

## File Upload Security

1. **Size Limits:** Set upload size limits at the reverse proxy level (nginx `client_max_body_size`) to prevent oversized uploads from reaching RustChat.
2. **Type Validation:** RustChat validates file MIME types and extensions. Do not disable this.
3. **Storage Isolation:** Uploaded files should reside in a dedicated S3 bucket, separate from other application data.
4. **Malware Scanning:** Consider integrating a virus scanner (ClamAV, commercial API) at the S3 ingress or via Lambda-style triggers.
5. **Content Security:** Serve uploaded files from a separate subdomain or with strict `Content-Disposition: attachment` headers to reduce XSS vectors.

---

## WebSocket Token Transport Security

**Disable query-string token transport in production:**

```bash
RUSTCHAT_SECURITY_WS_ALLOW_QUERY_TOKEN=false
```

When disabled, WebSocket connections **must** authenticate via the `Authorization` header. This prevents tokens from appearing in:
- Server access logs
- Browser history
- Referrer headers
- CDN/proxy caches

Ensure your reverse proxy forwards the `Authorization` header to WebSocket upgrade requests.

---

## OAuth Token Delivery

Only secure mode is supported: `RUSTCHAT_SECURITY_OAUTH_TOKEN_DELIVERY=cookie`.

- One-time exchange code in URL: `/login?code=xyz`
- Client exchanges code for token via `POST /api/v1/oauth2/exchange`
- Token never appears in redirect URLs

---

## Audit Logging

RustChat emits structured logs via `RUSTCHAT_LOG_LEVEL`. For security auditing:

1. **Set log level to `info` or `warn` minimum.**
   ```bash
   RUSTCHAT_LOG_LEVEL=info
   ```
2. **Forward logs to a SIEM or central log aggregator.** Export from journald, Docker logging driver, or file tailing.
3. **Key events to monitor:**
   - Failed login attempts (brute-force detection)
   - JWT validation failures (token replay or tampering)
   - Rate limit hits (DoS indicators)
   - Permission denied errors (privilege escalation attempts)
   - Admin actions (user deletions, role changes)
   - File upload/download anomalies

### Sample Promtail/Loki Labels

```yaml
- job_name: rustchat
  static_configs:
    - targets:
        - localhost
      labels:
        job: rustchat
        environment: production
        __path__: /var/log/rustchat/*.log
```

---

## Monitoring and Alerting

### Security Events to Monitor

| Event | Severity | Action |
|-------|----------|--------|
| Rate limit exceeded | Warning | Monitor for patterns |
| Invalid OAuth state | Warning | Possible CSRF attempt |
| Weak secret detected | Critical | Rotate immediately |
| WebSocket auth failure | Info | Normal for expired tokens |
| Exchange code reuse | Warning | Possible token theft |

### Log Redaction

Configure your reverse proxy to redact sensitive headers:

```nginx
# nginx - don't log tokens
log_format security '$remote_addr - $remote_user [$time_local] '
                    '"$request" $status $body_bytes_sent '
                    '"$http_referer" "$http_user_agent"';

access_log /var/log/nginx/access.log security;
```

---

## Backup Encryption

1. **Encrypt database backups at rest.** Use `pg_dump` with AES encryption or store backups in an encrypted object storage bucket.
   ```bash
   pg_dump -Fc rustchat | gpg --symmetric --cipher-algo AES256 -o rustchat-$(date +%F).dump.gpg
   ```
2. **Encrypt S3 bucket backups** with server-side encryption (SSE-S3, SSE-KMS, or client-side encryption).
3. **Store backup encryption keys separately** from the production environment.
4. **Test restore procedures** quarterly. An encrypted backup you cannot restore is worthless.
5. **Automate backup retention** with lifecycle policies. Keep encrypted off-site copies for disaster recovery.

---

## Quick Production Checklist

- [ ] `RUSTCHAT_ENVIRONMENT=production`
- [ ] `RUSTCHAT_SITE_URL` uses `https://`
- [ ] `RUSTCHAT_CORS_ALLOWED_ORIGINS` uses `https://` only
- [ ] `RUSTCHAT_SECURITY_WS_ALLOW_QUERY_TOKEN=false`
- [ ] `RUSTCHAT_SECURITY_OAUTH_TOKEN_DELIVERY=cookie`
- [ ] `RUSTCHAT_SECURITY_RATE_LIMIT_ENABLED=true`
- [ ] All secrets are unique, random, and >= 32 characters
- [ ] Reverse proxy enforces TLS 1.3 and forwards real IPs
- [ ] PostgreSQL uses SSL/TLS and dedicated user
- [ ] Redis uses AUTH and TLS (if networked)
- [ ] S3-compatible storage uses unique credentials and TLS
- [ ] `.env` file permissions are `0600`
- [ ] Backups are encrypted and tested
- [ ] Logs are forwarded to a central aggregator

---

## Migration from Insecure Defaults

If currently running with insecure defaults:

1. **Immediate (P0):** Rotate to strong secrets
2. **Week 1:** Update frontend to use header-based WebSocket auth
3. **Week 2:** Update frontend to support OAuth exchange codes
4. **Week 3:** Deploy with `OAUTH_TOKEN_DELIVERY=cookie`

Contact your security team if you need assistance with the migration.

For zero-trust architectural guidance, see [`docs/security-zero-trust-guide.md`](security-zero-trust-guide.md).
