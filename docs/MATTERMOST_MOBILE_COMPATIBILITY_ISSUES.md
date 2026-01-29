# Mattermost-Mobile Compatibility Issues Analysis

## Summary

Based on analysis of the mattermost-mobile client source code and RustChat backend implementation, three critical compatibility issues have been identified:

1. **Status Change Not Working** - Users always appear offline
2. **DM Channel Shows Wrong Name** - Shows own name instead of the other person
3. **400 Error When Creating DM** - Direct message creation fails

---

## Issue 1: Status Change Not Working

### Problem
When users set their status to online/away/dnd via the mobile app, the status doesn't update or always shows as offline.

### Root Cause Analysis

**Current Implementation Status:**
- ✅ `PUT /api/v4/users/{user_id}/status` - Updates status in database
- ✅ `GET /api/v4/users/status/ids` - Returns statuses by IDs
- ✅ `GET /api/v4/users/{user_id}/status` - Returns single user status
- ✅ `status_change` WebSocket event is being broadcast

**The Problem:**
The mobile app expects to receive status updates in real-time via WebSocket. The current implementation broadcasts status changes, but there's likely an issue with:
1. WebSocket subscription not including the user
2. Status not being included in the initial data load

**File:** `backend/src/api/v4/users.rs:1382-1420`

```rust
async fn update_status(...) {
    // Updates DB correctly
    sqlx::query("UPDATE users SET presence = $1 WHERE id = $2")
    
    // Broadcasts via WebSocket
    let broadcast = crate::realtime::WsEnvelope::event(
        crate::realtime::EventType::UserUpdated,
        ...
    );
    state.ws_hub.broadcast(broadcast).await;
}
```

**File:** `backend/src/api/v4/websocket.rs:355-388`

```rust
"user_updated" => {
    // Converts to status_change event
    event: "status_change".to_string(),
    data: json!({
        "user_id": user_id,
        "status": status_str,
        "manual": manual,
        "last_activity_at": last_activity_at
    }),
}
```

### Verification Steps
1. Check if WebSocket connection is established when app opens
2. Verify `status_change` events are being received by mobile client
3. Ensure initial status batch loading via `POST /api/v4/users/status/ids` works

### Likely Fix Needed
The status broadcasting looks correct. The issue might be:
1. WebSocket not subscribing to user events properly
2. Mobile client not handling the status_change event correctly
3. Need to verify the WebSocket event is being sent to the right users

---

## Issue 2: DM Channel Shows Own Name

### Problem
When creating a direct message channel, the mobile app shows the current user's name instead of the other person's name in the left sidebar.

### Root Cause Analysis

**Current Implementation:**

**File:** `backend/src/api/v4/channels.rs:284-342`

```rust
async fn create_direct_channel(...) {
    // Creates channel with empty display_name
    let channel: Channel = sqlx::query_as(
        r#"
        INSERT INTO channels (team_id, type, name, display_name, purpose, header, creator_id)
        VALUES ($1, 'direct', $2, '', '', '', $3)  // <-- display_name is EMPTY
        "#,
    )
    ...
}
```

**File:** `backend/src/mattermost_compat/mappers.rs:68-97`

```rust
impl From<Channel> for mm::Channel {
    fn from(channel: Channel) -> Self {
        mm::Channel {
            ...
            display_name: channel.display_name.unwrap_or_else(|| channel.name.clone()),
            // ^ Returns empty string or "dm_uuid1_uuid2"
            ...
        }
    }
}
```

### The Problem

1. DM channel is created with **empty** `display_name`
2. When returned to mobile, it uses `channel.name` (like "dm_a1b2c3d4_e5f6g7h8")
3. Mattermost mobile expects the `display_name` to be set to the OTHER user's display name
4. For DMs, the mobile client doesn't calculate the name - it expects the server to provide it

### Required Fix

When returning a DM channel, the server needs to:
1. Get the other user's info from channel_members
2. Set display_name to the other user's display_name/username
3. Return the populated display_name in the response

**Implementation Needed:**

Modify `get_channel` endpoint (`backend/src/api/v4/channels.rs:126-150`) to:

```rust
async fn get_channel(...) {
    let channel: Channel = ...
    
    // If DM channel, populate display_name with other user's name
    if channel.channel_type == ChannelType::Direct {
        let other_user = sqlx::query_as(
            "SELECT u.* FROM users u 
             JOIN channel_members cm ON cm.user_id = u.id 
             WHERE cm.channel_id = $1 AND u.id != $2"
        )
        .bind(channel.id)
        .bind(auth.user_id)
        .fetch_one(&state.db)
        .await?;
        
        channel.display_name = Some(other_user.display_name.unwrap_or(other_user.username));
    }
    
    Ok(Json(channel.into()))
}
```

---

## Issue 3: 400 Error When Creating DM

### Problem
When trying to create a direct message channel on mobile, clicking "Send Message" results in a 400 error.

### Root Cause Analysis

**Current Implementation:**

**File:** `backend/src/api/v4/channels.rs:279-282`

```rust
#[derive(serde::Deserialize)]
struct DirectChannelRequest {
    user_ids: Vec<String>,  // <-- Expects object with user_ids field
}
```

**Mattermost Mobile Implementation:**

**File:** `app/client/rest/channels.ts:115-118`

```typescript
createDirectChannel = async (userIds: string[]) => {
    return this.doFetch(
        `${this.getChannelsRoute()}/direct`,
        {method: 'post', body: userIds},  // <-- Sends raw array, not object!
    );
};
```

### The Problem

**MISMATCH IN REQUEST FORMAT:**

- **RustChat expects:** `{"user_ids": ["userId1", "userId2"]}`
- **Mobile sends:** `["userId1", "userId2"]` (raw array)

