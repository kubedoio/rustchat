# Svelte ↔ Vue UI Gap Analysis Report

**Date:** 2026-05-06  
**Branch:** `codex/svelte-build-fix`  
**Scope:** Chat UI (main messaging experience)  
**Method:** Source-code comparison of Vue worktree (`.claude/worktrees/agent-a9e092bc`) vs. current Svelte implementation (`frontend/src/svelte`)

---

## Executive Summary

The Svelte migration has achieved functional parity for core messaging (send/receive messages, channels, DMs, reactions, threads, calls), but the UI is visually and interactionally different from the Vue implementation. The biggest gaps are in **layout architecture** (missing Global Header, Team Rail, proper RHS container), **sidebar richness** (no categories, no context menus, no hover actions), **message rendering** (no markdown, no mention highlighting), and **styling consistency** (raw Tailwind colors mixed with semantic tokens).

| Severity | Count | Description |
|----------|-------|-------------|
| 🔴 High | 14 | Missing major features or broken UX |
| 🟡 Medium | 18 | Degraded experience or visual inconsistency |
| 🟢 Low | 12 | Polish, animations, or minor interactions |

---

## 1. Layout & Shell Architecture 🔴

### 1.1 Global Header — **MISSING ENTIRELY**

**Vue:** `components/layout/GlobalHeader.vue`
- Persistent top bar across all views
- Logo / site name
- Search bar with ⌘K shortcut hint
- Help button
- Notifications bell with unread dot
- Activity feed button with unread count badge
- User menu trigger (avatar + username + presence dot)
- Rich user dropdown: custom status, presence options (Online/Away/DND/Offline), DND duration submenu, Profile, Admin Console, Log out
- Breakpoint-responsive: search hidden on mobile, username hidden on small screens

**Svelte:** No equivalent component. `ChatView.svelte` jumps straight from `<html>` to `ChannelHeader` inside the chat section. There is no persistent app-level header.

**Impact:** Users lose quick access to search, notifications, activity feed, presence setting, and user profile from any view. The app feels less like a cohesive application and more like a bare chat widget.

---

### 1.2 Team Rail — **MISSING ENTIRELY**

**Vue:** `components/layout/TeamRail.vue`
- Discord-style vertical rail on the far left
- Team buttons with initials badges
- Active indicator (3px brand-colored bar on left)
- Unread notification dot (red, top-right) on non-active teams
- "Add Team" button at bottom (Plus icon in dashed border)
- `CreateTeamModal`

**Svelte:** No equivalent. Teams are listed inside `ChatSidebar.svelte` as plain text headers.

**Impact:** Multi-team navigation is buried in the sidebar instead of being a first-class, always-visible affordance.

---

### 1.3 Right Sidebar (RHS) Container — **DEGRADED**

**Vue:** `components/layout/RightSidebar.vue`
- Dedicated container component with fixed width (`w-[400px]`)
- `shadow-2xl` for depth
- Transform-based open/close animation (`translate-x-0` / `translate-x-full`)
- Consistent close button and header pattern
- Views: Thread, Search, Channel Info, Members, Pinned, Saved

**Svelte:** Panels (`ThreadPanel`, `ChannelInfoPanel`, `PinnedMessagesPanel`, `SavedMessagesPanel`) are rendered inline in `ChatView.svelte` as conditional blocks. There is no unified RHS container with consistent animations or styling.
- `ThreadPanel` has its own mobile overlay + side panel logic
- `ChannelInfoPanel` has `animate-slide-in-right`
- `PinnedMessagesPanel` and `SavedMessagesPanel` use completely different styling (`bg-white`, `border-gray-200`)

**Impact:** Inconsistent open/close behavior, no unified animation, and visual jarring when switching between RHS panels.

---

### 1.4 Mobile Drawer Architecture — **MISSING**

