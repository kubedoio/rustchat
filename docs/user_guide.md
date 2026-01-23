# RustChat User Guide

Welcome to RustChat! This guide will help you get started with our high-performance, self-hosted collaboration platform. Whether you're part of a small team or a large enterprise, RustChat provides all the tools you need for efficient real-time communication.

---

## 1. Introduction

### What is RustChat?
RustChat is a modern, enterprise-ready messaging and collaboration platform. It offers a secure, self-hosted alternative to traditional chat applications, ensuring your data remains under your control. Built with Rust for maximum performance and reliability, RustChat provides a familiar, feature-rich experience for teams of all sizes.

### Who is it for?
- **Small Teams & Startups:** Easily coordinate projects and keep everyone in sync.
- **Large Enterprises:** Securely manage thousands of users with advanced administrative controls.
- **DevOps & IT Teams:** Integrate with your favorite tools via webhooks and slash commands.
- **Remote & Hybrid Orgs:** Bridge the gap between distributed teams with real-time presence and collaboration features.

### Supported Platforms
RustChat is accessible through any modern web browser. For a more integrated experience, a desktop wrapper is available for macOS, Windows, and Linux. Stay tuned for our mobile apps (iOS and Android), currently on our development roadmap!

---

## 2. Getting Started

### Sign In & Log In
To get started, navigate to your team's RustChat URL.
1. **Join a Team:** If you've received an invitation, follow the link to create your account.
2. **Log In:** Enter your email address and password, or use your organization's Single Sign-On (SSO) provider (like Google or Octa) if enabled.

### Understanding the UI Layout
The RustChat interface is divided into several clear sections:
- **Left Sidebar:** Access your teams, navigate between channels, and find direct messages.
- **Channel List:** Shows all the public and private channels you've joined.
- **Message Pane:** Where the conversation happens. Send messages, view history, and interact with teammates.
- **Right Sidebar (RHS):** Appears when you open a thread, view user profiles, or browse pinned messages.
- **Status Bar:** Located at the top, it shows your current team, search bar, and user settings.

### Status Indicators
Let your teammates know what you're up to:
- 🟢 **Online:** You're active and ready to chat.
- 🟡 **Away:** You haven't been active for a while.
- 🔴 **Do Not Disturb:** You're busy; notifications are silenced.
- ⚪ **Offline:** You're not currently logged in.

### Basic Navigation
Switching between teams is easy—just click the team icons on the far left. To switch channels, simply select one from the sidebar. You can use search to quickly find a specific person or a topic.

---

## 3. Channels & Direct Messages

### Public vs. Private Channels
- **Public Channels (#):** Open to anyone on the team. Great for broad topics like company announcements or general discussion.
- **Private Channels (🔒):** Restricted to invited members. Use these for sensitive projects or private team coordination.

### Joining & Leaving Channels
- **Browse Channels:** Click the "+" icon next to the Channels header to see all public channels available to join.
- **Leave a Channel:** Open the channel header menu and select "Leave Channel."

### Creating Channels
Click the "+" icon next to the "Channels" section in the sidebar. Choose a name, set it to public or private, and invite your teammates.

### Direct Messages (DMs)
For 1:1 conversations, use Direct Messages. Click the "+" next to "Direct Messages" to find a teammate and start chatting.

### Group Messages
You can also start a group DM with multiple people. Just add more names when creating a new direct message.

---

## 4. Messaging & Collaboration

### Sending Messages
Type your message in the composer at the bottom of a channel and press Enter.

### Editing & Deleting Messages
Made a typo? Hover over your message, click the "..." menu, and select "Edit." To remove a message entirely, choose "Delete."

### Markdown Formatting
RustChat supports standard Markdown for rich text:
- **Bold:** `**text**`
- *Italic:* `*text*`
- List: `- Item 1`
- Inline Code: `` `code` ``
- Code Blocks:
  ```
  // Code here
  ```
- Quotes: `> This is a quote`
- Links: `[RustChat](https://rustchat.com)`
- Headings: `# Heading 1`, `## Heading 2`

### Mentions
Get someone's attention by typing `@` followed by their username. You can also use:
- `@channel`: Notifies everyone in the current channel.
- `@all`: Notifies everyone on the team (use sparingly!).

### Threads
Keep conversations organized. Hover over a message and click the "Reply" icon to start a thread. Thread replies are neatly tucked away and can be viewed in the Right Sidebar.

### Emoji Reactions
React to any message with an emoji! Hover over a message and click the "Add Reaction" icon. Click an existing reaction to add yourself to it.

### Typing Indicators
Know when a teammate is writing back. You'll see "User is typing..." at the bottom of the message pane.

### File Uploads
Share documents, images, and more.
- **Drag-and-Drop:** Simply drag a file into the message pane.
- **Upload Button:** Click the "+" icon in the message composer.
- **Previews:** Images and PDFs will show a preview directly in the chat.

---

## 5. Search & History

### Searching Messages & Files
Use the search bar at the top to find anything across all your channels. You can search by keywords or use filters.

### Filtering Results
Narrow down your search:
- `from:@username`: Messages from a specific person.
- `in:#channel`: Messages within a specific channel.
- `has:file`: Only messages with attachments.

### Jumping to Results
Click on a search result to jump directly to that point in the message history.

### Browsing Pinned Messages
Important messages can be "pinned" to a channel. Access them by clicking the "Pin" icon in the channel header.

---

## 6. Notifications

### Desktop Notifications
RustChat can send you push notifications on your desktop. Go to Settings > Notifications to enable them.

### Email Notifications
If you're away, RustChat can send you summaries or alerts for mentions and DMs via email.

### Per-Channel Muting
Is a channel too noisy? Click the channel header and select "Mute Channel" to stop receiving notifications for it.

### Notification Preferences
Customize how and when you get notified via User Settings > Notifications.

---

## 7. User Preferences & Settings

### Profile Info
Update your avatar, display name, and bio in User Settings > Profile.

### Language & Timezone
Set your preferred language and your local timezone for accurate message timestamps.

### Theme
Choose between Light and Dark mode, or have RustChat automatically follow your system settings.

### Keyboard Shortcuts
Boost your productivity with shortcuts. Type `Ctrl+/` (or `Cmd+/` on Mac) to see the full list.

---

## 8. Tips & Best Practices

- **Use Threads:** Keep main channels clean by replying in threads for detailed discussions.
- **Mention Sparingly:** Avoid using `@all` or `@channel` unless it's truly urgent.
- **Organize Your Sidebar:** Move your most-used channels to the top for quick access.
- **Clear Statuses:** Use your status to let others know when you're in deep work or out of the office.

---

## 9. Troubleshooting

### Can’t Log In?
Double-check your credentials. If your org uses SSO, make sure you're using the correct login method. Use the "Forgot Password" link if needed.

### Missing Notifications?
Ensure you've granted notification permissions to your browser or the desktop app. Check if you have "Do Not Disturb" mode enabled.

### Messages Not Updating?
If the chat feels "stuck," try refreshing your browser or restarting the desktop app.

### Desktop Notifications Not Working?
Check your operating system settings to ensure RustChat is allowed to send notifications.
