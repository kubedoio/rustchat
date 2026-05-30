# Compatibility Scope

This page is the canonical entry point for Mattermost compatibility expectations.

## Compatibility Commitment

RustChat prioritizes compatibility with Mattermost mobile and desktop clients on `/api/v4/*` and `/api/v4/websocket`.

## Current Headline Coverage

| Scope | Coverage | Last Audited |
|---|---:|---|
| Mobile-critical endpoints | 41/41 implemented | 2026-05-22 |

Previously tracked gaps now implemented:
- `POST /api/v4/emoji`
- `POST /api/v4/posts/search`

Known limitations:
- Post search is currently a basic message search implementation, not full Mattermost advanced search parity.
- Plugin, LDAP, SAML, compliance, and some enterprise/admin APIs remain compatibility stubs or intentionally scoped out.

## Protected Compatibility Surface

Changes in these paths should be treated as compatibility-sensitive:
- `backend/src/api/v4/`
- `backend/src/mattermost_compat/`
- `backend/compat/`
- `backend/src/realtime/`

## Verification Expectations

When compatibility-sensitive changes are made:

```bash
# Compatibility workflow
cd backend
cargo test --no-fail-fast -- --nocapture
```

Also review compatibility guidance in:
- [Development Compatibility Guide](./development/compatibility.md)
- [Reference Compatibility Matrix](./reference/compatibility-matrix.md)
