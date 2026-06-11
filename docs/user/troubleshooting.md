# Troubleshooting

Common issues and solutions.

## Login Issues

### Can't Log In
- Double-check your email and password
- Ensure caps lock is off
- If using SSO, verify you're using the correct login method
- Try the **Forgot Password** link

### Account Locked
Contact your system administrator if your account is locked.

## Message Issues

### Messages Not Loading
1. Check your internet connection
2. Refresh the page (F5 or Ctrl+R)
3. Clear browser cache and reload

### Messages Not Sending
- Check connection status indicator
- Wait and try again
- Refresh if the issue persists

## AI Agent Issues

### Agent Not Responding
- Make sure you mentioned the exact agent username, for example `@support-agent`
- Confirm the agent is listed as a member of the channel
- Wait a few seconds; responses may be delayed while the model or knowledge base is queried
- Try a shorter question if the channel has a long history
- Contact your administrator if the agent still does not respond

### Agent Response Missing Expected Knowledge
- The document may not be in a knowledge base assigned to that agent
- Recently uploaded or synced documents may still be indexing
- Ask the administrator to check knowledge base sync and indexing status

### Agent Response Looks Wrong
- Use the feedback controls on the agent message if available
- Reply with clarifying context
- Contact your administrator if the issue involves sensitive or incorrect policy information

## Notification Issues

### Not Receiving Notifications
1. Check notification permissions in browser settings
2. Verify notification settings in RustChat
3. Ensure "Do Not Disturb" mode is off
4. Check system notification settings

### Desktop Notifications Not Working
- Grant notification permission when prompted
- Check OS notification settings
- Ensure RustChat is allowed to send notifications

## Connection Issues

### WebSocket Disconnected
- Check internet connection
- Refresh the page
- Contact admin if issue persists

### Slow Performance
- Close unused browser tabs
- Check internet connection speed
- Try a different browser

## File Upload Issues

### Upload Fails
- Check file size (max size set by admin)
- Ensure file type is allowed
- Try again or contact admin

### Can't Preview Files
- Some file types don't have previews
- Download the file to view

## Mobile App Issues

### App Won't Connect
- Check server URL is correct
- Verify internet connection
- Try logging out and back in

### Push Notifications Not Working
- Enable push notifications in app settings
- Check device notification settings
- Verify push proxy is configured (admin)

## Still Having Issues?

Contact your system administrator with:
- What you were trying to do
- What happened instead
- Any error messages
- Browser and OS version

---

*For administrators: See [Operations Guide](../operations/README.md) for server-side troubleshooting.*
