# Channel Bugs Fix Plan

## Overview
This plan addresses 6 bugs/missing features related to channel management and messaging in RustChat.

---

## Bug 1: Channel Update/Delete by Creator

### Current State
- Channels can be created but not renamed or deleted by their creator
- Backend has `PUT /channels/{id}` endpoint but only allows updating `display_name`, `purpose`, `header`
- `ChannelUpdated` and `ChannelDeleted` WebSocket events are defined but never broadcast
- No UI for editing channel name/description

### Root Causes
1. Backend update endpoint doesn't allow changing `name` field
2. No permission check specifically for channel creator
3. WebSocket events not broadcast after update/delete operations
4. Frontend lacks channel settings/edit UI

### Implementation Tasks

#### Backend Tasks
1. **Update Channel Update Endpoint** (`backend/src/api/channels.rs` and `backend/src/api/v4/channels/crud.rs`)
   - Allow updating `name` field
   - Add permission check: creator OR channel admin OR team admin
   - Broadcast `ChannelUpdated` WebSocket event after successful update

2. **Add Channel Delete Endpoint** (`backend/src/api/channels.rs`)
   - Add `DELETE /channels/{id}` endpoint (native API)
   - Soft delete (set `deleted_at` timestamp)
   - Permission check: creator OR channel admin OR team admin
   - Broadcast `ChannelDeleted` WebSocket event

3. **Update WebSocket Broadcast** (`backend/src/api/v4/channels/crud.rs`)
   - Add `ChannelUpdated` broadcast in `update_channel` and `patch_channel` handlers
   - Add `ChannelDeleted` broadcast in `delete_channel` handler

#### Frontend Tasks
4. **Add Channel Edit UI** (`frontend/src/components/channels/`)
   - Create `EditChannelModal.vue` component
   - Form fields: name, display_name, purpose/description
   - Validation for name (lowercase, no spaces)

5. **Update Channel Context Menu** (`frontend/src/components/channels/ChannelContextMenu.vue`)
   - Add "Edit Channel" option for channel creator/admins
   - Add "Delete Channel" option with confirmation dialog

6. **Add Channel Service Methods** (`frontend/src/features/channels/services/channelService.ts`)
   - `updateChannel(channelId, data)` method
   - `deleteChannel(channelId)` method
   - Handle WebSocket events for channel updates

7. **Handle WebSocket Events** (`frontend/src/core/websocket/registerHandlers.ts`)
   - Add handler for `channel_updated` event
   - Add handler for `channel_deleted` event
   - Update channel store when events received

---

## Bug 2: Real-time Message Deletion

### Current State
- Backend broadcasts `post_deleted` WebSocket event
- Frontend has handler registered in `registerHandlers.ts`
- Message is removed from store via `handleMessageDelete`

### Root Cause Analysis
Based on code review, the implementation looks correct. The issue might be:
1. WebSocket event not being received by all users
2. Event payload structure mismatch
3. Handler not processing the event correctly

### Implementation Tasks

