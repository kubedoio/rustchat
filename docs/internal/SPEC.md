# SPEC: Frontend Supply-Chain Hardening and Dependency Minimization (2026-04-11)

## Problem Statement

The frontend supply chain is only partially hardened today:
1. frontend CI installs with `npm ci`, but lifecycle scripts are still enabled.
2. There is no dependency-review gate, vulnerability gate, or release-time dependency inventory artifact for frontend changes.
3. The repo has had mixed package-manager signals (`frontend/package-lock.json` is active in CI, while `frontend/pnpm-lock.yaml` existed in git history until the current branch removed it), which weakens contributor clarity.
4. The frontend already contains an internal fetch-based transport layer in `frontend/src/api/http/HttpClient.ts`, but raw `fetch()` calls still exist in theme preference code, leaving the HTTP boundary inconsistent.
5. Several direct dependencies appear to be unnecessary or no longer used in the current source tree, increasing review and remediation burden without clear payoff.

The goal of this task is to harden the existing Vue frontend delivery path without rewriting the framework or expanding the dependency surface to solve dependency problems.

## Current-State Findings

### Package Management / Install Surface

1. Frontend CI already assumes npm and `frontend/package-lock.json`:
   - `frontend/package-lock.json`
   - [.github/workflows/ci.yml](../../.github/workflows/ci.yml)
2. The current branch also deletes `frontend/pnpm-lock.yaml`, confirming package-manager normalization is already in motion:
   - `frontend/pnpm-lock.yaml`
3. The frontend Docker build still falls back to `npm install` if a lockfile is missing, which violates strict lockfile-only intent:
   - `docker/frontend.Dockerfile`

### CI / Review Gaps

1. No workflow currently performs dependency review, vulnerability scanning, SBOM generation, or dependency policy validation for frontend changes.
2. Frontend CI currently runs:
   - checkout
   - `npm ci`
   - unit tests
   - build
3. That means dependency drift visibility and install-time trust are not yet enforced in PRs.

### HTTP Boundary

1. The project already has a centralized internal fetch client:
   - `frontend/src/api/client.ts`
   - `frontend/src/api/http/HttpClient.ts`
2. Raw `fetch()` remains in theme preference code:
   - `frontend/src/features/theme/repositories/themeRepository.ts`
   - `frontend/src/stores/theme.ts`
3. The legacy store in `frontend/src/stores/theme.ts` duplicates behavior that already exists in the feature-layer theme service/repository path.

### Initial Dependency Audit

Likely required direct dependencies:
1. `vue`, `vue-router`, `pinia`
2. `vite`, `@vitejs/plugin-vue`, `typescript`, `vue-tsc`
3. `@vueuse/core`
4. `lucide-vue-next`
5. `date-fns`
6. `dompurify`, `marked`, `highlight.js`
7. `@tiptap/vue-3`, `@tiptap/starter-kit`, `@tiptap/extension-placeholder`
8. `match-sorter`
9. test/build dependencies already exercised by the repo (`vitest`, `@vue/test-utils`, `jsdom`, `@playwright/test`, Tailwind/PostCSS stack)

Likely removable direct dependencies based on current source usage:
1. `clsx`
2. `tailwind-merge`
3. `@tiptap/extension-mention`

Likely removable type-only packages if local typecheck confirms bundled upstream types are sufficient:
1. `@types/marked`
2. `@types/dompurify`

Needs confirmation before removal:
1. `@types/md5`
2. Any dependency only referenced through currently-unused legacy files

### Validation Constraint

The current Codex shell environment does not expose `node`, `npm`, `pnpm`, or `corepack` on the default `PATH`, so frontend verification may require explicitly adding the Homebrew toolchain path before running install/build commands. CI remains the primary trust boundary for enforcing the hardened dependency workflow.

## Goals

