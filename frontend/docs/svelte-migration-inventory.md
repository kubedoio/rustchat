# Svelte Migration Inventory

This inventory is for the active Vue-to-Svelte migration in `frontend`. It focuses on what can be reused, what must be rewritten, and the order that lets the team ship a compiling Svelte shell without blocking on the highest-risk chat surfaces.

## Current Frontend Shape

- Vue/Vite app with 115 `.vue` files under `src`.
- Route groups: auth, main channel, admin, settings, playbooks.
- Shared TypeScript layers already exist under `src/api`, `src/core`, `src/features`, `src/utils`, and `e2e`.
- Vue is still the production UI runtime for this milestone. Do not attempt Vue dependency removal until the Svelte app can replace the route groups and tests have parity coverage.

## Reusable Module Inventory

### API

Keep these modules framework-neutral and import them directly from Svelte code:

- `src/api/http/HttpClient.ts`, `src/api/http/errors.ts`, `src/api/http/querySerializer.ts`, `src/api/http/uploadWithProgress.ts`, and `src/api/http/index.ts`.
- Endpoint modules: `src/api/client.ts`, `src/api/admin.ts`, `src/api/adminEmail.ts`, `src/api/calls.ts`, `src/api/channels.ts`, `src/api/files.ts`, `src/api/membershipPolicies.ts`, `src/api/playbooks.ts`, `src/api/posts.ts`, `src/api/preferences.ts`, `src/api/search.ts`, `src/api/site.ts`, `src/api/teams.ts`, `src/api/threads.ts`, `src/api/users.ts`.
- Existing contract tests in `src/api/http/__tests__` should continue to protect API behavior during the framework swap.

Migration notes:

- Prefer reusing the existing client and endpoint modules before creating Svelte-specific wrappers.
- If auth token access is currently coupled to a Pinia store, introduce a small framework-neutral token bridge rather than importing Svelte stores into API modules.
- Keep upload progress and error normalization behavior unchanged; those are cross-framework contracts.

### Core

Mostly reusable after removing a small amount of Vue reactivity from websocket state:

- Entities: `src/core/entities/Auth.ts`, `Call.ts`, `Channel.ts`, `Entity.ts`, `Message.ts`, `Team.ts`, `User.ts`, and `index.ts`.
- Types and base infrastructure: `src/core/types/Result.ts`, `src/core/repositories/Repository.ts`, `src/core/errors/AppError.ts`, `src/core/services/retry.ts`, `src/core/index.ts`.
- Websocket: `src/core/websocket/registerHandlers.ts` is likely reusable; `src/core/websocket/WebSocketManager.ts` currently imports Vue (`ref`, `computed`, `markRaw`) and needs a Svelte-neutral state adapter.

Migration notes:

- Do not rewrite entity or repository types as part of first milestone.
- Treat `WebSocketManager.ts` as a boundary file. Either keep it behind a temporary Vue compatibility layer while auth routes are built, or extract the transport/event dispatch logic from Vue reactivity before migrating channel routes.

### Feature Services And Repositories

Reusable with store isolation:

- Repositories and services under `src/features/*/repositories` and `src/features/*/services` are the right business-logic seam for Svelte.
- Handlers under `src/features/*/handlers` can likely be reused once they accept Svelte store actions or framework-neutral callbacks.
- Stores under `src/features/*/stores` are Pinia/Vue-bound and should be rewritten as Svelte stores instead of adapted wholesale.

High-value reusable domains:

- Auth: `src/features/auth/repositories/authRepository.ts`, `src/features/auth/services/authService.ts`.
- Channels/messages/threads: service and repository modules under `src/features/channels`, `src/features/messages`.
- Teams/preferences/config/theme/unreads/presence/calls/admin/playbooks/activity: services and repositories should move over before UI-heavy components.

### Utils

Reusable:

- `src/utils/directMessage.ts`
- `src/utils/emoji.ts`
- `src/utils/idCompat.ts`
- `src/utils/markdown.ts`
- `src/utils/markdownFormatting.ts`
- Composer pure helpers: `src/components/composer/lib/markdownTransforms.ts`

Needs Svelte rewrite or adapter:

- Composables under `src/composables` import Vue refs/computed/watch or VueUse. Rebuild as Svelte stores/helpers case-by-case.
- Composer hooks under `src/components/composer/hooks` import Vue lifecycle/ref APIs. Preserve behavior, but rewrite for Svelte lifecycle and store semantics when migrating composer.

### Styling

Reusable:

- `src/style.css`
- `tailwind.config.js`
- `postcss.config.js`
- Existing global class names and CSS custom properties, especially theme-related tokens used by settings/theme surfaces.

Migration notes:

- Keep class names stable for first route ports where possible; this reduces snapshot and e2e churn.
- Svelte component-scoped styles are fine, but do not move global theme variables into component-local styles.
- Theme migration is high-risk because `src/stores/theme.ts` and `src/features/theme/stores/themeStore.ts` are Pinia/Vue-bound while `src/style.css` is shared.

### E2E And Tests

Reusable:

- Playwright config: `playwright.config.ts`.
- E2E specs: `e2e/auth.spec.ts`, `e2e/composer.spec.ts`, `e2e/dm-consistency.spec.ts`, `e2e/settings_parity.spec.ts`, `e2e/websocket-disconnection.spec.ts`.
- Settings snapshots can stay as parity references, but should not block the first auth milestone.

Needs update:

- Vue unit tests using `@vue/test-utils`, Pinia setup, or Vue refs need Svelte equivalents later.
- First milestone should add or update tests around Svelte auth route rendering and mocked login behavior, not attempt full unit parity.

## Vue Dependency Replacement Map

| Current dependency | Current use | Svelte replacement | Migration action |
| --- | --- | --- | --- |
| `vue` | Component runtime, refs/computed/watch, lifecycle, slots, type `Component` | `svelte`, Svelte lifecycle (`onMount`, `onDestroy`), `$state`/stores or classic `writable`/`derived` | Rewrite `.vue` files as `.svelte`; extract pure logic before replacing reactive primitives. |
| `pinia` | App-wide stores in `src/stores` and `src/features/*/stores` | Svelte stores (`writable`, `readable`, `derived`) or Svelte 5 rune-backed modules if project standardizes on runes | Recreate stores by domain. Preserve public actions and data shape where possible to reduce UI churn. |
| `vue-router` | Route declarations, navigation guards, `useRouter`, `useRoute`, `RouterView` | `svelte-spa-router`, `@mateothegreat/svelte5-router`, or a small internal route store depending on the chosen Svelte setup | First milestone only needs auth routes. Recreate auth guard and OAuth redirect handling without importing Pinia. |
| `@vueuse/core` | `useStorage`, `useMediaQuery`, browser helpers | `svelte-local-storage-store`, custom `localStorage` store, `matchMedia` readable store, or `@svelte-put/*` utilities | Replace per use. `useStorage` in auth/team/channel/composer can become persisted Svelte stores; `useMediaQuery` can become a tiny readable store. |
| `lucide-vue-next` | Icon components throughout UI | `lucide-svelte` | Swap imports during component ports. Icon names generally match, but check prop names and class forwarding. |
| `@tiptap/vue-3` | Tiptap editor binding in `ThreadComposer.vue` and related composer work | `@tiptap/core` plus Svelte integration via manual editor lifecycle, or a maintained Svelte Tiptap wrapper if adopted | Defer until composer migration. Do not block auth/app-shell milestone on Tiptap. |

Dependency policy:

- Keep Vue dependencies installed until all route groups compile and e2e parity is established.
- Do not remove `vue`, `pinia`, `vue-router`, `@vueuse/core`, `lucide-vue-next`, `@tiptap/vue-3`, `@vitejs/plugin-vue`, `@vue/test-utils`, `@vue/tsconfig`, or `vue-tsc` during the first milestone.

