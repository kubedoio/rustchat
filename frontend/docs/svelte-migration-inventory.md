> ✅ **MIGRATION COMPLETE** — This document is preserved for historical reference.

# Svelte Migration Inventory

This inventory documents the completed Vue-to-Svelte migration in `frontend`. It focuses on what was reused, what was rewritten, and the order that let the team ship a compiling Svelte shell without blocking on the highest-risk chat surfaces.

## Current Frontend Shape

- Svelte/Vite app with **0** `.vue` files under `src`.
- Route groups: auth, main channel, admin, settings, playbooks — all running on Svelte.
- Shared TypeScript layers under `src/api`, `src/core`, `src/features`, `src/utils`, and `e2e` remain framework-neutral and are imported directly from Svelte code.
- Vue has been fully removed from the production UI runtime.

## Reusable Module Inventory

### API

These modules were kept framework-neutral and imported directly from Svelte code:

- `src/api/http/HttpClient.ts`, `src/api/http/errors.ts`, `src/api/http/querySerializer.ts`, `src/api/http/uploadWithProgress.ts`, and `src/api/http/index.ts`.
- Endpoint modules: `src/api/client.ts`, `src/api/admin.ts`, `src/api/adminEmail.ts`, `src/api/calls.ts`, `src/api/channels.ts`, `src/api/files.ts`, `src/api/membershipPolicies.ts`, `src/api/playbooks.ts`, `src/api/posts.ts`, `src/api/preferences.ts`, `src/api/search.ts`, `src/api/site.ts`, `src/api/teams.ts`, `src/api/threads.ts`, `src/api/users.ts`.
- Existing contract tests in `src/api/http/__tests__` continue to protect API behavior.

Migration notes (historical):

- Reused the existing client and endpoint modules before creating Svelte-specific wrappers.
- Introduced a small framework-neutral token bridge for auth token access.
- Kept upload progress and error normalization behavior unchanged; those are cross-framework contracts.

### Core

Mostly reused after removing Vue reactivity from websocket state:

- Entities: `src/core/entities/Auth.ts`, `Call.ts`, `Channel.ts`, `Entity.ts`, `Message.ts`, `Team.ts`, `User.ts`, and `index.ts`.
- Types and base infrastructure: `src/core/types/Result.ts`, `src/core/repositories/Repository.ts`, `src/core/errors/AppError.ts`, `src/core/services/retry.ts`, `src/core/index.ts`.
- Websocket: `src/core/websocket/registerHandlers.ts` was reused; `src/core/websocket/WebSocketManager.ts` had Vue (`ref`, `computed`, `markRaw`) reactivity removed and replaced with Svelte-neutral state.

Migration notes (historical):

- Entity and repository types were not rewritten as part of the first milestone.
- `WebSocketManager.ts` was treated as a boundary file. Vue reactivity was extracted and replaced with Svelte-compatible state.

### Feature Services And Repositories

Reused with store isolation:

- Repositories and services under `src/features/*/repositories` and `src/features/*/services` were the right business-logic seam for Svelte.
- Handlers under `src/features/*/handlers` were reused once they accepted Svelte store actions or framework-neutral callbacks.
- Stores under `src/features/*/stores` were rewritten as Svelte stores instead of adapted wholesale.

High-value reusable domains:

- Auth: `src/features/auth/repositories/authRepository.ts`, `src/features/auth/services/authService.ts`.
- Channels/messages/threads: service and repository modules under `src/features/channels`, `src/features/messages`.
- Teams/preferences/config/theme/unreads/presence/calls/admin/playbooks/activity: services and repositories moved over before UI-heavy components.

### Utils

Reused:

- `src/utils/directMessage.ts`
- `src/utils/emoji.ts`
- `src/utils/idCompat.ts`
- `src/utils/markdown.ts`
- `src/utils/markdownFormatting.ts`
- Composer pure helpers: `src/components/composer/lib/markdownTransforms.ts`

Rewritten for Svelte:

- Composables under `src/composables` were rebuilt as Svelte stores/helpers case-by-case.
- Composer hooks under `src/components/composer/hooks` were rewritten for Svelte lifecycle and store semantics.

### Styling

Reused:

- `src/style.css`
- `tailwind.config.js`
- `postcss.config.js`
- Existing global class names and CSS custom properties, especially theme-related tokens used by settings/theme surfaces.

Migration notes (historical):