1. Reduce direct frontend dependency surface where removal is low-risk and justified.
2. Standardize the repo on one frontend package manager with one committed lockfile.
3. Enforce lockfile-only frontend installs in CI and release builds.
4. Block package lifecycle scripts by default in CI, with explicit documented exceptions only.
5. Add automated PR visibility and gating for dependency changes.
6. Support central transitive remediation via overrides and a documented patch workflow.
7. Tighten the frontend HTTP boundary around the existing internal fetch wrapper.
8. Document the dependency policy so contributors default to the secure path.

## Non-Goals

1. Replacing Vue or changing the frontend framework.
2. Broad frontend architecture rewrites unrelated to dependency/HTTP hardening.
3. Backend dependency hardening.
4. Browser runtime hardening beyond the frontend build/install supply-chain path.
5. Adding convenience dependencies to solve a dependency-management problem unless the ADR proves the tradeoff is necessary.

## Scope and Contract Impact

### In Scope

1. Frontend package manager and lockfile normalization.
2. Frontend dependency minimization and dependency policy documentation.
3. Frontend CI hardening for install behavior, dependency review, vulnerability checks, and dependency-policy enforcement.
4. Release/build-path hardening for frontend install behavior where practical.
5. Frontend override and patch workflow for vulnerable transitive dependencies.
6. Simplifying remaining raw frontend HTTP access behind internal wrappers where reasonable.
7. Machine-readable dependency inventory/SBOM generation in CI when feasible without expanding the dependency surface unnecessarily.

### Contract Impact

1. Frontend dependency changes will become review-gated in PRs rather than passive lockfile churn.
2. CI frontend installs will stop executing package lifecycle scripts by default.
3. Contributors will need to use the approved package-manager workflow and lockfile discipline.
4. Theme preference HTTP traffic will move onto the internal transport boundary instead of raw `fetch()` usage.

## Architecture Questions That Must Be Locked in ADR

1. Package manager choice:
   - npm vs pnpm
   - decision criteria: patchability, contributor clarity, existing repo direction, CI simplicity
2. HTTP boundary:
   - keep the internal fetch wrapper and migrate remaining raw `fetch()` theme flows behind it
   - or retain raw fetch in narrowly scoped areas if the ADR proves that is safer/simpler
3. Lifecycle-script policy:
   - CI-only blocking by default vs broader/global blocking
4. Transitive remediation:
   - overrides only vs overrides plus repository-managed patch workflow
5. Update workflow:
   - Dependabot or equivalent PR-based review flow
   - severity/merge policy for majors vs security patches
6. Dependency inventory:
   - minimal machine-readable manifest only
   - or SBOM artifact generation during CI/release

## Implementation Outline

### Phase 1: ADR + Policy Baseline

1. Write `docs/adr/ADR-frontend-supply-chain-security.md` with the required comparisons and explicit decisions.
2. Write `docs/frontend-dependency-policy.md` covering:
   - allowed dependency categories
   - PR requirements for new dependencies
   - override workflow
   - patch workflow
   - exception process for trusted install scripts

### Phase 2: Dependency Surface Reduction

1. Remove confirmed-unused direct dependencies from `frontend/package.json`.
2. Remove redundant type packages when upstream bundled types are sufficient.
3. Preserve only dependencies with clear functional or tooling justification in the policy document.

### Phase 3: HTTP Boundary Tightening

1. Route theme preference network access through the internal HTTP client boundary.
2. Remove or isolate duplicate legacy raw-fetch logic where it is safe to do so.
3. Keep request auth/header/error handling centralized.

### Phase 4: CI / Build Hardening

1. Update frontend CI to:
   - install from lockfile only
   - block lifecycle scripts by default
   - validate dependency policy when manifests/lockfiles change
   - run dependency review on PRs
   - run vulnerability scanning
   - generate dependency inventory / SBOM artifact if feasible
2. Update the frontend Docker build path so it no longer falls back to unlocked installs.

### Phase 5: Override / Patch Controls

