# Ringback Tone Implementation Plan

## Current State Analysis

### Mobile App (Mattermost Mobile)
- Has `InCallManager.startRingback()` available in the library
- **NEVER calls it** for caller-side ringback
- Only uses `startRingtone()` for incoming call notifications
- The call flow:
  1. User starts call → WebSocket connects → `InCallManager.start()`
  2. No audio feedback while waiting for others to join

### Backend (RustChat)
Already sends correct events:
- `calls_call_start` - when call is created
- `calls_user_joined` - when any user joins (including caller)
- `calls_call_state` - full call state updates

## Implementation Strategy

Since we cannot modify the mobile app, this is a **documentation and preparation** task.
The mobile app would need to implement ringback on their side.

## What Mobile App Should Do

### File: `app/products/calls/connection/connection.ts`

Add ringback logic in the `ws.on('join', ...)` handler (around line 321):

```typescript
ws.on('join', async () => {
    logDebug('calls: join ack received, initializing connection');
    
    // START RINGBACK - Caller is waiting for others
    // Check if we're the only participant
    const currentCall = getCurrentCall();
    if (currentCall && Object.keys(currentCall.sessions).length <= 1) {
        InCallManager.startRingback();  // Play ringback tone
    }
    
    // ... rest of existing code
    InCallManager.start();
    InCallManager.stopProximitySensor();
    // ...
});
```

### File: `app/products/calls/connection/websocket_event_handlers.ts`

Add ringback stop logic in `handleCallUserJoined`:

```typescript
export const handleCallUserJoined = (serverUrl: string, msg: WebSocketMessage<UserJoinedData>) => {
    // Load user model async (if needed).
    fetchUsersByIds(serverUrl, [msg.data.user_id]);

    userJoinedCall(serverUrl, msg.broadcast.channel_id, msg.data.user_id, msg.data.session_id);
    
    // STOP RINGBACK - Someone else joined
    const currentCall = getCurrentCall();
    if (currentCall && currentCall.channelId === msg.broadcast.channel_id) {
        // Check if this is the second participant (first remote)
        const sessionCount = Object.keys(currentCall.sessions).length;
        if (sessionCount >= 2) {
            InCallManager.stopRingback();
        }
    }
};
```

### File: `app/products/calls/connection/connection.ts`

Also stop ringback when remote stream is received (around line 430):

```typescript
peer.on('stream', (remoteStream: MediaStream) => {
    logDebug('calls: new remote stream received', remoteStream.id);
    
    // STOP RINGBACK - Remote audio/video received
    InCallManager.stopRingback();
    
    for (const track of remoteStream.getTracks()) {
        logDebug('calls: remote track', track.id);
    }
    // ... rest of existing code
});
```

## Backend Preparation

The RustChat backend is already compatible. No changes needed.

If we wanted to add a custom event for this, we could add:

```rust
// Event sent to caller when they're alone in call
"custom_com.mattermost.calls_ringback_start"

// Event sent when second participant joins
"custom_com.mattermost.calls_ringback_stop"
```

But this is unnecessary since mobile can derive this from:
- `calls_user_joined` events
- `calls_call_state` updates

## Conclusion

**Ringback tone is a client-side feature.** The mobile app needs to:
1. Call `InCallManager.startRingback()` when alone in call
2. Call `InCallManager.stopRingback()` when others join

The RustChat backend already provides all necessary events.