**Vue:** `AppShell.vue`
- Mobile sidebar overlay: dark backdrop with blur (`bg-black/50 backdrop-blur-sm`)
- Mobile sidebar drawer: slides in from left with `Transition` (opacity + translate)
- Auto-closes when switching to desktop or changing channels
- Mobile RHS overlay with backdrop blur

**Svelte:** No mobile drawer architecture. `ChatSidebar` is always rendered inline. `ThreadPanel` has its own mobile overlay but no unified system.

**Impact:** On mobile, the sidebar is either always visible (wasting space) or inaccessible. No gesture-based or overlay-based mobile navigation.

---

## 2. Left Sidebar (ChatSidebar) 🔴🟡

### 2.1 Team Header & Dropdown — **MISSING**

**Vue:** Sidebar header is a clickable dropdown with:
- Current team name + chevron
- Menu items: System Console, Browse Teams, Team Settings, Leave Team
- Click-outside backdrop

**Svelte:** Static text label with uppercase tracking. No dropdown, no team actions.

---

### 2.2 Channel Categories — **MISSING**

**Vue:** Channels organized into collapsible categories:
- **Favorites** (starred channels)
- **Channels** (public)
- **Private Channels**
- **Direct Messages**
- Each category has a header with collapse chevron and "+" add button

**Svelte:** Two flat lists: "regular channels" and "direct messages". No categories, no favorites, no collapse/expand.

**Impact:** Users with many channels lose organization. No way to favorite or categorize channels.

---

### 2.3 Hover Actions on Channels — **MISSING**

**Vue:** On channel hover:
- "Mark as read" check button appears (`opacity-0 group-hover:opacity-100`)
- "More" button (vertical dots) appears
- Right-click opens context menu (`@contextmenu.prevent`) with: Mark as read, Mute channel, Move to category, Leave channel, Copy link

**Svelte:** No hover actions. No context menu. Channels are static clickable rows.

---

### 2.4 Sidebar Footer — **MISSING**

**Vue:** Footer actions at bottom of sidebar:
- Mark all as read
- Browse channels
- Create channel

**Svelte:** No footer actions. The sidebar ends with the `UserMenu`.

---

### 2.5 DM Name Resolution & Presence — **DEGRADED**

**Vue:** DM names resolved from team members; presence status shown as colored dot with proper status text.

**Svelte:** DM rows show `display_name` from channel data. Presence dots use hardcoded Tailwind colors (`bg-emerald-400`, `bg-amber-400`, etc.) instead of semantic tokens.

---

### 2.6 Sidebar Styling — **INCONSISTENT**

**Vue:** Uses semantic design tokens throughout (`bg-bg-surface-2`, `text-text-1`, `border-border-1`)

**Svelte:** Hardcoded dark theme (`bg-slate-950 text-white border-r border-gray-200`). This means:
- The sidebar will always be dark even if the user selects a light theme
- No theme switching support for the sidebar

---

### 2.7 Default/Fallback Data — **BUG RISK**

**Svelte:** `ChatSidebar.svelte` has fallback default data baked into props:
```svelte
export let teams: ChatTeam[] = [{ id: 'rustchat', name: 'rustchat', ... }]
export let members: ChatMember[] = [{ id: 'adam', ... }, { id: 'member', ... }]
```

This means if props are not passed correctly, the sidebar shows fake demo data instead of empty states.

---

## 3. Message List 🔴🟡

### 3.1 Date Separators — **MISSING**

**Vue:** Sticky date separators interleaved in the message timeline. Pill-shaped labels with border, sticky top positioning.

**Svelte:** No date separators. Messages render as a flat list.

**Impact:** Users cannot easily identify when conversations span multiple days.

---

### 3.2 "New Messages" Divider — **MISSING**

**Vue:** Red-themed divider at the first unread message position (`bg-danger/30` lines, red text).

**Svelte:** No unread divider. No concept of "first unread" in the message list.

---

### 3.3 "New Messages" Floating Button — **MISSING**

**Vue:** Fixed floating button at bottom-center when user scrolls up and new messages arrive. Click smooth-scrolls to bottom.