1. Keep `overrides` in the package manifest as the first emergency mitigation tool.
2. Add a repository-managed patch workflow for cases where version forcing is insufficient.
3. Document version pinning, patch application, and patch retirement steps.

### Phase 6: Contributor Workflow Updates

1. Update contributor-facing docs to standardize frontend install/update commands.
2. Document how security patch updates differ from normal feature dependency updates.

## Contract / Verification Plan

### Contract A: Lockfile Enforcement

Goal:
1. A frontend manifest change without synchronized lockfile changes must fail CI.

Planned verification:
1. Frontend CI installs with a lockfile-only command.
2. A dependency-policy check runs on manifest/lockfile changes and treats missing/mismatched lockfile updates as failure.

### Contract B: Script Blocking

Goal:
1. Frontend CI must not execute dependency lifecycle scripts unless an explicit exception path is invoked.

Planned verification:
1. CI uses install commands with script blocking enabled.
2. Policy documentation records any exceptions and their rationale.

### Contract C: Dependency Review

Goal:
1. PRs that introduce vulnerable or disallowed dependency changes must be surfaced and blocked.

Planned verification:
1. GitHub dependency review action for PRs.
2. Repo-owned dependency policy check for direct dependency allowlisting and justification coverage.

### Contract D: Override / Patch Path

Goal:
1. A vulnerable transitive dependency can be remediated centrally without unrelated application edits.

Planned verification:
1. Use `overrides` for version forcing.
2. Add a documented and runnable repository patch path when override is insufficient.

### Contract E: Build Integrity

Goal:
1. Hardened frontend install/build flow still produces a working build.

Planned verification:
1. Frontend unit-test/build CI continues to pass under the hardened install settings.
2. Frontend Docker/release build path uses the same lockfile trust model.

### Contract F: HTTP Boundary Behavior

Goal:
1. Theme preference flows keep working after raw `fetch()` migration behind the internal client.

Planned verification:
1. Request auth header behavior preserved.
2. JSON serialization/deserialization preserved.
3. Error handling remains centralized and consistent.

## Risks and Notes

1. Package-manager normalization is the most consequential decision because it affects every contributor workflow; the ADR needs to be explicit and durable.
2. Blocking lifecycle scripts may expose hidden assumptions in packages such as Playwright or native optional dependencies; the implementation should keep exception handling explicit rather than silently re-enabling scripts.
3. Because the shell environment may require explicit PATH setup for local Node tooling, CI remains the most reliable enforcement point for frontend install/build hardening.
4. The repo currently contains both legacy and feature-structured theme code; this task should reduce boundary inconsistency without trying to refactor the whole theming subsystem.

# SPEC: Permanent Avatar URL Stabilization (2026-04-05)

## Problem Statement

Authenticated RustChat surfaces are rendering expired S3 presigned URLs from `users.avatar_url`, which causes repeated browser `403 AccessDenied` console errors after login and during channel/team rendering.

The current broken path is:
1. The profile UI uploads an avatar through `/api/v1/files`.
2. `/api/v1/files` returns a 1-hour presigned S3 download URL.
3. The frontend persists that URL directly into `users.avatar_url`.
4. Auth, user, team-member, activity, and post APIs return the stored value unchanged.
5. The frontend renders that stale URL later in `<img>` tags, which then fails with `Request has expired`.

The repository already contains the correct long-term pattern on the Mattermost-compatible avatar path:
- upload to a stable object key under `avatars/{user}`
- expose a stable authenticated app URL at `/api/v4/users/{id}/image`
- avoid persisting expiring S3 URLs in user profile records

## Goals

1. Eliminate expired-avatar console errors permanently.
2. Stop storing temporary presigned S3 URLs in `users.avatar_url`.
3. Normalize backend avatar responses so all authenticated app APIs return stable, non-expiring avatar URLs.
4. Preserve already-uploaded legacy avatars that were stored via the older generic-files path.
5. Keep avatar image access authenticated and compatible with the current frontend cookie-based `<img>` loading model.

## Non-Goals

