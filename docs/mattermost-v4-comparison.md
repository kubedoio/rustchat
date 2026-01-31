# Mattermost API v4 Comparison

This compares RustChat v4 routes implemented in code against the upstream Mattermost v4 OpenAPI path list.

## Summary

- Mattermost v4 endpoints (OpenAPI): 418
- Implemented in RustChat (code routes): 170
- Missing in RustChat: 277
- RustChat-only endpoints (not in OpenAPI list): 29

## Status (Mattermost OpenAPI paths)

| Endpoint | Status | RustChat Source |
| --- | --- | --- |
| `/api/v4/users/login` | Implemented | users.rs |
| `/api/v4/users/login/cws` | Implemented | users.rs |
| `/api/v4/users/login/sso/code-exchange` | Implemented | users.rs |
| `/api/v4/users/logout` | Implemented | users.rs |
| `/api/v4/users` | Implemented | users.rs |
| `/api/v4/users/ids` | Implemented | users.rs |
| `/api/v4/users/group_channels` | Implemented | users.rs |
| `/api/v4/users/usernames` | Implemented | users.rs |
| `/api/v4/users/search` | Implemented | users.rs |
| `/api/v4/users/autocomplete` | Implemented | users.rs |
| `/api/v4/users/known` | Implemented | users.rs |
| `/api/v4/users/stats` | Implemented | users.rs |
| `/api/v4/users/stats/filtered` | Implemented | users.rs |
| `/api/v4/users/{user_id}` | Implemented | users.rs |
| `/api/v4/users/{user_id}/patch` | Implemented | users.rs |
| `/api/v4/users/{user_id}/roles` | Implemented | users.rs |
| `/api/v4/users/{user_id}/active` | Implemented | users.rs |
| `/api/v4/users/{user_id}/image` | Implemented | users.rs |
| `/api/v4/users/{user_id}/image/default` | Implemented | users.rs |
| `/api/v4/users/username/{username}` | Implemented | users.rs |
| `/api/v4/users/password/reset` | Implemented | users.rs |
| `/api/v4/users/{user_id}/mfa` | Implemented | users.rs |
| `/api/v4/users/{user_id}/mfa/generate` | Implemented | users.rs |
| `/api/v4/users/{user_id}/demote` | Implemented | users.rs |
| `/api/v4/users/{user_id}/promote` | Implemented | users.rs |
| `/api/v4/users/{user_id}/convert_to_bot` | Implemented | users.rs |
| `/api/v4/users/mfa` | Implemented | users.rs |
| `/api/v4/users/{user_id}/password` | Implemented | users.rs |
| `/api/v4/users/password/reset/send` | Implemented | users.rs |
| `/api/v4/users/email/{email}` | Implemented | users.rs |
| `/api/v4/users/{user_id}/sessions` | Implemented | users.rs |
| `/api/v4/users/{user_id}/sessions/revoke` | Implemented | users.rs |
| `/api/v4/users/{user_id}/sessions/revoke/all` | Implemented | users.rs |
| `/api/v4/users/sessions/device` | Implemented | users.rs |
| `/api/v4/users/{user_id}/audits` | Implemented | users.rs |
| `/api/v4/users/{user_id}/email/verify/member` | Implemented | users.rs |
| `/api/v4/users/email/verify` | Implemented | users.rs |
| `/api/v4/users/email/verify/send` | Implemented | users.rs |
| `/api/v4/users/login/switch` | Implemented | users.rs |
| `/api/v4/users/login/type` | Implemented | users.rs |
| `/api/v4/users/{user_id}/tokens` | Implemented | users.rs |
| `/api/v4/users/tokens` | Implemented | users.rs |
| `/api/v4/users/tokens/revoke` | Implemented | users.rs |
| `/api/v4/users/tokens/{token_id}` | Implemented | users.rs |
| `/api/v4/users/tokens/disable` | Implemented | users.rs |
| `/api/v4/users/tokens/enable` | Implemented | users.rs |
| `/api/v4/users/tokens/search` | Implemented | users.rs |
| `/api/v4/users/{user_id}/auth` | Implemented | users.rs |
| `/api/v4/users/{user_id}/terms_of_service` | Implemented | users.rs |
| `/api/v4/users/sessions/revoke/all` | Implemented | users.rs |
| `/api/v4/users/{user_id}/typing` | Implemented | users.rs |
| `/api/v4/users/{user_id}/uploads` | Implemented | users.rs |
| `/api/v4/users/{user_id}/channel_members` | Implemented | users.rs |
| `/api/v4/users/migrate_auth/ldap` | Implemented | users.rs |
| `/api/v4/users/migrate_auth/saml` | Implemented | users.rs |
| `/api/v4/users/{user_id}/teams/{team_id}/threads` | Implemented | threads.rs |
| `/api/v4/users/{user_id}/teams/{team_id}/threads/mention_counts` | Implemented | threads.rs |
| `/api/v4/users/{user_id}/teams/{team_id}/threads/read` | Implemented | threads.rs |
| `/api/v4/users/{user_id}/teams/{team_id}/threads/{thread_id}/read/{timestamp}` | Implemented | threads.rs |
| `/api/v4/users/{user_id}/teams/{team_id}/threads/{thread_id}/set_unread/{post_id}` | Implemented | threads.rs |
| `/api/v4/users/{user_id}/teams/{team_id}/threads/{thread_id}/following` | Implemented | threads.rs |
| `/api/v4/users/{user_id}/teams/{team_id}/threads/{thread_id}` | Implemented | threads.rs |
| `/api/v4/users/{user_id}/data_retention/team_policies` | Not implemented | - |
| `/api/v4/users/{user_id}/data_retention/channel_policies` | Not implemented | - |
| `/api/v4/users/invalid_emails` | Implemented | users.rs |
| `/api/v4/users/{user_id}/reset_failed_attempts` | Implemented | users.rs |
| `/api/v4/users/{user_id}/status` | Implemented | users.rs |
| `/api/v4/users/status/ids` | Implemented | users.rs |
| `/api/v4/users/{user_id}/status/custom` | Implemented | users.rs |
| `/api/v4/users/{user_id}/status/custom/recent` | Implemented | users.rs |
| `/api/v4/users/{user_id}/status/custom/recent/delete` | Implemented | users.rs |
| `/api/v4/teams` | Implemented | teams.rs |
| `/api/v4/teams/{team_id}` | Implemented | teams.rs |
| `/api/v4/teams/{team_id}/patch` | Not implemented | - |
| `/api/v4/teams/{team_id}/privacy` | Not implemented | - |
| `/api/v4/teams/{team_id}/restore` | Not implemented | - |
| `/api/v4/teams/name/{name}` | Not implemented | - |
| `/api/v4/teams/search` | Implemented | teams.rs |
| `/api/v4/teams/name/{name}/exists` | Not implemented | - |
| `/api/v4/users/{user_id}/teams` | Implemented | users.rs |
| `/api/v4/teams/{team_id}/members` | Not implemented | - |
| `/api/v4/teams/members/invite` | Not implemented | - |
| `/api/v4/teams/{team_id}/members/batch` | Not implemented | - |
| `/api/v4/users/{user_id}/teams/members` | Not implemented | - |
| `/api/v4/teams/{team_id}/members/{user_id}` | Not implemented | - |
| `/api/v4/teams/{team_id}/members/ids` | Not implemented | - |
| `/api/v4/teams/{team_id}/stats` | Not implemented | - |
| `/api/v4/teams/{team_id}/regenerate_invite_id` | Not implemented | - |
| `/api/v4/teams/{team_id}/image` | Implemented | teams.rs |
| `/api/v4/teams/{team_id}/members/{user_id}/roles` | Not implemented | - |
| `/api/v4/teams/{team_id}/members/{user_id}/schemeRoles` | Not implemented | - |
| `/api/v4/users/{user_id}/teams/unread` | Not implemented | - |
| `/api/v4/users/{user_id}/teams/{team_id}/unread` | Not implemented | - |
| `/api/v4/teams/{team_id}/invite/email` | Not implemented | - |
| `/api/v4/teams/{team_id}/invite-guests/email` | Not implemented | - |
| `/api/v4/teams/invites/email` | Not implemented | - |
| `/api/v4/teams/{team_id}/import` | Not implemented | - |
| `/api/v4/teams/invite/{invite_id}` | Not implemented | - |
| `/api/v4/teams/{team_id}/scheme` | Not implemented | - |
| `/api/v4/teams/{team_id}/members_minus_group_members` | Not implemented | - |
| `/api/v4/channels` | Implemented | channels.rs |
| `/api/v4/channels/direct` | Implemented | channels.rs |
| `/api/v4/channels/group` | Implemented | channels.rs |
| `/api/v4/channels/search` | Implemented | channels.rs |
| `/api/v4/channels/group/search` | Not implemented | - |
| `/api/v4/teams/{team_id}/channels/ids` | Not implemented | - |
| `/api/v4/channels/{channel_id}/timezones` | Implemented | channels.rs |
| `/api/v4/channels/{channel_id}` | Implemented | channels.rs |
| `/api/v4/channels/{channel_id}/patch` | Not implemented | - |
| `/api/v4/channels/{channel_id}/privacy` | Not implemented | - |
| `/api/v4/channels/{channel_id}/restore` | Not implemented | - |
| `/api/v4/channels/{channel_id}/move` | Not implemented | - |
| `/api/v4/channels/{channel_id}/stats` | Implemented | channels.rs |
| `/api/v4/channels/{channel_id}/pinned` | Implemented | channels.rs |
| `/api/v4/teams/{team_id}/channels` | Implemented | teams.rs |
| `/api/v4/teams/{team_id}/channels/private` | Not implemented | - |
| `/api/v4/teams/{team_id}/channels/deleted` | Not implemented | - |
| `/api/v4/teams/{team_id}/channels/autocomplete` | Not implemented | - |
| `/api/v4/teams/{team_id}/channels/search_autocomplete` | Not implemented | - |
| `/api/v4/teams/{team_id}/channels/search` | Implemented | teams.rs |
| `/api/v4/teams/{team_id}/channels/name/{channel_name}` | Not implemented | - |
| `/api/v4/teams/name/{team_name}/channels/name/{channel_name}` | Not implemented | - |
| `/api/v4/channels/{channel_id}/members` | Implemented | channels.rs |
| `/api/v4/channels/{channel_id}/members/ids` | Implemented | channels.rs |
| `/api/v4/channels/{channel_id}/members/{user_id}` | Implemented | channels.rs |
| `/api/v4/channels/{channel_id}/members/{user_id}/roles` | Implemented | channels.rs |
| `/api/v4/channels/{channel_id}/members/{user_id}/schemeRoles` | Not implemented | - |
| `/api/v4/channels/{channel_id}/members/{user_id}/notify_props` | Implemented | channels.rs |
| `/api/v4/channels/members/{user_id}/view` | Not implemented | - |
| `/api/v4/users/{user_id}/teams/{team_id}/channels/members` | Not implemented | - |
| `/api/v4/users/{user_id}/teams/{team_id}/channels` | Implemented | users.rs |
| `/api/v4/users/{user_id}/channels` | Implemented | users.rs |
| `/api/v4/users/{user_id}/channels/{channel_id}/unread` | Not implemented | - |
| `/api/v4/channels/{channel_id}/scheme` | Not implemented | - |
| `/api/v4/channels/{channel_id}/members_minus_group_members` | Not implemented | - |
| `/api/v4/channels/{channel_id}/member_counts_by_group` | Not implemented | - |
| `/api/v4/channels/{channel_id}/moderations` | Not implemented | - |
| `/api/v4/channels/{channel_id}/moderations/patch` | Not implemented | - |
| `/api/v4/users/{user_id}/teams/{team_id}/channels/categories` | Implemented | categories.rs |
| `/api/v4/users/{user_id}/teams/{team_id}/channels/categories/order` | Implemented | categories.rs |
| `/api/v4/users/{user_id}/teams/{team_id}/channels/categories/{category_id}` | Not implemented | - |
| `/api/v4/channels/{channel_id}/common_teams` | Not implemented | - |
| `/api/v4/posts` | Implemented | posts.rs |
| `/api/v4/posts/ephemeral` | Implemented | posts.rs |
| `/api/v4/posts/{post_id}` | Implemented | posts.rs |
| `/api/v4/users/{user_id}/posts/{post_id}/set_unread` | Implemented | posts.rs |
| `/api/v4/posts/{post_id}/patch` | Implemented | posts.rs |
| `/api/v4/posts/{post_id}/thread` | Implemented | posts.rs |
| `/api/v4/users/{user_id}/posts/flagged` | Implemented | posts.rs |
| `/api/v4/posts/{post_id}/files/info` | Implemented | posts.rs |
| `/api/v4/channels/{channel_id}/posts` | Implemented | channels.rs |
| `/api/v4/users/{user_id}/channels/{channel_id}/posts/unread` | Not implemented | - |
| `/api/v4/teams/{team_id}/posts/search` | Not implemented | - |
| `/api/v4/posts/{post_id}/pin` | Not implemented | - |
| `/api/v4/posts/{post_id}/unpin` | Not implemented | - |
| `/api/v4/posts/{post_id}/actions/{action_id}` | Not implemented | - |
| `/api/v4/posts/ids` | Not implemented | - |
| `/api/v4/users/{user_id}/posts/{post_id}/reminder` | Implemented | posts.rs |
| `/api/v4/users/{user_id}/posts/{post_id}/ack` | Not implemented | - |
| `/api/v4/posts/{post_id}/move` | Not implemented | - |
| `/api/v4/posts/{post_id}/restore/{restore_version_id}` | Not implemented | - |
| `/api/v4/posts/{post_id}/reveal` | Not implemented | - |
| `/api/v4/posts/{post_id}/burn` | Not implemented | - |
| `/api/v4/posts/rewrite` | Not implemented | - |
| `/api/v4/users/{user_id}/preferences` | Implemented | users.rs |
| `/api/v4/users/{user_id}/preferences/delete` | Implemented | users.rs |
| `/api/v4/users/{user_id}/preferences/{category}` | Implemented | users.rs |
| `/api/v4/users/{user_id}/preferences/{category}/name/{preference_name}` | Implemented | users.rs |
| `/api/v4/files` | Implemented | files.rs |
| `/api/v4/files/{file_id}` | Implemented | files.rs |
| `/api/v4/files/{file_id}/thumbnail` | Implemented | files.rs |
| `/api/v4/files/{file_id}/preview` | Implemented | files.rs |
| `/api/v4/files/{file_id}/link` | Implemented | files.rs |
| `/api/v4/files/{file_id}/info` | Implemented | files.rs |
| `/api/v4/teams/{team_id}/files/search` | Not implemented | - |
| `/api/v4/files/search` | Not implemented | - |
| `/api/v4/recaps` | Not implemented | - |
| `/api/v4/recaps/{recap_id}` | Not implemented | - |
| `/api/v4/recaps/{recap_id}/read` | Not implemented | - |
| `/api/v4/recaps/{recap_id}/regenerate` | Not implemented | - |
| `/api/v4/ai/agents` | Not implemented | - |
| `/api/v4/ai/services` | Not implemented | - |
| `/api/v4/uploads` | Not implemented | - |
| `/api/v4/uploads/{upload_id}` | Not implemented | - |
| `/api/v4/jobs` | Not implemented | - |
| `/api/v4/jobs/{job_id}` | Not implemented | - |
| `/api/v4/jobs/{job_id}/download` | Not implemented | - |
| `/api/v4/jobs/{job_id}/cancel` | Not implemented | - |
| `/api/v4/jobs/type/{type}` | Not implemented | - |
| `/api/v4/jobs/{job_id}/status` | Not implemented | - |
| `/api/v4/system/timezones` | Not implemented | - |
| `/api/v4/system/ping` | Implemented | system.rs |
| `/api/v4/system/notices/{teamId}` | Not implemented | - |
| `/api/v4/system/notices/view` | Not implemented | - |
| `/api/v4/database/recycle` | Implemented | system.rs |
| `/api/v4/email/test` | Not implemented | - |
| `/api/v4/notifications/test` | Not implemented | - |
| `/api/v4/site_url/test` | Not implemented | - |
| `/api/v4/file/s3_test` | Not implemented | - |
| `/api/v4/config` | Not implemented | - |
| `/api/v4/config/reload` | Not implemented | - |
| `/api/v4/config/client` | Implemented | config_client.rs |
| `/api/v4/config/environment` | Not implemented | - |
| `/api/v4/config/patch` | Not implemented | - |
| `/api/v4/license` | Not implemented | - |
| `/api/v4/license/client` | Implemented | config_client.rs |
| `/api/v4/license/load_metric` | Not implemented | - |
| `/api/v4/license/renewal` | Not implemented | - |
| `/api/v4/trial-license` | Not implemented | - |
| `/api/v4/trial-license/prev` | Not implemented | - |
| `/api/v4/audits` | Implemented | admin.rs |
| `/api/v4/caches/invalidate` | Implemented | system.rs |
| `/api/v4/logs` | Implemented | system.rs |
| `/api/v4/analytics/old` | Not implemented | - |
| `/api/v4/server_busy` | Not implemented | - |
| `/api/v4/notifications/ack` | Not implemented | - |
| `/api/v4/redirect_location` | Not implemented | - |
| `/api/v4/image` | Not implemented | - |
| `/api/v4/upgrade_to_enterprise` | Not implemented | - |
| `/api/v4/upgrade_to_enterprise/status` | Not implemented | - |
| `/api/v4/upgrade_to_enterprise/allowed` | Not implemented | - |
| `/api/v4/restart` | Not implemented | - |
| `/api/v4/integrity` | Not implemented | - |
| `/api/v4/system/support_packet` | Not implemented | - |
| `/api/v4/emoji` | Implemented | emoji.rs |
| `/api/v4/emoji/{emoji_id}` | Implemented | emoji.rs |
| `/api/v4/emoji/name/{emoji_name}` | Not implemented | - |
| `/api/v4/emoji/{emoji_id}/image` | Not implemented | - |
| `/api/v4/emoji/search` | Implemented | emoji.rs |
| `/api/v4/emoji/autocomplete` | Implemented | emoji.rs |
| `/api/v4/emoji/names` | Not implemented | - |
| `/api/v4/hooks/incoming` | Implemented | hooks.rs |
| `/api/v4/hooks/incoming/{hook_id}` | Not implemented | - |
| `/api/v4/hooks/outgoing` | Implemented | hooks.rs |
| `/api/v4/hooks/outgoing/{hook_id}` | Not implemented | - |
| `/api/v4/hooks/outgoing/{hook_id}/regen_token` | Not implemented | - |
| `/api/v4/saml/metadata` | Not implemented | - |
| `/api/v4/saml/metadatafromidp` | Not implemented | - |
| `/api/v4/saml/certificate/idp` | Not implemented | - |
| `/api/v4/saml/certificate/public` | Not implemented | - |
| `/api/v4/saml/certificate/private` | Not implemented | - |
| `/api/v4/saml/certificate/status` | Not implemented | - |
| `/api/v4/saml/reset_auth_data` | Not implemented | - |
| `/api/v4/compliance/reports` | Not implemented | - |
| `/api/v4/compliance/reports/{report_id}` | Not implemented | - |
| `/api/v4/compliance/reports/{report_id}/download` | Not implemented | - |
| `/api/v4/ldap/sync` | Not implemented | - |
| `/api/v4/ldap/test` | Not implemented | - |
| `/api/v4/ldap/test_connection` | Not implemented | - |
| `/api/v4/ldap/test_diagnostics` | Not implemented | - |
| `/api/v4/ldap/groups` | Not implemented | - |
| `/api/v4/ldap/groups/{remote_id}/link` | Not implemented | - |
| `/api/v4/ldap/migrateid` | Not implemented | - |
| `/api/v4/ldap/certificate/public` | Not implemented | - |
| `/api/v4/ldap/certificate/private` | Not implemented | - |
| `/api/v4/ldap/users/{user_id}/group_sync_memberships` | Not implemented | - |
| `/api/v4/groups` | Not implemented | - |
| `/api/v4/groups/{group_id}` | Not implemented | - |
| `/api/v4/groups/{group_id}/patch` | Not implemented | - |
| `/api/v4/groups/{group_id}/restore` | Not implemented | - |
| `/api/v4/groups/{group_id}/teams/{team_id}/link` | Not implemented | - |
| `/api/v4/groups/{group_id}/channels/{channel_id}/link` | Not implemented | - |
| `/api/v4/groups/{group_id}/teams/{team_id}` | Not implemented | - |
| `/api/v4/groups/{group_id}/channels/{channel_id}` | Not implemented | - |
| `/api/v4/groups/{group_id}/teams` | Not implemented | - |
| `/api/v4/groups/{group_id}/channels` | Not implemented | - |
| `/api/v4/groups/{group_id}/teams/{team_id}/patch` | Not implemented | - |
| `/api/v4/groups/{group_id}/channels/{channel_id}/patch` | Not implemented | - |
| `/api/v4/groups/{group_id}/members` | Not implemented | - |
| `/api/v4/groups/{group_id}/stats` | Not implemented | - |
| `/api/v4/channels/{channel_id}/groups` | Not implemented | - |
| `/api/v4/teams/{team_id}/groups` | Not implemented | - |
| `/api/v4/teams/{team_id}/groups_by_channels` | Not implemented | - |
| `/api/v4/users/{user_id}/groups` | Not implemented | - |
| `/api/v4/groups/names` | Not implemented | - |
| `/api/v4/cluster/status` | Not implemented | - |
| `/api/v4/brand/image` | Not implemented | - |
| `/api/v4/commands` | Implemented | commands.rs |
| `/api/v4/teams/{team_id}/commands/autocomplete` | Not implemented | - |
| `/api/v4/teams/{team_id}/commands/autocomplete_suggestions` | Implemented | commands.rs |
| `/api/v4/commands/{command_id}` | Not implemented | - |
| `/api/v4/commands/{command_id}/move` | Not implemented | - |
| `/api/v4/commands/{command_id}/regen_token` | Not implemented | - |
| `/api/v4/commands/execute` | Implemented | commands.rs |
| `/api/v4/oauth/apps` | Not implemented | - |
| `/api/v4/oauth/apps/{app_id}` | Not implemented | - |
| `/api/v4/oauth/apps/{app_id}/regen_secret` | Not implemented | - |
| `/api/v4/oauth/apps/{app_id}/info` | Not implemented | - |
| `/api/v4/oauth/apps/register` | Not implemented | - |
| `/api/v4/users/{user_id}/oauth/apps/authorized` | Not implemented | - |
| `/api/v4/elasticsearch/test` | Not implemented | - |
| `/api/v4/elasticsearch/purge_indexes` | Not implemented | - |
| `/api/v4/bleve/purge_indexes` | Not implemented | - |
| `/api/v4/data_retention/policy` | Not implemented | - |
| `/api/v4/data_retention/policies_count` | Not implemented | - |
| `/api/v4/data_retention/policies` | Not implemented | - |
| `/api/v4/data_retention/policies/{policy_id}` | Not implemented | - |
| `/api/v4/data_retention/policies/{policy_id}/teams` | Not implemented | - |
| `/api/v4/data_retention/policies/{policy_id}/teams/search` | Not implemented | - |
| `/api/v4/data_retention/policies/{policy_id}/channels` | Not implemented | - |
| `/api/v4/data_retention/policies/{policy_id}/channels/search` | Not implemented | - |
| `/api/v4/plugins` | Not implemented | - |
| `/api/v4/plugins/install_from_url` | Not implemented | - |
| `/api/v4/plugins/{plugin_id}` | Not implemented | - |
| `/api/v4/plugins/{plugin_id}/enable` | Not implemented | - |
| `/api/v4/plugins/{plugin_id}/disable` | Not implemented | - |
| `/api/v4/plugins/webapp` | Implemented | plugins.rs |
| `/api/v4/plugins/statuses` | Implemented | plugins.rs |
| `/api/v4/plugins/marketplace` | Not implemented | - |
| `/api/v4/plugins/marketplace/first_admin_visit` | Not implemented | - |
| `/api/v4/roles` | Not implemented | - |
| `/api/v4/roles/{role_id}` | Not implemented | - |
| `/api/v4/roles/name/{role_name}` | Not implemented | - |
| `/api/v4/roles/{role_id}/patch` | Not implemented | - |
| `/api/v4/roles/names` | Implemented | users.rs |
| `/api/v4/schemes` | Not implemented | - |
| `/api/v4/schemes/{scheme_id}` | Not implemented | - |
| `/api/v4/schemes/{scheme_id}/patch` | Not implemented | - |
| `/api/v4/schemes/{scheme_id}/teams` | Not implemented | - |
| `/api/v4/schemes/{scheme_id}/channels` | Not implemented | - |
| `/api/v4/terms_of_service` | Not implemented | - |
| `/api/v4/remotecluster` | Not implemented | - |
| `/api/v4/remotecluster/{remote_id}` | Not implemented | - |
| `/api/v4/remotecluster/{remote_id}/generate_invite` | Not implemented | - |
| `/api/v4/remotecluster/accept_invite` | Not implemented | - |
| `/api/v4/sharedchannels/{team_id}` | Not implemented | - |
| `/api/v4/remotecluster/{remote_id}/sharedchannelremotes` | Not implemented | - |
| `/api/v4/sharedchannels/remote_info/{remote_id}` | Not implemented | - |
| `/api/v4/remotecluster/{remote_id}/channels/{channel_id}/invite` | Not implemented | - |
| `/api/v4/remotecluster/{remote_id}/channels/{channel_id}/uninvite` | Not implemented | - |
| `/api/v4/sharedchannels/{channel_id}/remotes` | Not implemented | - |
| `/api/v4/sharedchannels/users/{user_id}/can_dm/{other_user_id}` | Not implemented | - |
| `/api/v4/reactions` | Implemented | posts.rs |
| `/api/v4/posts/{post_id}/reactions` | Implemented | posts.rs |
| `/api/v4/users/{user_id}/posts/{post_id}/reactions/{emoji_name}` | Implemented | posts.rs |
| `/api/v4/posts/ids/reactions` | Not implemented | - |
| `/api/v4/actions/dialogs/open` | Not implemented | - |
| `/api/v4/actions/dialogs/submit` | Not implemented | - |
| `/api/v4/actions/dialogs/lookup` | Not implemented | - |
| `/api/v4/bots` | Implemented | bots.rs |
| `/api/v4/bots/{bot_user_id}` | Not implemented | - |
| `/api/v4/bots/{bot_user_id}/disable` | Not implemented | - |
| `/api/v4/bots/{bot_user_id}/enable` | Not implemented | - |
| `/api/v4/bots/{bot_user_id}/assign/{user_id}` | Not implemented | - |
| `/api/v4/bots/{bot_user_id}/icon` | Not implemented | - |
| `/api/v4/bots/{bot_user_id}/convert_to_user` | Not implemented | - |
| `/api/v4/cloud/limits` | Not implemented | - |
| `/api/v4/cloud/products` | Not implemented | - |
| `/api/v4/cloud/payment` | Not implemented | - |
| `/api/v4/cloud/payment/confirm` | Not implemented | - |
| `/api/v4/cloud/customer` | Not implemented | - |
| `/api/v4/cloud/customer/address` | Not implemented | - |
| `/api/v4/cloud/subscription` | Not implemented | - |
| `/api/v4/cloud/installation` | Not implemented | - |
| `/api/v4/cloud/subscription/invoices` | Not implemented | - |
| `/api/v4/cloud/subscription/invoices/{invoice_id}/pdf` | Not implemented | - |
| `/api/v4/cloud/webhook` | Not implemented | - |
| `/api/v4/cloud/preview/modal_data` | Not implemented | - |
| `/api/v4/usage/posts` | Not implemented | - |
| `/api/v4/usage/storage` | Not implemented | - |
| `/api/v4/permissions/ancillary` | Not implemented | - |
| `/api/v4/imports` | Not implemented | - |
| `/api/v4/imports/{import_name}` | Not implemented | - |
| `/api/v4/exports` | Not implemented | - |
| `/api/v4/exports/{export_name}` | Not implemented | - |
| `/api/v4/ip_filtering` | Not implemented | - |
| `/api/v4/ip_filtering/my_ip` | Not implemented | - |
| `/api/v4/channels/{channel_id}/bookmarks` | Not implemented | - |
| `/api/v4/channels/{channel_id}/bookmarks/{bookmark_id}` | Not implemented | - |
| `/api/v4/channels/{channel_id}/bookmarks/{bookmark_id}/sort_order` | Not implemented | - |
| `/api/v4/reports/users` | Not implemented | - |
| `/api/v4/reports/users/count` | Not implemented | - |
| `/api/v4/reports/users/export` | Not implemented | - |
| `/api/v4/reports/posts` | Not implemented | - |
| `/api/v4/limits/server` | Not implemented | - |
| `/api/v4/logs/download` | Not implemented | - |
| `/api/v4/oauth/outgoing_connections` | Not implemented | - |
| `/api/v4/oauth/outgoing_connections/{connection_id}` | Not implemented | - |
| `/api/v4/oauth/outgoing_connections/validate` | Not implemented | - |
| `/api/v4/client_perf` | Implemented | system.rs |
| `/api/v4/posts/schedule` | Implemented | posts.rs |
| `/api/v4/posts/scheduled/team/{team_id}` | Implemented | posts.rs |
| `/api/v4/posts/schedule/{scheduled_post_id}` | Not implemented | - |
| `/api/v4/custom_profile_attributes/fields` | Implemented | users.rs |
| `/api/v4/custom_profile_attributes/fields/{field_id}` | Not implemented | - |
| `/api/v4/custom_profile_attributes/values` | Not implemented | - |
| `/api/v4/custom_profile_attributes/group` | Not implemented | - |
| `/api/v4/users/{user_id}/custom_profile_attributes` | Implemented | users.rs |
| `/api/v4/audit_logs/certificate` | Not implemented | - |
| `/api/v4/access_control_policies` | Not implemented | - |
| `/api/v4/access_control_policies/cel/check` | Not implemented | - |
| `/api/v4/access_control_policies/cel/validate_requester` | Not implemented | - |
| `/api/v4/access_control_policies/cel/test` | Not implemented | - |
| `/api/v4/access_control_policies/search` | Not implemented | - |
| `/api/v4/access_control_policies/cel/autocomplete/fields` | Not implemented | - |
| `/api/v4/access_control_policies/{policy_id}` | Not implemented | - |
| `/api/v4/access_control_policies/{policy_id}/activate` | Not implemented | - |
| `/api/v4/access_control_policies/{policy_id}/assign` | Not implemented | - |
| `/api/v4/access_control_policies/{policy_id}/unassign` | Not implemented | - |
| `/api/v4/access_control_policies/{policy_id}/resources/channels` | Not implemented | - |
| `/api/v4/access_control_policies/{policy_id}/resources/channels/search` | Not implemented | - |
| `/api/v4/channels/{channel_id}/access_control/attributes` | Not implemented | - |
| `/api/v4/access_control_policies/cel/visual_ast` | Not implemented | - |
| `/api/v4/access_control_policies/activate` | Not implemented | - |
| `/api/v4/content_flagging/flag/config` | Not implemented | - |
| `/api/v4/content_flagging/team/{team_id}/status` | Not implemented | - |
| `/api/v4/content_flagging/post/{post_id}/flag` | Not implemented | - |
| `/api/v4/content_flagging/fields` | Not implemented | - |
| `/api/v4/content_flagging/post/{post_id}/field_values` | Not implemented | - |
| `/api/v4/content_flagging/post/{post_id}` | Not implemented | - |
| `/api/v4/content_flagging/post/{post_id}/remove` | Not implemented | - |
| `/api/v4/content_flagging/post/{post_id}/keep` | Not implemented | - |
| `/api/v4/content_flagging/config` | Not implemented | - |
| `/api/v4/content_flagging/team/{team_id}/reviewers/search` | Not implemented | - |
| `/api/v4/content_flagging/post/{post_id}/assign/{content_reviewer_id}` | Not implemented | - |
| `/api/v4/agents` | Not implemented | - |
| `/api/v4/agents/status` | Not implemented | - |
| `/api/v4/llmservices` | Not implemented | - |