#### Verification Tasks
1. **Verify WebSocket Broadcast** (`backend/src/api/posts.rs` and `backend/src/api/v4/posts.rs`)
   - Confirm `post_deleted` event is broadcast to all channel members
   - Check broadcast `channel_id` is set correctly
   - Verify `exclude_user_id` is set (deleter doesn't need the event)

2. **Fix WebSocket Payload Consistency**
   - Native API sends: `{id, channel_id}`
   - MM v4 API sends: full `deleted_post` object
   - Standardize payload to include: `post_id`, `channel_id`

#### Frontend Tasks
3. **Verify Handler Registration** (`frontend/src/core/websocket/registerHandlers.ts`)
   - Confirm handler is properly registered for `post_deleted` event
   - Check that handler extracts `post_id` and `channel_id` correctly

4. **Debug Message Removal** (`frontend/src/features/messages/handlers/messageSocketHandlers.ts`)
   - Ensure `handleMessageDelete` is called
   - Verify store's `removeMessage` correctly filters by channel AND message ID
   - Add logging for debugging

---

## Bug 3: Private Channels in Sidebar

### Current State
- `ChannelSidebar.vue` shows 4 sections: Favorites, Channels (public), Private Channels, DMs
- Private channels already use Lock icon

### Desired State
- Merge private channels into the main "Channels" section
- Keep Lock icon to distinguish private channels
- Remove separate "Private Channels" section

### Implementation Tasks

#### Frontend Tasks
1. **Update ChannelSidebar.vue** (`frontend/src/components/layout/ChannelSidebar.vue`)
   - Remove separate "Private Channels" section (lines ~143-148)
   - Merge `privateChannels` into `publicChannels` computed property
   - Rename computed property to just `channels` for clarity
   - Keep Lock icon for private channels (already implemented)

2. **Update Channel Sorting**
   - Combine and sort all channels alphabetically (public + private)
   - Favorites should still appear at top

---

## Bug 4: Browse Channels Not Working

### Current State
- `BrowseChannelsModal.vue` exists but shows empty list
- Backend endpoint exists: `GET /channels?team_id=<id>&available_to_join=true`

### Root Cause Analysis
1. API might not be returning public channels correctly
2. Frontend might not be calling API correctly
3. User might not be a team member (required permission)

### Implementation Tasks

#### Backend Tasks
1. **Fix Browse Channels Query** (`backend/src/api/channels.rs`)
   - Verify SQL query returns public channels user is not member of
   - Check that team membership check is working correctly
   - Ensure `available_to_join=true` filter works for public channels

#### Frontend Tasks
2. **Debug API Call** (`frontend/src/features/channels/services/channelService.ts`)
   - Add logging to `loadJoinableChannels` method
   - Verify `teamId` is being passed correctly
   - Check API response handling

3. **Fix BrowseChannelsModal** (`frontend/src/components/modals/BrowseChannelsModal.vue`)
   - Ensure modal calls `fetchJoinableChannels` on open
   - Handle empty state properly (show "No channels available" message)
   - Add loading state

4. **Add Channel Type Indicator**
   - Show lock icon for private channels in browse list
   - Show hash icon for public channels

---

## Bug 5: Channel Bold + Notifications for New Messages

### Current State
- Channels become bold based on unread count (working)
- Browser notifications only trigger on mentions (not all messages)

### Desired State
- Channel becomes bold when new message arrives (working - verify)
- Browser notifications for new messages (not just mentions)

### Implementation Tasks

#### Frontend Tasks
1. **Verify Bold Channel Names** (`frontend/src/components/layout/ChannelSidebar.vue`)
   - Check that `unreadCount > 0` applies `font-semibold` class
   - Ensure WebSocket `posted` event increments unread count
   - Verify current channel doesn't show as bold

2. **Fix Browser Notifications** (`frontend/src/composables/useWebSocket.ts`)
   - Currently only notifies on mentions: `if (mentionsUser)`
   - Remove mention check to notify on all messages
   - Or add user preference for notification level (all vs mentions)
   - Notification should include channel name and message preview

3. **Add Desktop Notifications Toggle**
   - Add to user preferences/settings
   - Options: "All messages", "Mentions only", "None"

---

## Bug 6: Composer Send Button with Attachments

### Current State
- `canSend` computed: `(hasContent || hasUploadedFiles) && !hasUploadInProgress`
- User reports button is disabled after attachment upload completes

### Root Cause Analysis
The logic looks correct. The issue might be:
1. `uploaded` property not being set correctly after upload
2. Reactivity issue with `attachedFiles` array
3. Race condition between upload completion and state update

### Implementation Tasks

#### Frontend Tasks
1. **Debug Upload State** (`frontend/src/components/composer/MessageComposer.vue`)
   - Add console logging to track `attachedFiles` state
   - Verify `uploaded` property is set after successful upload
   - Check that `uploading` is set to false when upload completes

2. **Fix Reactivity Issue**
   - Ensure `attachedFiles` array is properly reactive
   - Use `Vue.set` or array replacement instead of mutation if needed
   - Verify `canSend` computed re-evaluates when `attachedFiles` changes

3. **Verify File Upload Flow**
   - Check `handleFiles` method sets up upload correctly
   - Verify `onUploadProgress` callback updates state
   - Ensure error handling doesn't leave upload in stuck state

---

## Testing Checklist

### Backend Tests
- [ ] Channel update endpoint allows name change by creator
- [ ] Channel delete endpoint soft-deletes channel
- [ ] WebSocket events broadcast on channel update/delete
- [ ] Browse channels returns public channels not yet joined
- [ ] Message deletion broadcasts to all channel members

### Frontend Tests
- [ ] Channel creator sees Edit/Delete options in context menu
- [ ] Channel edit modal updates name/description
- [ ] Channel delete shows confirmation and removes channel
- [ ] Private channels appear in main channels list with lock icon
- [ ] Browse channels shows public channels to join
- [ ] Joining channel adds it to sidebar immediately
- [ ] Channel becomes bold when new message arrives
- [ ] Browser notification shows for new messages
- [ ] Send button enabled after attachment upload completes
- [ ] Send button disabled during attachment upload

### Integration Tests
- [ ] User A deletes message, User B sees it removed immediately
- [ ] User A updates channel name, User B sees updated name
- [ ] User A deletes channel, User B sees channel removed from sidebar

---

## Files to Modify

### Backend
1. `backend/src/api/channels.rs` - Update channel update, add delete endpoint
2. `backend/src/api/v4/channels/crud.rs` - Add WebSocket broadcasts
3. `backend/src/realtime/events.rs` - Verify event types (if needed)

### Frontend
1. `frontend/src/components/channels/EditChannelModal.vue` - NEW FILE
2. `frontend/src/components/channels/ChannelContextMenu.vue` - Add edit/delete options
3. `frontend/src/components/layout/ChannelSidebar.vue` - Merge private channels
4. `frontend/src/components/modals/BrowseChannelsModal.vue` - Fix loading/display
5. `frontend/src/components/composer/MessageComposer.vue` - Fix send button logic
6. `frontend/src/composables/useWebSocket.ts` - Fix notifications
7. `frontend/src/core/websocket/registerHandlers.ts` - Add channel event handlers
8. `frontend/src/features/channels/services/channelService.ts` - Add update/delete methods
9. `frontend/src/features/channels/stores/channelStore.ts` - Handle channel updates

---

## Estimated Effort

| Bug | Backend | Frontend | Total |
|-----|---------|----------|-------|
| 1. Channel update/delete | 4h | 6h | 10h |
| 2. Message deletion realtime | 1h | 1h | 2h |
| 3. Private channels sidebar | 0h | 1h | 1h |
| 4. Browse channels | 1h | 2h | 3h |
| 5. Channel bold + notifications | 0h | 2h | 2h |
| 6. Composer send button | 0h | 2h | 2h |
| **Total** | **6h** | **14h** | **20h** |

---

## Priority Order

1. **Bug 6** - Composer send button (critical usability issue)
2. **Bug 3** - Private channels sidebar (simple UI change)
3. **Bug 2** - Message deletion realtime (verify/fix existing code)
4. **Bug 4** - Browse channels (medium complexity)
5. **Bug 5** - Channel notifications (medium complexity)
6. **Bug 1** - Channel update/delete (highest complexity)
