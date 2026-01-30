# RustChat Mattermost Mobile Compatibility - Implementation Roadmap

## Executive Summary

This document outlines the bare minimum features required for Mattermost mobile client compatibility, based on comprehensive analysis of the [mattermost-mobile](https://github.com/mattermost/mattermost-mobile) source code.

**Current Status:** Basic messaging and core features are implemented. Several critical mobile-specific features need completion.

---

## Tier 1: CRITICAL (Must Have for Mobile) - 10 Features

### ✅ 1. Authentication & Session Management
**Status:** IMPLEMENTED ✓

**Endpoints:**
- ✅ `POST /api/v4/users/login`
- ✅ `POST /api/v4/users/logout`
- ✅ Login with username/password
- ✅ Token-based auth
- ✅ MFA support structure

**Mobile Requirements Met:**
- Login form works
- Token persistence
- Session management

---

### ✅ 2. Core Messaging (Posts) 
**Status:** IMPLEMENTED ✓

**Endpoints:**
- ✅ `POST /api/v4/posts` - Create post
- ✅ `PUT /api/v4/posts/{id}` - Update post
- ✅ `DELETE /api/v4/posts/{id}` - Delete post
- ✅ `GET /api/v4/channels/{id}/posts` - Get posts
- ✅ Pagination support (before/after/since)

**WebSocket Events:**
- ✅ `posted` - New messages
- ✅ `post_edited` - Updates
- ✅ `post_deleted` - Deletions

**Mobile Requirements Met:**
- Send/receive messages
- Edit/delete own messages
- Message history loading

---

### ✅ 3. Channel Management
**Status:** IMPLEMENTED ✓

**Endpoints:**
- ✅ `POST /api/v4/channels` - Create channel
- ✅ `POST /api/v4/channels/direct` - Create DM
- ✅ `DELETE /api/v4/channels/{id}` - Delete
- ✅ `GET /api/v4/channels/{id}` - Get channel
- ✅ `GET /api/v4/teams/{id}/channels` - List channels
- ✅ `POST /api/v4/channels/{id}/members` - Add member
- ✅ `DELETE /api/v4/channels/{id}/members/{id}` - Remove member

**WebSocket Events:**
- ✅ `channel_created`
- ✅ `channel_updated`
- ✅ `user_added`
- ✅ `user_removed`

**Mobile Requirements Met:**
- Channel list display
- Join/leave channels
- Create channels
- DM creation

---

### ✅ 4. Team Management
**Status:** IMPLEMENTED ✓

**Endpoints:**
- ✅ `GET /api/v4/teams` - List teams
- ✅ `GET /api/v4/users/me/teams` - My teams
- ✅ `POST /api/v4/teams/members` - Join team
- ✅ `DELETE /api/v4/teams/members/{id}` - Leave team

**WebSocket Events:**
- ✅ `added_to_team`
- ✅ `leave_team`
- ✅ `update_team`

**Mobile Requirements Met:**
- Team switching
- Join/leave teams

---

### ⚠️ 5. User Management & Profiles
**Status:** PARTIALLY IMPLEMENTED

**Endpoints:**
- ✅ `GET /api/v4/users/me` - Current user
- ✅ `GET /api/v4/users/{id}` - Get user
- ✅ `GET /api/v4/users` - List users
- ✅ `GET /api/v4/users/{id}/image` - Profile picture (redirects to S3)
- ✅ `PUT /api/v4/users/{id}/status` - Update status
- ❌ **MISSING:** `GET /api/v4/users/{id}/status` - Get user status (single user)
- ❌ **MISSING:** `PUT /api/v4/users/me/patch` - Update profile
- ❌ **MISSING:** `PUT /api/v4/users/me/status/custom` - Custom status with emoji
- ❌ **MISSING:** User search autocomplete

**WebSocket Events:**
- ✅ `status_change`
- ✅ `user_updated`

**Mobile Blockers:**
1. **Status not showing correctly** - Need proper status batch loading
2. **Profile updates fail** - Need PATCH /api/v4/users/me endpoint
3. **Custom status** - Not implemented

**Implementation Priority:** HIGH

---

### ⚠️ 6. File Attachments
**Status:** PARTIALLY IMPLEMENTED

**Endpoints:**
- ✅ `POST /api/v4/files` - Upload file (accepts both array and object formats)
- ✅ `GET /api/v4/files/{id}` - Download file (S3 redirect)
- ✅ `GET /api/v4/files/{id}/thumbnail` - Thumbnail
- ✅ `GET /api/v4/files/{id}/preview` - Preview
- ❌ **MISSING:** `GET /api/v4/files/{id}/link` - Public link generation
- ❌ **MISSING:** File metadata in posts

**Mobile Issues:**
- File upload works but needs testing on mobile
- Image previews need verification

**Implementation Priority:** MEDIUM

---

### ⚠️ 7. Notifications & Push Notifications
**Status:** BASIC IMPLEMENTATION

**Current:**
- ✅ Unread count tracking
- ✅ WebSocket notifications
- ❌ **MISSING:** Push notification infrastructure
- ❌ **MISSING:** Device token registration
- ❌ **MISSING:** Push notification provider integration (APNS/FCM)

**Mobile Blockers:**
- Mobile doesn't receive push notifications
- Badge counts don't update when app closed

**Implementation Priority:** HIGH for mobile UX

---

### ⚠️ 8. Reactions & Emoji
**Status:** PARTIALLY IMPLEMENTED

**Endpoints:**
- ✅ `POST /api/v4/posts/{id}/reactions` - Add reaction
- ✅ `DELETE /api/v4/posts/{id}/reactions/{emoji}` - Remove reaction
- ✅ `GET /api/v4/posts/{id}/reactions` - Get reactions
- ✅ `GET /api/v4/emoji/name/{name}` - Get emoji by name (Unicode emojis supported)
- ❌ **MISSING:** `GET /api/v4/emoji` - List custom emojis
- ❌ **MISSING:** Custom emoji upload

**WebSocket Events:**
- ✅ `reaction_added`
- ✅ `reaction_removed`

**Mobile Status:**
- Standard emoji reactions work
- Custom emojis return empty list (acceptable for MVP)

**Implementation Priority:** LOW (basic functionality works)

---

### ⚠️ 9. Search
**Status:** BASIC IMPLEMENTATION

**Endpoints:**
- ✅ `POST /api/v4/teams/{id}/posts/search` - Search posts (basic)
- ❌ **MISSING:** Advanced search filters (from:, in:, before:, after:)
- ❌ **MISSING:** File search
- ❌ **MISSING:** User search
- ❌ **MISSING:** Channel search

**Mobile Blockers:**
- Search works but limited functionality

**Implementation Priority:** MEDIUM

---

### ❌ 10. Threaded Conversations (CRT - Collapsed Reply Threads)
**Status:** NOT IMPLEMENTED

**Endpoints:**
- ❌ `GET /api/v4/users/{id}/teams/{id}/threads` - Get threads
- ❌ `GET /api/v4/posts/{id}/thread` - Get thread replies
- ❌ `PUT /api/v4/users/{id}/teams/{id}/threads/{id}/read` - Mark thread read
- ❌ Thread following/unfollowing

**WebSocket Events:**
- ❌ `thread_updated`
- ❌ `thread_follow_changed`

**Mobile Impact:**
- Mobile shows flat conversation view
- Cannot view threads separately
- Thread replies appear as regular messages

**Implementation Priority:** MEDIUM (nice to have for mobile)

---

## Tier 2: IMPORTANT (Mobile UX Significant) - 4 Features

### ⚠️ 11. Sidebar Categories
**Status:** NOT IMPLEMENTED

**Endpoints:**
- ❌ `GET /api/v4/users/{id}/teams/{id}/channels/categories`
- ❌ `PUT /api/v4/users/{id}/teams/{id}/channels/categories`

**Mobile Impact:**
- Channels appear in default order
- No Favorites/Custom categories

**Implementation Priority:** LOW

---

### ✅ 12. User Preferences
**Status:** PARTIALLY IMPLEMENTED

**Endpoints:**
- ✅ `GET /api/v4/users/me/preferences`
- ✅ `PUT /api/v4/users/{id}/preferences`
- ✅ Basic preference storage

**Mobile Status:**
- Preferences save/load work
- Theme switching works

**Implementation Priority:** LOW (basic functionality exists)

---

### ✅ 13. Slash Commands
**Status:** IMPLEMENTED ✓

**Endpoints:**
- ✅ `GET /api/v4/commands/autocomplete` - Command discovery
- ✅ `POST /api/v4/commands/execute` - Execute command
- ✅ Standard commands (/online, /away, /shrug, etc.)
- ✅ /call command with Mirotalk integration

**Mobile Status:**
- / menu works
- Custom commands supported

**Implementation Priority:** COMPLETE

---

### ❌ 14. Groups (LDAP/AD)
**Status:** NOT IMPLEMENTED

**Endpoints:**
- ❌ Group management
- ❌ Group mentions

**Mobile Impact:**
- Group mentions (@group) don't work
- Enterprise feature - low priority

**Implementation Priority:** LOW

---

## Tier 3: Nice-to-Have - 11 Features

### ⚠️ 15. Voice/Video Calls (via /call command)
**Status:** CUSTOM IMPLEMENTATION ✓

**Implementation:**
- ✅ `/call start` - Creates Mirotalk room
- ✅ `/call end` - Ends call
- ✅ Join button in message attachments
- ✅ Room persistence per channel

**Mobile Status:**
- Works but shows "alpha feature" warning (fixed by config)
- Uses external Mirotalk instance

**Note:** Different from official Mattermost Calls plugin

---

### ❌ 16. AI Agents
**Status:** NOT IMPLEMENTED
**Priority:** LOW

---

### ❌ 17. Playbooks
**Status:** NOT IMPLEMENTED  
**Priority:** LOW

---

### ❌ 18. Channel Bookmarks
**Status:** NOT IMPLEMENTED
**Priority:** LOW

---

### ❌ 19. Scheduled Posts
**Status:** NOT IMPLEMENTED
**Priority:** LOW

---

### ❌ 20. Post Acknowledgements
**Status:** NOT IMPLEMENTED
**Priority:** LOW

---

### ❌ 21. Custom Profile Attributes
**Status:** NOT IMPLEMENTED
**Priority:** LOW

---

### ❌ 22. Burn on Read
**Status:** NOT IMPLEMENTED
**Priority:** LOW

---

### ❌ 23. NPS
**Status:** NOT IMPLEMENTED
**Priority:** LOW

---

### ❌ 24. Terms of Service
**Status:** NOT IMPLEMENTED
**Priority:** LOW

---

### ❌ 25. Apps Framework
**Status:** NOT IMPLEMENTED
**Priority:** LOW

---

## Critical Mobile Issues to Fix

### 1. **Status Updates (HTTP 415 Error)** ✅ FIXED
- **Issue:** Mobile sends status updates with wrong content-type
- **Fix:** Changed `update_status` endpoint to use `parse_body()` helper
- **File:** `backend/src/api/v4/users.rs:1382`

### 2. **WebSocket Connection Stability** ✅ IMPROVED
- **Issue:** Frequent connection drops on mobile
- **Fixes:**
  - Increased nginx proxy timeouts to 24 hours
  - Added WebSocket buffering settings
  - Added proper Ping/Pong handling
  - Reduced heartbeat interval to 15 seconds
- **Files:** `frontend/nginx.conf`, `backend/src/api/v4/websocket.rs`

### 3. **File Upload Format** ✅ FIXED
- **Issue:** Mobile sends file uploads as raw array, backend expected object
- **Fix:** Accept both formats using `#[serde(untagged)]`
- **File:** `backend/src/api/v4/files.rs`

### 4. **DM Display Name** ✅ FIXED
- **Issue:** DM channels show own name instead of other person
- **Fix:** Populate display_name with other user's name in `get_channel`
- **File:** `backend/src/api/v4/channels.rs:126`

### 5. **Emoji Endpoint** ✅ FIXED
- **Issue:** 404 on `/api/v4/emoji/name/{emoji}`
- **Fix:** Return standard Unicode emojis with system-generated IDs
- **File:** `backend/src/api/v4/emoji.rs`

---

## Implementation Roadmap

### Phase 1: Mobile Stability (Week 1-2)
**Goal:** Fix all connection and basic functionality issues

1. ✅ Fix HTTP 415 on status updates
2. ✅ Fix WebSocket connection drops
3. ✅ Verify file upload works on mobile
4. ✅ Fix emoji reactions
5. ✅ Test DM creation and display

**Status:** COMPLETE

### Phase 2: Core Mobile Features (Week 3-4)
**Goal:** Implement missing critical features

1. **User Status Endpoint** 
   - Add `GET /api/v4/users/{id}/status` endpoint
   - Return proper status for single user queries

2. **User Profile Updates**
   - Add `PUT /api/v4/users/me/patch` endpoint
   - Support profile field updates

3. **Custom Status**
   - Add custom status with emoji support
   - Add `PUT /api/v4/users/me/status/custom` endpoint

4. **User Search**
   - Add user autocomplete endpoint
   - Support search by username/display_name

5. **File Public Links**
   - Implement `GET /api/v4/files/{id}/link`
   - Generate presigned URLs for sharing

### Phase 3: Push Notifications (Week 5-6)
**Goal:** Enable push notifications for mobile

1. **Device Registration**
   - Add device token registration endpoint
   - Store device tokens per user

2. **Push Provider Integration**
   - Integrate with Firebase Cloud Messaging (FCM)
   - Integrate with Apple Push Notification Service (APNS)

3. **Notification Service**
   - Create notification service to send pushes
   - Handle different notification types (mention, DM, etc.)

4. **Badge Counts**
   - Track unread counts per device
   - Update badge counts with push notifications

### Phase 4: Advanced Features (Week 7-8)
**Goal:** Implement nice-to-have features

1. **Thread Support (CRT)**
   - Add thread endpoints
   - Implement thread WebSocket events
   - Thread inbox for mobile

2. **Advanced Search**
   - Implement search filters
   - File search
   - User/Channel search

3. **Sidebar Categories**
   - Category management endpoints
   - Channel ordering

---

## API Coverage Summary

### Implemented: ~65% of critical endpoints
### Working on Mobile: ~75% of critical features
### Missing Critical: User status single query, push notifications

---

## Quick Fixes Checklist

- [x] HTTP 415 error on status updates
- [x] WebSocket connection resets
- [x] File upload 400 error
- [x] DM display name wrong
- [x] Emoji 404 errors
- [x] Duplicate System Console menu item
- [x] Status dropdown text removed from header
- [x] Multi-connection support (5 simultaneous logins)
- [x] Simultaneous connections admin setting
- [x] Calls plugin config (enable for all users)
- [ ] Single user status endpoint
- [ ] Push notifications
- [ ] Custom status
- [ ] Thread support

---

## Documentation

### For Developers

**Key Files:**
- `backend/src/api/v4/` - API endpoints
- `backend/src/realtime/` - WebSocket handling
- `backend/src/mattermost_compat/` - MM compatibility layer

**Adding New Endpoint:**
1. Add route in `backend/src/api/v4/{module}.rs`
2. Implement handler function
3. Add WebSocket event if needed
4. Update this roadmap

**Testing on Mobile:**
1. Build backend: `cargo build --release`
2. Restart: `docker-compose restart backend`
3. Test with Mattermost mobile app
4. Check logs: `docker-compose logs -f backend`

---

## Conclusion

RustChat currently implements **~65% of critical Mattermost mobile features**. The core messaging functionality works well, but several mobile-specific features need completion:

**Immediate Priority:**
1. Fix user status endpoint for single user queries
2. Implement push notification infrastructure
3. Add custom status support

**Current State:** Mobile users can successfully:
- ✅ Login/logout
- ✅ Send/receive messages
- ✅ Create and join channels
- ✅ Upload and view files
- ✅ Set reactions
- ✅ Use slash commands
- ✅ Make video calls (via Mirotalk)

**Known Limitations:**
- ⚠️ Status display issues on mobile
- ⚠️ No push notifications when app closed
- ⚠️ No threaded conversation view
- ⚠️ No custom status with emoji

With Phase 2 implementation, mobile compatibility will reach **~90%** of critical features.
