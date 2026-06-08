# Analysis and Execution Plan: RustChat Next Steps Handoff

Date: 2026-05-30  
Branch: `low-hanging-gap-fixes` (commit `67ddb01e`)  
PR: #165

---

## 1. Executive Summary

The handoff document `docs/plans/2026-05-30-rustchat-next-steps-handoff.md` is well-structured and correctly identifies technical gaps. However, **its recommended Phase 1 (Session Revocation) conflicts with the repository's own triage decisions** recorded in `ROADMAP.md` and `docs/roadmap/rustchat-consistency-reconciliation-plan.md`. Session revocation was explicitly classified as **post-preview hardening**, not a preview blocker.

This analysis recommends a **corrected execution order** that respects the existing triage, prioritizes merge readiness over new implementation scope, and addresses the real remaining risks before public preview.

---

## 2. Critical Finding: Phase 1 vs. Existing Triage Conflict

### The Conflict

| Document | Session Revocation Classification |
|---|---|
| `ROADMAP.md` (Public Preview Gate) | **Post-preview hardening** — "Token/session revocation and broader audit coverage require separate design and migration work before stable release." |
| `docs/roadmap/rustchat-consistency-reconciliation-plan.md` (Batch 4) | **Post-preview hardening unless security review promotes it to preview blocker** |
| Handoff plan Phase 1 | **Recommended first implementation priority** |

### Why This Matters

1. **Architecture cost is high**: The current JWT implementation is purely stateless. There is no session table, no Redis token denylist, and no session versioning. Real revocation requires either:
   - A Redis denylist checked on every authenticated request, or
   - A session/version table requiring schema migration and backfill, or
   - Short token TTLs + refresh token rotation (OAuth-level change)

2. **All revocation endpoints are intentionally stubbed**: Code inspection confirms:
   - `backend/src/api/v4/users/sessions.rs:178` — `logout()` → `{"status": "OK"}` (no-op)
   - `backend/src/api/v4/users/sessions.rs:190` — `revoke_user_session()` → `{"status": "OK"}` (no-op)
   - `backend/src/api/v4/users/sessions.rs:201` — `revoke_user_sessions()` → `{"status": "OK"}` (no-op)
   - `backend/src/api/v4/users/sessions.rs:209` — `revoke_all_sessions()` → `{"status": "OK"}` (no-op)
   - `backend/src/api/v4/users/tokens.rs:18` — `revoke_token()` → `status_ok()` (no-op)

3. **The frontend does not currently expose logout revocation expectations**: The mobile app calls these endpoints but treats them as compatibility no-ops. Changing them to real revocation without frontend handling for forced disconnect would introduce regression risk.

### Recommendation

**Defer Phase 1 (Session Revocation)** to post-preview, consistent with `ROADMAP.md`. Do not start this work now unless an explicit security review upgrades it to a preview blocker.

---

## 3. What Is Actually Left for Public Preview

Based on `ROADMAP.md` Public Preview Gate, the remaining work is **verification and documentation alignment**, not large new implementation:

| Preview Gate Item | Status | Remaining Work |
|---|---|---|
| File uploads/downloads (P0) | Implemented on branch | **Verify CI integration tests pass** |
| Unsupported compatibility actions | Implemented on branch | **Verify 501 contract tests pass** |
| Config/deployment docs drift | Implemented on branch | **Docs CI drift check already added** |
| Mention/unread semantics | Preview caveat | Document token-boundary semantics; **no schema migration required for preview** |
| SAML/LDAP/plugins/advanced admin | Preview caveat | **Verify README/compatibility docs alignment** |
| Session revocation/audit | Post-preview | **Defer** |

---

## 4. Evidence from Current Branch

### P0 File Safety IS Implemented and Tested

The branch already includes:

1. **Native upload membership check**: `backend/src/api/files.rs` validates channel membership before multipart processing.
2. **Authenticated URL responses**: `backend/tests/api_v4_post_routes.rs:489` (`native_file_upload_returns_authenticated_api_url`) confirms upload responses use `/api/v4/files/` paths, not S3 URLs.
3. **Non-member rejection test**: `backend/tests/api_v4_post_routes.rs:419` (`native_file_upload_rejects_non_member_channel_association`) asserts 403 and zero DB rows for unauthorized channel uploads.
4. **v4 file link contract**: `backend/tests/api_v4_post_routes.rs:412` asserts `link.starts_with("/api/v4/files/")`.

### P1 Docs/Config Drift IS Guarded

- `docs/scripts/check-config-drift.mjs` added in commit `67ddb01e`
- Wired into `.github/workflows/docs-ci-cd.yml`
- Checks `RUSTCHAT_S3_PUBLIC_ENDPOINT`, rejects retired `RUSTCHAT_S3_PUBLIC_URL` and `RUSTCHAT_SECURITY_WS_ALLOW_QUERY_TOKEN`

### Unsupported Actions ARE 501

- `backend/src/api/v4/posts.rs` returns explicit `501` for move/restore/reveal/burn
- Frontend search confirms no controls expose these actions

---

## 5. Corrected Execution Plan

### Goal
Get the `low-hanging-gap-fixes` branch to merge-ready state for public preview, without expanding scope into post-preview hardening.

### Phase A: CI Verification and Merge Readiness (Priority 1)

**Purpose**: Confirm the implemented fixes actually pass integration tests.