**Svelte:** No floating button. Auto-scroll only happens if user is already near bottom.

**Impact:** If user scrolls up to read history and new messages arrive, they have no indicator or quick way to jump back down.

---

### 3.4 Infinite Scroll (Older Messages) — **MISSING**

**Vue:** Reverse infinite scroll — when `scrollTop < 100`, loads older messages via API.

**Svelte:** No infinite scroll. Only initial fetch. Users cannot load message history beyond the first page.

**Impact:** Channels with long history are truncated to the most recent ~50 messages with no way to load more.

---

### 3.5 Message Highlight on Jump — **MISSING**

**Vue:** When jumping to a message (from search/pinned/saved), the message gets a temporary highlight (`ring-1 ring-brand/20 bg-brand/5`) for 2 seconds with smooth scroll.

**Svelte:** `scrollToMessage()` smooth-scrolls to the message but provides no visual highlight.

---

### 3.6 Message List Styling — **INCONSISTENT**

**Vue:** Uses semantic tokens (`max-w-[var(--msg-max-width)] mx-auto`, custom scrollbar CSS)

**Svelte:** Hardcoded `bg-gray-50 p-4`. Empty state uses `border-gray-300 bg-white text-gray-500`.

---

## 4. Message Item (MessageItem) 🔴🟡

### 4.1 Markdown Rendering — **MISSING**

**Vue:** Messages rendered as markdown HTML (`renderMarkdown()`). Supports bold, italic, links, code blocks, lists, mentions.

**Svelte:** Messages rendered as plain text:
```svelte
<div class="... whitespace-pre-wrap">{body}</div>
```

**Impact:** Users see raw markdown syntax (`**bold**`, `_italic_`, `[link](url)`) instead of formatted text. This is a major regression.

---

### 4.2 Mention Highlighting — **MISSING**

**Vue:** Messages mentioning the current user get highlighted:
- Message row: `bg-brand/5`
- Mention text: `-mx-2 px-2 py-1 rounded`

**Svelte:** No mention detection or highlighting.

---

### 4.3 Video Call / Calls Plugin Messages — **MISSING**

**Vue:** Special rendering for:
- Video call messages: call card with Video icon, Join Call button
- Calls plugin messages: phone icon, call start/end text, duration

**Svelte:** Only handles `system_join_leave`, `system_purpose`, `system_header`. No video call or calls protocol rendering.

---

### 4.4 Image Gallery / Lightbox — **MISSING**

**Vue:** Clicking an image file opens `ImageGallery` lightbox teleported to body.

**Svelte:** Images render as small thumbnails (`w-20 h-20`) with no click-to-expand behavior.

**Impact:** Users cannot view attached images at full size.

---

### 4.5 More Menu Items — **INCOMPLETE**

**Vue:** More menu has: Edit, Save/Unsave, Pin/Unpin, Mark as unread, Delete

**Svelte:** More menu has only: Edit, Delete

**Impact:** Users cannot save messages for later, pin messages, or mark messages as unread from the message actions.

---

### 4.6 Pin / Save Badges — **PRESENT BUT MENU ACTIONS MISSING**

**Vue & Svelte:** Both show Pin and Save badges on messages.

**But:** Svelte has no way to toggle pin/save from the message. The badges are display-only.

---

### 4.7 Reactions API Integration — **BROKEN**

**Vue:** Reactions toggle calls the API and updates global store.

**Svelte:** `toggleReaction()` only mutates the local `reactions` array:
```typescript
reactions = reactions.map((r) => { ... }).filter((r) => r.count > 0)
```

No API call. Refreshing the page loses reaction changes.

---

### 4.8 Edit Time Limit — **MISSING**

**Vue:** Enforces `post_edit_time_limit_seconds` from site config.

**Svelte:** No time limit check. Users can edit any message indefinitely.

---

### 4.9 File Preview Size — **DEGRADED**

