# Compatibility Scope

**Last updated:** 2026-05-22
**Source docs consolidated:** `docs/MATTERMOST_CLIENTS.md`, `docs/reference/compatibility-matrix.md`

---

## 1. Compatibility Commitment

rustchat commits to supporting Mattermost mobile (iOS/Android) and desktop clients as first-class external clients. Specifically:

- Mattermost Mobile v2.x clients can authenticate and use core functionality
- Mattermost Desktop clients can connect via the `/api/v4/` HTTP surface
- The Mattermost WebSocket protocol (`/api/v4/websocket`) is supported including auth challenge, session resumption, and standard event types

This is not a full Mattermost clone. Mobile-critical emoji upload and post search endpoints exist, but some advanced Mattermost semantics and enterprise/plugin surfaces are intentionally incomplete.

---

## 2. Current Coverage

### Mobile-critical endpoints (primary metric)

**41/41 endpoints implemented** for the previously tracked mobile-critical list — as of 2026-05-22 repository inspection.

| Category | Coverage |
|---|---|
| Authentication & Session | 6/6 ✅ |
| User Management | 5/5 ✅ |
| Teams | 4/4 ✅ |
| Channels | 6/6 ✅ |
| Posts & Messaging | 7/7 ✅ |
| Threads | 3/3 ✅ |
| Files | 3/3 ✅ |
| Preferences | 2/2 ✅ |
| WebSocket | 1/1 ✅ |
| System | 2/2 ✅ |
| **Total** | **41/41** |

**Previously tracked gaps now implemented:**

| Endpoint | Status | Impact | Planned |
|---|---|---|---|
| `POST /api/v4/emoji` | ✅ Implemented | Custom emoji upload available | Shipped |
| `POST /api/v4/posts/search` | ✅ Implemented | Basic post search available; advanced Mattermost search semantics remain limited | Shipped / improve later |

For the endpoint coverage summary, see [Reference Compatibility Matrix](../reference/compatibility-matrix.md).

### Full v4 API surface (secondary metric)

Historical full-surface analysis is available in archived compatibility notes and is intentionally broader than the mobile-critical metric. The gap is expected: rustchat does not aim to implement the full Mattermost v4 API, only the subset required for mobile and desktop client operation.

Do not conflate these two numbers. **41/41** is the current mobile-critical endpoint implementation headline. The broader full-API coverage figure remains lower because many enterprise and administrative Mattermost APIs are out of scope or compatibility stubs.

---

## 3. Protected Surface

The following paths form the Mattermost compatibility surface. Changes here require compat-reviewer co-approval (`@zoorpha`) and are automatically elevated to the `elevated` risk tier per `.governance/protected-paths.yml`:

| Path | Purpose |
|---|---|
| `backend/src/api/v4/` | Mattermost HTTP API v4 handlers |
| `backend/src/mattermost_compat/` | Response transformation, field mapping utilities |
| `backend/compat/` | Contract JSON schemas + contract validation tests |
| `backend/src/realtime/` | WebSocket hub (v4 event contracts) |

No changes to these paths without compat review. If unsure whether your change touches this surface, treat it as if it does.

---

## 4. Contract Tests

Contract tests live in `backend/compat/tests/`. They validate that rustchat's API responses conform to the expected Mattermost v4 JSON schemas.

```bash
# Run contract tests (requires live Postgres + Redis)
cd backend
RUSTCHAT_TEST_DATABASE_URL=postgres://... cargo test contract
```

Contract JSON schemas: `backend/compat/` — one schema per major entity type (user, channel, post, error).

When the compat surface changes, update the schema and regenerate the contract test. Do not merge compat surface changes without passing contract tests.

---

## 5. Gap Handling

New gaps discovered during development:
1. Open a GitHub issue using the `Bug Report` template, set `Compatibility impact: Yes — affects Mattermost clients`
2. Label will be applied as `area/compat` during triage
3. Implementation planned via the standard spec → plan → execution workflow with compat-reviewer co-approval

---

## 6. Connecting Mattermost Clients

**Server URL:** `http://your-host:8080` (Nginx proxy, recommended) or `http://your-host:3000` (direct backend)

| Port | Service | Notes |
|---|---|---|
| 8080 | Nginx proxy | Recommended — handles web UI and `/api/v4` |
| 3000 | Backend direct | Bypass Nginx if needed |

**Steps:**
1. In Mattermost Mobile or Desktop: Settings → Add Server
2. Enter the Server URL (port 8080)
3. Use email/password login (SSO configurable separately)
4. WebSocket connects automatically to `/api/v4/websocket`

**Known client limitations:**
- Push notifications require the push-proxy service to be running with valid FCM/APNS credentials
- Some advanced features (plugin management, LDAP/SAML, compliance-style enterprise flows, and advanced search semantics) are not available or are compatibility stubs.
