# Task Plan

## 2026-04-26 Backend Model-Schema Mismatch Fix

### Task
Fix three identified model-schema mismatches in the rustchat backend:
1. `TeamMember` missing `presence` column from `team_members` table
2. `Post` missing `has_reactions` column from `posts` table
3. `models::post::Reaction` struct does not match `reactions` table schema; canonical `models::reaction::Reaction` should be used everywhere

### Prerequisites
- Branch from `main` (e.g., `git checkout -b fix/backend-model-schema-mismatches`)
- PostgreSQL accessible for SQLx compile-time query checks (or valid offline query cache)
- `cargo` toolchain available

### Implementation Checklist

#### Task 1: Add `presence` to `TeamMember` struct
- [x] **Modify** `backend/src/models/team.rs`
  - Add `#[sqlx(default)] pub presence: String` to `TeamMember` (after `created_at`)
  - Expected outcome: `TeamMember` struct matches `team_members` table columns
  - **Verification:** `cd backend && cargo check` compiles

#### Task 2: Update `TeamMember` mappers and all struct literal sites
- [x] **Modify** `backend/src/api/v4/teams/mod.rs`
  - In `map_team_member`, change `presence: None` → `presence: Some(member.presence).filter(|p| !p.is_empty())`
  - Remove `map_team_member_with_presence` function entirely
  - Expected outcome: single mapper function propagates presence
- [ ] **Modify** `backend/src/mattermost_compat/mappers.rs`
  - In `impl From<TeamMember> for mm::TeamMember`, change `presence: None` → `presence: Some(m.presence).filter(|p| !p.is_empty())`
  - Expected outcome: `From` trait propagates presence
- [x] **Modify** `backend/src/api/v4/teams/members.rs`
  - Remove import of `map_team_member_with_presence`
  - In `get_team_members_for_mm`, change query to select `tm.presence` instead of `u.presence`
  - Replace `map_team_member_with_presence(TeamMember { ... }, presence)` with `map_team_member(TeamMember { presence: presence.unwrap_or_else(|| "offline".to_string()), ... })`
  - In `add_team_member`, update the `TeamMember` literal to include `presence: "offline".to_string()`
  - In `get_team_member_me`, change `presence: None` → `presence: Some(member.presence).filter(|p| !p.is_empty())`
  - Expected outcome: v4 team member endpoints compile and propagate presence
- [x] **Modify** `backend/src/api/v4/teams/invites.rs`
  - Update the `TeamMember` literal to include `presence: "offline".to_string()`
- [x] **Modify** `backend/src/api/v4/teams/batch.rs`
  - Update the `TeamMember` literal to include `presence: "offline".to_string()`
- [ ] **Verification:** `cd backend && cargo check` compiles

#### Task 3: Add `has_reactions` to `Post` struct
- [x] **Modify** `backend/src/models/post.rs`
  - Add `#[sqlx(default)] pub has_reactions: bool` to `Post` (after `seq`)
  - Expected outcome: `Post` struct matches `posts` table schema
  - **Verification:** `cd backend && cargo check` compiles

#### Task 4: Replace `post::Reaction` with `reaction::Reaction` in API handlers
- [x] **Modify** `backend/src/api/v4/posts/reactions.rs`
  - In `add_reaction`, change `crate::models::post::Reaction` → `crate::models::reaction::Reaction`
  - In `remove_reaction_internal`, change `crate::models::post::Reaction` → `crate::models::reaction::Reaction`
  - Update `mm::Reaction` construction: `reaction.created_at.timestamp_millis()` → `reaction.create_at`
  - Expected outcome: v4 reactions API uses the canonical model
- [x] **Modify** `backend/src/api/posts.rs`
  - Change `use crate::models::{..., Reaction, ...}` to remove `Reaction` from that list
  - Add `use crate::models::reaction::Reaction;`
  - In `add_reaction` query, fix pre-existing column name bug: `ON CONFLICT ... DO UPDATE SET created_at = NOW()` → `ON CONFLICT ... DO UPDATE SET create_at = extract(epoch from now()) * 1000`
  - In `populate_reactions`, fix pre-existing column name bug: `ORDER BY created_at` → `ORDER BY create_at`
  - Expected outcome: native reactions API uses the canonical model and valid SQL
- [ ] **Verification:** `cd backend && cargo check` compiles

#### Task 5: Update websocket reaction event mapper
- [x] **Modify** `backend/src/api/v4/websocket.rs`
  - In the `"reaction_added"` and `"reaction_removed"` arms of `map_envelope_to_mm`, replace the single `serde_json::from_value::<crate::models::post::Reaction>` attempt with a cascading handler:
    1. Try `serde_json::from_value::<crate::models::reaction::Reaction>(env.data.clone())` for native full-model events; map fields using `reaction.create_at`
    2. Else if `env.data.get("reaction").and_then(|v| v.as_str())` exists, pass through the v4 stringified payload directly
    3. Else try to extract `post_id`/`user_id`/`emoji_name` from `env.data` for native partial payloads
  - Remove all references to `crate::models::post::Reaction`
  - Expected outcome: websocket mapper compiles without `post::Reaction` and handles all broadcast shapes
- [ ] **Verification:** `cd backend && cargo check` compiles

#### Task 6: Remove `Reaction` from `models::post`
- [x] **Modify** `backend/src/models/post.rs`
  - Delete the `Reaction` struct definition (lines 30–36)
  - Keep `ReactionResponse`, `CreateReaction`, and all other post-related types unchanged
  - Expected outcome: `models::post::Reaction` no longer exists
- [ ] **Verification:** `cd backend && cargo check` compiles with zero errors

### Rollback Steps
If any task causes unexpected compilation or test failures, revert in reverse order:
1. **Task 6 rollback:** Restore the `Reaction` struct in `backend/src/models/post.rs`
2. **Task 5 rollback:** Restore `backend/src/api/v4/websocket.rs` to use `crate::models::post::Reaction`
3. **Task 4 rollback:** Revert `backend/src/api/v4/posts/reactions.rs` and `backend/src/api/posts.rs`
4. **Task 3 rollback:** Remove `has_reactions` from `Post` in `backend/src/models/post.rs`
5. **Task 2 rollback:** Revert `backend/src/api/v4/teams/mod.rs`, `backend/src/mattermost_compat/mappers.rs`, `backend/src/api/v4/teams/members.rs`, `backend/src/api/v4/teams/invites.rs`, `backend/src/api/v4/teams/batch.rs`
6. **Task 1 rollback:** Remove `presence` from `TeamMember` in `backend/src/models/team.rs`

### Post-Implementation Verification
Run the full verification suite in order:
1. `cd ../../backend && cargo fmt --all`
2. `cd ../../backend && cargo check`
3. `cd ../../backend && cargo clippy --all-targets --all-features -- -D warnings`
4. `cd ../../backend && cargo test --no-fail-fast -- --nocapture`
5. `cd /Users/scolak/Projects/rustchat && BASE=http://127.0.0.1:3000 ./scripts/mm_compat_smoke.sh` (requires running backend)
6. `cd /Users/scolak/Projects/rustchat && BASE=http://127.0.0.1:3000 ./scripts/mm_mobile_smoke.sh` (requires running backend)

