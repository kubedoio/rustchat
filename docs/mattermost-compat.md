# Mattermost Compatibility

RustChat implements a subset of the Mattermost API v4 to support mobile clients (Mattermost Mobile for Android/iOS).

## Compatibility Version
The server reports version `9.5.0` to clients.

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

### Sidebar Categories
- `GET /api/v4/users/{user_id}/teams/{team_id}/channels/categories`: Get sidebar categories for a team.
- `POST /api/v4/users/{user_id}/teams/{team_id}/channels/categories`: Create a sidebar category.
- `PUT /api/v4/users/{user_id}/teams/{team_id}/channels/categories`: Bulk update sidebar categories.
- `PUT /api/v4/users/{user_id}/teams/{team_id}/channels/categories/order`: Update category ordering.

### Posts
- `POST /api/v4/posts`: Create a new post.
- `GET /api/v4/posts/{post_id}`: Get a specific post.
- `PUT /api/v4/posts/{post_id}/patch`: Edit a post.
- `DELETE /api/v4/posts/{post_id}`: Delete a post.
- `GET /api/v4/posts/{post_id}/thread`: Get post thread.

### Reactions
- `POST /api/v4/reactions`: Add a reaction.
- `DELETE /api/v4/users/me/posts/{post_id}/reactions/{emoji_name}`: Remove a reaction.
- `GET /api/v4/posts/{post_id}/reactions`: Get reactions for a post.

### WebSocket
- `/api/v4/websocket`: WebSocket connection for real-time events.

## Sidebar Categories Curl
```bash
curl -s -H "Authorization: Bearer $TOKEN" \
  "$BASE_URL/api/v4/users/$USER_ID/teams/$TEAM_ID/channels/categories" | jq
```

## Architecture
All `/api/v4/*` requests are routed to the Rust backend. The frontend (Nginx) acts as a reverse proxy but does not serve these requests directly (no SPA fallback).
Responses from `/api/v4/` include the `X-MM-COMPAT: 1` header.

## Unimplemented Endpoints
Unimplemented endpoints return HTTP 501 Not Implemented with a JSON error body.
