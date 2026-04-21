# Mobile Compatibility Matrix

This page tracks headline compatibility for Mattermost clients.

For implementation details, protected paths, and contract testing workflow:
- See [Compatibility Scope](../compatibility-scope.md)
- See [Development Compatibility Guide](../development/compatibility.md)

## Current Headline Coverage

| Scope | Coverage | Last Audited |
|---|---:|---|
| Mobile-critical endpoints | 39/41 (95.1%) | 2026-04-21 |
| WebSocket event parity | 8/8 core events | 2026-04-21 |
| Push notification deep-link payload | ✅ Implemented | 2026-04-21 |

## Known Gaps

| Endpoint | Status | Impact | Planned |
|---|---|---|---|
| `POST /api/v4/emoji` | Not implemented | Custom emoji upload unavailable | Phase 2 |
| `POST /api/v4/posts/search` | Not implemented | Advanced post search unavailable | Phase 2 |
| `GET /api/v4/users/ids` | Implemented | Batch user fetch for DM surfaces | ✅ Shipped |

## Notes

- The mobile-critical metric is not the same as full Mattermost API v4 coverage.
- Full v4 surface analysis remains lower because RustChat intentionally focuses on mobile and desktop client viability first.
- Avatar URL normalization now converts all presigned S3 URLs to stable `/api/v4/users/{id}/image` paths, eliminating 403 errors from expired links.
- Websocket reconnect snapshots include rich status (`text`, `emoji`, `expires_at`) in addition to plain presence.