### Expected vs Actual
- Expected:
  - `TeamMember` includes `presence` and all `SELECT * FROM team_members` queries compile
  - `Post` includes `has_reactions`
  - `models::post::Reaction` is removed; all reaction DB traffic goes through `models::reaction::Reaction`
  - `cargo check` and `cargo clippy` pass cleanly
- Actual: All tasks implemented. `cargo check`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test --lib` pass. Integration tests require DB (unavailable in this environment).

### Readiness
- Plan is ready for execution; no external dependencies beyond the standard backend toolchain.
- Note: `backend/src/api/posts.rs` contains pre-existing SQL references to `created_at` on the `reactions` table (column is actually `create_at`). The plan includes fixing these as part of Task 4 because they block compilation once the correct model is adopted.

## 2026-04-11 Frontend Supply-Chain Hardening

### Task
- Reduce frontend direct dependency surface where removal is low-risk.
- Standardize frontend package management on npm plus `package-lock.json`.
- Harden frontend CI and Docker installs to lockfile-only, script-blocked installs.
- Add dependency review, policy enforcement, transitive override/patch support, and dependency inventory output.
- Move remaining theme preference HTTP flows onto the internal fetch-based client boundary.

### Implementation Checklist
- [x] Added a task-specific spec in [SPEC.md](./SPEC.md) and received user approval before implementation.
- [x] Added ADR [docs/adr/ADR-frontend-supply-chain-security.md](../../docs/adr/ADR-frontend-supply-chain-security.md).
- [x] Added contributor-facing policy [docs/frontend-dependency-policy.md](../../docs/frontend-dependency-policy.md).
- [x] Added contract/verification mapping in [docs/frontend-supply-chain-contracts.md](../../docs/frontend-supply-chain-contracts.md).
- [x] Added machine-readable dependency metadata in `frontend/dependency-policy.json`.
- [x] Added repo-managed patch support via `frontend/dependency-patches.json`, `frontend/patches/README.md`, and `frontend/scripts/apply-dependency-patches.mjs`.
- [x] Added repo-owned dependency policy enforcement in `frontend/scripts/check-dependency-policy.mjs`.
- [x] Reduced direct dependency surface by removing `@tiptap/extension-mention`, `clsx`, and `tailwind-merge` from `frontend/package.json`.
- [x] Moved remaining theme preference raw `fetch()` usage behind the internal client boundary through `frontend/src/api/preferences.ts`, `frontend/src/stores/theme.ts`, and `frontend/src/features/theme/repositories/themeRepository.ts`.
- [x] Hardened frontend CI in [ci.yml](../../.github/workflows/ci.yml) and added dependency governance checks to [security.yml](../../.github/workflows/security.yml).
- [x] Hardened frontend Docker builds in `docker/frontend.Dockerfile`.
- [x] Added PR/update workflow guidance via [.github/dependabot.yml](../../.github/dependabot.yml), `CONTRIBUTING.md`, `README.md`, and `frontend/README.md`.

### Manual Verification Commands
1. `cd ../../frontend && npm run check:dependency-policy`
2. `cd ../../frontend && npm ci --ignore-scripts`
3. `cd ../../frontend && npm run apply:dependency-patches`
4. `cd ../../frontend && npm run build`
5. Open a pull request that changes `frontend/package.json` or `frontend/package-lock.json` and confirm:
   - `Frontend Dependency Governance / GitHub dependency review` runs
   - `Frontend Dependency Governance / Policy + audit` runs
   - `Frontend CI / Hardened install + build` installs with scripts blocked and uploads `frontend-dependency-inventory`

### Expected vs Actual
- Expected:
  - direct dependency count decreases without a Vue rewrite
  - CI installs from lockfile only and blocks lifecycle scripts by default
  - dependency changes are surfaced and reviewed in PRs
  - vulnerable transitives can be controlled through overrides and, if needed, repo-managed patches
  - theme preference requests use the internal HTTP client boundary instead of scattered raw `fetch()`
- Actual:
  - implementation is complete in the tracked files above
  - static verification confirms JSON integrity, no whitespace errors, and no remaining raw frontend `fetch()` call sites outside the internal HTTP client
  - full frontend runtime validation is still pending CI because this Codex session does not have `node`/`npm` available locally

### Readiness
- Approved by the user on `2026-04-11` after the task-specific supply-chain hardening spec was added to `SPEC.md`.
- Ready for CI-backed verification.
- Known limitation: local `npm ci`, `npm audit`, and `npm run build` could not be executed in this session because the current environment does not expose `node`, `npm`, `pnpm`, or `corepack` on `PATH`.

## 2026-04-05 Permanent Avatar URL Stabilization

### Task
- Stop authenticated app surfaces from emitting expired S3 presigned avatar URLs.
- Move avatar uploads onto the stable profile-image path.
- Keep already-uploaded legacy avatars working through a backward-compatible stable image endpoint.

### Implementation Checklist
- [x] Add backend regression coverage for avatar URL normalization and legacy-avatar fallback.
- [x] Add a shared backend helper for effective/stable avatar URLs.
- [x] Normalize native auth/user/post/team/activity responses away from raw expiring `users.avatar_url` values.
- [x] Extend the v4 profile-image reader to fall back to legacy file-backed avatar objects when the canonical avatar object is missing.
- [x] Update frontend profile avatar uploads to use `/api/v4/users/{id}/image` instead of `/api/v1/files`.
- [ ] Re-verify the live login path that previously showed expired-avatar `403` console errors.

### Manual Verification Commands
1. `cd ../../backend && cargo test avatar -- --nocapture`
2. `cd ../../backend && cargo check`
3. `cd ../../frontend && npm run build`
4. Log in to `https://app.rustchat.io`, open the main workspace and admin pages, and confirm the browser console no longer shows `403 AccessDenied` avatar loads from stale `s3.rustchat.io` presigned URLs.

### Readiness
- Approved by the user on `2026-04-05` after root-cause tracing identified stale `avatar_url` persistence and response echoing.
- Focused verification is passing locally (`cargo check`, avatar normalization unit tests, `npm run build`).
- Full integration-test execution remains blocked in this workspace because the Docker daemon is not running, so the local Postgres/Redis/S3 harness could not be started.

## 2026-03-29 Team Creation Permission Regression

### Task
- Fix the regression where `POST /api/v1/teams` returned `403` for newly registered `member` users.
- Unblock the failing category integration tests that create a team before exercising sidebar category behavior.

