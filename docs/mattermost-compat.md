# Mattermost Compatibility

RustChat implements a subset of the Mattermost API v4 to support mobile clients (Mattermost Mobile for Android/iOS).

## Compatibility Version
The server reports version `10.11.10` to clients.

## Supported Endpoints

### System & Handshake
- `GET /api/v4/system/ping`: Returns system status and version.
- `GET /api/v4/system/version`: Returns the server version string.
- `GET /api/v4/config/client`: Returns client configuration.
- `GET /api/v4/license/client`: Returns license information.

### Authentication & Users
- `POST /api/v4/users/login`: Login with username/email and password.
- `GET /api/v4/users/me`: Get current user info.
- `GET /api/v4/users/me/teams`: Get user's teams.
- `GET /api/v4/users/me/channels`: Get user's channels.
- `GET /api/v4/users/status/ids`: Get status for list of users.
- `GET /api/v4/users/{user_id}/status`: Get status for a user.
- `PUT /api/v4/users/me/status`: Update current user status.

### Teams & Channels
- `GET /api/v4/teams/{team_id}/channels`: Get channels for a team.
- `GET /api/v4/channels/{channel_id}`: Get channel details.
- `GET /api/v4/channels/{channel_id}/members`: Get channel members.
- `GET /api/v4/channels/{channel_id}/posts`: Get posts in a channel (with pagination).
- `POST /api/v4/channels/direct`: Create direct message channel.
- `POST /api/v4/channels/group`: Create group message channel.
- `POST /api/v4/channels/search`: Search channels.
- `POST /api/v4/teams/search`: Search teams.

### Sidebar Categories
- `GET /api/v4/users/me/channels/categories`: Get sidebar categories for the current user.
- `GET /api/v4/users/{user_id}/teams/{team_id}/channels/categories`: Get sidebar categories for a team.

### Posts
- `POST /api/v4/posts`: Create a new post.
- `GET /api/v4/posts/{post_id}`: Get a specific post.
- `GET /api/v4/channels/{channel_id}/posts`: Fetch post list for a channel.

### Threads
- `GET /api/v4/users/{user_id}/threads`: Get user's followed threads.
- `GET /api/v4/users/{user_id}/teams/{team_id}/threads`: Get team-scoped threads.
- `POST /api/v4/users/{user_id}/teams/{team_id}/threads/{thread_id}/following`: Follow a thread.

### Files
- `GET /api/v4/files/{file_id}/info`: Get file metadata.
- `GET /api/v4/files/{file_id}`: Stream file content (via S3 redirect).

### WebSocket
- `/api/v4/websocket`: WebSocket connection for real-time events.
  - Supported events: `posted`, `typing`, `post_edited`, `post_deleted`, `reaction_added`, `status_change`.

## Architecture
All `/api/v4/*` requests are routed to the Rust backend. The frontend (Nginx) acts as a reverse proxy but does not serve these requests directly (no SPA fallback).
Responses from `/api/v4/` include the `X-MM-COMPAT: 1` header.

## Unimplemented Endpoints
Unimplemented endpoints return HTTP 501 Not Implemented with a JSON error body.