## RustChat-only implemented endpoints

| Endpoint | Notes | RustChat Source |
| --- | --- | --- |
| `/api/v4/channels/members/me/view` | Not found in OpenAPI path list | channels.rs |
| `/api/v4/channels/{channel_id}/members/me` | Not found in OpenAPI path list | channels.rs |
| `/api/v4/channels/{channel_id}/posts/{post_id}/pin` | Not found in OpenAPI path list | channels.rs |
| `/api/v4/channels/{channel_id}/posts/{post_id}/unpin` | Not found in OpenAPI path list | channels.rs |
| `/api/v4/channels/{channel_id}/unread` | Not found in OpenAPI path list | channels.rs |
| `/api/v4/emoji/name/{name}` | Not found in OpenAPI path list | emoji.rs |
| `/api/v4/posts/{post_id}/ack` | Not found in OpenAPI path list | posts.rs |
| `/api/v4/system/version` | Not found in OpenAPI path list | system.rs |
| `/api/v4/teams/{team_id}/members/me` | Not found in OpenAPI path list | teams.rs |
| `/api/v4/users/me` | Not found in OpenAPI path list | users.rs |
| `/api/v4/users/me/channels` | Not found in OpenAPI path list | users.rs |
| `/api/v4/users/me/channels/categories` | Not found in OpenAPI path list | users.rs |
| `/api/v4/users/me/patch` | Not found in OpenAPI path list | users.rs |
| `/api/v4/users/me/posts/{post_id}/reactions/{emoji_name}` | Not found in OpenAPI path list | posts.rs |
| `/api/v4/users/me/preferences` | Not found in OpenAPI path list | users.rs |
| `/api/v4/users/me/sessions` | Not found in OpenAPI path list | users.rs |
| `/api/v4/users/me/status` | Not found in OpenAPI path list | users.rs |
| `/api/v4/users/me/teams` | Not found in OpenAPI path list | users.rs |
| `/api/v4/users/me/teams/members` | Not found in OpenAPI path list | users.rs |
| `/api/v4/users/me/teams/unread` | Not found in OpenAPI path list | users.rs |
| `/api/v4/users/me/teams/{team_id}/channels` | Not found in OpenAPI path list | users.rs |
| `/api/v4/users/me/teams/{team_id}/channels/members` | Not found in OpenAPI path list | users.rs |
| `/api/v4/users/me/teams/{team_id}/channels/not_members` | Not found in OpenAPI path list | users.rs |
| `/api/v4/users/notifications` | Not found in OpenAPI path list | users.rs |
| `/api/v4/users/{user_id}/channels/{channel_id}/typing` | Not found in OpenAPI path list | users.rs |
| `/api/v4/users/{user_id}/sidebar/categories` | Not found in OpenAPI path list | users.rs |
| `/api/v4/users/{user_id}/sidebar/categories/order` | Not found in OpenAPI path list | users.rs |
| `/api/v4/users/{user_id}/threads` | Not found in OpenAPI path list | threads.rs |
| `/api/v4/websocket` | Not found in OpenAPI path list | mod.rs |