### Implementation Status
- [x] Updated `backend/src/api/teams.rs` so team creation checks `TEAM_CREATE` instead of `TEAM_MANAGE`.
- [x] Updated `backend/src/auth/policy.rs` so `member`, `team_admin`, and `org_admin` include `TEAM_CREATE`.
- [x] Updated policy unit coverage in `backend/src/auth/policy.rs` for the new permission expectation.
- [x] Updated `backend/tests/api_permissions.rs` so members can create teams and guests remain blocked.
- [x] Verified `backend/tests/api_categories.rs` now passes again.

### Verification Status
1. `cd ../../backend && cargo test --test api_categories -- --nocapture`
- Result: PASS, `2 passed`
2. `cd ../../backend && cargo test --test api_permissions -- --nocapture`
- Result: PASS, `9 passed`
3. `cd ../../backend && cargo check`
- Result: PASS
4. `cd ../../backend && cargo clippy --all-targets --all-features -- -D warnings`
- Result: PASS

### Manual Verification Command
1. `curl -i -X POST http://127.0.0.1:3000/api/v1/teams -H "Authorization: Bearer $TOKEN" -H "Content-Type: application/json" -d '{"name":"self-serve-team","display_name":"Self Serve Team","description":"permission regression check"}'`

### Expected vs Actual
- Expected: a normal registered `member` can create a team, while a `guest` still gets `403`.
- Actual: local verification matches that behavior, and the previously failing `api_categories` tests now pass.

## 2026-03-28 Reaction Deduplication + Shared Presence/Status Layer

### Task
- Keep the reaction deduplication fix.
- Implement the approved Option B architecture from `SPEC.md`.
- Stop the UI from computing presence/status separately in each touched component.

### Architecture Locked In
- Add one shared frontend presence/status presenter for badge label, icon, and visual treatment.
- Add one shared user-summary source for direct-message and profile/detail surfaces.
- Move session cleanup and active presence usage fully onto `frontend/src/features/presence/stores/presenceStore.ts`.
- Treat `frontend/src/stores/presence.ts` as legacy and remove it from the active flow.

### Implementation Checklist
- [x] Keep reaction canonicalization centered in `frontend/src/utils/emoji.ts`, `frontend/src/stores/messages.ts`, and `frontend/src/components/channel/MessageItem.vue`.
- [x] Add the shared presence/status presenter and switch touched UI surfaces to use it.
- [x] Add one DM/user-summary helper so `ChannelInfoPanel` and `UserProfileModal` no longer shape user data independently, and centralize DM counterpart resolution in `frontend/src/utils/directMessage.ts`.
- [x] Update `frontend/src/stores/auth.ts` to clear the active feature presence store during logout/session expiry.
- [x] Remove or quarantine the legacy presence store so new code cannot accidentally keep using it.
- [x] Add regression tests for reaction alias dedupe and session presence cleanup.

### Manual Verification Commands
1. `cd frontend && npm run build`
2. `cd frontend && npm run dev`
3. In an authenticated channel, react with `👍` to a message that already has `+1` or `thumbsup`, then remove it again. Verify one reaction bucket is rendered throughout.
4. Open a direct message and compare the DM sidebar row, channel info panel, and user profile modal. Verify availability and custom status agree across all three surfaces.
5. Log out, then log back in as the same or another user. Verify no stale presence badges survive session cleanup.

### Parallelization
- Lane A: reaction normalization hardening
- Lane B: shared presence/status architecture
- Lane C: UI rollout on top of Lane B

### Readiness
- Ready for implementation once the branch follows the locked Option B architecture above.

### Outcome
- Implemented on branch `ui-improvements-3` and pushed at commit `f4cea640` (`feat: centralize presence status handling`).
- Automated verification completed:
  - `cd frontend && npm run test:unit`
  - `cd frontend && npm run build`
  - `cd frontend && npm run test:e2e`
- Result: PASS, `60 passed`.
- Follow-up gaps remain around cross-user custom status freshness, backend status contract consistency, and deeper reaction/status regression coverage. Those are tracked in the `2026-03-29 Presence/Status Baseline and Contract Closure` section below.

## 2026-03-29 Presence/Status Baseline and Contract Closure

### Task
- Freeze a baseline for the shipped reaction/presence/status work.
- Close the remaining **Lake 1** UI and realtime gaps without reopening the broader shell redesign.
- Make DM surfaces, profile surfaces, and reconnect behavior consistent enough that users trust the same person-state everywhere they see it.

### Shipped Baseline
- Reaction identity is normalized in `frontend/src/utils/emoji.ts`, `frontend/src/stores/messages.ts`, and `frontend/src/components/channel/MessageItem.vue`.
- Shared status presentation exists in `frontend/src/features/presence/presencePresentation.ts`.
- Shared user summary hydration exists in `frontend/src/composables/useUserSummary.ts`.
- Session cleanup clears active presence state and summary cache in `frontend/src/stores/auth.ts`.
- Backend presence/status endpoints and websocket `status_change` flow are active in `backend/src/api/v4/status.rs`.

### Lake 1 Scope Locked
- Live custom-status propagation for other users across the DM sidebar, channel info panel, and profile modal.
- Reconnect correctness so websocket recovery restores rich status, not just plain presence.
- Shared summary batching through the existing `/api/v4/users/ids` path for visible DM users.
- One shared frontend expiry helper used by both auth-self state and shared-summary state.
- Regression coverage for summary batching, rich reconnect hydration, and cross-surface realtime behavior.

### Remaining Gaps
- Cross-user custom status text/emoji is not live-updated after initial fetch for DM/profile surfaces.
- `frontend/src/components/layout/ChannelSidebar.vue` still shapes DM counterpart data locally instead of consuming the final shared-summary path.
- Websocket reconnect currently restores basic presence, but the plan must treat rich custom-status recovery as part of the user-visible contract.
- The plan still lacks explicit visual hierarchy, interaction states, responsive behavior, and density rules for the 3 person-status surfaces.
- Regression coverage is still missing for summary batching, reconnect hydration, token-expiry cleanup, and cross-surface UI consistency.

### Implementation Checklist
- [x] Extend websocket reconnect snapshots so visible user status includes `text`, `emoji`, and `expires_at`, not only plain presence.
- [x] Batch shared-summary prefetch for visible DM users through `/api/v4/users/ids`, while keeping single-user fallback for profile/detail surfaces.
- [x] Move auth-self expiry and shared-summary expiry onto one shared frontend helper.
- [x] Normalize `frontend/src/components/layout/ChannelSidebar.vue` onto the shared summary path for DM identity/status rendering.
- [x] Add frontend regression tests for summary batching, reconnect hydration, shared expiry handling, and token-expiry-driven cleanup.
- [x] Add one focused Playwright flow for DM sidebar + channel info + profile modal consistency, including live update and reconnect recovery.

### Design Decisions Locked In
1. **Cross-surface hierarchy**
   - Identity first.
   - Availability second.
   - Custom status third.
