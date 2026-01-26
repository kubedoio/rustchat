# Mattermost Compatibility Layer

RustChat includes a compatibility layer for the Mattermost API v4, allowing Mattermost clients (Mobile/Desktop) to connect to a RustChat server.

## Supported Features

- **Authentication**: Login using RustChat credentials.
- **Teams**: Listing teams (workspaces).
- **Channels**: Listing channels, joining channels.
- **Messaging**: Viewing history, posting messages.
- **WebSocket**: Real-time events for new posts, reactions, and typing.

## Configuration

The compatibility layer uses a hardcoded version `10.11.0` to ensure compatibility with modern Mattermost clients.

## Endpoints

### System
- `GET /api/v4/system/ping`: Server status.
- `GET /api/v4/system/version`: Server version (compat).
- `GET /api/v4/config/client`: Client configuration.

### Authentication
- `POST /api/v4/users/login`: Login with username/email and password.
- `GET /api/v4/users/me`: Get current user.

### Teams & Channels
- `GET /api/v4/teams`: List teams.
- `GET /api/v4/users/me/teams`: List user's teams.
- `GET /api/v4/teams/{team_id}/channels`: List channels in a team.
- `GET /api/v4/users/me/teams/{team_id}/channels`: List user's channels in a team.
- `GET /api/v4/users/me/channels`: List user's channels across teams.
- `GET /api/v4/channels/{channel_id}`: Get channel details.
- `GET /api/v4/channels/{channel_id}/posts`: Get posts (supports pagination).

### Posts
- `POST /api/v4/posts`: Create a post.
- `GET /api/v4/posts/{post_id}`: Get a post.
- `PUT /api/v4/posts/{post_id}`: Edit a post.
- `DELETE /api/v4/posts/{post_id}`: Delete a post.

### WebSocket
- `GET /api/v4/websocket`: WebSocket connection.

## Implementation Details

- **Auth Token**: Uses RustChat's JWT token. Accepted via `Authorization: Bearer <token>` or `Token: <token>` header.
- **IDs**: Uses UUIDs (RustChat standard) which are compatible with Mattermost's 26-char ID requirement (as strings).
- **Versioning**: Reports version `10.11.0` to satisfy mobile client checks.

## Limitations

- Not all Mattermost features are supported (e.g., extensive search, plugins, deeply nested threads usage in some clients).
- Push notifications are currently disabled in config.
