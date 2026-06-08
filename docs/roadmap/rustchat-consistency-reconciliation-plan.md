# RustChat Consistency Reconciliation Plan

Date: 2026-05-30  
Source audit: `docs/audits/rustchat-implementation-consistency-and-gap-analysis.md`  
Scope: reconcile remaining implementation, tests, and existing specs/contracts/ADRs. This plan does not create duplicate product specs.

## 1. Current P0/P1 Status

This document reflects the current branch state after the first low-hanging-fix pass. Items already implemented on this branch are removed from the active implementation batches and kept only as verification/follow-up items.

| Finding | Severity | Classification | Current status | Remaining reconciliation |
|---|---:|---|---|---|
| F-001 Native file upload allows unauthorized channel association | P0 | Implementation missing; test missing | Implemented on this branch: native upload validates membership before multipart body parsing, native presign no longer issues S3 URLs, and tests were added. | Verify targeted integration tests in non-QA environment. |
| F-002 Native/v4 file APIs return presigned S3 URLs despite proxy-only docs | P0 | Contract mismatch; decision conflict | Implemented on this branch for native upload/download responses, native presign, v4 file links, and custom emoji images: clients receive RustChat API URLs or authenticated backend streams; native presign returns explicit 501. | Verify frontend file preview/download/emoji behavior and targeted integration tests in non-QA environment. |
| F-003 v4 post action endpoints acknowledge unsupported mutations | P1 | Contract mismatch; implementation missing; test missing | Implemented on this branch: unsupported actions now return explicit 501 and tests were updated. | Verify integration tests in non-QA environment; update compatibility docs/matrix if not already done before merge. |
| F-004 Scheduled post creation lacks channel membership check | P1 | Implementation missing; test missing | Implemented on this branch for create/update membership checks and non-member create test. | Verify integration tests in non-QA environment; consider follow-up tests for update/delete/root/file mismatch. |
| F-005 Mention and unread semantics are string-matching based | P1 | Product gap; implementation missing; test missing | Implemented on this branch for current direct unread count paths: service, repository, v4 REST, and reconnect snapshot queries use token-boundary matching. Persisted mention-target storage remains future work. | Verify targeted tests in a non-QA environment; decide whether persisted mention targets are required before public preview. |
| F-006 Configuration docs are stale for removed security options and renamed S3 option | P1 | Documentation stale; decision conflict | Implemented on this branch in `docs/admin/configuration.md`, `docs/security-model.md`, deployment docs, and deployment templates. | Docs/config drift lint added in `docs/scripts/check-config-drift.mjs` and wired into docs CI. |
| F-007 README overstates enterprise/preview maturity | P1 | Documentation stale; product gap | Partially implemented on this branch in README for SAML/LDAP and advanced post actions. | Verify README and compatibility docs stay aligned before release. |

## 2. Completed Or Verification-Only Items

These items should not remain active implementation batches unless the branch changes are reverted.

| Item | Files already touched | Verification still needed |
|---|---|---|
| Native file channel association membership check | `backend/src/api/files.rs`, `backend/tests/api_v4_post_routes.rs` | Run targeted integration test in non-QA environment. |
| Native/v4 file proxy-only response contract | `backend/src/api/files.rs`, `backend/src/api/v4/files.rs`, `docs/architecture.md`, `backend/tests/api_v4_post_routes.rs` | Run targeted integration and frontend checks in non-QA environment. |
| v4 advanced post actions return explicit unsupported responses | `backend/src/api/v4/posts.rs`, `backend/tests/api_v4_post_routes.rs`, `backend/tests/api_v4_posts_extra.rs`, README | Run targeted integration tests. Update `docs/compatibility-scope.md` and `docs/reference/compatibility-matrix.md` if they still imply success behavior. |
| Scheduled post create/update membership checks | `backend/src/api/v4/posts.rs`, `backend/tests/api_v4_posts_extra.rs` | Run targeted integration tests. Add optional update/delete/root/file mismatch coverage if required for preview. |
| Config/security docs drift | `docs/admin/configuration.md`, `docs/security-model.md`, `docs/deployment.md`, `docs/security-deployment-guide.md`, `.env.example`, `docker-compose.prod.yml` | Guarded by `npm run docs:check-config` in docs CI; active deployment docs and templates no longer carry the retired query-token setting. |
| README maturity wording for SAML/LDAP and advanced post actions | `README.md` | Ensure compatibility docs mirror README before release. |
| Testing model and WebSocket active endpoint docs | `docs/development/testing.md`, `docs/architecture/overview.md` | No active implementation work; keep aligned with future WebSocket consolidation work. |
| Token-boundary unread matching across direct SQL paths | `backend/src/services/unreads.rs`, `backend/src/repositories/post_repository.rs`, `backend/src/api/v4/channels/utils.rs`, `backend/src/api/v4/users/unreads.rs`, `backend/src/api/v4/users/channel_members.rs`, `backend/src/api/v4/users/channels_ext.rs`, `backend/src/api/v4/websocket/resumption.rs` | Run targeted unread/mention integration tests in non-QA environment. |

