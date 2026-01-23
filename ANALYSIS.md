# Rustchat vs Mattermost Analysis

## Overview

Rustchat is a high-performance team collaboration platform built in Rust. It aims to provide a self-hosted alternative to Slack/Mattermost. This analysis compares the current state of Rustchat with Mattermost (specifically the open-source Team Edition) to identify missing features and areas for improvement.

## Feature Comparison

| Feature Category | Mattermost (Team Edition) | Rustchat (Current) | Status |
|------------------|---------------------------|--------------------|--------|
| **Core Messaging** | Channels, DMs, Threads, Emoji Reactions, Mentions, Formatting | Implemented (Channels, DMs, Threads, Reactions, Mentions) | ✅ Parity |
| **Real-time** | WebSocket-based events, Typing indicators, Presence | Implemented (WebSocket, Typing, Basic Presence) | ✅ Parity |
| **File Sharing** | S3, Local storage, Image previews, File search | Implemented (S3-compatible, Previews) | ✅ Parity |
| **Playbooks** | Integrated Incident Collaboration, Checklists, Retrospectives | Implemented (Playbooks, Checklists, Tasks, Runs) | ✅ Parity (Surprisingly complete) |
| **Calls/Audio/Video**| WebRTC-based voice/video calls, Screen sharing (via Plugin) | **State only**. `calls.rs` exists but only tracks session state. No media transport. | ❌ **Missing Media Layer** |
| **Integrations** | Incoming/Outgoing Webhooks, Slash Commands, Interactive Messages, Plugins | Basic Webhooks & Slash Command *Definition*. **No Execution Logic**. | ⚠️ Partial |
| **User Management** | System Admin, Team Admin, Member, Guest, Custom Roles (Enterprise) | System Admin, Org Admin, Member. Basic RBAC. | ⚠️ Basic |
| **User Status** | Online/Away/DND/Offline + **Custom Status** (Emoji + Text) | Online/Offline (Automatic). **No Custom Status**. | ❌ **Missing Custom Status** |
| **Search** | Full-text search (PostgreSQL/Elasticsearch), Date filters, User filters | Basic SQL `ILIKE` search. | ⚠️ Basic |
| **Localization** | Multi-language support (15+ languages) | English only (Hardcoded strings). | ❌ **Missing i18n** |
| **Client Apps** | Web, Desktop (Electron), Mobile (React Native) | Web (Vue 3). No Mobile/Desktop apps. | ❌ **Web Only** |

## Missing Parts & Recommendations

### 1. Slash Command Execution
**Current State:** The backend (`integrations.rs`) allows creating and storing Slash Command definitions (trigger, URL, etc.), but there is no mechanism to actually *execute* them. The `create_post` service does not parse slash commands.
**Recommendation:** Implement an execution endpoint or middleware that intercepts messages starting with `/`. If it matches a defined command, call the registered URL. Also support built-in commands like `/echo`, `/invite`, etc.

### 2. Custom User Status
**Current State:** Users have an automatic `presence` (online/offline) tracked by WebSocket connections.
**Recommendation:** Add a `custom_status` field (JSON) to the User model to store a custom emoji and text (e.g., "🍔 Out for lunch"). Add endpoints to set/clear this status.

### 3. Media Server for Calls
**Current State:** The `calls` module only tracks who is in a "call" database-wise.
**Recommendation:** Integrate a WebRTC signaling server or use a media server like Janus or LiveKit. This is a significant undertaking. For now, the "Calls" feature is incomplete.

### 4. Internationalization (i18n)
**Current State:** All UI strings are hardcoded in English.
**Recommendation:** Set up `vue-i18n` on the frontend and extract strings into JSON locale files.

### 5. Advanced Search
**Current State:** Simple SQL `ILIKE` query.
**Recommendation:** For "Enterprise-ready" performance with large message histories, integrate full-text search (PostgreSQL `tsvector` or Elasticsearch).

## proposed Updates for this Session

1.  **Backend - Slash Command Execution:** Implement the logic to execute slash commands.
2.  **Backend - Custom User Status:** Add data model and API to support custom statuses.