The mobile client sends the user IDs as a **plain JSON array**, not wrapped in an object with a `user_ids` field.

This causes serde deserialization to fail with:
```
Invalid user_ids: expected object, got array
```

### Required Fix

Change the request handler to accept either format:

**File:** `backend/src/api/v4/channels.rs`

```rust
#[derive(serde::Deserialize)]
#[serde(untagged)]
enum DirectChannelRequest {
    Array(Vec<String>),
    Object { user_ids: Vec<String> },
}

async fn create_direct_channel(...) {
    let user_ids = match input {
        DirectChannelRequest::Array(ids) => ids,
        DirectChannelRequest::Object { user_ids } => user_ids,
    };
    
    if user_ids.len() != 2 {
        return Err(...);
    }
    
    let mut ids: Vec<Uuid> = user_ids
        .iter()
        .filter_map(|id| parse_mm_or_uuid(id))
        .collect();
    
    // ... rest of the logic
}
```

---

## Additional Compatibility Issues Discovered

### Missing Endpoints
Based on the mattermost-mobile source code analysis, the following endpoints are used by mobile but may not be fully implemented:

1. **GET /api/v4/users/me/teams** - Get user's teams
2. **GET /api/v4/users/me/teams/members** - Get team memberships
3. **GET /api/v4/teams/{teamId}/channels** - Get channels for team
4. **GET /api/v4/channels/{channelId}/members/me** - Get my channel membership
5. **POST /api/v4/channels/{channelId}/members/ids** - Get members by IDs
6. **GET /api/v4/channels/{channelId}/stats** - Get channel stats
7. **GET /api/v4/users/{userId}/image** - Get user profile image

### WebSocket Events
The mobile client expects these WebSocket events:
- `posted` - New message
- `post_edited` - Message updated
- `post_deleted` - Message deleted
- `status_change` - User status changed
- `channel_viewed` - Channel viewed
- `user_typing` - User typing indicator
- `reaction_added` / `reaction_removed` - Reactions
- `hello` - Connection established

---

## Implementation Priority

### P0 (Critical - Fix Immediately)
1. ✅ **Fix DM 400 Error** - Change request format to accept array
2. ✅ **Fix DM Display Name** - Populate display_name with other user's name

### P1 (High Priority)
3. **Verify Status WebSocket** - Ensure status updates are received
4. **Add Missing GET Endpoints** - Teams, channel members, stats

### P2 (Medium Priority)
5. **User Profile Images** - /api/v4/users/{id}/image endpoint
6. **Additional WebSocket Events** - Full event coverage

---

## Quick Fix Implementation

### Fix 1: DM Creation 400 Error

```rust
// backend/src/api/v4/channels.rs

#[derive(serde::Deserialize)]
#[serde(untagged)]
enum DirectChannelRequest {
    Array(Vec<String>),
    Object { user_ids: Vec<String> },
}

async fn create_direct_channel(...) {
    let input: DirectChannelRequest = parse_body(&headers, &body, "Invalid user_ids")?;
    
    let user_ids = match input {
        DirectChannelRequest::Array(ids) => ids,
        DirectChannelRequest::Object { user_ids } => user_ids,
    };
    
    if user_ids.len() != 2 {
        return Err(crate::error::AppError::BadRequest("user_ids must contain 2 users".to_string()));
    }
    
    let mut ids: Vec<Uuid> = user_ids
        .iter()
        .filter_map(|id| parse_mm_or_uuid(id))
        .collect();
    
    // ... rest unchanged
}
```

### Fix 2: DM Display Name

```rust
// backend/src/api/v4/channels.rs

async fn get_channel(...) {
    let channel_id = parse_mm_or_uuid(&channel_id)?;
    
    let channel: crate::models::Channel = sqlx::query_as("SELECT * FROM channels WHERE id = $1")
        .bind(channel_id)
        .fetch_one(&state.db)
        .await?;
    
    let mut mm_channel: mm::Channel = channel.clone().into();
    
    // For DM channels, populate display_name with other user's name
    if channel.channel_type == crate::models::channel::ChannelType::Direct {
        if let Ok(other_user) = sqlx::query_as::<_, crate::models::User>(
            "SELECT u.* FROM users u 
             JOIN channel_members cm ON cm.user_id = u.id 
             WHERE cm.channel_id = $1 AND u.id != $2"
        )
        .bind(channel_id)
        .bind(auth.user_id)
        .fetch_one(&state.db)
        .await
        {
            mm_channel.display_name = other_user.display_name
                .unwrap_or(other_user.username);
        }
    }
    
    Ok(Json(mm_channel))
}
```

### Fix 3: Status WebSocket Verification

Add logging to verify status events are being sent:

```rust
// backend/src/api/v4/users.rs

async fn update_status(...) {
    // ... existing code ...
    
    println!("[STATUS] Broadcasting status change for user {}: {}", 
        auth.user_id, input.status);
    
    state.ws_hub.broadcast(broadcast).await;
    
    println!("[STATUS] Broadcast complete");
    
    Ok(Json(status))
}
```

---

## Testing Checklist

### After implementing fixes:

- [ ] DM creation works without 400 error
- [ ] DM shows other person's name in sidebar
- [ ] Status changes reflect immediately on mobile
- [ ] WebSocket connection stays alive
- [ ] Status appears correctly after app restart
- [ ] All users in DM can see each other's messages
- [ ] Group messages (if supported) work correctly

---

## Conclusion

The three main issues have clear root causes:

1. **400 Error**: Request format mismatch - mobile sends array, backend expects object
2. **Wrong DM Name**: display_name not populated for DM channels
3. **Status Issue**: Likely WebSocket subscription or event handling - needs verification

All three can be fixed with targeted changes to the channels and users API endpoints.