## Route And Component Migration Order

### Milestone 1: Svelte Shell And Auth Routes

Target routes:

- `/login`
- `/register`
- `/forgot-password`
- `/reset-password`
- `/set-password`

Components to port or create:

- `src/layouts/AuthLayout.vue` -> Svelte auth layout.
- `src/views/auth/LoginView.vue` -> Svelte login route.
- `src/views/auth/RegisterView.vue` -> Svelte register route.
- `src/views/auth/ForgotPasswordView.vue` -> Svelte forgot-password route.
- `src/views/auth/ResetPasswordView.vue` -> Svelte reset/set-password route.
- `src/components/auth/TurnstileWidget.vue` -> Svelte only if auth forms currently require it in the mocked flow; otherwise stub it behind a feature flag for milestone 1.

Why first:

- Auth has small route scope, limited websocket dependency, and validates Svelte bootstrapping, routing, styles, API mocking, and store persistence.

### Milestone 2: Shared App Shell Without Full Chat Behavior

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

Why second:

- Most authenticated routes depend on layout, theme, identity, toasts, and navigation. Build this once before porting channel/admin/playbook content.

### Milestone 3: Main Channel Read Path

Target route and components:

- `src/views/main/ChannelView.vue`
- `src/components/channel/ChannelHeader.vue`
- `src/components/channel/MessageList.vue`
- `src/components/channel/MessageItem.vue`
- `src/components/channel/TypingIndicator.vue`
- Read-only side panels: `ChannelInfoPanel.vue`, `ChannelMembersPanel.vue`, `PinnedMessagesPanel.vue`, `SavedMessagesPanel.vue`, `SearchPanel.vue`
- `src/components/navigation/BreadcrumbBar.vue`, `QuickSwitcherModal.vue`, `QuickSwitcherItem.vue`

Why third:

- This is the core product path. Starting with read-only behavior reduces risk before composer, files, calls, and threading.

### Milestone 4: Composer, Threads, Uploads, Emoji, And Search Modals

Target components:

- `src/components/composer/MattermostComposer.vue`
- `MessageComposer.vue`, `ThreadComposer.vue`, `FormattingToolbar.vue`, `MarkdownPreview.vue`, `MentionAutocomplete.vue`
- Autocomplete components under `src/components/composer/autocomplete`
- `src/components/atomic/EmojiPicker.vue`, `FileUploader.vue`, `FilePreview.vue`, `ImageGallery.vue`
- Thread surfaces under `src/components/thread`
- Modal surfaces: `SearchModal.vue`, `CreateChannelModal.vue`, `EditChannelModal.vue`, `DirectMessageModal.vue`, `AddChannelMembersModal.vue`, `ChannelSettingsModal.vue`

Why fourth:

- This combines high interaction density, keyboard handling, file uploads, markdown transforms, autocomplete, websocket updates, and Tiptap.

### Milestone 5: Settings And Preferences

Target routes and components:

- `src/views/settings/ProfileView.vue`
- `src/components/settings/SettingsModal.vue`
- Tabs under `src/components/settings/*`
- `src/components/modals/EditProfileModal.vue`, `SetStatusModal.vue`, `UserProfileModal.vue`

Why fifth:

- Settings has broad state coupling and snapshot parity coverage. It should move after the shared app shell and theme store are stable.

### Milestone 6: Admin Console

Target route group:

- `src/views/admin/AdminConsole.vue`
- Admin views under `src/views/admin`
- Admin components under `src/components/admin`
- User/team/policy modals that admin routes depend on.

Why sixth:

- Admin has many forms and service domains but lower daily-user criticality than channel messaging. Port after store and form patterns are proven.

### Milestone 7: Playbooks And Calls

Target components:

- `src/views/main/PlaybooksView.vue`
- `src/components/playbooks/PlaybookList.vue`, `PlaybookEditor.vue`, `PlaybookRun.vue`
- Calls components: `ActiveCall.vue`, `IncomingCallModal.vue`, `JitsiMeet.vue`, `VideoCallModal.vue`
- `src/views/admin/plugins/CallsPluginSettings.vue`

Why last:

- Playbooks are self-contained but include nested forms and route params. Calls include external Jitsi lifecycle behavior and are best migrated after websocket/store patterns are stable.

## High-Risk Components And Boundaries

- `src/views/main/ChannelView.vue`: central orchestration point for teams, channels, messages, sidebars, websocket state, and persisted UI state.
- `src/components/composer/MattermostComposer.vue` and `src/components/composer/MessageComposer.vue`: dense keyboard, draft, upload, autocomplete, markdown, and submit behavior.
- `src/components/composer/ThreadComposer.vue`: uses `@tiptap/vue-3`; requires a different Svelte editor lifecycle.
- `src/core/websocket/WebSocketManager.ts`: Vue reactivity inside core infrastructure; needs extraction before authenticated realtime parity.
- `src/stores/auth.ts` and `src/features/auth/stores/authStore.ts`: token persistence, current user hydration, OAuth redirect behavior, and route guard coupling.
- `src/stores/theme.ts` and `src/features/theme/stores/themeStore.ts`: theme variables affect global rendering and settings snapshot parity.
- `src/components/layout/AppShell.vue` and `GlobalHeader.vue`: broad dependencies on navigation, auth, notifications, activity, command palette, and responsive state.
- `src/components/settings/SettingsModal.vue` and settings tabs: snapshot-covered, preference-heavy, and likely to reveal CSS/theme drift.
- `src/components/calls/JitsiMeet.vue`: external SDK lifecycle and cleanup risk.
- Admin forms such as `UsersManagement.vue`, `TeamsManagement.vue`, `MembershipPolicies.vue`, and `EmailSettings.vue`: data mutations, validation, modals, and role/permission assumptions.

## First Milestone Acceptance Criteria

The first milestone is intentionally narrow. It proves Svelte can run in this repo while Vue remains installed.

Required:

- Svelte app compiles through the chosen Vite/Svelte entrypoint.
- Auth routes render in the browser: `/login`, `/register`, `/forgot-password`, `/reset-password`, and `/set-password`.
- Login form can authenticate against a mocked API response and persist enough auth state to represent a logged-in user.
- Failed mocked login shows an error state without crashing or navigating incorrectly.
- Auth layout uses existing global styling/theme tokens where practical.
- Existing Vue dependency removal is not attempted.
- Existing Vue source files, package metadata, and Vite config are not modified as part of this inventory task.

Suggested checks:

- Unit or component test for successful mocked login.
- Unit or component test for failed mocked login.
- Browser smoke check for each auth route.
- Verify the app can still be developed with Vue dependencies present during the transition.

Explicit non-goals for milestone 1:

- Full authenticated app shell parity.
- Websocket migration.
- Channel/message/composer migration.
- Settings/admin/playbooks migration.
- Removing Vue, Pinia, Vue Router, VueUse, Lucide Vue, Tiptap Vue, Vue test tooling, or Vue Vite plugin.

## Working Rules For Concurrent Migration

- Keep source-code edits scoped to the active migration task owner. This inventory should be the only file changed for the documentation task.
- Do not rewrite reusable API/core modules unless a specific Svelte route needs a framework-neutral seam.
- Avoid bulk mechanical `.vue` to `.svelte` ports for high-risk surfaces. Move route groups in the order above and keep each milestone independently testable.
- Preserve endpoint contracts and e2e behavior before changing implementation details.
- When a file imports Vue only for reactivity in otherwise pure logic, prefer extracting pure logic over adapting Vue APIs in Svelte.