1. No redesign of general file attachment URLs in this pass.
2. No S3 bucket policy changes.
3. No profile UI redesign beyond changing the avatar upload flow.
4. No broad migration of unrelated `avatar_url`-like fields unless they are required to close this bug.

## Scope and Contract Impact

### In Scope

1. Frontend avatar upload flows:
   - `frontend/src/views/settings/ProfileView.vue`
   - `frontend/src/components/settings/profile/ProfileTab.vue`
2. Backend avatar response normalization for native authenticated APIs that currently emit raw `users.avatar_url`.
3. Backward-compatible serving of legacy avatar uploads that were stored under `files/{user}/{file}` and persisted as expiring presigned URLs.
4. Regression coverage for stable avatar URL emission and legacy fallback handling.

### Contract Impact

1. `avatar_url` in authenticated API responses should become a stable RustChat URL, not a temporary S3 presigned URL.
2. New avatar uploads from the frontend should use the profile-image upload endpoint instead of the generic files upload endpoint.
3. Previously stored legacy avatar URLs should continue to resolve via a stable authenticated URL after this change.

### Out of Scope

1. Rewriting generic `files.url` payloads for normal post attachments.
2. Guest/public avatar access semantics.
3. Full API v4 parity review for every user/team/member endpoint not touched by this fix.

## Current Findings

1. `/api/v1/files` currently returns a presigned download URL:
   - `backend/src/api/files.rs`
2. Both profile UIs persist that returned URL directly into `users.avatar_url`:
   - `frontend/src/views/settings/ProfileView.vue`
   - `frontend/src/components/settings/profile/ProfileTab.vue`
3. `UserResponse::from(User)` currently returns `user.avatar_url` verbatim:
   - `backend/src/models/user.rs`
4. Native auth, posts, teams, and related joins also expose `u.avatar_url` directly:
   - `backend/src/api/auth.rs`
   - `backend/src/api/posts.rs`
   - `backend/src/api/teams.rs`
5. The stable avatar implementation already exists on the v4 upload/read path:
   - `backend/src/api/v4/users.rs`

## Implementation Outline

### Phase 1: Regression Harness

1. Add backend tests that prove:
   - presigned S3 avatar URLs are normalized into stable `/api/v4/users/{id}/image` responses
   - stable relative avatar URLs are preserved
   - legacy file-backed avatars still resolve through the stable image endpoint

### Phase 2: Backend Stabilization

1. Add one shared backend helper that computes the effective public avatar URL for a user.
2. Update `UserResponse::from(User)` and other native response builders/joins to emit the effective stable avatar URL instead of the raw stored value.
3. Extend `GET /api/v4/users/{user_id}/image` so it:
   - first checks the new `avatars/{user}` object
   - then falls back to a legacy file object derived from the previously stored S3 URL or stored file key
4. Keep `POST /api/v4/users/{user_id}/image` as the canonical long-term write path.

### Phase 3: Frontend Upload Flow

1. Change both profile-avatar upload flows to use `POST /api/v4/users/{id}/image`.
2. Refresh the authenticated user after upload instead of persisting a presigned URL locally.
3. Keep remove-avatar behavior compatible with the current backend semantics, or leave it unchanged if deletion support is not part of the current stable image contract.

### Phase 4: Verification

1. Run focused backend tests for avatar normalization and fallback.
2. Run targeted frontend tests or build checks for the updated upload flow.
3. Reproduce the original live login scenario and confirm the prior expired-avatar console errors no longer occur.

## Verification Plan

Automated:
1. `cd ../../backend && cargo test avatar -- --nocapture`
2. `cd ../../backend && cargo test --no-fail-fast -- --nocapture`
3. `cd ../../backend && cargo check`
4. `cd ../../frontend && npm run build`