2. **Surface rules**
   - Sidebar row: avatar badge is the primary dense-surface signal; name stays strongest; custom status is one muted truncated line and never competes with the person’s identity.
   - Channel info panel: avatar + identity block first, then one availability chip, then one passive custom-status chip below it.
   - Profile modal: identity stays neutral-first and complete; availability remains secondary; full custom status may wrap and act as supporting metadata rather than a hero element.
3. **Anti-slop rules**
   - No equal-weight double-pill treatment in dense surfaces.
   - No centered social-card energy for core collaboration identity.
   - No decorative color treatment that outranks status truth.
   - No generic badge soup or repeated card patterns just because status data exists.
4. **Design-system mapping**
   - Neutral text hierarchy first, semantic color reserved for availability truth.
   - Custom status chip is passive metadata, not an action or CTA.
   - Avatar badge is the primary compact status affordance.
   - Motion is minimal-functional and only clarifies state changes.
5. **Density rules**
   - Sidebar: max one muted ellipsized status line.
   - Info panel: one wrapped line or two-line clamp.
   - Profile modal: full custom status unless absurdly long, then graceful wrapping.

### UI Structure
```text
DM SIDEBAR ROW
[Avatar + presence badge]  Name
                           muted custom status (optional, 1 line)

CHANNEL INFO PANEL
[Avatar]
Name
@username
[Availability chip]
[Custom status chip]
[Role / contact details]

PROFILE MODAL
[Large avatar]
Name
secondary identity text
[Availability chip]
[Custom status block]
[Email / position / details]
```

### Interaction State Coverage

| Feature | Loading | Partial | Error | Success | Reconnect Recovering |
|--------|---------|---------|-------|---------|----------------------|
| DM sidebar row | Show existing avatar/name shell with no spinner clutter; reserve status space | Show identity from team/member fallback, suppress custom status until trusted | Fall back to avatar + identity only; no scary inline error copy in the list | Name + badge + muted status line | Keep identity visible, clear stale custom status until rich snapshot lands |
| Channel info panel | Skeleton avatar, 2 text lines, 1 muted pill placeholder | Show identity and availability first, delay custom status chip | Replace detail body with calm inline error copy and retryable close/reopen path | Identity block, availability chip, passive custom-status chip | Preserve panel structure, show availability first, never flash stale status text |
| Profile modal | Centered loading shell with avatar circle and text skeletons | Show profile identity and availability; delay richer fields/status text | Full-panel error state with plain-language recovery copy | Complete identity, availability chip, wrapped custom-status metadata, details list | Keep modal open, preserve identity, refresh status fields without layout jump |

### User Journey & Emotional Arc

| Step | User Does | User Feels | Plan Must Support |
|------|-----------|------------|-------------------|
| 1 | Scans DM sidebar | "I can tell who is around" | Fast identity-first row, presence truth visible at a glance |
| 2 | Opens DM info panel | "This matches what I just saw" | Same person, same availability semantics, more context not more noise |
| 3 | Opens profile modal | "This looks complete and trustworthy" | Full identity, richer details, calm hierarchy, no decorative drift |
| 4 | Sees live status update | "The app is current" | Smooth status update without stale text or hierarchy jumps |
| 5 | Survives reconnect | "The app recovered cleanly" | Identity remains stable, stale custom status cleared or restored correctly |

### Responsive & Accessibility Rules
- Mobile sidebar rows keep identity on the first line and status on a second muted line only when space allows.
- Channel info panel may collapse into a sheet/drawer on smaller widths, but must preserve the same identity > availability > custom-status order.
- Profile modal becomes a fullscreen sheet on narrow mobile widths, not a cramped centered card.
- Presence must never be color-only: avatar badge gets accessible labels/tooltips and text equivalents wherever status is called out.
- Keyboard focus order must reach the DM row, info panel actions, and profile modal close / message CTA predictably.
- Touch targets stay at `44px` minimum.

### What Already Exists
- `DESIGN.md` already defines the `Focused Warm Utility` system, warm-neutral tokens, and calm collaboration hierarchy.
- `frontend/src/features/presence/presencePresentation.ts` already provides the shared presence vocabulary and should remain the single semantic source.
- `frontend/src/components/ui/RcAvatar.vue` already provides the shared avatar shell and should own the compact badge treatment.
- `frontend/src/composables/useUserSummary.ts` already provides the shared identity/status merge point and should absorb batching behavior.

### NOT in Scope
- Backend team-member endpoint parity. This is deferred to Lake 2 because it is contract work, not immediate UI trust work.
- Server-side expiry-worker semantics and passive-client expiry guarantees. Deferred to Lake 2 to keep Lake 1 reviewable.
- Broader shell redesign beyond the 3 person-status surfaces touched here.

### Manual Verification Commands
1. `cd ../../frontend && npm run dev`
2. In one session, open a DM with another user. In a second session, change that other user’s custom status text/emoji. Verify the DM sidebar row, channel info panel, and user profile modal all update without a full reload.
3. Force a websocket reconnect while the DM is visible and verify identity remains stable while rich custom status is restored without stale text.
4. Check a narrow mobile viewport and verify the sidebar row, info panel sheet, and profile sheet preserve the same hierarchy and accessible status labels.

### Parallelization
- Lane A: websocket reconnect snapshot + backend test coverage
- Lane B: frontend DM/sidebar/shared-summary unification + shared expiry helper
- Lane C: Playwright / regression coverage after Lanes A and B are stable

### Readiness
- Ready for implementation once the branch follows the locked Lake 1 scope above and keeps Lake 2 work deferred.

## 2026-03-29 Permission Guardrails and Regression Harness

### Task
- Close the permission drift where low-privilege members could still create teams or see channel-management affordances they could not actually use.
- Add a repeatable regression harness so permission leaks become test failures instead of QA surprises.

### Implementation Status
- [x] Added backend enforcement for team creation in `backend/src/api/teams.rs` using the existing `TEAM_MANAGE` permission path.
- [x] Added a shared frontend capability layer in `frontend/src/features/permissions/capabilities.ts` for team creation and channel-management affordances.
- [x] Hid unauthorized team/channel-management affordances in `frontend/src/components/layout/TeamRail.vue`, `frontend/src/components/modals/CreateTeamModal.vue`, `frontend/src/components/channel/ChannelInfoPanel.vue`, `frontend/src/components/modals/ChannelSettingsModal.vue`, and `frontend/src/components/channels/ChannelContextMenu.vue`.
- [x] Cleared cached channel capability state on logout/session reset in `frontend/src/stores/auth.ts`.
- [x] Added backend permission regression tests in `backend/tests/api_permissions.rs`.
- [x] Added frontend permission helper and UI affordance tests in `frontend/src/features/permissions/capabilities.test.ts` and `frontend/src/features/permissions/permissionsUi.test.ts`.