**Vue:** `FilePreview` grid with gallery support, proper sizing.

**Svelte:** Images squashed to `w-20 h-20`. Non-images shown as small pills.

---

## 5. Message Composer 🔴🟡

### 5.1 Drag & Drop File Upload — **MISSING**

**Vue:** Entire composer wrapped in `FileUploader` component. Dragging files anywhere over the composer shows drop zone.

**Svelte:** No drag-and-drop wrapper. Files can only be attached via the file input button.

---

### 5.2 Markdown Preview — **MISSING**

**Vue:** `MarkdownPreview` panel toggleable with `Ctrl+Alt+T` or toolbar button.

**Svelte:** No preview mode.

---

### 5.3 Autocomplete Completeness — **DEGRADED**

**Vue:** Four autocomplete systems:
- `@` mentions (with avatar, username, display name)
- `:` emojis (full emoji shortcode database)
- `~` channels (channel list)
- `^k` commands (call start/join/leave/end)

**Svelte:** Two partial systems:
- `@` mentions (basic username filter)
- `:` emojis (only 2 hardcoded suggestions: `:smile:`, `:smiley:`)

**Impact:** Users cannot autocomplete channel names or use command palette from the composer.

---

### 5.4 Formatting Shortcuts — **MISSING**

**Vue:** Rich keyboard shortcuts:
- `Ctrl+B` / `Ctrl+I` / `Ctrl+X`
- `Ctrl+Shift+7` / `Ctrl+Shift+8` for ordered/unordered lists
- Global `Ctrl+Alt+T` for markdown preview toggle

**Svelte:** Only `Ctrl+B` for bold is implemented.

---

### 5.5 Call Integration Button — **MISSING**

**Vue:** Start audio call button in composer toolbar. Context-aware: shows "Show active call" if already in call.

**Svelte:** No call button in composer.

---

### 5.6 Composer Styling — **INCONSISTENT**

**Vue:** Semantic tokens:
```
border-t border-border-1 bg-bg-surface-1 p-3
rounded-r-2 border border-border-1
focus-within:border-brand/60 focus-within:ring-2 focus-within:ring-brand/10
```

**Svelte:** Raw colors:
```
border-t border-gray-200 bg-white p-4
rounded-xl border border-gray-300 bg-white shadow-sm
focus-within:border-indigo-500 focus-within:ring-2 focus-within:ring-indigo-100
```

The Svelte composer will always look like a light-theme component regardless of the active theme.

---

### 5.7 Send Button Styling — **INCONSISTENT**

**Vue:** Uses semantic `bg-brand text-brand-foreground`

**Svelte:** Uses `bg-indigo-600 text-white hover:bg-indigo-700`

---

## 6. Channel Header 🟡

### 6.1 Mobile Sidebar Toggle — **MISSING**

**Vue:** `PanelLeft` icon button to open mobile sidebar drawer.

**Svelte:** No mobile toggle. `ChatSidebar` is always rendered.

---

### 6.2 Topic Text — **MISSING**

**Vue:** Channel topic displayed below channel name (truncated on mobile).

**Svelte:** No topic display.

---

### 6.3 Members Button — **MISSING**

**Vue:** Members button toggles `ChannelMembersPanel` in RHS.

**Svelte:** Members are shown in `ChannelInfoPanel` only. No dedicated members button in header.

---

### 6.4 Call Buttons — **MISSING**

**Vue:** Context-aware call buttons:
- Join active call (pulsing animation)
- Show call (green)
- Start audio call

**Svelte:** "Start Call" is only in the "More" dropdown menu. No join/show call buttons.

---

### 6.5 Header Action Container — **DIFFERENT**

**Vue:** Action buttons are individual icons with `hover:bg-bg-surface-2`

**Svelte:** Actions wrapped in a `rounded-r-3 border border-border-1 bg-bg-surface-2/70` container. This is a different visual pattern.

---

## 7. Thread Panel 🟡

**Vue & Svelte:** Both have thread panels with similar functionality.