- Class names were kept stable for first route ports where possible; this reduced snapshot and e2e churn.
- Svelte component-scoped styles are used where appropriate; global theme variables remain in global styles.

### E2E And Tests

Reused:

- Playwright config: `playwright.config.ts`.
- E2E specs: `e2e/auth.spec.ts`, `e2e/composer.spec.ts`, `e2e/dm-consistency.spec.ts`, `e2e/settings_parity.spec.ts`, `e2e/websocket-disconnection.spec.ts`.

Updated:

- Vue unit tests using `@vue/test-utils`, Pinia setup, or Vue refs were replaced with Svelte equivalents.

## Vue Dependency Replacement Map

All replacements listed below are **complete**.

| Dependency | Previous use | Svelte replacement | Status |
| --- | --- | --- | --- |
| `vue` | Component runtime, refs/computed/watch, lifecycle, slots, type `Component` | `svelte`, Svelte lifecycle (`onMount`, `onDestroy`), `$state`/stores | ✅ Complete |
| `pinia` | App-wide stores in `src/stores` and `src/features/*/stores` | Svelte stores (`writable`, `readable`, `derived`) and Svelte 5 rune-backed modules | ✅ Complete |
| `vue-router` | Route declarations, navigation guards, `useRouter`, `useRoute`, `RouterView` | `svelte-spa-router` / internal route store | ✅ Complete |
| `@vueuse/core` | `useStorage`, `useMediaQuery`, browser helpers | `svelte-local-storage-store`, custom `localStorage` store, `matchMedia` readable store | ✅ Complete |
| `lucide-vue-next` | Icon components throughout UI | `lucide-svelte` | ✅ Complete |
| `@tiptap/vue-3` | Tiptap editor binding in composer | `@tiptap/core` plus Svelte integration via manual editor lifecycle | ✅ Complete |

Dependency policy (historical):

- ✅ Vue dependencies (`vue`, `pinia`, `vue-router`, `@vueuse/core`, `lucide-vue-next`, `@tiptap/vue-3`, `@vitejs/plugin-vue`, `@vue/test-utils`, `@vue/tsconfig`, `vue-tsc`) have been removed.

## Route And Component Migration Order

All milestones below are **complete**.

### Milestone 1: Svelte Shell And Auth Routes ✅

Target routes:

- `/login`
- `/register`
- `/forgot-password`
- `/reset-password`
- `/set-password`

Components ported or created:

- `src/layouts/AuthLayout.vue` → Svelte auth layout.
- `src/views/auth/LoginView.vue` → Svelte login route.
- `src/views/auth/RegisterView.vue` → Svelte register route.
- `src/views/auth/ForgotPasswordView.vue` → Svelte forgot-password route.
- `src/views/auth/ResetPasswordView.vue` → Svelte reset/set-password route.
- `src/components/auth/TurnstileWidget.vue` → Svelte equivalent.

Why first:

- Auth had small route scope, limited websocket dependency, and validated Svelte bootstrapping, routing, styles, API mocking, and store persistence.

### Milestone 2: Shared App Shell Without Full Chat Behavior ✅

Target components:

- `src/App.vue`
- `src/components/layout/AppShell.vue`
- `src/components/layout/GlobalHeader.vue`
- `src/components/layout/TeamRail.vue`
- `src/components/layout/ChannelSidebar.vue`
- `src/components/layout/RightSidebar.vue`
- `src/components/ui/ToastManager.vue`
- `src/components/ui/ConnectionStatusBar.vue`
- `src/components/ui/ConnectionLostModal.vue`
- `src/components/ui/UserAvatar.vue`, `RcAvatar.vue`, `ThemeToggle.vue`

### Milestone 3: Main Channel Read Path ✅

Target route and components:

- `src/views/main/ChannelView.vue`
- `src/components/channel/ChannelHeader.vue`
- `src/components/channel/MessageList.vue`
- `src/components/channel/MessageItem.vue`
- `src/components/channel/TypingIndicator.vue`
- Read-only side panels: `ChannelInfoPanel.vue`, `ChannelMembersPanel.vue`, `PinnedMessagesPanel.vue`, `SavedMessagesPanel.vue`, `SearchPanel.vue`
- `src/components/navigation/BreadcrumbBar.vue`, `QuickSwitcherModal.vue`, `QuickSwitcherItem.vue`

### Milestone 4: Composer, Threads, Uploads, Emoji, And Search Modals ✅

Target components:

