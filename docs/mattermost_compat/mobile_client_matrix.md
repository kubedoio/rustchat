# Mattermost Mobile Client Compatibility Matrix

This document tracks the implementation status of Mattermost API v4 methods used by the official mobile client.

| Client4 Method | HTTP | Path | RustChat Mapping | Status |
|---|---|---|---|---|
| login | POST | /api/v4/users/login | `users::login` | Implemented |
| getMe | GET | /api/v4/users/me | `users::me` | Implemented |
| getMyTeams | GET | /api/v4/users/me/teams | `users::my_teams` | Implemented |
| getMyTeamMembers | GET | /api/v4/users/me/teams/members | `users::my_team_members` | Implemented |
| getMyTeamChannels | GET | /api/v4/users/me/teams/{team_id}/channels | `users::my_team_channels` | Implemented |
| getMyChannelMembers | GET | /api/v4/users/me/teams/{team_id}/channels/members | `users::my_team_channel_members` | Implemented |
| getClientConfig | GET | /api/v4/config/client | `config::client_config` | Implemented |
| getClientLicense | GET | /api/v4/license/client | `config::client_license` | Implemented |
| getPosts | GET | /api/v4/channels/{id}/posts | `channels::get_posts` | Implemented |
| createPost | POST | /api/v4/posts | `posts::create_post` | Implemented |
| attachDevice | POST | /api/v4/users/sessions/device | `users::attach_device` | TODO |
| detachDevice | DELETE | /api/v4/users/sessions/device | `users::detach_device` | TODO |
| getPreferences | GET | /api/v4/users/me/preferences | `users::get_preferences` | TODO |
| updatePreferences | PUT | /api/v4/users/me/preferences | `users::update_preferences` | TODO |
| getStatusesByIds | POST | /api/v4/users/status/ids | `users::get_statuses_by_ids` | TODO |
| getStatus | GET | /api/v4/users/{id}/status | `users::get_status` | TODO |
| updateStatus | PUT | /api/v4/users/me/status | `users::update_status` | TODO |
| getTeam | GET | /api/v4/teams/{id} | `teams::get_team` | TODO |
| getChannel | GET | /api/v4/channels/{id} | `channels::get_channel` | TODO |
| getChannelMembers | GET | /api/v4/channels/{id}/members | `channels::get_channel_members` | TODO |
| getPost | GET | /api/v4/posts/{id} | `posts::get_post` | TODO |
| deletePost | DELETE | /api/v4/posts/{id} | `posts::delete_post` | TODO |
| patchPost | PUT | /api/v4/posts/{id}/patch | `posts::patch_post` | TODO |
| addReaction | POST | /api/v4/reactions | `posts::add_reaction` | TODO |
| getReactions | GET | /api/v4/posts/{id}/reactions | `posts::get_reactions` | TODO |
| removeReaction | DELETE | /api/v4/users/me/posts/{post_id}/reactions/{emoji} | `posts::remove_reaction` | TODO |
| uploadFile | POST | /api/v4/files | `files::upload_file` | TODO |
| getFile | GET | /api/v4/files/{id} | `files::get_file` | TODO |
| getFileThumbnail | GET | /api/v4/files/{id}/thumbnail | `files::get_thumbnail` | TODO |
| systemPing | GET | /api/v4/system/ping | `system::ping` | Implemented |