**Differences:**
- Vue `ThreadPanel` uses different styling tokens (`bg-surface`, `bg-surface-dim`) vs Svelte's semantic tokens
- Vue parent message found by scanning all channels (O(n))
- Svelte uses `chatStore.fetchThreadReplies()`
- Both support: reply composer, gallery, empty state

**Verdict:** Functionally equivalent. Minor styling differences.

---

## 8. Calls (ActiveCall) 🟡

**Vue & Svelte:** Both have rich call UIs with similar features:
- Compact / expanded modes
- Mute/unmute, raise hand, screen share
- Participants sidebar
- Host moderation
- Duration timer

**Differences:**
- Vue uses standalone dark slate theme (`bg-slate-900`)
- Svelte uses semantic tokens (`bg-bg-surface-1`, `border-border-1`)
- Svelte uses Svelte 5 runes; Vue uses Pinia stores

**Verdict:** Near parity. Svelte version may actually look more consistent with the rest of the app.

---

## 9. Notifications Dropdown 🔴

**Vue:** `components/layout/NotificationsDropdown.vue`
- Shows **unread channel list**
- Each row: Hash icon, channel name, mention badge (red), unread count badge (gray)
- Header: "Unread Activity" + "Mark all as read"
- Empty state: "All caught up!"

**Svelte:** `components/ui/NotificationsDropdown.svelte`
- Shows **individual notification items**
- Each item: avatar/initials, message text, relative timestamp
- Header: "Notifications" + "Mark all as read"
- Empty state: "No new notifications"

**Impact:** Completely different data model and UX. The Vue version shows "which channels have unread messages" (useful for navigation). The Svelte version shows a notification feed (different mental model). The API endpoints may not even match.

---

## 10. Styling & Theming 🔴🟡

### 10.1 Semantic Token Drift

The Vue app uses a comprehensive semantic design token system:
- Colors: `bg-bg-app`, `bg-bg-surface-1`, `bg-bg-surface-2`, `text-text-1`, `text-text-2`, `border-border-1`, `bg-brand`
- Radius: `rounded-r-1`, `rounded-r-2`, `rounded-r-3`
- Shadows: `shadow-1`, `shadow-2xl`
- Transitions: `transition-standard`

The Svelte app has **regressed to raw Tailwind colors** in many components:

| Component | Vue Tokens | Svelte Colors |
|-----------|-----------|---------------|
| ChatSidebar | `bg-bg-surface-2 text-text-1` | `bg-slate-950 text-white` |
| MessageList | `bg-bg-app` | `bg-gray-50` |
| MessageList empty | `bg-bg-surface-1 border-border-1` | `bg-white border-gray-300` |
| Composer | `bg-bg-surface-1 border-border-1` | `bg-white border-gray-200` |
| Composer focus | `border-brand/60 ring-brand/10` | `border-indigo-500 ring-indigo-100` |
| Send button | `bg-brand text-brand-foreground` | `bg-indigo-600 text-white` |
| PinnedMessagesPanel | `bg-bg-surface-1 border-border-1` | `bg-white border-gray-200` |
| SavedMessagesPanel | `bg-bg-surface-1 border-border-1` | `bg-white border-gray-200` |

**Impact:**
1. **Theme switching is broken.** Components using raw colors will not respond to light/dark/high-contrast theme changes.
2. **Visual inconsistency.** The app looks pieced together rather than cohesive.
3. **Brand customization lost.** The `bg-brand` token allowed easy re-theming. `bg-indigo-600` is hardcoded.

---

### 10.2 Missing CSS Custom Properties

**Vue:** Uses CSS custom properties for layout sizing:
```css
--sidebar-width
--rhs-width
--header-height
--msg-max-width
```

**Svelte:** Some properties referenced (`var(--header-height)`, `var(--rhs-width)`) but not consistently applied. No `--msg-max-width`.

---

## 11. Animation & Transitions 🟢

