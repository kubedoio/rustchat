# Mattermost Compatibility Layer

This module implements a subset of the Mattermost API v4 to support the official Mattermost mobile applications (Android/iOS) with the RustChat backend.

## Supported Features

### REST API Endpoints

- **Authentication**
  - `POST /api/v4/users/login`: Login with username/email and password. Returns a session token.
  - `GET /api/v4/users/me`: Retrieve current user details.

- **Teams**
  - `GET /api/v4/users/me/teams`: List teams the user belongs to.
  - `GET /api/v4/users/me/teams/members`: List team memberships.

- **Channels**
  - `GET /api/v4/users/me/teams/{team_id}/channels`: List channels in a team.
  - `GET /api/v4/users/me/teams/{team_id}/channels/members`: List channel memberships.
  - `GET /api/v4/channels/{channel_id}/posts`: Retrieve posts for a channel.

- **Posts**
  - `POST /api/v4/posts`: Create a new post (message).

- **Configuration**
  - `GET /api/v4/config/client`: Minimal client configuration to support mobile app startup.
  - `GET /api/v4/license/client`: Returns an OSS license status.

### WebSocket

- Endpoint: `/api/v4/websocket`
- Supports the `authentication_challenge` flow.
- Broadcasts the following events in Mattermost JSON format:
  - `posted`: New messages.
  - `typing`: Typing indicators.

## Configuration

To use the Mattermost mobile app with RustChat:

1. Download the Mattermost app from the App Store or Play Store.
2. Enter your RustChat server URL (e.g., `https://chat.yourdomain.com`).
3. Log in with your RustChat credentials.

## Implementation Details

- **Mappers**: `backend/src/mattermost_compat/mappers.rs` converts internal RustChat models to Mattermost DTOs.
- **API Handlers**: `backend/src/api/v4/` contains the endpoint logic.
- **Authentication**: Supports both `Authorization: Bearer <token>` and `Token: <token>` headers.
