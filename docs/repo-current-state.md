# Repo Current State

**Last updated:** 2026-06-16
**Version:** v0.4.0

> This document describes the state of the repository as of its last update. For live issue tracking see GitHub Issues.

---

## 1. Version

**Current:** v0.4.0

Version is synchronized across two files:
- `backend/Cargo.toml` → `[package] version`
- `frontend/package.json` → `"version"`

To check: `grep '^version' backend/Cargo.toml` or `jq .version frontend/package.json`

Versioning follows semver. Releases are cut by pushing a `v*` tag — the `release.yml` workflow auto-generates a GitHub Release with changelog.

---

## 2. Services

| Service | Path | Port (default) | Status |
|---|---|---|---|
| Backend API | `backend/` | 3000 | ✅ Active |
| Frontend SPA | `frontend/` | 5173 (dev) / 8080 (via Nginx) | ✅ Active |
| Push Proxy | `push-proxy/` | 8065 | ✅ Active |
| Nginx proxy | `frontend/nginx.conf` | 8080 | ✅ Active (Docker) |

**External services required:**
- PostgreSQL 16+ (port 5432)
- Redis 7+ (port 6379)
- S3-compatible storage (port 9000 for RustFS)

For local setup see `docs/running_environment.md`.

---

## 3. Compatibility Status

- **Mobile-critical endpoints:** 41/41 previously tracked endpoints implemented
- **Last audited:** 2026-05-22 by repository inspection
- **Notes:** `POST /api/v4/emoji` and `POST /api/v4/posts/search` are now implemented. Post search is currently a pragmatic `ILIKE` implementation, not full Mattermost advanced search parity.

For details see `docs/compatibility-scope.md`.

---

## 4. Known Gaps

| Gap | Area | Priority |
|---|---|---|
| Advanced post search semantics beyond simple `ILIKE` | compat/search | Phase 2 |
| Plugin upload/install/enable/disable/remove flows are compatibility stubs | compat/plugins | Phase 2 |
| LDAP and SAML v4 endpoints are compatibility stubs | compat/enterprise | Phase 2 |
| No unit/component test framework for frontend | testing | Low |
| Approach C (CI enforcement) not implemented | governance | Deferred |

---

## 5. Active Work Streams

For live in-flight work see [GitHub Issues](https://github.com/rustchatio/rustchat/issues).

**Recently completed:**

| Phase | Description | Date |
|---|---|---|
| Phase 1: Entity Foundation | Entity registration, API keys, rate limiting, WebSocket JWT expiry, mobile compat audit | 2026-03-17 |
| Governance Layer | `.governance/` policy files, CODEOWNERS, PR template, issue forms, branch protection, GitHub labels | 2026-03-22 |
| Foundation Docs | 7 structured docs consolidating existing flat docs | 2026-03-22 |

---

## 6. Quick Start

```bash
# Clone
git clone https://github.com/rustchatio/rustchat
cd rustchat

# Start infrastructure
docker compose up -d postgres redis rustfs

# Backend
cd backend
cargo run

# Frontend (separate terminal)
cd frontend
npm install
npm run dev
```

For full environment setup (env vars, config) see `docs/running_environment.md`.
