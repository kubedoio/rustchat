# Push Proxy Troubleshooting Guide

## Problem: "Nothing happens" when sending push notifications

If you're using the test curl command and nothing seems to happen, follow this diagnostic checklist:

## Quick Diagnostic Steps

### 1. Check if Push Proxy is Running

```bash
# Check container status
docker-compose ps push-proxy

# Check logs
docker-compose logs push-proxy --tail=50

# Test health endpoint
curl http://localhost:3000/health
```

**Expected:** `{"status":"ok","service":"rustchat-push-proxy"}`

### 2. Test with Verbose Output

```bash
# Run the test script with your actual device token
./scripts/test_push_notification.sh YOUR_DEVICE_TOKEN android

# Or manually with verbose curl
curl -v -X POST http://localhost:3000/send \
  -H "Content-Type: application/json" \
  -d '{
    "token": "YOUR_DEVICE_TOKEN",
    "title": "Test Call",
    "body": "Incoming call",
    "platform": "android",
    "type": "call",
    "data": {
      "channel_id": "test",
      "post_id": "test",
      "type": "message",
      "sub_type": "calls",
      "sender_name": "Test User",
      "server_url": "https://rustchat.com"
    }
  }'
```

### 3. Check FCM Configuration

If you see "FCM not configured" in logs:

```bash
# Verify environment variables
echo $FIREBASE_PROJECT_ID
echo $GOOGLE_APPLICATION_CREDENTIALS

# Check if key file exists and is readable
ls -la secrets/firebase-key.json
```

**Fix:** Set these variables in your `.env` file and restart:
```bash
FIREBASE_PROJECT_ID=your-project-id
FIREBASE_KEY_PATH=./secrets/firebase-key.json
```

### 4. Check FCM Token Validity

If you see "Token unregistered" or "Invalid token":

- The device token is invalid or the app was uninstalled
- The token format is wrong
- The token is for a different Firebase project

**Fix:** Get a fresh device token from your Android app.

## Common Issues and Solutions

### Issue: No response from curl (connection refused)

**Symptoms:**
```
curl: (7) Failed to connect to localhost port 3000
```

**Causes:**
1. Push proxy container is not running
2. Port mapping is incorrect
3. Firewall blocking the port

**Solutions:**
```bash
# Restart the push proxy
docker-compose restart push-proxy

# Check if port is listening
netstat -tlnp | grep 3000

# Check docker port mapping
docker-compose ps push-proxy
```

### Issue: FCM returns 401 Unauthorized

**Symptoms:**
```json
{"error": "FCM error: Status 401: ..."}
```

**Causes:**
1. Invalid service account key
2. Wrong Firebase project ID
3. Key file not readable

**Solutions:**
```bash
# Verify key file is valid JSON
jq . secrets/firebase-key.json > /dev/null && echo "Valid JSON"

# Check project_id in key file matches FIREBASE_PROJECT_ID
jq '.project_id' secrets/firebase-key.json

# Regenerate key in Firebase Console if needed
```

### Issue: FCM returns 404 Not Found

**Symptoms:**
```json
{"error": "Token unregistered"}
```

**Causes:**
1. Device token is invalid
2. App was uninstalled
3. Token belongs to different app/package

**Solutions:**
- Get a fresh token from the device
- Check that the token matches your app's package name
- Ensure you're using the correct Firebase project

### Issue: Android app not receiving data messages

**Symptoms:**
- FCM reports success (HTTP 200)
- But `onMessageReceived()` is never called

**Causes:**
1. `sub_type` is not "calls" 
2. Message includes `notification` field (triggers system tray instead)
3. App is in Doze mode
4. Data payload format is wrong

**Solutions:**

1. Verify the request has `sub_type: "calls"`:
```json
{
  "data": {
    "sub_type": "calls"
  }
}
```

2. Check that the message is data-only (no top-level `notification` field). The push proxy should handle this automatically.

3. Check Android app manifest:
```xml
<service
    android:name=".CallMessagingService"
    android:exported="false">
    <intent-filter>
        <action android:name="com.google.firebase.MESSAGING_EVENT" />
    </intent-filter>
</service>
```

4. Add logging to your `FirebaseMessagingService`:
```java
@Override
public void onMessageReceived(RemoteMessage remoteMessage) {
    Log.d("FCM", "Received message: " + remoteMessage.getData());
    // ... handle message
}
```

### Issue: Android shows notification but doesn't ring

**Symptoms:**
- Notification appears in system tray
- No full-screen incoming call UI
- No ringing sound

**Causes:**
1. Message includes `notification` field (causes system tray)
2. App not handling data message properly
3. Full-screen intent not configured

**Solutions:**

1. Ensure message is data-only (the latest push-proxy code does this automatically)

2. In your Android app, handle the data message and show full-screen intent:
```java
@Override
public void onMessageReceived(RemoteMessage remoteMessage) {
    Map<String, String> data = remoteMessage.getData();
    if ("call".equals(data.get("type"))) {
        // Show full-screen incoming call UI
        Intent intent = new Intent(this, IncomingCallActivity.class);
        intent.setFlags(Intent.FLAG_ACTIVITY_NEW_TASK | 
                       Intent.FLAG_ACTIVITY_CLEAR_TOP |
                       Intent.FLAG_ACTIVITY_SHOW_WHEN_LOCKED |
                       Intent.FLAG_ACTIVITY_TURN_SCREEN_ON);
        startActivity(intent);
    }
}
```

3. Add required permissions to AndroidManifest.xml:
```xml
<uses-permission android:name="android.permission.WAKE_LOCK" />
<uses-permission android:name="android.permission.USE_FULL_SCREEN_INTENT" />
```

## Debug Logging

Enable verbose logging:

```bash
# In docker-compose.yml or .env
RUST_LOG=push_proxy=debug,tower_http=debug
```

Then restart and check logs:
```bash
docker-compose restart push-proxy
docker-compose logs push-proxy -f
```

You should see logs like:
```
Received push notification request platform=android is_call=true
Routing to Android/FCM handler
FCM client is available, building payload
Building FCM message is_call=true
FCM message built, sending to FCM API
OAuth token obtained, building FCM message
Received FCM API response status=200
Successfully sent notification to FCM
```

## Testing Without Mobile App

You can test the push proxy without a mobile app:

```bash
# Start push proxy with test configuration
docker run -e RUST_LOG=debug -p 3000:3000 rustchat-push-proxy

# Send test request
curl -X POST http://localhost:3000/send \
  -H "Content-Type: application/json" \
  -d '{"token":"test","title":"Test","body":"Test","platform":"android","type":"message","data":{"channel_id":"test","post_id":"test","type":"message"}}'
```

## Getting Help

If you're still having issues:

1. Collect the following information:
   - Push proxy logs (`docker-compose logs push-proxy`)
   - Output of test script (`./scripts/test_push_notification.sh YOUR_TOKEN android`)
   - Your environment variables (redact sensitive values)

2. Verify your setup:
   - Firebase project ID and key file
   - Device token format
   - Android app manifest configuration
   - Push proxy version (check `docker images`)

3. Check FCM diagnostics:
   - Go to Firebase Console → Cloud Messaging
   - Check for any reported errors
   - Verify message delivery statistics