**Vue:** Rich `<Transition>` animations throughout:
- Dropdowns: opacity + translate + scale
- Sidebars: translate-x with easing
- Modals: fade + scale
- Hover actions: opacity transitions

**Svelte:** Minimal animations:
- `transition-standard` class (CSS transition)
- `animate-slide-in-right` for ChannelInfoPanel
- `animate-pulse` for sending status
- No enter/leave transitions for modals or dropdowns

**Impact:** The Svelte UI feels snappier but also more abrupt. Modals and dropdowns appear/disappear instantly.

---

## 12. Accessibility 🟡

**Vue:**
- `focus-ring` utility for accessible focus states
- `aria-label` on interactive elements
- `role` attributes
- `data-testid` for testing
- Teleport for modals/menus to escape stacking context

**Svelte:**
- `aria-label` present on many elements
- `role` attributes present
- `aria-current` for selected channels
- **Missing:** Focus trapping in modals
- **Missing:** Some keyboard handling (Escape in modals is inconsistent)

---

## 13. Known Bugs in Svelte 🔴

1. **`ChannelInfoPanel` created date:** `formatDate` receives `undefined` hardcoded — always shows `—`
2. **`MessageItem` reactions:** Local-only toggle, no API integration
3. **`ChatSidebar` fallback data:** Default demo data baked into props risks showing fake data
4. **Mixed reactivity:** Svelte 5 runes (`$derived`, `$state`) mixed with legacy stores (`$chatStore`) may cause subtle reactivity bugs

---

## Priority Action Matrix

### 🔴 High Priority (Fix Before Production)

| # | Gap | Effort | File(s) |
|---|-----|--------|---------|
| 1 | **Markdown rendering** in messages | Medium | `MessageItem.svelte` |
| 2 | **Reaction API integration** | Low | `MessageItem.svelte` |
| 3 | **Infinite scroll** for message history | Medium | `MessageList.svelte`, `chat.ts` |
| 4 | **Global Header** (search, user menu, notifications) | High | New: `GlobalHeader.svelte` |
| 5 | **Team Rail** | Medium | New: `TeamRail.svelte` |
| 6 | **Image gallery / lightbox** | Medium | `MessageItem.svelte`, `ImageGallery.svelte` |
| 7 | **Mobile drawer architecture** | High | `ChatView.svelte`, `ChatSidebar.svelte` |
| 8 | **Semantic token alignment** (sidebar, composer, message list) | Medium | `ChatSidebar.svelte`, `MessageComposer.svelte`, `MessageList.svelte`, `PinnedMessagesPanel.svelte`, `SavedMessagesPanel.svelte` |
| 9 | **Channel category headers** with favorites | Medium | `ChatSidebar.svelte` |
| 10 | **Mention highlighting** | Low | `MessageItem.svelte` |
| 11 | **Date separators** in message list | Low | `MessageList.svelte` |
| 12 | **"New messages" divider & floating button** | Medium | `MessageList.svelte` |
| 13 | **More menu items:** Save, Pin, Mark unread | Low | `MessageItem.svelte` |
| 14 | **Notifications data model alignment** | High | `NotificationsDropdown.svelte`, backend API |

### 🟡 Medium Priority (Polish & UX)

