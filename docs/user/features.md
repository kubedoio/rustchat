# Feature Guide

Complete guide to RustChat features.

## Table of Contents

- [Messaging](#messaging)
- [Channels](#channels)
- [AI Agents](#ai-agents)
- [Search](#search)
- [Notifications](#notifications)
- [Settings](#settings)

---

## Messaging

### Sending Messages
- Type in the composer at the bottom of a channel
- Press **Enter** to send
- Press **Shift+Enter** for a new line

### Editing Messages
1. Hover over your message
2. Click the **...** menu
3. Select **Edit**

### Deleting Messages
1. Hover over your message
2. Click the **...** menu
3. Select **Delete**

### Markdown Formatting

| Style | Syntax | Example |
|-------|--------|---------|
| Bold | `**text**` | **bold** |
| Italic | `*text*` | *italic* |
| Code | `` `code` `` | `code` |
| Code block | ```` ``` ```` | Multi-line code |
| Quote | `> text` | Blockquote |
| Link | `[text](url)` | [Link](https://example.com) |

### Mentions
Use mentions to get someone's attention:
- `@username` - Notify a specific user
- `@channel` - Notify everyone in the current channel
- `@all` - Notify everyone on the team (use sparingly)

### Threads
Keep conversations organized:
1. Hover over a message
2. Click the **Reply** icon
3. Thread replies appear in the Right Sidebar

### Emoji Reactions
React to messages quickly:
1. Hover over a message
2. Click **Add Reaction**
3. Select an emoji

### File Uploads
Share files:
- **Drag and drop** files into the message pane
- Click the **+** icon in the composer
- Images and PDFs show previews

---

## Channels

### Public Channels
- Open to anyone on the team
- Marked with **#**
- Good for broad discussions

### Private Channels
- Invitation only
- Marked with **🔒**
- For sensitive topics

### Creating Channels
1. Click **+** next to "Channels"
2. Choose **Public** or **Private**
3. Enter a name
4. Add members (optional)

### Direct Messages (DMs)
1:1 conversations:
1. Click **+** next to "Direct Messages"
2. Search for a teammate
3. Start chatting

### Group Messages
Group DMs with multiple people:
1. Click **+** next to "Direct Messages"
2. Select multiple people
3. Start the conversation

---

## AI Agents

AI agents are assistant accounts your administrator can add to channels. They appear like other channel members and can answer messages when mentioned.

### Asking an Agent

Use the agent's username in a message:

```text
@support-agent summarize the deployment steps from the runbook
```

If an administrator enabled always-on responses for a channel, the agent may respond without a mention.

### Agent Responses

Agent replies appear in the channel history as regular messages from the agent account. Depending on the agent's configuration, responses may use:
- recent channel context
- saved agent memory
- assigned knowledge bases
- enabled tools such as web search

### Feedback

When feedback controls are available on an agent response, mark whether the answer was helpful. Feedback helps administrators review prompt quality, knowledge base coverage, and agent behavior.

---

## Search

### Basic Search
Type keywords in the search bar at the top.

### Search Filters

| Filter | Syntax | Description |
|--------|--------|-------------|
| From user | `from:@username` | Messages from specific person |
| In channel | `in:#channel` | Messages in specific channel |
| With file | `has:file` | Messages with attachments |
| Before date | `before:2024-01-01` | Messages before date |
| After date | `after:2024-01-01` | Messages after date |

### Jump to Results
Click any search result to jump to that point in history.

### Pinned Messages
Important messages can be pinned:
- Click the **Pin** icon in the channel header to view pinned messages

---

## Notifications

### Desktop Notifications
Enable in **Settings > Notifications**:
- Browser notifications
- Desktop app notifications

### Email Notifications
Configure when to receive emails:
- Mentions and DMs
- Daily/weekly digests

### Channel Muting
Mute noisy channels:
1. Click the channel header
2. Select **Mute Channel**

### Notification Preferences
Customize in **Settings > Notifications**:
- What triggers notifications
- Sound settings
- Email frequency

---

## Settings

### Profile Settings
Update in **Settings > Profile**:
- Display name
- Avatar
- Bio

### Appearance
Customize in **Settings > Display**:
- Light/Dark theme
- Font size
- Message density

### Language & Timezone
Set in **Settings > Display**:
- Language preference
- Local timezone

### Keyboard Shortcuts
View all shortcuts with **Ctrl+/** (or **Cmd+/** on Mac)

Common shortcuts:

| Action | Shortcut |
|--------|----------|
| New message | `Ctrl+N` |
| Search | `Ctrl+K` |
| Previous channel | `Alt+Up` |
| Next channel | `Alt+Down` |
| Settings | `Ctrl+,` |

---

*For troubleshooting: See [Troubleshooting](./troubleshooting.md)*