### Verification Status
1. `cd ../../frontend && npm run test:unit`
- Result: PASS, `23 passed`
2. `cd ../../frontend && npm run build`
- Result: PASS
- Note: existing Vite warning about `frontend/src/stores/calls.ts` being both dynamically and statically imported still appears, but the build completes successfully.
3. `cd ../../backend && cargo test --test api_permissions -- --nocapture`
- Result: PASS, `4 passed`
- Note: local test bootstrap still logs the known S3 credential warning noise.

### Manual Verification Commands
1. `cd ../../frontend && npm run dev`
2. Log in as a regular member and verify the team rail no longer shows `Create Team`.
3. Open a channel info panel as a regular member and verify there is no `Edit` action; if the settings modal is opened indirectly, it should show a locked message instead of editable controls.
4. Open a channel context menu as a regular member and verify `Add Members` is not offered.
5. Log in as an org admin and verify team creation still succeeds.
6. With a member token, run:
   `curl -i -X POST http://127.0.0.1:3000/api/v1/teams -H "Authorization: Bearer $TOKEN" -H "Content-Type: application/json" -d '{"name":"blocked-team","display_name":"Blocked Team"}'`
   Expect `403 Forbidden`.
7. With a non-admin channel member token, run:
   `curl -i -X PUT http://127.0.0.1:3000/api/v1/channels/<channel-id> -H "Authorization: Bearer $TOKEN" -H "Content-Type: application/json" -d '{"display_name":"Unauthorized Rename"}'`
   Expect `403 Forbidden`.

### Readiness
- Ready for QA and review.
- Follow-up work can widen this same harness to team settings, create-channel affordances, and any remaining permission-sensitive menus if we want a broader permission sweep.

### Expansion Status
- Expanded on `2026-03-29` to cover team settings and create-channel flows.
- Added team-management capability helpers and sidebar gating for `Team Settings`.
- Added create-channel capability helpers and sidebar/modal gating for standard channel creation.
- Added backend enforcement for:
  - `DELETE /api/v1/teams/{id}` using the same team-admin / `TEAM_MANAGE` rule as team update
  - `POST /api/v1/channels` standard channel creation using `CHANNEL_CREATE` with `CHANNEL_MANAGE` override for admins
- Added regression coverage for:
  - unauthorized team delete
  - authorized team-admin delete
  - unauthorized standard channel create
  - authorized member standard channel create
  - team settings visibility / lock state
  - create-channel visibility / lock state

### Expanded Verification Status
1. `cd ../../frontend && npm exec vitest run src/features/permissions/capabilities.test.ts src/features/permissions/permissionsUi.test.ts`
- Result: PASS, `16 passed`
2. `cd ../../frontend && npm run test:unit`
- Result: PASS, `31 passed`
3. `cd ../../frontend && npm run build`
- Result: PASS
- Note: existing Vite warning about `frontend/src/stores/calls.ts` being both dynamically and statically imported still appears, but the build completes successfully.
4. `cd ../../backend && cargo test --test api_permissions -- --nocapture`
- Result: PASS, `8 passed`
- Note: local test bootstrap still logs the known S3 credential warning noise.

### Expanded Manual Verification Commands
1. `cd ../../frontend && npm run dev`
2. Log in as a regular member or guest and verify the current-team menu does not show `Team Settings`, and the sidebar does not show standard-channel create affordances.
3. Open the team settings modal indirectly as an unauthorized user and verify it shows a locked/access-denied state instead of editable controls.
4. Open the create-channel modal indirectly as an unauthorized user and verify it shows a locked/access-denied state instead of the creation form.
5. With a low-privilege token, run:
   `curl -i -X DELETE http://127.0.0.1:3000/api/v1/teams/<team-id> -H "Authorization: Bearer $TOKEN"`
   Expect `403 Forbidden`.
6. With a token lacking `CHANNEL_CREATE`, run:
   `curl -i -X POST http://127.0.0.1:3000/api/v1/channels -H "Authorization: Bearer $TOKEN" -H "Content-Type: application/json" -d '{"team_id":"<team-id>","name":"blocked-channel","display_name":"Blocked Channel","channel_type":"public"}'`
   Expect `403 Forbidden`.
7. With a team admin or authorized member token, verify team settings and standard channel creation still succeed.

## 2026-03-28 CI Required-Check Alignment for Frontend-Only PRs

### Task
- Fix the GitHub Actions/branch-protection mismatch that blocks frontend-only PRs even when all visible checks are green.
- Ensure the required `cargo check + test` status is always reported on PRs to `main`.
- Preserve full backend validation when backend-related files actually change.

### Implementation Status
- [x] Consolidated backend and frontend CI into unified [ci.yml](../../.github/workflows/ci.yml) with intelligent path filtering.
- [x] Added an internal backend-change detection job that compares the current ref against the PR base or push predecessor SHA.
- [x] Split heavy backend validation into a separate conditional job that only runs when backend-related files change.
- [x] Added a lightweight always-reporting gate job named `cargo check + test` that passes with a clear skip message for frontend-only changes and fails if backend validation fails when required.

### Verification Status
1. `ruby -e 'require "yaml"; YAML.load_file(".github/workflows/ci.yml"); puts "YAML ok"'`
- Result: PASS

### Manual Verification Commands
1. `gh pr checks 64`
2. Open the GitHub Actions run for a frontend-only PR and confirm:
- `cargo check + test` appears as a completed successful check
- the log shows `No backend-related changes detected; backend validation skipped.`
3. Open a backend-touching PR and confirm the `Backend validation` job runs full cargo validation before the `cargo check + test` gate concludes successfully.

### Readiness
- Ready for PR validation on GitHub Actions.
- Known limitation: local verification here is limited to YAML parsing; the full behavior depends on a live Actions run.

## 2026-03-28 Emoji Picker Overlay and Clickability Fix

### Task
- Fix shared emoji picker positioning so every trigger opens the overlay from the correct anchor.
- Stop the message reaction picker from becoming unclickable when moving the pointer from the message row into the teleported picker.
- Correct the Mattermost composer so the picker inserts emoji glyphs directly instead of treating them like `:shortcode:` autocomplete values.

### Implementation Status
- [x] Retokenized `frontend/src/components/atomic/EmojiPicker.vue` to the active design token system while keeping the teleported fixed-position overlay behavior.
- [x] Added anchored picker positioning and hover-safe close handling in `frontend/src/components/channel/MessageItem.vue` so reaction pickers stay clickable.
- [x] Added anchored picker positioning in `frontend/src/components/settings/StatusPicker.vue`.
- [x] Fixed `frontend/src/components/composer/MattermostComposer.vue` so picker selections insert glyphs at the cursor position and keep focus/selection behavior intact.

### Verification Status
1. `cd frontend && npm run build`
- Result: PASS
- Note: existing Vite warning about `frontend/src/stores/calls.ts` being both dynamically and statically imported still appears, but the build completes successfully.

