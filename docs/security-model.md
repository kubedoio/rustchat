# Security Model

## Authentication and Sessions

RustChat uses JWT tokens for authentication:

- Tokens are signed with `RUSTCHAT_JWT_SECRET` (HS256)
- Token expiry is configurable (default 24 hours)
- Passwords are hashed with Argon2id
- OAuth/OIDC SSO is supported via configurable providers. SAML and LDAP compatibility endpoints are present, but backend operations are not implemented in this build.

### WebSocket Authentication

WebSocket connections accept tokens via:
- `Authorization: Bearer <token>` header (preferred)
- `Sec-WebSocket-Protocol` header fallback

Query-string token transport is rejected.

### Session Storage

JWTs authenticate requests. Redis stores realtime connection state, presence, and rate-limit state. PostgreSQL is the source of truth for users, teams, and permissions.

## Authorization

Role-based access control (RBAC) is implemented:

- `system_admin` — Full platform access
- `team_admin` — Team-level administration
- `channel_admin` — Channel-level administration
- Standard user roles

Permissions are checked at the API handler layer before reaching services.

## Dependency Security Process

- **Dependabot** monitors Cargo, npm, GitHub Actions, and Docker dependencies
- **cargo audit** runs in CI on every push and weekly to detect known vulnerable crates
- **npm audit** runs in CI for frontend dependencies
- **CodeQL** static analysis runs for Rust and JavaScript
- Security-related dependency updates are treated as high priority

## Vulnerability Reporting

**Do not open public issues for security vulnerabilities.**

Report privately via GitHub Security Advisories or contact maintainers directly. See `SECURITY.md` for full instructions.

## Supported Versions

RustChat is currently pre-1.0 and under active development.

| Version | Supported |
|---------|-----------|
| `main` (current development) | Yes |
| Latest tagged release | Yes |
| Older releases | No |

Before 1.0, minor version bumps may include breaking changes. Review `CHANGELOG.md` before upgrading.

## Nightly Stability Disclaimer

Nightly container images are built automatically from `main`. They are not guaranteed to be stable and should be used for testing only.

## Production Hardening Checklist

- [ ] Set `RUSTCHAT_ENVIRONMENT=production`
- [ ] Generate strong secrets (≥32 chars, high entropy, unique per secret)
- [ ] Set explicit `RUSTCHAT_CORS_ALLOWED_ORIGINS` (no wildcards)
- [ ] Use `RUSTCHAT_SECURITY_OAUTH_TOKEN_DELIVERY=cookie`
- [ ] Keep WebSocket tokens out of query strings; query-token transport is rejected by the server
- [ ] Enable rate limiting (on by default)
- [ ] Terminate TLS at a reverse proxy
- [ ] Forward `X-Forwarded-For` for accurate rate limiting
- [ ] Configure security headers (HSTS, X-Frame-Options, X-Content-Type-Options)

See [Security Deployment Guide](security-deployment-guide.md) for a complete hardening guide.