## 3. Active Implementation Batches

### Batch 1: Verify P0 File Safety And File Delivery Contract

Purpose: verify the P0 file reconciliation already implemented on this branch.

Affected files/modules:

| Area | Files/modules |
|---|---|
| Native file API | `backend/src/api/files.rs` |
| v4 file API parity | `backend/src/api/v4/files.rs` |
| Storage | `backend/src/storage/s3.rs`, `backend/src/repositories/file_repository.rs` |
| Frontend file calls | `frontend/src/api/files.ts`, upload/download/preview components if URL shape changes |
| Existing docs/contracts | `docs/architecture.md`, `docs/admin/configuration.md`, README security section, `docs/development/testing.md` |

Existing specs/contracts/ADRs to update:

- Reuse `docs/architecture.md` for the file flow decision.
- Reuse `docs/admin/configuration.md` for S3/public endpoint behavior.
- Reuse `docs/development/testing.md` for file authorization and file delivery tests.
- Do not create a new file-storage spec unless a genuinely new operator-selectable delivery mode is introduced; if needed, add a subsection to `docs/admin/configuration.md`.

Implementation plan:

1. Confirm native upload rejects unauthorized `channel_id` before multipart body work.
2. Confirm native upload/download responses return RustChat API URLs, not S3 URLs.
3. Confirm native presign returns explicit unsupported behavior.
4. Validate frontend download/preview handling with authenticated API URLs.

Tests to add or update:

- Backend integration: non-member native upload with private channel ID returns 403 and creates no DB row.
- Backend integration: native presign returns explicit 501 and does not emit S3 upload URLs.
- Backend integration/API contract: upload/download response URLs match the chosen contract.
- Regression: member upload, preview, thumbnail, and download still work.
- Frontend test or E2E: file preview/download handles the chosen URL form.

Migration risks:

- No schema migration expected.
- If proxy-only URLs replace direct S3 URLs, stored file keys remain valid; response generation and frontend handling change.

API compatibility risks:

- Clients expecting direct `url` or `upload_url` fields may need a compatibility path or release note.
- Presigned upload semantics must be explicit if retained.

Frontend behavior risks:

- File previews, downloads, image thumbnails, and mobile compatibility may assume direct URLs.

Rollback considerations:

- Keep membership checks even if delivery contract changes are rolled back.
- File delivery mode can be rolled back behind config only if the weaker mode is documented.

Acceptance criteria:

- Unauthorized users are rejected before upload body work when `channel_id` is unauthorized.
- Unauthorized users cannot create file metadata, S3 keys, presign URLs, or download references for inaccessible channels.
- Docs and API responses agree on RustChat-authenticated URLs.
- Backend and frontend checks cover member and non-member flows.

### Batch 2: Mention And Unread Semantics

Purpose: verify the preview unread and mention semantics now applied to current direct unread count paths, and decide whether persisted mention targets are required for public preview.

Affected files/modules:

| Area | Files/modules |
|---|---|
| Unread service | `backend/src/services/unreads.rs` |
| Remaining direct unread SQL paths | `backend/src/api/v4/channels/utils.rs`, `backend/src/api/v4/users/unreads.rs`, `backend/src/api/v4/users/channel_members.rs`, `backend/src/api/v4/users/channels_ext.rs` |
| Post create/update paths | `backend/src/services/posts.rs`, `backend/src/api/posts.rs`, `backend/src/api/v4/posts.rs` |
| Notification preferences | `backend/src/api/preferences.rs`, v4 user preference handlers, `frontend/src/components/settings/notifications/NotificationsTab.vue` |
| Frontend rendering/autocomplete | composer mention components, message rendering components |
| Data model | migrations only if persisted mention targets are introduced |
| Existing docs/contracts | README feature caveats, `docs/compatibility-scope.md`, user/admin notification docs, `docs/architecture/data-model.md` if schema changes |