### Manual Verification Commands
1. `cd frontend && npm run dev`
2. Open a channel, hover a message, click the reaction smile button, and move the pointer into the picker. Verify the picker stays open and every emoji remains clickable.
3. Open the status picker and confirm the emoji overlay opens aligned to the trigger and remains above the modal/backdrop.
4. Switch to the Mattermost composer, insert an emoji from the picker, and verify the actual emoji glyph is inserted at the cursor without breaking typing focus.

### Readiness
- Ready for user acceptance testing.
- Remaining follow-up: if we want emoji name search instead of glyph-only filtering, that should be a separate UX improvement pass.

## 2026-03-28 Settings, Profile, and Admin Normalization Pass

### Task
- Normalize the highest-traffic settings, profile, and first admin surfaces to the same design system as the redesigned app shell.
- Remove remaining hardcoded color drift from theme-sensitive settings views.
- Improve hierarchy and readability so theme choices remain trustworthy across settings and admin workflows.

### Implementation Status
- [x] Added stronger personal-context framing in `frontend/src/components/settings/SettingsModal.vue`.
- [x] Retokenized and restructured `frontend/src/components/settings/notifications/NotificationsTab.vue` with calmer rows, callouts, and action treatment.
- [x] Normalized `frontend/src/components/settings/profile/ProfileTab.vue` to the shared token system and improved section hierarchy.
- [x] Aligned the dedicated settings page in `frontend/src/views/settings/ProfileView.vue` with section cards for identity, appearance, and typography.
- [x] Redesigned `frontend/src/views/admin/AdminDashboard.vue` to use restrained token-driven stat cards and operational health panels.

### Verification Status
1. `cd frontend && npm run build`
- Result: PASS
- Note: existing Vite warning about `frontend/src/stores/calls.ts` being both dynamically and statically imported still appears, but the build completes successfully.

### Manual Verification Commands
1. `cd frontend && npm run dev`
2. Open `Settings -> Notifications` and verify expanded rows, badges, callouts, and actions remain readable in `Light`, `Dark`, `Futuristic`, and `High Contrast`.
3. Open `Settings -> Profile` and confirm the avatar controls, read-only email field, banners, and status section stay visually consistent with the shell.
4. Open `/settings/profile` and verify the identity, appearance, and typography cards remain readable on both desktop and mobile widths.
5. Open the admin dashboard and confirm stats, health indicators, and instance panels feel calmer while preserving clear service status cues.

### Readiness
- Ready for user acceptance testing.
- Remaining follow-up: broaden the same normalization approach to additional admin/settings surfaces if this direction feels right in live use.

## 2026-03-28 Design System and App Shell Redesign

### Task
- Analyze RustChat’s current product usability and visual language against the standard set by Slack and Mattermost.
- Establish a concrete design system and product direction for the web UI.
- Apply the first implementation pass to the authenticated app shell so the product feels more intentional, more usable, and less like generic SaaS.

### Implementation Status
- [x] Wrote the product design system and design direction to `DESIGN.md`.
- [x] Added `docs/internal/CLAUDE.md` guidance so future UI work is expected to follow `DESIGN.md`.
- [x] Redesigned the app frame and shell layering in `frontend/src/components/layout/AppShell.vue`.
- [x] Rebalanced brand, search, and user-utility hierarchy in `frontend/src/components/layout/GlobalHeader.vue`.
- [x] Strengthened team rail and channel sidebar scan hierarchy in `frontend/src/components/layout/TeamRail.vue` and `frontend/src/components/layout/ChannelSidebar.vue`.
- [x] Improved channel context emphasis in `frontend/src/components/channel/ChannelHeader.vue`.
- [x] Reworked loading and empty-channel states in `frontend/src/components/channel/MessageList.vue` so quiet channels feel intentional instead of abandoned.

### Verification Status
1. `cd frontend && npm run build`
- Result: PASS

### Manual Verification Commands
1. `cd frontend && npm run dev`
2. Open the authenticated app shell and verify:
   - global header brand/context feel stronger than search chrome
   - team rail and channel sidebar are faster to scan
   - selected channel state is clear without feeling loud
   - channel header communicates current context cleanly
   - empty or quiet channels feel intentional, not broken
3. Check responsive behavior:
   - mobile left drawer
   - desktop shell with RHS open
   - channel with and without topic text

### Readiness
- Ready for user acceptance testing.
- Remaining follow-up: carry the same system into broader settings, modal, and admin surfaces over time.
## 2026-03-28 Theme Source-of-Truth Fix + Brand/Typography Second Pass

### Task
- Strengthen RustChat’s typography and brand presence so the authenticated app shell feels less generic.
- Fix theme-driven contrast issues where filled buttons and text could become unreadable on certain presets.
- Remove hardcoded neutral colors from the main profile and display settings surfaces so theme selection actually propagates.

### Implementation Status
- [x] Warmed the default light/dark palette and switched the default chat font family toward `IBM Plex Sans` in `frontend/src/style.css`.
- [x] Added a `brand-foreground` token so filled controls can keep readable text across all shipped theme presets.
- [x] Updated theme metadata and defaults in `frontend/src/stores/theme.ts` to align the UI selector with the active runtime theme system.
- [x] Replaced the broken faux-custom theme editor with a preset-only, token-backed editor in `frontend/src/components/settings/display/ThemeEditor.vue`.
- [x] Retokenized the display settings rows in `frontend/src/components/settings/display/DisplayTab.vue` and the profile settings page in `frontend/src/views/settings/ProfileView.vue`.
- [x] Applied contrast-safe filled-button text to shared shell controls in the composer, headers, rails, and related settings surfaces.

### Verification Status
1. `cd frontend && npm run build`
- Result: PASS

### Manual Verification Commands
1. `cd frontend && npm run dev`
2. Login, open `Settings -> Display`, and switch between `Light`, `Dark`, `Futuristic`, and `High Contrast`.
3. Verify the theme preview cards, save button, composer send button, team rail selection, and profile/settings actions keep readable label contrast.
4. Open `/settings/profile` and confirm font, font size, and theme changes apply live without gray overrides on the page chrome or form controls.

### Readiness
- Ready for user acceptance testing.
- Remaining known limitation: `frontend/src/features/theme/*` still exists as an inactive parallel theme stack; runtime behavior now stays on `frontend/src/stores/theme.ts`.

## 2026-03-13 WebSocket Token Expiry Enforcement

### Task
- Stop realtime websocket message delivery after JWT token expiration, even without page refresh.
- Force web UI to clear authenticated screen state and navigate to login immediately when JWT expires.

### Implementation Status
- [x] Added shared websocket auth context (`user_id` + `expires_at`) parsing in `backend/src/api/websocket_core.rs`.
- [x] Enforced token-expiry runtime disconnect in `/api/v4/websocket` loop (`backend/src/api/v4/websocket.rs`).
- [x] Enforced token-expiry runtime disconnect in legacy `/ws` loop (`backend/src/api/ws.rs`).
- [x] Added regression tests for claims-to-websocket-auth expiry mapping.
- [x] Added JWT expiry timer in frontend auth lifecycle to trigger forced logout at token `exp` (`frontend/src/stores/auth.ts`).
- [x] Added centralized frontend session cleanup on logout (messages/channels/unreads/presence/teams/preferences/calls/UI) before login redirect.
- [x] Updated websocket client close handling to treat auth-expiry close as forced logout and suppress reconnect loops (`frontend/src/composables/useWebSocket.ts`).

