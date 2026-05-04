# Ringback Tone Analysis - Mattermost Mobile vs RustChat

## Executive Summary

**Ringback tone is a client-side feature that is NOT implemented in the stock Mattermost mobile app.** The RustChat backend cannot add this feature without mobile app modifications.

## What is Ringback Tone?

Ringback tone is the sound played to the **caller** while waiting for someone to answer. This is different from:
- **Ringing tone**: Sound played to the **callee** (incoming call notification)

## Mattermost Mobile Analysis

### What We Found

1. **InCallManager.startRingback() exists but is never called**
   - File: `app/products/calls/connection/connection.ts`
   - The `InCallManager` from `react-native-incall-manager` has a `startRingback()` method
   - This method is imported but **never invoked** in the codebase

2. **Mobile app doesn't implement caller-side ringback**
   - When a user starts a call, the app immediately connects WebRTC
   - No audio is played to indicate "waiting for others to join"
   - If the phone is locked, the app is suspended and cannot access audio anyway

3. **Call flow on mobile:**
   ```
   1. User taps "Start Call"
   2. App requests microphone permission (if not granted)
   3. App initializes WebRTC peer connection
   4. App waits for remote participants
   5. NO ringback tone is played during this time
   ```

## Why Backend Cannot Fix This

### Technical Limitations

1. **Audio Access Requires Foreground**
   - iOS/Android restrict audio access for background/suspended apps
   - Push notifications cannot wake an app fast enough to play continuous audio
   - Only an active call session (like CallKit on iOS) can play audio when locked

2. **CallKit Integration Missing**
   - CallKit is the iOS framework for VoIP calls
   - Mattermost mobile doesn't use CallKit for outgoing call ringback
   - This would require client-side changes to `connection.service.ts`

3. **WebSocket Events Already Sent**
   - RustChat backend sends `calls_call_start` event to caller
   - Mobile app receives this but doesn't trigger ringback
   - Adding more events won't change mobile behavior

## What Would Be Required

To implement ringback tone, the mobile app would need:

1. **Foreground Detection**
   - Check if app is in foreground when starting a call
   - If yes, play ringback tone via `InCallManager.startRingback()`
   - Stop ringback when first participant joins

2. **CallKit Integration (iOS)**
   - Use `CXProvider` to show native call UI immediately
   - This allows audio even when phone is locked
   - Requires significant changes to call initialization flow

3. **TelecomManager Integration (Android)**
   - Similar to CallKit for Android
   - Requires `android.telecom` framework integration

## Current RustChat Implementation

The RustChat backend already sends all relevant events:

### 1. Call Start Event
```rust
broadcast_call_event(
    "custom_com.mattermost.calls_call_start",
    &channel_uuid,
    json!({
        "id": call_id,
        "channel_id": channel_id,
        "start_at": now,
        "owner_id": caller_id,
        "host_id": caller_id,
    }),
    Some(caller_id), // Excludes caller (sent to others)
)
```

### 2. User Joined Event
```rust
broadcast_call_event(
    "custom_com.mattermost.calls_user_joined",
    &channel_uuid,
    json!({
        "channel_id": channel_id,
        "user_id": user_id,
        "session_id": session_id,
    }),
    None, // Broadcast to all
)
```

### 3. Call State Event
```rust
broadcast_call_event(
    "custom_com.mattermost.calls_call_state",
    &channel_uuid,
    json!({"call": call_state, "call_id": call_id}),
    None,
)
```

## Conclusion

**Ringback tone cannot be implemented from the backend alone.** It requires client-side changes to:
1. Initialize audio/CallKit when starting a call
2. Play ringback sound while waiting
3. Stop when first participant joins

The RustChat backend is fully compatible with the Mattermost mobile app, but the mobile app lacks ringback tone functionality.