Existing specs/contracts/ADRs to update:

- Reuse `docs/compatibility-scope.md` or existing user/admin notification docs for preview mention semantics.
- Reuse `docs/architecture/data-model.md` only if a persisted mention table/column is introduced.
- Do not create a standalone mention spec unless existing notification/compatibility docs cannot hold the policy.

Implementation plan:

1. Keep the preview semantics documented in `docs/compatibility-scope.md`:
   - exact `@username` token boundaries
   - `@channel` and `@all`
   - `@here` eligibility
   - code block/link handling
   - custom keywords and channel-level suppression
2. Confirm no current direct SQL unread endpoint still uses broad substring matching.
3. Add a parser/extractor used by post creation, unread reconciliation, and notification fanout.
4. Prefer persisted mention targets for new/updated posts if unread recalculation must be stable and fast.
5. Backfill or gracefully compute mentions for old posts if persistence is introduced.
6. Align frontend autocomplete and rendering assumptions with backend parsing.

Tests to add or update:

- Unit tests for mention extraction: boundaries, punctuation, duplicate mentions, code blocks, inactive/deleted users.
- Service/integration tests: `@ann` does not mention `anna`; channel mention preferences are honored; `@here` follows the chosen eligibility rule.
- Reconciliation tests for old posts if no backfill migration is added.
- Frontend tests for autocomplete/rendering assumptions if parser-visible behavior changes.

Migration risks:

- Persisted mention targets require schema migration and backfill strategy.
- Backfill can be expensive on large instances; use batched jobs if needed.

API compatibility risks:

- Mention counts, unread counts, and notifications may change after rollout. This should be called out in release notes.

Frontend behavior risks:

- Autocomplete may suggest names the backend parser does not count, or vice versa, unless token rules are shared/tested.

Rollback considerations:

- Keep the old SQL `LIKE` path only as a temporary fallback, not the final behavior.
- If persisted mentions are migrated, rollback must preserve columns/tables until old code no longer reads them.

Acceptance criteria:

- Mention/unread semantics are documented in existing docs.
- Token-boundary tests cover common false positives and false negatives.
- All unread API paths use the same mention matching semantics.
- New-post unread counts no longer depend on broad SQL substring matching.

### Batch 3: Compatibility And Product Status Verification

Purpose: finish reconciliation around implemented fixes without expanding implementation scope.

Affected files/modules:

| Area | Files/modules |
|---|---|
| Compatibility status | `docs/compatibility-scope.md`, `docs/reference/compatibility-matrix.md`, `docs/development/compatibility.md` |
| Product status | `README.md`, `ROADMAP.md`, release notes when applicable |
| Config/docs lint | Existing CI scripts or docs tooling if present |
| Frontend unsupported controls | Search `frontend/src` for advanced post actions and unsupported admin actions |

Existing specs/contracts/ADRs to update:

- Update compatibility docs to list advanced post actions as explicit 501 or deferred.
- Keep README maturity wording aligned with compatibility docs.
- Keep ADRs historical; append status notes only if an active decision changed.

Implementation plan:

1. Audit compatibility docs against current v4 501 behavior.
2. Add or update a release checklist entry requiring README/compatibility review when stubs change.
3. Add a lightweight docs/config drift check.
4. Confirm frontend does not expose actions that now return explicit 501, or that it handles the unsupported response cleanly. Current branch verification found no frontend calls or controls for the advanced post action endpoints.

Tests to add or update:

- Docs/config drift check for removed query-token configuration and retired S3 public URL naming.
- Contract test for Mattermost-compatible 501 error shape on unsupported post actions.
- Frontend unit/E2E only if unsupported controls become visible.

Migration risks:

- None.

API compatibility risks:

- Docs now honestly describe 501 behavior; clients relying on false success need release notes.

Frontend behavior risks:

- Visible controls that call unsupported endpoints can produce user-facing errors.

Rollback considerations:

- Documentation/checklist changes are independently revertible.
- 501 behavior should only roll back if a client-critical compatibility requirement is identified.

Acceptance criteria:

