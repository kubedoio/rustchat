# Compatibility Analysis

## Topic and Date

- **Topic:** <!-- Brief description, e.g., "Post pagination default page_size behavior" -->
- **Date:** YYYY-MM-DD
- **Analyst:** <!-- Name or handle -->
- **Related PR/Issue:** <!-- Link if applicable -->

---

## Upstream Version Analyzed

- **Mattermost Server:** <!-- Version tag or commit hash, e.g., v9.8.1 or abc1234 -->
- **Mattermost Mobile:** <!-- Version tag or commit hash, e.g., v2.16.0 or def5678 -->
- **Mattermost Web App:** <!-- Version tag or commit hash (if relevant) -->
- **Analysis Tools:** <!-- e.g., curl, Postman, mitmproxy, browser DevTools, local Go server -->

---

## Endpoint/Method Under Investigation

- **HTTP Method:** <!-- GET, POST, PUT, DELETE, etc. -->
- **API Path:** <!-- e.g., /api/v4/channels/{channel_id}/posts -->
- **WebSocket Event:** <!-- e.g., posted, user_updated (if applicable) -->
- **Context:** <!-- Where is this used? Mobile channel load, thread view, search, etc. -->

---

## Expected Behavior (from Upstream)

### Request

<!-- Include exact request details: method, path, query parameters, headers, body -->

```http
GET /api/v4/... HTTP/1.1
Authorization: Bearer <token>
```

### Response

<!-- Include exact response: status code, headers, body -->

```http
HTTP/1.1 200 OK
Content-Type: application/json

{
  ...
}
```

### WebSocket Event (if applicable)

<!-- Include the full JSON payload emitted by upstream -->

```json
{
  "event": "...",
  "data": {
    ...
  },
  "broadcast": {
    ...
  }
}
```

### Key Observations

<!-- Note any unusual behavior, default values, ordering guarantees, or edge cases -->

---

## Observed Behavior (in RustChat)

### Request

```http
GET /api/v4/... HTTP/1.1
Authorization: Bearer <token>
```

### Response

```http
HTTP/1.1 ...
Content-Type: application/json

{
  ...
}
```

### WebSocket Event (if applicable)

```json
{
  "event": "...",
  "data": {
    ...
  }
}
```

### Key Observations

<!-- Note where RustChat diverges from upstream -->

---

## Gap Analysis

| Dimension | Upstream | RustChat | Severity |
|-----------|----------|----------|----------|
| Status code | <!-- e.g., 200 --> | <!-- e.g., 200 --> | <!-- match / minor / major --> |
| Response schema | <!-- e.g., includes `metadata` object --> | <!-- e.g., omits `metadata` --> | <!-- ... --> |
| Header | <!-- e.g., `ETag` present --> | <!-- e.g., missing --> | <!-- ... --> |
| Pagination | <!-- e.g., default `per_page=60` --> | <!-- e.g., default `per_page=30` --> | <!-- ... --> |
| Ordering | <!-- e.g., descending by `create_at` --> | <!-- e.g., ascending --> | <!-- ... --> |
| WebSocket event | <!-- e.g., `posted` with `mentions` array --> | <!-- e.g., `posted` without `mentions` --> | <!-- ... --> |

### Detailed Gap Description

<!-- Describe each gap in prose. Explain why it matters for compatibility (mobile parsing, client caching, user-visible behavior). -->

---

## Recommended Fix

### Option A: Preferred

<!-- Describe the preferred fix. Include pseudo-code, SQL changes, or config changes if applicable. -->

### Option B: Alternative (if Option A is too invasive)

<!-- Describe a lighter-weight alternative and its trade-offs. -->

### Risks

<!-- What could break if we apply this fix? Performance? Other clients? -->

---

## Test Plan

### Automated Tests

- [ ] Add/update integration test in `backend/tests/...`
- [ ] Add/update compatibility smoke test in `scripts/mm_compat_smoke.sh`
- [ ] Add/update mobile smoke test in `scripts/mm_mobile_smoke.sh`

### Manual Verification

1. <!-- Step 1 -->
2. <!-- Step 2 -->
3. <!-- Step 3 -->

### Mobile/Desktop Client Verification

- [ ] Verified with Mattermost Mobile <!-- version -->
- [ ] Verified with Mattermost Desktop <!-- version -->
- [ ] Verified with FluffyChat / other third-party client (if applicable)

---

## Sign-off

- [ ] Analyst confirms upstream behavior is reproducible and accurately captured.
- [ ] Analyst confirms gap analysis is complete and severity is justified.
- [ ] Reviewer confirms expected behavior matches their understanding of upstream.
- [ ] Reviewer approves recommended fix and test plan.

| Role | Name | Date | Signature/Note |
|------|------|------|----------------|
| Analyst | | | |
| Reviewer | | | |

---

## Appendix

### Raw Logs

<!-- Paste any raw curl output, server logs, or trace data here. -->

### References

- Mattermost API documentation link (if any)
- Related previous analyses
- Related GitHub issues or discussions