Manual:
1. Log in at `https://app.rustchat.io` and confirm avatars render without `403 AccessDenied` console errors.
2. Open the main channel view, team member lists, DM surfaces, and admin pages and confirm avatar requests target stable app URLs rather than `s3.rustchat.io` presigned URLs.
3. Upload a new avatar from profile settings, refresh the app, and confirm the avatar still loads after re-login.

## Risks and Notes

1. Some existing users only have legacy file-backed avatar data, so removing the old path without a fallback would break their avatars.
2. Native API joins that return `u.avatar_url` directly may need response-level normalization rather than schema changes to avoid a wide DB migration in this pass.
3. The cleanest permanent state is to migrate old users onto the canonical `avatars/{user}` object path, but runtime fallback is still needed to make the fix safe immediately.

# SPEC: Categories Test Fix Without Relaxing Team Creation Permissions (2026-03-29)

## Problem Statement

`backend/tests/api_categories.rs` fails before any sidebar-category logic runs because the test setup tries to create a team through `POST /api/v1/teams` as a normal `member` user:
- `backend/tests/api_categories.rs:69`
- `backend/tests/api_categories.rs:257`

But the product rule is that regular `member` users should **not** be allowed to create teams. Team creation is intentionally restricted.

So the correct fix is not to relax backend permissions. The correct fix is to change the failing tests and any contradictory regression coverage so they set up teams using an allowed path.

## Goals

1. Preserve the existing restriction that `member` users cannot create teams.
2. Fix the category integration tests so they no longer rely on forbidden behavior.
3. Restore regression coverage so it matches the intended permission model.

## Non-Goals

1. No change to the live team-creation permission contract.
2. No RBAC redesign.
3. No change to sidebar category behavior unless test setup reveals a second real bug.
4. No frontend permission work in this pass.

## Scope and Contract Impact

### In Scope

1. Revert the local permission relaxation in:
   - `backend/src/api/teams.rs`
   - `backend/src/auth/policy.rs`
2. Update `backend/tests/api_categories.rs` to create teams via an allowed path.
3. Update `backend/tests/api_permissions.rs` so it asserts that members are blocked from team creation.

### Contract Impact

1. `POST /api/v1/teams` remains forbidden for `member`.
2. `POST /api/v1/teams` remains allowed only for the intended elevated role path.
3. Category tests stop depending on a forbidden bootstrap flow.

## Implementation Outline

1. Revert the local change that switched team creation from `TEAM_MANAGE` to `TEAM_CREATE`.
2. Revert the local `TEAM_CREATE` grants added to `member` and the related policy assertions.
3. Change category-test setup to one of these allowed paths:
   - register/login as `org_admin` when the test needs to create a team via API, or
   - insert the team and membership directly in the test database
4. Restore permission regression tests so:
   - `member` cannot create team
   - elevated role can create team
5. Re-run the focused suites.

## Verification Plan

Automated:
1. `cd ../../backend && cargo test --test api_categories -- --nocapture`
2. `cd ../../backend && cargo test --test api_permissions -- --nocapture`
3. `cd ../../backend && cargo check`
4. `cd ../../backend && cargo clippy --all-targets --all-features -- -D warnings`

Manual:
1. Attempt `POST /api/v1/teams` with a `member` token and confirm `403 Forbidden`.
2. Attempt the same with an elevated allowed role and confirm success.

## Risks and Notes

1. The current local workspace contains an uncommitted permission relaxation that should be reverted before this lands.
2. If many tests assume self-serve team creation, we may need a shared admin bootstrap helper to keep future tests honest.
3. Direct DB setup is acceptable for integration-test scaffolding if it keeps the API contract strict and the test intent focused.

# SPEC: Permission Guardrails Expansion for Team Settings and Create-Channel Flows (2026-03-29)

## Problem Statement

We already closed the first permission drift around team creation and channel-management affordances, but two adjacent areas still have the same failure mode:

1. Team settings are not consistently permission-gated.
   - `frontend/src/components/layout/ChannelSidebar.vue` shows `Team Settings` unconditionally from the team menu.
   - `frontend/src/components/modals/TeamSettingsModal.vue` assumes full edit/delete/member-management access.
   - `backend/src/api/teams.rs` protects `PUT /teams/{id}` with team-admin or `TEAM_MANAGE`, but `DELETE /teams/{id}` currently has no permission check at all.
2. Create-channel flows are not aligned to the explicit permission model.
   - `frontend/src/components/layout/ChannelSidebar.vue` exposes `Create channel` from category headers and the footer without a shared capability gate.
   - `frontend/src/components/modals/CreateChannelModal.vue` assumes the action is available if a team is selected.
   - `backend/src/api/channels.rs` currently allows standard channel creation for any team member, even though the policy model already defines `CHANNEL_CREATE`.

That means we still have permission drift:
- some actions are visible when they should not be
- some destructive operations are insufficiently enforced on the backend
- the test harness does not yet cover these neighboring flows

## Goals

1. Extend the shared permission/capability layer to cover team settings and create-channel flows.
2. Make team settings obey one rule everywhere:
   - global `TEAM_MANAGE`
   - or current-team membership role `admin` / `owner`
3. Make standard channel creation obey one rule everywhere:
   - explicit `CHANNEL_CREATE` permission
   - plus current-team membership
4. Add regression tests so these permission leaks become CI failures.
5. Preserve authorized flows for team admins, org admins, system admins, and regular members where the policy explicitly allows it.

## Non-Goals

1. No full RBAC redesign.
2. No change to direct-message creation semantics unless current research shows a concrete permission bug there.
3. No admin-console redesign.
4. No sweep across every permission-sensitive screen in the product.
5. No silent change to unrelated API v4 compatibility routes in this pass.

## Scope and Contract Impact

### In Scope

1. Team settings frontend gating:
   - team menu entry visibility
   - modal lock state if opened indirectly
   - member-management actions inside the modal
2. Team settings backend enforcement:
   - add permission checks to `DELETE /api/v1/teams/{id}`
   - keep delete/update semantics aligned
3. Create-channel frontend gating:
   - category header plus-buttons
   - footer `Create channel`
   - modal lock state if opened indirectly
4. Create-channel backend enforcement:
   - require `CHANNEL_CREATE` for standard public/private channel creation
   - keep current team-membership validation
5. Regression tests:
   - backend integration coverage for team delete and channel create permissions
   - frontend unit/UI coverage for hidden or locked team-settings and create-channel affordances

### Contract Impact

1. Unauthorized team deletion should return `403 Forbidden` instead of succeeding.
2. Standard channel creation should be explicitly gated by `CHANNEL_CREATE` instead of only team membership.
3. Unauthorized users should no longer see team-settings or create-channel controls as normal affordances.
4. Existing authorized users should keep the same successful behavior.

### Out of Scope

1. Team-join policy redesign.
2. Browse teams / browse channels access-model redesign.
3. Full admin teams-management permission normalization.
4. Direct-message or group-message creation redesign unless implementation uncovers a blocker.

## Current Findings and Proposed Rules

### Team Settings

Existing backend rule for update:
- allow if user has global `TEAM_MANAGE`
- otherwise allow if current-team membership role is `admin` or `owner`

Proposed rule for this pass:
- reuse that exact rule for:
  - frontend `Team Settings` menu visibility
  - frontend `TeamSettingsModal` edit/delete/member-management controls
  - backend `DELETE /teams/{id}`

### Create Channel

Existing policy model:
- `member` includes `CHANNEL_CREATE`
- `guest` does not

Existing native backend behavior:
- standard channel creation only checks team membership

Proposed rule for this pass:
- standard public/private channel creation requires:
  - current-team membership
  - `CHANNEL_CREATE`
- frontend create-channel affordances should follow the same rule
- direct-message creation is left unchanged unless implementation shows the same mismatch exists there too

## Implementation Outline

### Phase 1: Shared Capability Expansion

Extend `frontend/src/features/permissions/capabilities.ts` with:

1. Team capability helpers:
   - `canManageTeam(...)`
   - `useCurrentTeamManagementPermission(...)`
2. Channel creation helpers:
   - `canCreateChannel(...)`
   - optional current-team-aware helper if the UI needs team-membership context

Data sources:
- global role from `authStore.user?.role`
- current team membership from `teamStore.members` when already loaded
- team-members API fallback only if needed

### Phase 2: Frontend Affordance Guardrails

Update:
- `frontend/src/components/layout/ChannelSidebar.vue`
- `frontend/src/components/modals/TeamSettingsModal.vue`
- `frontend/src/components/modals/CreateChannelModal.vue`

Behavior:
- hide `Team Settings` unless the user can manage the current team
- if the modal is opened indirectly without permission, show a locked/access-denied state instead of editable controls
- hide `Create channel` affordances unless the user can create channels in the current team
- if the create-channel modal is opened indirectly without permission, show a locked/access-denied state

### Phase 3: Backend Enforcement

Update `backend/src/api/teams.rs`:
- add the same team-admin / `TEAM_MANAGE` check to `delete_team`

Update `backend/src/api/channels.rs`:
- add explicit `CHANNEL_CREATE` enforcement for standard public/private channel creation
- keep the existing team-membership check
- leave direct-message creation unchanged unless a concrete mismatch is found during implementation

### Phase 4: Regression Harness Expansion

Backend:
- extend `backend/tests/api_permissions.rs` with:
  - unauthorized team delete returns `403`
  - authorized team admin delete succeeds
  - guest or non-privileged user cannot create a standard channel
  - member with `CHANNEL_CREATE` can still create a standard channel

Frontend:
- extend `frontend/src/features/permissions/capabilities.test.ts`
- extend `frontend/src/features/permissions/permissionsUi.test.ts`

UI cases to pin:
- `Team Settings` hidden for unauthorized users
- `Create channel` hidden for unauthorized users
- `TeamSettingsModal` locked state
- `CreateChannelModal` locked state

## Verification Plan

Automated:
1. `cd ../../frontend && npm run test:unit`
2. `cd ../../frontend && npm run build`
3. `cd ../../backend && cargo test --test api_permissions -- --nocapture`

Manual:
1. Log in as a low-privilege user and verify the sidebar no longer shows:
   - `Team Settings`
   - `Create channel`
2. Attempt team deletion via API as a low-privilege user and verify `403`.
3. Attempt standard channel creation via API as a low-privilege user lacking `CHANNEL_CREATE` and verify `403`.
4. Log in as an allowed user and verify:
   - team settings still opens and saves
   - standard channel creation still works

Suggested manual verification commands:
1. `curl -i -X DELETE http://127.0.0.1:3000/api/v1/teams/<team-id> -H "Authorization: Bearer $TOKEN"`
2. `curl -i -X POST http://127.0.0.1:3000/api/v1/channels -H "Authorization: Bearer $TOKEN" -H "Content-Type: application/json" -d '{"team_id":"<team-id>","name":"gated-channel","display_name":"Gated Channel","channel_type":"public"}'`

## Risks and Notes

1. `member` currently has `CHANNEL_CREATE` in the policy engine, so this pass should not accidentally remove valid member channel creation if that is intentional.
2. Team-management frontend logic needs current-team membership data; we should reuse `teamStore.members` where possible instead of creating another parallel fetch path.
3. Backend enforcement and frontend hiding must land together, otherwise the app either looks broken or remains unsafe.
4. `DELETE /teams/{id}` is the highest-risk backend gap in this scope because it is destructive and currently unguarded.

## Success Criteria

1. Unauthorized users cannot delete teams.
2. Unauthorized users do not see or meaningfully use team-settings controls.
3. Standard channel creation follows explicit permission rules on both frontend and backend.
4. The regression harness covers these flows in both API and UI layers.
5. The new scope remains narrow and does not reopen unrelated permission work.
