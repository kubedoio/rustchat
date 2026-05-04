# Mobile Client Ringing Support - Implementation Guide

## Issue
Ringing is not working on mattermost-mobile because the mobile client doesn't handle the `calls_ringing` websocket event.

## Root Cause
The mobile client is missing:
1. `CALLS_RINGING` websocket event constant
2. Event handler for `calls_ringing` events
3. Logic to add calls to incoming calls list when ringing event is received

## Backend Changes (Completed)
The Rustchat backend has been updated to include additional fields in the ringing event:

### Event Payload
```json
{
  "event": "custom_com.mattermost.calls_ringing",
  "data": {
    "call_id": "encoded_call_uuid",
    "call_id_raw": "call_uuid",
    "sender_id": "encoded_sender_uuid",
    "sender_id_raw": "sender_uuid",
    "username": "caller_username",
    "display_name": "Caller Display Name"
  },
  "broadcast": {
    "channel_id": "encoded_channel_uuid",
    "omit_users": null,
    "team_id": "",
    "user_id": ""
  }
}
```

## Mobile Client Changes Required

### 1. Add CALLS_RINGING Constant
**File**: `app/constants/websocket.ts`

Add to `WebsocketEvents` object:
```typescript
CALLS_RINGING: `custom_${Calls.PluginId}_ringing`,
```

### 2. Add Ringing Event Handler
**File**: `app/actions/websocket/event.ts`

Add case in `handleWebSocketEvent` switch statement:
```typescript
case WebsocketEvents.CALLS_RINGING:
    calls.handleCallRinging(serverUrl, msg);
    break;
```

### 3. Implement handleCallRinging Handler
**File**: `app/products/calls/connection/websocket_event_handlers.ts`

Add the handler function:
```typescript
export const handleCallRinging = async (serverUrl: string, msg: WebSocketMessage<CallRingingData>) => {
    // Don't ring if user is already in a call
    if (getCurrentCall()) {
        return;
    }

    const { call_id, sender_id, channel_id } = msg.data;
    const channelId = msg.broadcast.channel_id;

    // Load call info if not already loaded
    const callsState = getCallsState(serverUrl);
    if (!callsState.calls[channelId]) {
        // Fetch call state to get full call info
        await fetchCallState(serverUrl, channelId);
    }

    const call = callsState.calls[channelId];
    if (!call) {
        logDebug('calls: handleCallRinging could not find call for channel', channelId);
        return;
    }

    // Add to incoming calls
    await processIncomingCalls(serverUrl, [call], true);
};
```

### 4. Add CallRingingData Type
**File**: `@mattermost/calls/lib/types` (or appropriate types file)

```typescript
export type CallRingingData = {
    call_id: string;
    call_id_raw?: string;
    sender_id: string;
    sender_id_raw?: string;
    username?: string;
    display_name?: string;
};
```

## When Ringing Events Are Sent

### Auto-Ringing (DM/GM Channels)
When a call starts in a DM or GM channel, the backend automatically sends a `calls_ringing` event to all channel members except the caller.

### Manual Ringing
When a user clicks the "Ring" button in the call UI:
- Frontend calls `POST /plugins/com.mattermost.calls/calls/{channel_id}/ring`
- Backend broadcasts `calls_ringing` event to all channel members

## WebUI (Frontend) Implementation Reference

The WebUI already correctly handles ringing in `frontend/src/stores/calls.ts`:

```typescript
onEvent('custom_com.mattermost.calls_ringing', (data) => {
    if (isInCall.value) return
    const eventChannelId = readEventChannelId(data)
    const callerId = data.sender_id || data.sender_id_raw
    if (eventChannelId && callerId) {
        setIncomingCall({ channelId: eventChannelId, callerId })
    }
})
```

## Testing Steps

1. **Start a call in DM/GM channel**
   - User A starts a call
   - User B (mobile) should receive ringing notification
   
2. **Manual ringing**
   - User A starts a call in any channel
   - User A clicks "Ring" button
   - User B (mobile) should receive ringing notification

3. **Dismiss ringing**
   - User B dismisses the call notification
   - User B should not receive further ringing for this call

## Expected Behavior

### WebUI
✅ Working: Shows incoming call modal when ringing event is received

### Mobile
❌ Not Working: Missing handler (needs implementation per this guide)

## Backend Verification

The backend correctly sends ringing events:
- Event name: `custom_com.mattermost.calls_ringing`
- Sent for: DM/GM auto-ringing + manual ring button
- Payload includes: call_id, sender_id, username, display_name
- Excludes: The sender/caller from receiving their own ring