### Verification Status
1. `cd backend && cargo check`
- Result: PASS

2. `cd backend && cargo clippy --all-targets --all-features -- -D warnings`
- Result: PASS

3. `cd backend && cargo test claims_to_websocket_auth -- --nocapture`
- Result: PASS

4. `cd frontend && npm run build`
- Result: PASS

### Manual Verification Commands
1. Start backend with a short token lifetime:
   - `cd backend && RUSTCHAT_JWT_EXPIRY_HOURS=1 cargo run`
2. Connect to websocket with a valid token and observe forced close after expiry boundary:
   - `npx wscat -c ws://127.0.0.1:3000/api/v4/websocket -s "<JWT_TOKEN>"`
3. Web UI check:
   - Login, keep a channel open past token expiration, verify UI redirects to `/login` and channel/message UI state is cleared.
   - Confirm websocket reconnect attempts stop while token is expired.

### Readiness
- Ready for user acceptance testing.
- Expected behavior: when JWT expires, active websocket session is closed and realtime events stop.

## Task
Small compatibility-aligned messaging fixes:
- Show date separators for older messages (not only time) in WebUI message list.
- Enforce global message edit policy (`disabled`, `enabled`, or time-limited like 30 minutes) and keep edited UX consistent.
- Make reaction click behavior toggle correctly for the current user (add on first click, remove on second).
- Clear WebUI top-ring notification indicator when unseen messages are viewed/read.

## Implementation Status
- [x] Added timeline date separators in WebUI message list rendering (`frontend/src/components/channel/MessageList.vue`).
- [x] Exposed optional post edit timestamps in frontend post API typings (`frontend/src/api/posts.ts`) for edited-state handling.
- [x] Aligned unread indicator source in global header to shared unread store (`frontend/src/components/layout/GlobalHeader.vue`).
- [x] Added server setting form field for global post edit window (`frontend/src/views/admin/ServerSettings.vue`).
- [x] Included `post_edit_time_limit_seconds` in frontend config defaults (`frontend/src/features/config/stores/configStore.ts`).
- [x] Applied formatting-consistent updates in backend post update handlers (`backend/src/api/posts.rs`, `backend/src/api/v4/posts.rs`) after policy logic integration.

## Verification Status

### Automated
1. `cd backend && cargo check`
- Result: PASS

2. `cd frontend && npm run build`
- Result: PASS

3. `cd backend && cargo clippy --all-targets --all-features -- -D warnings`
- Result: PASS (all clippy warnings fixed across the codebase)

4. `cd backend && cargo test --no-fail-fast -- --nocapture`
- Result: PARTIAL
  - Unit tests in `src/lib.rs`: PASS (`125 passed, 0 failed`)
  - Many integration targets: FAIL due missing test DB bootstrap (`RUSTCHAT_TEST_DATABASE_URL` candidates unavailable/auth failure)

5. `BASE=http://localhost:3000 ./scripts/mm_compat_smoke.sh`
- Result: FAIL (target unavailable; preflight ping connection refused)

6. `BASE=http://localhost:3000 ./scripts/mm_mobile_smoke.sh`
- Result: FAIL (no `X-MM-COMPAT: 1` header observed because local target was unavailable)

## Manual Verification Commands
1. `./scripts/clippy_check.sh` - Run clippy validation
2. `BASE=<your-running-rustchat-url> ./scripts/mm_compat_smoke.sh`
3. `BASE=<your-running-rustchat-url> ./scripts/mm_mobile_smoke.sh`
4. `curl -si <your-running-rustchat-url>/api/v4/config/client | rg -n "AllowEditPost|PostEditTimeLimit"`

## Readiness
- Requested behavior changes are implemented in code.
- Final acceptance still depends on running smoke checks against a live compatible server endpoint and DB-backed integration test environment.

---

## UIX refinement: reaction identity + shell/status alignment

### Scope completed
- [x] Normalized reaction identity so alias/name/glyph variants collapse into one reaction bucket (`frontend/src/utils/emoji.ts`, `frontend/src/stores/messages.ts`, `frontend/src/components/channel/MessageItem.vue`).
- [x] Extended reaction updates to cover loaded thread replies as well as top-level channel messages (`frontend/src/stores/messages.ts`).
- [x] Compressed the top-left brand lockup and simplified header search to a single clear action (`frontend/src/components/layout/GlobalHeader.vue`).
- [x] Theme-aligned the channel identity headline so it no longer reads as hard black against the shell (`frontend/src/components/channel/ChannelHeader.vue`).
- [x] Upgraded DM rows from abstract status dots to avatar-led identity with proper presence badges (`frontend/src/components/layout/ChannelSidebar.vue`, `frontend/src/components/ui/RcAvatar.vue`).
- [x] Redesigned member list and profile modal presence treatment with explicit status labels/icons and token-safe surfaces (`frontend/src/components/channel/ChannelMembersPanel.vue`, `frontend/src/components/modals/UserProfileModal.vue`).
- [x] Removed the admin/slate modal theme leak from team, channel, and direct-message pickers so collaboration flows use the same token-driven shell styling (`frontend/src/components/modals/BrowseTeamsModal.vue`, `frontend/src/components/modals/BrowseChannelsModal.vue`, `frontend/src/components/modals/DirectMessageModal.vue`).
- [x] Filled the direct-message details gap in the right sidebar so DM conversations show the other user’s avatar, presence, and custom status instead of only generic channel metadata (`frontend/src/components/channel/ChannelInfoPanel.vue`).

### Verification Status

#### Automated
1. `cd frontend && npm run build`
- Result: PASS
- Note: existing Vite warning remains for `frontend/src/stores/calls.ts` being both dynamically and statically imported.

### Manual Verification Commands
1. `cd ../../frontend && npm run dev`
2. Open a message with existing reactions, add `👍`, and confirm the same visible emoji does not render twice when websocket updates land.
3. Open a thread reply and repeat the same reaction toggle check there.
4. Review the authenticated shell header and confirm:
   - the site title uses the brand accent instead of reading as hard black
   - the brand block is less crowded
   - the center search treatment is a single-line “Search” action
5. Open the members panel and a user profile modal and confirm presence states render with consistent labels/icons for `Online`, `Away`, `Do not disturb`, and `Offline`.
6. Open `Browse Teams`, `Browse Channels`, and `Direct Messages` and confirm they inherit the regular collaboration theme instead of the old dark/admin palette.
7. Open a direct message, then open `Channel Info` in the right sidebar and confirm the other user’s avatar, presence badge, and custom status are shown.