| # | Gap | Effort | File(s) |
|---|-----|--------|---------|
| 15 | **Drag & drop file upload** | Medium | `MessageComposer.svelte` |
| 16 | **Markdown preview** toggle | Low | `MessageComposer.svelte` |
| 17 | **Channel autocomplete** (`~`) | Low | `MessageComposer.svelte` |
| 18 | **Command autocomplete** (`^k`) | Medium | `MessageComposer.svelte` |
| 19 | **Formatting shortcuts** (Ctrl+I, lists) | Low | `MessageComposer.svelte` |
| 20 | **Call button in composer** | Low | `MessageComposer.svelte` |
| 21 | **Mobile sidebar toggle** in header | Low | `ChannelHeader.svelte` |
| 22 | **Channel topic display** | Low | `ChannelHeader.svelte` |
| 23 | **Members button** in header | Low | `ChannelHeader.svelte` |
| 24 | **Context menus** on channel right-click | Medium | `ChatSidebar.svelte` |
| 25 | **Hover actions** on channels (mark read, more) | Low | `ChatSidebar.svelte` |
| 26 | **Sidebar footer** (mark all read, browse, create) | Low | `ChatSidebar.svelte` |
| 27 | **Message highlight on jump** | Low | `MessageList.svelte`, `MessageItem.svelte` |
| 28 | **Edit time limit** enforcement | Low | `MessageItem.svelte` |
| 29 | **Video call / calls plugin** message types | Medium | `MessageItem.svelte` |
| 30 | **Transition animations** for modals/dropdowns | Medium | Multiple |
| 31 | **Focus trapping** in modals | Medium | Multiple |
| 32 | **RHS container** with unified animations | Medium | `ChatView.svelte` |

### 🟢 Low Priority (Nice to Have)

| # | Gap | Effort | File(s) |
|---|-----|--------|---------|
| 33 | **Team dropdown** in sidebar header | Low | `ChatSidebar.svelte` |
| 34 | **ChannelInfoPanel created date** fix | Trivial | `ChannelInfoPanel.svelte` |
| 35 | **Remove fallback demo data** from sidebar | Trivial | `ChatSidebar.svelte` |
| 36 | **Density setting** support (compact/comfortable) | Medium | Multiple |
| 37 | **Message action bar** positioning refinements | Low | `MessageItem.svelte` |
| 38 | **Custom scrollbar** styling | Low | `MessageList.svelte` |
| 39 | `--msg-max-width` CSS property | Trivial | `MessageList.svelte` |
| 40 | **File preview sizing** improvements | Low | `MessageItem.svelte` |
| 41 | **Typing indicator** styling polish | Low | `TypingIndicator.svelte` |
| 42 | **Empty state** illustrations | Low | Multiple |
| 43 | **Keyboard shortcut hints** (Enter to send, etc.) | Trivial | `MessageComposer.svelte` |
| 44 | **DND duration submenu** in user menu | Low | `UserMenu.svelte` |

---

## Visual Comparison Summary

From the screenshot (current Svelte UI):

| Element | Vue | Svelte (Current) |
|---------|-----|------------------|
| **Top bar** | Global header with logo, search, user menu | Channel header only |
| **Far left** | Team rail with team badges | Nothing |
| **Sidebar** | Rich: categories, favorites, hover actions, context menus | Flat list, static, hardcoded dark theme |
| **Messages** | Markdown formatted, mentions highlighted, date separators | Plain text, no dates, no highlights |
| **Composer** | Rich toolbar, drag-drop, preview, full autocomplete | Basic toolbar, no drag-drop, 2 emoji suggestions |
| **RHS panels** | Unified container with animations | Inline conditional blocks, inconsistent styling |
| **Theme** | Fully semantic, theme-switchable | Mixed raw colors, sidebar always dark |

---

## Recommendations

1. **Establish a Svelte design token CSS file** that mirrors the Vue token system. Audit every component and replace raw colors (`bg-slate-950`, `bg-indigo-600`, `bg-gray-50`) with semantic tokens.

2. **Create `GlobalHeader.svelte`** as the highest-priority new component. It unlocks search, notifications, user presence, and app-level navigation.

3. **Add markdown rendering** immediately. This is the most visible regression users will notice.

4. **Implement infinite scroll** in `MessageList`. Without it, the app is unusable for active channels.

5. **Fix reaction API integration** — a one-line bug with high user impact.

6. **Add mobile drawer architecture** before considering the app "responsive."

7. **Reconsider the NotificationsDropdown data model.** Decide whether to match Vue's "unread channels" model or commit to the new "notification feed" model and update the backend accordingly.

---

*Report generated by automated source-code comparison.*