- `src/components/composer/MattermostComposer.vue`
- `MessageComposer.vue`, `ThreadComposer.vue`, `FormattingToolbar.vue`, `MarkdownPreview.vue`, `MentionAutocomplete.vue`
- Autocomplete components under `src/components/composer/autocomplete`
- `src/components/atomic/EmojiPicker.vue`, `FileUploader.vue`, `FilePreview.vue`, `ImageGallery.vue`
- Thread surfaces under `src/components/thread`
- Modal surfaces: `SearchModal.vue`, `CreateChannelModal.vue`, `EditChannelModal.vue`, `DirectMessageModal.vue`, `AddChannelMembersModal.vue`, `ChannelSettingsModal.vue`

### Milestone 5: Settings And Preferences ✅

Target routes and components:

- `src/views/settings/ProfileView.vue`
- `src/components/settings/SettingsModal.vue`
- Tabs under `src/components/settings/*`
- `src/components/modals/EditProfileModal.vue`, `SetStatusModal.vue`, `UserProfileModal.vue`

### Milestone 6: Admin Console ✅

Target route group:

- `src/views/admin/AdminConsole.vue`
- Admin views under `src/views/admin`
- Admin components under `src/components/admin`
- User/team/policy modals that admin routes depend on.

### Milestone 7: Playbooks And Calls ✅

Target components:

- `src/views/main/PlaybooksView.vue`
- `src/components/playbooks/PlaybookList.vue`, `PlaybookEditor.vue`, `PlaybookRun.vue`
- Calls components: `ActiveCall.vue`, `IncomingCallModal.vue`, `JitsiMeet.vue`, `VideoCallModal.vue`
- `src/views/admin/plugins/CallsPluginSettings.vue`

## High-Risk Components And Boundaries

All high-risk components below were successfully migrated:

- `src/views/main/ChannelView.vue`: central orchestration point for teams, channels, messages, sidebars, websocket state, and persisted UI state.
- `src/components/composer/MattermostComposer.vue` and `src/components/composer/MessageComposer.vue`: dense keyboard, draft, upload, autocomplete, markdown, and submit behavior.
- `src/components/composer/ThreadComposer.vue`: replaced `@tiptap/vue-3` with Svelte-compatible editor lifecycle.
- `src/core/websocket/WebSocketManager.ts`: Vue reactivity removed from core infrastructure; replaced with Svelte-neutral state.
- `src/stores/auth.ts` and `src/features/auth/stores/authStore.ts`: token persistence, current user hydration, OAuth redirect behavior, and route guard coupling.
- `src/stores/theme.ts` and `src/features/theme/stores/themeStore.ts`: theme variables migrated without affecting global rendering.
- `src/components/layout/AppShell.vue` and `GlobalHeader.vue`: broad dependencies on navigation, auth, notifications, activity, command palette, and responsive state.
- `src/components/settings/SettingsModal.vue` and settings tabs: snapshot-covered, preference-heavy surfaces.
- `src/components/calls/JitsiMeet.vue`: replaced with Svelte `ActiveCall`.
- Admin forms such as `UsersManagement.vue`, `TeamsManagement.vue`, `MembershipPolicies.vue`, and `EmailSettings.vue`: data mutations, validation, modals, and role/permission assumptions.

## First Milestone Acceptance Criteria (Historical)

The first milestone was intentionally narrow. It proved Svelte could run in this repo while Vue remained installed.

Required (all ✅):

- Svelte app compiles through the chosen Vite/Svelte entrypoint.
- Auth routes render in the browser: `/login`, `/register`, `/forgot-password`, `/reset-password`, and `/set-password`.
- Login form can authenticate against a mocked API response and persist enough auth state to represent a logged-in user.
- Failed mocked login shows an error state without crashing or navigating incorrectly.
- Auth layout uses existing global styling/theme tokens where practical.

Suggested checks (all ✅):

- Unit or component test for successful mocked login.
- Unit or component test for failed mocked login.
- Browser smoke check for each auth route.

## Working Rules For Concurrent Migration (Historical)

- Source-code edits were scoped to the active migration task owner.
- Reusable API/core modules were not rewritten unless a specific Svelte route needed a framework-neutral seam.
- Bulk mechanical `.vue` to `.svelte` ports were avoided for high-risk surfaces.
- Route groups were moved in the order above and each milestone was independently testable.
- Endpoint contracts and e2e behavior were preserved before changing implementation details.
- When a file imported Vue only for reactivity in otherwise pure logic, pure logic was extracted over adapting Vue APIs in Svelte.
