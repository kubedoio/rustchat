# RustChat Next Steps Handoff Plan

Date: 2026-05-30  
Source context: `docs/audits/rustchat-implementation-consistency-and-gap-analysis.md`, `docs/roadmap/rustchat-consistency-reconciliation-plan.md`, current branch `low-hanging-gap-fixes`

## Purpose

This document is a handoff brief for the next LLM. It describes what has already been reconciled on the branch, what remains after the low-hanging pass, and the order in which the remaining work should be tackled.

Do not recreate the audit or rewrite the specs. Reuse the existing docs, especially:

- `docs/roadmap/rustchat-consistency-reconciliation-plan.md`
- `docs/compatibility-scope.md`
- `docs/reference/compatibility-matrix.md`
- `docs/security-model.md`
- `ROADMAP.md`

## Current Branch State

- Branch: `low-hanging-gap-fixes`
- PR: `https://github.com/kubedoio/rustchat/pull/165`
- Latest pushed commit at the time of this handoff: `67ddb01e`
- The branch already includes the first reconciliation pass:
  - native file upload authorization hardening
  - proxy-only file delivery responses
  - explicit `501` behavior for unsupported advanced post actions
  - scheduled post membership checks
  - token-boundary unread matching
  - docs/config drift guardrails

## Constraints

1. Do not run `cargo test` locally on the QA machine.
2. Use CI or the self-hosted runner for integration verification.
3. Do not create duplicate specs or ADRs.
4. Do not refactor unrelated code.
5. Do not invent Mattermost parity goals beyond the existing compatibility docs.
6. Keep changes evidence-based; if a capability is not confirmed, mark it as not confirmed.

## What Is Already Done

The low-hanging pass is complete enough that the next model should not repeat it:

- File handling now rejects unauthorized associations before upload work starts.
- File responses now point at RustChat-authenticated URLs rather than direct S3 URLs.
- Unsupported Mattermost-style advanced post actions now return explicit `501` responses.
- Query-token WebSocket / OAuth documentation drift was cleaned up and guarded with a docs drift check.
- The frontend does not currently expose controls for the advanced post actions that now return `501`.

## Remaining Implementation Phases

### Phase 1: Session Revocation And Token Invalidation

Goal: make session invalidation real rather than relying only on token expiry and disconnect handling.

Why this matters:

- This is one of the remaining preview-hardening gaps.
- It affects logout, admin disable flows, password resets, and security posture.

Likely files/modules:

- `backend/src/auth/`
- `backend/src/api/auth.rs`
- `backend/src/api/admin_users.rs`
- `backend/src/api/users.rs`
- `backend/src/api/oauth/callback.rs`
- `backend/src/api/v4/websocket/`
- `backend/src/api/websocket_core.rs`
- `docs/security-model.md`
- `docs/security-zero-trust-guide.md`

Suggested order:

1. Inspect current token validation and Redis/session usage.
2. Decide whether invalidation is implemented as a denylist, session versioning, or another existing pattern in the repo.
3. Wire invalidation into the relevant logout/admin/password-reset flows.
4. Add tests for revoked-token rejection and forced disconnect behavior.

Acceptance criteria:

- A revoked or invalidated session cannot continue using API or WebSocket access.
- The invalidation behavior is documented in existing security docs.
- Tests cover the main revocation path.

### Phase 2: Persisted Mention Targets

Goal: move from token-boundary unread matching to a more stable mention model if public-preview requirements justify it.

Why this matters:

- Current behavior is better than substring matching, but it still computes mentions from message text.
- If scale or correctness needs require it, mention targets should be persisted with the post.

Likely files/modules:

- `backend/src/services/posts.rs`
- `backend/src/services/unreads.rs`
- `backend/src/repositories/post_repository.rs`
- `backend/src/api/v4/posts.rs`
- `backend/src/api/posts.rs`
- `backend/src/api/v4/users/unreads.rs`
- `backend/src/api/v4/websocket/resumption.rs`
- schema migrations only if persistence is introduced
- `docs/architecture/data-model.md` only if schema changes are made

Suggested order:

1. Confirm whether the current token-boundary approach is sufficient for preview.
2. If not, design persisted mention targets and decide whether backfill is required.
3. Add write-path extraction for new/edited posts.
4. Keep unread calculation and websocket resumption consistent with the same mention source.

Acceptance criteria:

- Mention and unread semantics are consistent across REST, websocket, and reconnection paths.
- The repository has a single authoritative mention source.
- Any schema change has an explicit migration strategy.

### Phase 3: Audit And Admin Hardening

Goal: cover the remaining security/admin gaps that are relevant for stable self-hosted chat operation.

Why this matters:

- Public preview expects clear administrative visibility and safer control paths.
- The audit identified admin/security coverage as a remaining product gap.

Likely files/modules:

- `backend/src/api/admin*.rs`
- `backend/src/auth/policy.rs`
- `backend/src/api/preferences.rs`
- `backend/src/models/`
- admin UI views in `frontend/src/views/admin/` and `frontend/src/components/admin/`
- `docs/security-model.md`
- `docs/security-zero-trust-guide.md`
- `ROADMAP.md`

Suggested order:

1. Identify which audit/admin events are already emitted and which are missing.
2. Decide whether the next step is broader audit logging or only the highest-value admin actions.
3. Verify role and permission checks for the sensitive flows already exposed in the UI.

Acceptance criteria:

- Sensitive admin actions have a traceable server-side event or log.
- Permission checks match UI visibility.
- The docs do not overstate coverage that the code does not provide.

### Phase 4: Preview Gate Triage

Goal: decide which remaining P1 maturity items are blockers for public preview and which are explicit caveats.

Likely files:

- `ROADMAP.md`
- `docs/compatibility-scope.md`
- `docs/reference/compatibility-matrix.md`
- `README.md`

Suggested order:

1. Reclassify each remaining P1 item as blocker, caveat, or post-preview hardening.
2. Update the roadmap and compatibility docs to match that classification.
3. Keep the README aligned with those decisions.

Acceptance criteria:

- Preview blockers are explicit.
- Deferred items are explicit.
- Nothing in the README or compatibility docs implies support that the code does not provide.

### Phase 5: Frontend And E2E Coverage

Goal: add targeted UI verification for the behavior that now matters most.

Suggested focus:

- authenticated file preview/download flows
- unread and mention behavior
- forced logout/session revocation
- unsupported action error handling if any UI path is added later

Likely files:

- `frontend/src/api/`
- `frontend/src/components/`
- `frontend/src/features/`
- `frontend/e2e/`

Acceptance criteria:

- The frontend does not regress on file delivery or unread behavior.
- End-to-end checks cover the highest-risk user paths.

## Recommended Implementation Order

1. Session revocation and token invalidation
2. Persisted mention targets, if still required after preview triage
3. Audit and admin hardening
4. Preview gate triage and doc alignment
5. Frontend and E2E coverage

## What Not To Do

- Do not reopen the low-hanging file/URL/501/docs-drift work unless the branch regresses.
- Do not add a new product spec just to describe work already covered by the audit and roadmap.
- Do not treat Mattermost parity as the product goal; use it only as a stable self-hosted chat baseline.
- Do not run local `cargo test` on this QA machine.

## Handoff Rule

If the next model needs to continue implementation, it should first inspect the current branch state and the latest PR checks, then pick up at Phase 1 unless the user explicitly asks for a different priority.