1. **Push the branch or verify PR #165 CI status**
   - Check that the integration workflow passes on the self-hosted runner or GitHub Actions
   - Specifically confirm:
     - `native_file_upload_rejects_non_member_channel_association`
     - `native_file_upload_returns_authenticated_api_url`
     - `mm_post_files_info_returns_files`
     - v4 post action 501 tests

2. **If CI fails, fix failures only**
   - Do not add new features
   - Do not refactor unrelated code
   - Targeted fixes to make existing tests green

3. **Verify docs CI passes**
   - `npm run docs:check-config` must pass
   - Confirm `docs/admin/configuration.md` and `docs/security-model.md` are current

**Acceptance criteria**:
- All CI checks on PR #165 are green
- Integration tests covering P0 file fixes pass
- Docs CI drift check passes

### Phase B: Compatibility Matrix and README Alignment (Priority 2)

**Purpose**: Ensure docs honestly describe what the code does.

1. **Audit `docs/reference/compatibility-matrix.md`**
   - Confirm advanced post actions are listed as 501/not implemented
   - Confirm SAML/LDAP are listed as stubs

2. **Audit `README.md`**
   - Verify SAML/LDAP/advanced actions are not listed as "implemented"
   - Verify search is described as "basic" not "powerful"
   - Add explicit caveats for preview caveat items

3. **Audit `docs/compatibility-scope.md`**
   - Confirm mention/unread semantics are documented as token-boundary matching
   - Confirm plugin/LDAP/SAML caveats exist

**Acceptance criteria**:
- No doc claims unsupported capabilities as implemented
- Compatibility docs match the 501 behavior in code
- README links to compatibility scope for detailed maturity

### Phase C: Token-Boundary Mention Semantics Verification (Priority 3)

**Purpose**: Confirm the already-implemented token-boundary unread matching is consistent across all paths.

1. **Inspect remaining SQL unread paths**
   - `backend/src/services/unreads.rs`
   - `backend/src/api/v4/websocket/resumption.rs`
   - `backend/src/api/v4/users/unreads.rs`
   - Confirm no `LIKE '%@username%'` substring paths remain

2. **If inconsistencies found, fix them**
   - Keep changes minimal
   - Use the same token-boundary function already applied to direct unread paths

3. **Add targeted service tests for edge cases**
   - `@ann` does not mention `anna`
   - `@channel` and `@all` are counted
   - Punctuation boundaries work (`@user.`, `@user,`)

**Acceptance criteria**:
- All unread SQL paths use token-boundary matching
- Service tests cover common false positives
- No schema migration introduced

### Phase D: Frontend E2E Targeted Coverage (Priority 4)

**Purpose**: Add verification for the highest-risk user paths without expanding E2E scope broadly.

1. **File preview/download E2E**
   - Verify authenticated download URL flow end-to-end
   - Confirm non-member cannot access file via direct URL

2. **Unread/mention E2E** (if frontend paths exist)
   - Verify mention badge updates on post with `@username`
   - Verify no false mention on substring

3. **Forced logout handling**
   - Even though revocation is not implemented, verify frontend handles 401/403 on token expiry gracefully

**Acceptance criteria**:
- File delivery E2E covers member and non-member cases
- Frontend does not regress on unread behavior
- No new unsupported controls are exposed

### Phase E: Post-Preview Triage Documentation (Priority 5)

**Purpose**: Document what is explicitly deferred to post-preview.

1. **Update `ROADMAP.md` if needed**
   - Session revocation design issue created
   - Audit completeness design issue created

2. **Update `docs/security-model.md`**
   - Document current JWT statelessness
   - Document that revocation requires future design work
   - Do not imply revocation works today

**Acceptance criteria**:
- Post-preview items have clear issue placeholders
- Security docs do not overstate current revocation capabilities

---

## 6. What NOT To Do

| Do Not | Reason |
|---|---|
| Implement real session revocation now | Explicitly deferred to post-preview by ROADMAP |
| Add persisted mention-target schema | Preview caveat; only needed if scale testing proves token-boundary SQL insufficient |
| Run `cargo test` locally on QA machine | Operator instruction; use CI or self-hosted runner |
| Create new specs/ADRs for deferred work | Reuse existing docs per handoff constraints |
| Refactor frontend WebSocket consolidation | P2 item; out of scope for preview readiness |
| Expand Mattermost parity beyond mobile-critical | Explicitly not the product goal |

---

## 7. Risk Assessment

| Risk | Likelihood | Mitigation |
|---|---|---|
| Integration tests fail on CI for file fixes | Medium | Fix targeted failures only; do not expand scope |
| Frontend assumes direct S3 URLs somewhere | Low | Search `frontend/src` for `s3`/`presign`/`amazonaws`; E2E catches |
| Docs drift check has false positives | Low | Run `npm run docs:check-config` and fix regex if needed |
| Token-boundary SQL has performance issues at scale | Unknown | Document as preview caveat; measure after preview launch |
| Security review demands revocation for preview | Low | If happens, escalate to design issue; do not rush implementation |

---

## 8. Success Criteria for This Branch

The branch is merge-ready when:

1. CI passes (unit, integration, frontend, docs)
2. README and compatibility docs do not overstate maturity
3. P0 file authorization is tested and documented
4. No API route returns success for an unsupported mutation
5. Config/docs drift is guarded by CI
6. Mention/unread semantics are consistent across all SQL paths
7. Session revocation is **explicitly documented as deferred**, not hidden

---

*This plan respects the existing triage decisions in ROADMAP.md and the reconciliation plan, prioritizes merge readiness over new scope, and targets only the work required to clear the public preview gate.*