- Compatibility docs match current explicit unsupported behavior.
- README does not overstate SAML/LDAP/plugins/advanced post actions.
- Removed env vars and stale names do not reappear in active docs or deployment templates.
- Frontend either hides unsupported controls or handles explicit unsupported responses. Current branch search confirms unsupported advanced post action controls are not exposed.

### Batch 4: Public Preview Product Gap Triage

Purpose: decide which remaining P1 maturity gaps are required for public preview and which are explicitly deferred.

Affected files/modules:

| Area | Files/modules |
|---|---|
| Session/revocation | `backend/src/auth/`, `docs/security-model.md`, `docs/security-zero-trust-guide.md` |
| Permissions/admin | `backend/src/auth/policy.rs`, admin APIs, frontend admin views |
| Audit/admin controls | `backend/src/api/admin*.rs`, audit migrations, frontend admin UI |
| Release planning | `ROADMAP.md`, `docs/compatibility-scope.md` |

Existing specs/contracts/ADRs to update:

- Reuse `ROADMAP.md` for preview gating.
- Reuse security docs for session/revocation expectations.
- Reuse compatibility docs for deferred Mattermost/client parity.

Implementation plan:

1. Mark each remaining P1 maturity item as preview blocker, preview caveat, or post-preview.
2. For blockers, create implementation issues tied to existing docs/contracts.
3. For caveats, update README/ROADMAP/compatibility docs without adding duplicate specs.

Current classification:

| Remaining P1 maturity item | Classification | Preview disposition | Existing doc home |
|---|---|---|---|
| P0/P1 file safety and authenticated delivery verification | Test missing | Preview blocker until CI integration tests pass on the self-hosted runner | This plan, `docs/development/testing.md`, `docs/architecture.md` |
| Unsupported advanced post actions | Test missing after implementation | Preview blocker until explicit `501` contract tests pass; no frontend controls are currently exposed | `docs/compatibility-scope.md`, `docs/reference/compatibility-matrix.md`, README |
| Config/deployment drift | Documentation stale | Preview blocker guarded by docs CI config drift check | `docs/admin/configuration.md`, `docs/deployment.md`, `docs/security-deployment-guide.md` |
| Mention/unread semantics without persisted mention targets | Product gap | Preview caveat; post-preview hardening unless scale testing proves current token-boundary SQL matching is insufficient | `docs/compatibility-scope.md`, `docs/architecture/data-model.md` if schema changes later |
| SAML/LDAP, plugins, advanced admin/compliance maturity | Product gap | Preview caveat; document as unsupported/stubbed/future | README, `docs/compatibility-scope.md`, `docs/reference/compatibility-matrix.md` |
| Session revocation and audit completeness | Product gap | Post-preview hardening unless security review promotes it to preview blocker | `docs/security-model.md`, `docs/security-zero-trust-guide.md`, ROADMAP |

Tests to add or update:

- Only after decisions are made. Candidate tests: token revocation/session expiry, admin audit events, permission denial UX, deployment smoke.

Migration risks:

- Session revocation may require a token/session table or Redis denylist policy.
- Audit completeness may require additional event inserts.

API compatibility risks:

- Session revocation changes may alter token lifetime behavior.

Frontend behavior risks:

- Forced logout/revocation needs clean frontend handling.

Rollback considerations:

- Feature-flag session revocation enforcement if deployed incrementally.

Acceptance criteria:

- Public preview gate clearly says which P1 maturity capabilities are required, deferred, or unsupported.
- Required gaps have owners/tests before implementation begins.

## 4. Recommended Active Order

1. Batch 1: Verify P0 file safety and file delivery contract.
2. Batch 3: Compatibility and product status verification.
3. Batch 2: Mention and unread semantics.
4. Batch 4: Public preview product gap triage.

This order keeps verification of the P0 file contract first, removes already-implemented items from the active implementation plan, and defers persisted mention-target storage until preview semantics are verified across unread paths.

## 5. Global Acceptance Criteria

The reconciliation effort is complete when:

1. P0 file handling is safe, tested, and documented against one file delivery contract.
2. Every P1 finding is fixed, explicitly unsupported, or documented as deferred with public-preview impact.
3. Native and v4 APIs do not silently diverge on authorization for shared resources.
4. No API route returns success for an unsupported mutation.
5. README and compatibility docs do not overstate feature maturity.
6. CI or documented local checks catch docs/config drift for known removed variables.
7. Frontend behavior is aligned with backend behavior or explicitly hides/handles unsupported actions.
