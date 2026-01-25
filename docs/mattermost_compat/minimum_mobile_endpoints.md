# Minimum Mobile Endpoints

This checklist tracks the implementation status of endpoints required for the Mattermost mobile app.

## 1. Authentication & User

- [x] `POST /api/v4/users/login`
- [x] `GET /api/v4/users/me`
- [x] `GET /api/v4/users/me/teams/unread`

## 2. Configuration & License

- [x] `GET /api/v4/config/client`
- [x] `GET /api/v4/license/client`

## 3. Teams & Channels

- [x] `GET /api/v4/users/me/teams`
- [x] `GET /api/v4/users/me/teams/members`
- [x] `GET /api/v4/users/me/teams/{team_id}/channels`
- [x] `GET /api/v4/users/me/teams/{team_id}/channels/members`

## 4. Posts (Messages)

- [x] `GET /api/v4/channels/{channel_id}/posts`
- [x] `POST /api/v4/posts`

## 5. WebSocket

- [x] `/api/v4/websocket` connection and auth challenge.
- [x] `posted` event.
- [x] `typing` event.