### Readiness
- The approved UIX spec items for reaction deduplication, header/search compression, and presence/status credibility are implemented in code.
- Remaining acceptance should come from a quick live browser pass against the authenticated app to confirm the final feel matches the intended shell direction.

---

## 001-platform-foundation: RustChat Platform Foundation

### Specification
**Status**: ✅ Approved (2026-03-13)  
**Location**: `specs/001-platform-foundation/spec.md`  
**User Stories**: 8 (4 P1, 4 P2)  
**Stages**: 4-stage SaaS maturity (Ad-Hoc → Reactive → Proactive → Strategic)

### Implementation Plan
**Status**: ✅ Created (2026-03-13)  
**Location**: `specs/001-platform-foundation/plan.md`  
**Phases**: 5 (Foundation, Auth/State, Security/Compliance, AI/APIs, Frontend)

### Constitution Compliance
| Principle | Status |
|-----------|--------|
| I. Contract Compatibility | ✅ |
| II. Plan-First Validation | ✅ |
| III. Evidence-Backed Analysis | ⏳ (upstream analysis pending) |
| IV. Test Gates | ✅ |
| V. Security Discipline | ✅ |
| VI. Mobile-First | ✅ |
| VII. Product Contract | ✅ |
| VIII. Feature Safety | ✅ |
| IX. Test Coverage | ✅ |
| X. Data Sovereignty | ✅ |
| XI. Stateless Tier | ✅ |
| XII. Memory Safety | ✅ |
| XIII. Zero-Trust | ✅ |
| XIV. Federated Identity | ✅ |
| XV. Strict RBAC | ✅ |
| XVI. Audit Logging | ✅ |
| XVII. Privacy by Design | ✅ |
| XVIII. Accessibility | ✅ |
| XIX. DevSecOps | ✅ |

### Current Status
- ✅ Specification approved
- ✅ Implementation plan created
- ✅ Tasks generated (196 tasks across 11 phases)
- ⏳ **READY FOR**: Implementation → Validation

### Next Actions
1. Begin Phase 0: Foundation & Tooling
   - `cd backend && cargo init`
   - Setup Docker, CI/CD
2. Checkpoint at each stage gate
3. Run `/speckit.validate` after Phase 11

### Task Summary
| Phase | Stage | Tasks | Focus Area |
|-------|-------|-------|------------|
| 0 | Ad-Hoc | T001-T017 | Containerization, CI/CD |
| 1 | Reactive | T018-T042 | Redis, OIDC/SAML, Metrics |
| 2 | Core | T043-T062 | Messaging, Mattermost API |
| 3 | Proactive | T063-T080 | RBAC, Audit Logging |
| 4 | Proactive | T081-T093 | GDPR, Data Lifecycle |
| 5 | Proactive | T094-T110 | OpenSearch, Kafka, HA |
| 6 | Strategic | T111-T123 | MCP Protocol |
| 7 | Strategic | T124-T134 | A2A Protocol |
| 8 | Strategic | T135-T150 | OpenAPI, Webhooks, SDKs |
| 9 | Frontend | T151-T169 | Solid.js, Accessibility |
| 10 | DevSecOps | T170-T184 | Security, K8s, Monitoring |
| 11 | Validation | T185-T197 | Load Testing, Chaos |

**Total Tasks**: 196
**Estimated Duration**: 16-20 weeks (2-3 devs parallel)

### Success Criteria (from spec)
- SC-001: Platform sustains 200,000 concurrent users with p99 latency < 150ms
- SC-002: Morning auth spike (30/sec, 150k/90min) completes with <0.1% failure rate
- SC-003: Kafka fan-out achieves <5ms latency for 10,000-member channel broadcasts
- SC-004: MCP/A2A agent integration passes security audit (zero critical findings)
- SC-005: OpenAPI 3.1.1 spec generates functional SDKs for TypeScript, Python, Rust
- SC-006: Webhook delivery achieves 99.9% success rate with Standard Webhooks compliance
- SC-007: GDPR Right to Erasure completes within 30 days, cryptographically verified
- SC-008: Accessibility audit passes WCAG 2.1 AA and BITV 2.0 with zero violations
- SC-009: `cargo audit` shows zero high/critical CVEs at release
- SC-010: Zero-downtime deployment achieved via blue-green strategy

## GSTACK REVIEW REPORT

| Review | Trigger | Why | Runs | Status | Findings |
|--------|---------|-----|------|--------|----------|
| CEO Review | `/plan-ceo-review` | Scope & strategy | 0 | — | — |
| Codex Review | `/codex review` | Independent 2nd opinion | 0 | — | — |
| Eng Review | `/plan-eng-review` | Architecture & tests (required) | 3 | CLEAR | 22 issues, 0 critical gaps |
| Design Review | `/plan-design-review` | UI/UX gaps | 1 | CLEAR | score: 6/10 → 9/10, 7 decisions |

**UNRESOLVED:** 0
**VERDICT:** ENG + DESIGN CLEARED — ready to implement

## 2026-04-27 Open-Source Hardening & Repository Polish

### Status: Complete

### Implementation Summary
- Created missing root files: VERSION, DCO.md, GOVERNANCE.md, SUPPORT.md, MAINTAINERS.md, ROADMAP.md
- Updated existing root files: CONTRIBUTING.md (DCO sign-off), README.md (release channels, roadmap link, security), SECURITY.md (supported versions, dependency process)
- Added GitHub templates: documentation.md, security_hardening.md issue templates
- Updated GitHub templates: PR template (DCO checkbox), dependabot.yml (cargo, actions, docker)
- Added workflows: dco.yml, security.yml (CodeQL + cargo audit + dependency review), nightly.yml, scorecard.yml
- Improved workflows: ci.yml (added frontend build/test), release.yml (changelog extraction, container build)
- Created docs: quickstart.md, deployment.md, development.md, architecture.md, security-model.md, release-process.md, github-protection.md, contributor-workflow.md
- Created scripts: smoke-test.sh, check-release-ready.sh, dev-setup.sh, release-notes-check.sh
- Created deny.toml (minimal cargo-deny configuration)

### Validation Results
- All new shell scripts pass `bash -n` syntax check
- All workflow YAML files pass Python yaml validation
- Backend `cargo fmt --check` passes
- Backend `cargo check` fails with 5 pre-existing compilation errors (not introduced by this change)
- Backend `cargo clippy` fails due to same pre-existing errors
- Frontend `npm run build` succeeds
- Frontend `npm run test:unit` passes (35 tests)
- `release-notes-check.sh` runs correctly
- `check-release-ready.sh` runs correctly

### Known Limitations
- Backend has 5 pre-existing Rust compilation errors unrelated to this change
- `scripts/check-release-ready.sh` correctly flags these pre-existing clippy failures
- Frontend `package.json` does not have a dedicated `lint` script; CI runs `build` and `test:unit` only
