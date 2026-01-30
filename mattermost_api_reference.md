# Mattermost API v4 Reference

Total endpoints found: 527

| Method | Path | Summary | Source |
| :--- | :--- | :--- | :--- |
| PUT | `/api/v4/access_control_policies` | Create an access control policy | `access_control.yaml` |
| POST | `/api/v4/access_control_policies/cel/check` | Check an access control policy expression | `access_control.yaml` |
| POST | `/api/v4/access_control_policies/cel/validate_requester` | Validate if the current user matches a CEL expression | `access_control.yaml` |
| POST | `/api/v4/access_control_policies/cel/test` | Test an access control policy expression | `access_control.yaml` |
| POST | `/api/v4/access_control_policies/search` | Search access control policies | `access_control.yaml` |
| GET | `/api/v4/access_control_policies/cel/autocomplete/fields` | Get autocomplete fields for access control policies | `access_control.yaml` |
| GET | `/api/v4/access_control_policies/{policy_id}` | Get an access control policy | `access_control.yaml` |
| DELETE | `/api/v4/access_control_policies/{policy_id}` | Delete an access control policy | `access_control.yaml` |
| GET | `/api/v4/access_control_policies/{policy_id}/activate` | Activate or deactivate an access control policy | `access_control.yaml` |
| POST | `/api/v4/access_control_policies/{policy_id}/assign` | Assign an access control policy to channels | `access_control.yaml` |
| DELETE | `/api/v4/access_control_policies/{policy_id}/unassign` | Unassign an access control policy from channels | `access_control.yaml` |
| GET | `/api/v4/access_control_policies/{policy_id}/resources/channels` | Get channels for an access control policy | `access_control.yaml` |
| POST | `/api/v4/access_control_policies/{policy_id}/resources/channels/search` | Search channels for an access control policy | `access_control.yaml` |
| GET | `/api/v4/channels/{channel_id}/access_control/attributes` | Get access control attributes for a channel | `access_control.yaml` |
| POST | `/api/v4/access_control_policies/cel/visual_ast` | Get the visual AST for a CEL expression | `access_control.yaml` |
| PUT | `/api/v4/access_control_policies/activate` | Activate or deactivate access control policies | `access_control.yaml` |
| POST | `/api/v4/actions/dialogs/open` | Open a dialog | `actions.yaml` |
| POST | `/api/v4/actions/dialogs/submit` | Submit a dialog | `actions.yaml` |
| POST | `/api/v4/actions/dialogs/lookup` | Lookup dialog elements | `actions.yaml` |
| GET | `/api/v4/agents` | Get available agents | `agents.yaml` |
| GET | `/api/v4/agents/status` | Get agents bridge status | `agents.yaml` |
| GET | `/api/v4/llmservices` | Get available LLM services | `agents.yaml` |
| GET | `/api/v4/ai/agents` | Get available AI agents | `ai.yaml` |
| GET | `/api/v4/ai/services` | Get available AI services | `ai.yaml` |
| POST | `/api/v4/audit_logs/certificate` | Upload audit log certificate | `audit_logging.yaml` |
| DELETE | `/api/v4/audit_logs/certificate` | Remove audit log certificate | `audit_logging.yaml` |
| POST | `/api/v4/bleve/purge_indexes` | Purge all Bleve indexes | `bleve.yaml` |
| GET | `/api/v4/channels/{channel_id}/bookmarks` | Get channel bookmarks for Channel | `bookmarks.yaml` |
| POST | `/api/v4/channels/{channel_id}/bookmarks` | Create channel bookmark | `bookmarks.yaml` |
| PATCH | `/api/v4/channels/{channel_id}/bookmarks/{bookmark_id}` | Update channel bookmark | `bookmarks.yaml` |
| DELETE | `/api/v4/channels/{channel_id}/bookmarks/{bookmark_id}` | Delete channel bookmark | `bookmarks.yaml` |
| POST | `/api/v4/channels/{channel_id}/bookmarks/{bookmark_id}/sort_order` | Update channel bookmark's order | `bookmarks.yaml` |
| POST | `/api/v4/bots` | Create a bot | `bots.yaml` |
| GET | `/api/v4/bots` | Get bots | `bots.yaml` |
| PUT | `/api/v4/bots/{bot_user_id}` | Patch a bot | `bots.yaml` |
| GET | `/api/v4/bots/{bot_user_id}` | Get a bot | `bots.yaml` |
| POST | `/api/v4/bots/{bot_user_id}/disable` | Disable a bot | `bots.yaml` |
| POST | `/api/v4/bots/{bot_user_id}/enable` | Enable a bot | `bots.yaml` |
| POST | `/api/v4/bots/{bot_user_id}/assign/{user_id}` | Assign a bot to a user | `bots.yaml` |
| GET | `/api/v4/bots/{bot_user_id}/icon` | Get bot's LHS icon | `bots.yaml` |
| POST | `/api/v4/bots/{bot_user_id}/icon` | Set bot's LHS icon image | `bots.yaml` |
| DELETE | `/api/v4/bots/{bot_user_id}/icon` | Delete bot's LHS icon image | `bots.yaml` |
| POST | `/api/v4/bots/{bot_user_id}/convert_to_user` | Convert a bot into a user | `bots.yaml` |
| GET | `/api/v4/brand/image` | Get brand image | `brand.yaml` |
| POST | `/api/v4/brand/image` | Upload brand image | `brand.yaml` |
| DELETE | `/api/v4/brand/image` | Delete current brand image | `brand.yaml` |
| GET | `/api/v4/channels` | Get a list of all channels | `channels.yaml` |
| POST | `/api/v4/channels` | Create a channel | `channels.yaml` |
| POST | `/api/v4/channels/direct` | Create a direct message channel | `channels.yaml` |
| POST | `/api/v4/channels/group` | Create a group message channel | `channels.yaml` |
| POST | `/api/v4/channels/search` | Search all private and open type channels across all teams | `channels.yaml` |
| POST | `/api/v4/channels/group/search` | Search Group Channels | `channels.yaml` |
| POST | `/api/v4/teams/{team_id}/channels/ids` | Get a list of channels by ids | `channels.yaml` |
| GET | `/api/v4/channels/{channel_id}/timezones` | Get timezones in a channel | `channels.yaml` |
| GET | `/api/v4/channels/{channel_id}` | Get a channel | `channels.yaml` |
| PUT | `/api/v4/channels/{channel_id}` | Update a channel | `channels.yaml` |
| DELETE | `/api/v4/channels/{channel_id}` | Delete a channel | `channels.yaml` |
| PUT | `/api/v4/channels/{channel_id}/patch` | Patch a channel | `channels.yaml` |
| PUT | `/api/v4/channels/{channel_id}/privacy` | Update channel's privacy | `channels.yaml` |
| POST | `/api/v4/channels/{channel_id}/restore` | Restore a channel | `channels.yaml` |
| POST | `/api/v4/channels/{channel_id}/move` | Move a channel | `channels.yaml` |
| GET | `/api/v4/channels/{channel_id}/stats` | Get channel statistics | `channels.yaml` |
| GET | `/api/v4/channels/{channel_id}/pinned` | Get a channel's pinned posts | `channels.yaml` |
| GET | `/api/v4/teams/{team_id}/channels` | Get public channels | `channels.yaml` |
| GET | `/api/v4/teams/{team_id}/channels/private` | Get private channels | `channels.yaml` |
| GET | `/api/v4/teams/{team_id}/channels/deleted` | Get deleted channels | `channels.yaml` |
| GET | `/api/v4/teams/{team_id}/channels/autocomplete` | Autocomplete channels | `channels.yaml` |
| GET | `/api/v4/teams/{team_id}/channels/search_autocomplete` | Autocomplete channels for search | `channels.yaml` |
| POST | `/api/v4/teams/{team_id}/channels/search` | Search channels | `channels.yaml` |
| GET | `/api/v4/teams/{team_id}/channels/name/{channel_name}` | Get a channel by name | `channels.yaml` |
| GET | `/api/v4/teams/name/{team_name}/channels/name/{channel_name}` | Get a channel by name and team name | `channels.yaml` |
| GET | `/api/v4/channels/{channel_id}/members` | Get channel members | `channels.yaml` |
| POST | `/api/v4/channels/{channel_id}/members` | Add user(s) to channel | `channels.yaml` |
| POST | `/api/v4/channels/{channel_id}/members/ids` | Get channel members by ids | `channels.yaml` |
| GET | `/api/v4/channels/{channel_id}/members/{user_id}` | Get channel member | `channels.yaml` |
| DELETE | `/api/v4/channels/{channel_id}/members/{user_id}` | Remove user from channel | `channels.yaml` |
| PUT | `/api/v4/channels/{channel_id}/members/{user_id}/roles` | Update channel roles | `channels.yaml` |
| PUT | `/api/v4/channels/{channel_id}/members/{user_id}/schemeRoles` | Update the scheme-derived roles of a channel member. | `channels.yaml` |
| PUT | `/api/v4/channels/{channel_id}/members/{user_id}/notify_props` | Update channel notifications | `channels.yaml` |
| POST | `/api/v4/channels/members/{user_id}/view` | View channel | `channels.yaml` |
| GET | `/api/v4/users/{user_id}/teams/{team_id}/channels/members` | Get channel memberships and roles for a user | `channels.yaml` |
| GET | `/api/v4/users/{user_id}/teams/{team_id}/channels` | Get channels for user | `channels.yaml` |
| GET | `/api/v4/users/{user_id}/channels` | Get all channels from all teams | `channels.yaml` |
| GET | `/api/v4/users/{user_id}/channels/{channel_id}/unread` | Get unread messages | `channels.yaml` |
| PUT | `/api/v4/channels/{channel_id}/scheme` | Set a channel's scheme | `channels.yaml` |
| GET | `/api/v4/channels/{channel_id}/members_minus_group_members` | Channel members minus group members. | `channels.yaml` |
| GET | `/api/v4/channels/{channel_id}/member_counts_by_group` | Channel members counts for each group that has atleast one member in the channel | `channels.yaml` |
| GET | `/api/v4/channels/{channel_id}/moderations` | Get information about channel's moderation. | `channels.yaml` |
| PUT | `/api/v4/channels/{channel_id}/moderations/patch` | Update a channel's moderation settings. | `channels.yaml` |
| GET | `/api/v4/users/{user_id}/teams/{team_id}/channels/categories` | Get user's sidebar categories | `channels.yaml` |
| POST | `/api/v4/users/{user_id}/teams/{team_id}/channels/categories` | Create user's sidebar category | `channels.yaml` |
| PUT | `/api/v4/users/{user_id}/teams/{team_id}/channels/categories` | Update user's sidebar categories | `channels.yaml` |
| GET | `/api/v4/users/{user_id}/teams/{team_id}/channels/categories/order` | Get user's sidebar category order | `channels.yaml` |
| PUT | `/api/v4/users/{user_id}/teams/{team_id}/channels/categories/order` | Update user's sidebar category order | `channels.yaml` |
| GET | `/api/v4/users/{user_id}/teams/{team_id}/channels/categories/{category_id}` | Get sidebar category | `channels.yaml` |
| PUT | `/api/v4/users/{user_id}/teams/{team_id}/channels/categories/{category_id}` | Update sidebar category | `channels.yaml` |
| DELETE | `/api/v4/users/{user_id}/teams/{team_id}/channels/categories/{category_id}` | Delete sidebar category | `channels.yaml` |
| GET | `/api/v4/sharedchannels/{channel_id}/remotes` | Get remote clusters for a shared channel | `channels.yaml` |
| GET | `/api/v4/channels/{channel_id}/common_teams` | Get common teams for members of a Group Message. | `channels.yaml` |
| GET | `/api/v4/cloud/limits` | Get cloud workspace limits | `cloud.yaml` |
| GET | `/api/v4/cloud/products` | Get cloud products | `cloud.yaml` |
| POST | `/api/v4/cloud/payment` | Create a customer setup payment intent | `cloud.yaml` |
| POST | `/api/v4/cloud/payment/confirm` | Completes the payment setup intent | `cloud.yaml` |
| GET | `/api/v4/cloud/customer` | Get cloud customer | `cloud.yaml` |
| PUT | `/api/v4/cloud/customer` | Update cloud customer | `cloud.yaml` |
| PUT | `/api/v4/cloud/customer/address` | Update cloud customer address | `cloud.yaml` |
| GET | `/api/v4/cloud/subscription` | Get cloud subscription | `cloud.yaml` |
| GET | `/api/v4/cloud/installation` | GET endpoint for Installation information | `cloud.yaml` |
| GET | `/api/v4/cloud/subscription/invoices` | Get cloud subscription invoices | `cloud.yaml` |
| GET | `/api/v4/cloud/subscription/invoices/{invoice_id}/pdf` | Get cloud invoice PDF | `cloud.yaml` |
| POST | `/api/v4/cloud/webhook` | POST endpoint for CWS Webhooks | `cloud.yaml` |
| GET | `/api/v4/cloud/preview/modal_data` | Get cloud preview modal data | `cloud.yaml` |
| GET | `/api/v4/cluster/status` | Get cluster status | `cluster.yaml` |
| POST | `/api/v4/commands` | Create a command | `commands.yaml` |
| GET | `/api/v4/commands` | List commands for a team | `commands.yaml` |
| GET | `/api/v4/teams/{team_id}/commands/autocomplete` | List autocomplete commands | `commands.yaml` |
| GET | `/api/v4/teams/{team_id}/commands/autocomplete_suggestions` | List commands' autocomplete data | `commands.yaml` |
| GET | `/api/v4/commands/{command_id}` | Get a command | `commands.yaml` |
| PUT | `/api/v4/commands/{command_id}` | Update a command | `commands.yaml` |
| DELETE | `/api/v4/commands/{command_id}` | Delete a command | `commands.yaml` |
| PUT | `/api/v4/commands/{command_id}/move` | Move a command | `commands.yaml` |
| PUT | `/api/v4/commands/{command_id}/regen_token` | Generate a new token | `commands.yaml` |
| POST | `/api/v4/commands/execute` | Execute a command | `commands.yaml` |
| POST | `/api/v4/compliance/reports` | Create report | `compliance.yaml` |
| GET | `/api/v4/compliance/reports` | Get reports | `compliance.yaml` |
| GET | `/api/v4/compliance/reports/{report_id}` | Get a report | `compliance.yaml` |
| GET | `/api/v4/compliance/reports/{report_id}/download` | Download a report | `compliance.yaml` |
| GET | `/api/v4/content_flagging/flag/config` | Get content flagging configuration | `content_flagging.yaml` |
| GET | `/api/v4/content_flagging/team/{team_id}/status` | Get content flagging status for a team | `content_flagging.yaml` |
| POST | `/api/v4/content_flagging/post/{post_id}/flag` | Flag a post | `content_flagging.yaml` |
| GET | `/api/v4/content_flagging/fields` | Get content flagging property fields | `content_flagging.yaml` |
| GET | `/api/v4/content_flagging/post/{post_id}/field_values` | Get content flagging property field values for a post | `content_flagging.yaml` |
| GET | `/api/v4/content_flagging/post/{post_id}` | Get a flagged post with all its content. | `content_flagging.yaml` |
| PUT | `/api/v4/content_flagging/post/{post_id}/remove` | Remove a flagged post | `content_flagging.yaml` |
| PUT | `/api/v4/content_flagging/post/{post_id}/keep` | Keep a flagged post | `content_flagging.yaml` |
| GET | `/api/v4/content_flagging/config` | Get the system content flagging configuration | `content_flagging.yaml` |
| PUT | `/api/v4/content_flagging/config` | Update the system content flagging configuration | `content_flagging.yaml` |
| GET | `/api/v4/content_flagging/team/{team_id}/reviewers/search` | Search content reviewers in a team | `content_flagging.yaml` |
| POST | `/api/v4/content_flagging/post/{post_id}/assign/{content_reviewer_id}` | Assign a content reviewer to a flagged post | `content_flagging.yaml` |
| GET | `/api/v4/custom_profile_attributes/fields` | List all the Custom Profile Attributes fields | `custom_profile_attributes.yaml` |
| POST | `/api/v4/custom_profile_attributes/fields` | Create a Custom Profile Attribute field | `custom_profile_attributes.yaml` |
| PATCH | `/api/v4/custom_profile_attributes/fields/{field_id}` | Patch a Custom Profile Attribute field | `custom_profile_attributes.yaml` |
| DELETE | `/api/v4/custom_profile_attributes/fields/{field_id}` | Delete a Custom Profile Attribute field | `custom_profile_attributes.yaml` |
| PATCH | `/api/v4/custom_profile_attributes/values` | Patch Custom Profile Attribute values | `custom_profile_attributes.yaml` |
| GET | `/api/v4/custom_profile_attributes/group` | Get Custom Profile Attribute property group data | `custom_profile_attributes.yaml` |
| GET | `/api/v4/users/{user_id}/custom_profile_attributes` | List Custom Profile Attribute values | `custom_profile_attributes.yaml` |
| PATCH | `/api/v4/users/{user_id}/custom_profile_attributes` | Update custom profile attribute values for a user | `custom_profile_attributes.yaml` |
| GET | `/api/v4/data_retention/policy` | Get the global data retention policy | `dataretention.yaml` |
| GET | `/api/v4/data_retention/policies_count` | Get the number of granular data retention policies | `dataretention.yaml` |
| GET | `/api/v4/data_retention/policies` | Get the granular data retention policies | `dataretention.yaml` |
| POST | `/api/v4/data_retention/policies` | Create a new granular data retention policy | `dataretention.yaml` |
| GET | `/api/v4/data_retention/policies/{policy_id}` | Get a granular data retention policy | `dataretention.yaml` |
| PATCH | `/api/v4/data_retention/policies/{policy_id}` | Patch a granular data retention policy | `dataretention.yaml` |
| DELETE | `/api/v4/data_retention/policies/{policy_id}` | Delete a granular data retention policy | `dataretention.yaml` |
| GET | `/api/v4/data_retention/policies/{policy_id}/teams` | Get the teams for a granular data retention policy | `dataretention.yaml` |
| POST | `/api/v4/data_retention/policies/{policy_id}/teams` | Add teams to a granular data retention policy | `dataretention.yaml` |
| DELETE | `/api/v4/data_retention/policies/{policy_id}/teams` | Delete teams from a granular data retention policy | `dataretention.yaml` |
| POST | `/api/v4/data_retention/policies/{policy_id}/teams/search` | Search for the teams in a granular data retention policy | `dataretention.yaml` |
| GET | `/api/v4/data_retention/policies/{policy_id}/channels` | Get the channels for a granular data retention policy | `dataretention.yaml` |
| POST | `/api/v4/data_retention/policies/{policy_id}/channels` | Add channels to a granular data retention policy | `dataretention.yaml` |
| DELETE | `/api/v4/data_retention/policies/{policy_id}/channels` | Delete channels from a granular data retention policy | `dataretention.yaml` |
| POST | `/api/v4/data_retention/policies/{policy_id}/channels/search` | Search for the channels in a granular data retention policy | `dataretention.yaml` |
| POST | `/api/v4/elasticsearch/test` | Test Elasticsearch configuration | `elasticsearch.yaml` |
| POST | `/api/v4/elasticsearch/purge_indexes` | Purge all Elasticsearch indexes | `elasticsearch.yaml` |
| POST | `/api/v4/emoji` | Create a custom emoji | `emoji.yaml` |
| GET | `/api/v4/emoji` | Get a list of custom emoji | `emoji.yaml` |
| GET | `/api/v4/emoji/{emoji_id}` | Get a custom emoji | `emoji.yaml` |
| DELETE | `/api/v4/emoji/{emoji_id}` | Delete a custom emoji | `emoji.yaml` |
| GET | `/api/v4/emoji/name/{emoji_name}` | Get a custom emoji by name | `emoji.yaml` |
| GET | `/api/v4/emoji/{emoji_id}/image` | Get custom emoji image | `emoji.yaml` |
| POST | `/api/v4/emoji/search` | Search custom emoji | `emoji.yaml` |
| GET | `/api/v4/emoji/autocomplete` | Autocomplete custom emoji | `emoji.yaml` |
| POST | `/api/v4/emoji/names` | Get custom emojis by name | `emoji.yaml` |
| GET | `/api/v4/exports` | List export files | `exports.yaml` |
| GET | `/api/v4/exports/{export_name}` | Download an export file | `exports.yaml` |
| DELETE | `/api/v4/exports/{export_name}` | Delete an export file | `exports.yaml` |
| POST | `/api/v4/files` | Upload a file | `files.yaml` |
| GET | `/api/v4/files/{file_id}` | Get a file | `files.yaml` |
| GET | `/api/v4/files/{file_id}/thumbnail` | Get a file's thumbnail | `files.yaml` |
| GET | `/api/v4/files/{file_id}/preview` | Get a file's preview | `files.yaml` |
| GET | `/api/v4/files/{file_id}/link` | Get a public file link | `files.yaml` |
| GET | `/api/v4/files/{file_id}/info` | Get metadata for a file | `files.yaml` |
| GET | `/files/{file_id}/public` | Get a public file | `files.yaml` |
| POST | `/api/v4/teams/{team_id}/files/search` | Search files in a team | `files.yaml` |
| POST | `/api/v4/files/search` | Search files across the teams of the current user | `files.yaml` |
| GET | `/api/v4/groups` | Get groups | `groups.yaml` |
| POST | `/api/v4/groups` | Create a custom group | `groups.yaml` |
| GET | `/api/v4/groups/{group_id}` | Get a group | `groups.yaml` |
| DELETE | `/api/v4/groups/{group_id}` | Deletes a custom group | `groups.yaml` |
| PUT | `/api/v4/groups/{group_id}/patch` | Patch a group | `groups.yaml` |
| POST | `/api/v4/groups/{group_id}/restore` | Restore a previously deleted group. | `groups.yaml` |
| POST | `/api/v4/groups/{group_id}/teams/{team_id}/link` | Link a team to a group | `groups.yaml` |
| DELETE | `/api/v4/groups/{group_id}/teams/{team_id}/link` | Delete a link from a team to a group | `groups.yaml` |
| POST | `/api/v4/groups/{group_id}/channels/{channel_id}/link` | Link a channel to a group | `groups.yaml` |
| DELETE | `/api/v4/groups/{group_id}/channels/{channel_id}/link` | Delete a link from a channel to a group | `groups.yaml` |
| GET | `/api/v4/groups/{group_id}/teams/{team_id}` | Get GroupSyncable from Team ID | `groups.yaml` |
| GET | `/api/v4/groups/{group_id}/channels/{channel_id}` | Get GroupSyncable from channel ID | `groups.yaml` |
| GET | `/api/v4/groups/{group_id}/teams` | Get group teams | `groups.yaml` |
| GET | `/api/v4/groups/{group_id}/channels` | Get group channels | `groups.yaml` |
| PUT | `/api/v4/groups/{group_id}/teams/{team_id}/patch` | Patch a GroupSyncable associated to Team | `groups.yaml` |
| PUT | `/api/v4/groups/{group_id}/channels/{channel_id}/patch` | Patch a GroupSyncable associated to Channel | `groups.yaml` |
| GET | `/api/v4/groups/{group_id}/members` | Get group users | `groups.yaml` |
| DELETE | `/api/v4/groups/{group_id}/members` | Removes members from a custom group | `groups.yaml` |
| POST | `/api/v4/groups/{group_id}/members` | Adds members to a custom group | `groups.yaml` |
| GET | `/api/v4/groups/{group_id}/stats` | Get group stats | `groups.yaml` |
| GET | `/api/v4/channels/{channel_id}/groups` | Get channel groups | `groups.yaml` |
| GET | `/api/v4/teams/{team_id}/groups` | Get team groups | `groups.yaml` |
| GET | `/api/v4/teams/{team_id}/groups_by_channels` | Get team groups by channels | `groups.yaml` |
| GET | `/api/v4/users/{user_id}/groups` | Get groups for a userId | `groups.yaml` |
| POST | `/api/v4/groups/names` | Get groups by name | `groups.yaml` |
| GET | `/api/v4/imports` | List import files | `imports.yaml` |
| DELETE | `/api/v4/imports/{import_name}` | Delete an import file | `imports.yaml` |
| GET | `/api/v4/ip_filtering` | Get all IP filters | `ip_filters.yaml` |
| POST | `/api/v4/ip_filtering` | Get all IP filters | `ip_filters.yaml` |
| GET | `/api/v4/ip_filtering/my_ip` | Get all IP filters | `ip_filters.yaml` |
| GET | `/api/v4/jobs` | Get the jobs. | `jobs.yaml` |
| POST | `/api/v4/jobs` | Create a new job. | `jobs.yaml` |
| GET | `/api/v4/jobs/{job_id}` | Get a job. | `jobs.yaml` |
| GET | `/api/v4/jobs/{job_id}/download` | Download the results of a job. | `jobs.yaml` |
| POST | `/api/v4/jobs/{job_id}/cancel` | Cancel a job. | `jobs.yaml` |
| GET | `/api/v4/jobs/type/{type}` | Get the jobs of the given type. | `jobs.yaml` |
| PATCH | `/api/v4/jobs/{job_id}/status` | Update the status of a job | `jobs.yaml` |
| POST | `/api/v4/ldap/sync` | Sync with LDAP | `ldap.yaml` |
| POST | `/api/v4/ldap/test` | Test LDAP configuration | `ldap.yaml` |
| POST | `/api/v4/ldap/test_connection` | Test LDAP connection with specific settings | `ldap.yaml` |
| POST | `/api/v4/ldap/test_diagnostics` | Test LDAP diagnostics with specific settings | `ldap.yaml` |
| GET | `/api/v4/ldap/groups` | Returns a list of LDAP groups | `ldap.yaml` |
| POST | `/api/v4/ldap/groups/{remote_id}/link` | Link a LDAP group | `ldap.yaml` |
| DELETE | `/api/v4/ldap/groups/{remote_id}/link` | Delete a link for LDAP group | `ldap.yaml` |
| POST | `/api/v4/ldap/migrateid` | Migrate Id LDAP | `ldap.yaml` |
| POST | `/api/v4/ldap/certificate/public` | Upload public certificate | `ldap.yaml` |
| DELETE | `/api/v4/ldap/certificate/public` | Remove public certificate | `ldap.yaml` |
| POST | `/api/v4/ldap/certificate/private` | Upload private key | `ldap.yaml` |
| DELETE | `/api/v4/ldap/certificate/private` | Remove private key | `ldap.yaml` |
| POST | `/api/v4/ldap/users/{user_id}/group_sync_memberships` | Create memberships for LDAP configured channels and teams for this user | `ldap.yaml` |
| GET | `/api/v4/limits/server` | Gets the server limits for the server | `limits.yaml` |
| GET | `/api/v4/logs/download` | Download system logs | `logs.yaml` |
| POST | `/api/v4/client_perf` | Report client performance metrics | `metrics.yaml` |
| POST | `/api/v4/oauth/apps` | Register OAuth app | `oauth.yaml` |
| GET | `/api/v4/oauth/apps` | Get OAuth apps | `oauth.yaml` |
| GET | `/api/v4/oauth/apps/{app_id}` | Get an OAuth app | `oauth.yaml` |
| PUT | `/api/v4/oauth/apps/{app_id}` | Update an OAuth app | `oauth.yaml` |
| DELETE | `/api/v4/oauth/apps/{app_id}` | Delete an OAuth app | `oauth.yaml` |
| POST | `/api/v4/oauth/apps/{app_id}/regen_secret` | Regenerate OAuth app secret | `oauth.yaml` |
| GET | `/api/v4/oauth/apps/{app_id}/info` | Get info on an OAuth app | `oauth.yaml` |
| GET | `/.well-known/oauth-authorization-server` | Get OAuth 2.0 Authorization Server Metadata | `oauth.yaml` |
| POST | `/api/v4/oauth/apps/register` | Register OAuth client using Dynamic Client Registration | `oauth.yaml` |
| GET | `/api/v4/users/{user_id}/oauth/apps/authorized` | Get authorized OAuth apps | `oauth.yaml` |
| GET | `/api/v4/oauth/outgoing_connections` | List all connections | `outgoing_oauth_connections.yaml` |
| POST | `/api/v4/oauth/outgoing_connections` | Create a connection | `outgoing_oauth_connections.yaml` |
| GET | `/api/v4/oauth/outgoing_connections/{connection_id}` | Get a connection | `outgoing_oauth_connections.yaml` |
| PUT | `/api/v4/oauth/outgoing_connections/{connection_id}` | Update a connection | `outgoing_oauth_connections.yaml` |
| DELETE | `/api/v4/oauth/outgoing_connections/{connection_id}` | Delete a connection | `outgoing_oauth_connections.yaml` |
| POST | `/api/v4/oauth/outgoing_connections/validate` | Validate a connection configuration | `outgoing_oauth_connections.yaml` |
| POST | `/api/v4/permissions/ancillary` | Return all system console subsection ancillary permissions | `permissions.yaml` |
| POST | `/api/v4/plugins` | Upload plugin | `plugins.yaml` |
| GET | `/api/v4/plugins` | Get plugins | `plugins.yaml` |
| POST | `/api/v4/plugins/install_from_url` | Install plugin from url | `plugins.yaml` |
| DELETE | `/api/v4/plugins/{plugin_id}` | Remove plugin | `plugins.yaml` |
| POST | `/api/v4/plugins/{plugin_id}/enable` | Enable plugin | `plugins.yaml` |
| POST | `/api/v4/plugins/{plugin_id}/disable` | Disable plugin | `plugins.yaml` |
| GET | `/api/v4/plugins/webapp` | Get webapp plugins | `plugins.yaml` |
| GET | `/api/v4/plugins/statuses` | Get plugins status | `plugins.yaml` |
| POST | `/api/v4/plugins/marketplace` | Installs a marketplace plugin | `plugins.yaml` |
| GET | `/api/v4/plugins/marketplace` | Gets all the marketplace plugins | `plugins.yaml` |
| GET | `/api/v4/plugins/marketplace/first_admin_visit` | Get if the Plugin Marketplace has been visited by at least an admin. | `plugins.yaml` |
| POST | `/api/v4/plugins/marketplace/first_admin_visit` | Stores that the Plugin Marketplace has been visited by at least an admin. | `plugins.yaml` |
| POST | `/api/v4/posts` | Create a post | `posts.yaml` |
| POST | `/api/v4/posts/ephemeral` | Create a ephemeral post | `posts.yaml` |
| GET | `/api/v4/posts/{post_id}` | Get a post | `posts.yaml` |
| DELETE | `/api/v4/posts/{post_id}` | Delete a post | `posts.yaml` |
| PUT | `/api/v4/posts/{post_id}` | Update a post | `posts.yaml` |
| POST | `/api/v4/users/{user_id}/posts/{post_id}/set_unread` | Mark as unread from a post. | `posts.yaml` |
| PUT | `/api/v4/posts/{post_id}/patch` | Patch a post | `posts.yaml` |
| GET | `/api/v4/posts/{post_id}/thread` | Get a thread | `posts.yaml` |
| GET | `/api/v4/users/{user_id}/posts/flagged` | Get a list of flagged posts | `posts.yaml` |
| GET | `/api/v4/posts/{post_id}/files/info` | Get file info for post | `posts.yaml` |
| GET | `/api/v4/channels/{channel_id}/posts` | Get posts for a channel | `posts.yaml` |
| GET | `/api/v4/users/{user_id}/channels/{channel_id}/posts/unread` | Get posts around oldest unread | `posts.yaml` |
| POST | `/api/v4/teams/{team_id}/posts/search` | Search for team posts | `posts.yaml` |
| POST | `/api/v4/posts/{post_id}/pin` | Pin a post to the channel | `posts.yaml` |
| POST | `/api/v4/posts/{post_id}/unpin` | Unpin a post to the channel | `posts.yaml` |
| POST | `/api/v4/posts/{post_id}/actions/{action_id}` | Perform a post action | `posts.yaml` |
| POST | `/api/v4/posts/ids` | Get posts by a list of ids | `posts.yaml` |
| POST | `/api/v4/users/{user_id}/posts/{post_id}/reminder` | Set a post reminder | `posts.yaml` |
| POST | `/api/v4/users/{user_id}/posts/{post_id}/ack` | Acknowledge a post | `posts.yaml` |
| DELETE | `/api/v4/users/{user_id}/posts/{post_id}/ack` | Delete a post acknowledgement | `posts.yaml` |
| POST | `/api/v4/posts/{post_id}/move` | Move a post (and any posts within that post's thread) | `posts.yaml` |
| POST | `/api/v4/posts/{post_id}/restore/{restore_version_id}` | Restores a past version of a post | `posts.yaml` |
| GET | `/api/v4/posts/{post_id}/reveal` | Reveal a burn-on-read post | `posts.yaml` |
| DELETE | `/api/v4/posts/{post_id}/burn` | Burn a burn-on-read post | `posts.yaml` |
| POST | `/api/v4/posts/rewrite` | Rewrite a message using AI | `posts.yaml` |
| GET | `/api/v4/users/{user_id}/preferences` | Get the user's preferences | `preferences.yaml` |
| PUT | `/api/v4/users/{user_id}/preferences` | Save the user's preferences | `preferences.yaml` |
| POST | `/api/v4/users/{user_id}/preferences/delete` | Delete user's preferences | `preferences.yaml` |
| GET | `/api/v4/users/{user_id}/preferences/{category}` | List a user's preferences by category | `preferences.yaml` |
| GET | `/api/v4/users/{user_id}/preferences/{category}/name/{preference_name}` | Get a specific user preference | `preferences.yaml` |
| POST | `/api/v4/reactions` | Create a reaction | `reactions.yaml` |
| GET | `/api/v4/posts/{post_id}/reactions` | Get a list of reactions to a post | `reactions.yaml` |
| DELETE | `/api/v4/users/{user_id}/posts/{post_id}/reactions/{emoji_name}` | Remove a reaction from a post | `reactions.yaml` |
| POST | `/api/v4/posts/ids/reactions` | Bulk get the reaction for posts | `reactions.yaml` |
| POST | `/api/v4/recaps` | Create a channel recap | `recaps.yaml` |
| GET | `/api/v4/recaps` | Get current user's recaps | `recaps.yaml` |
| GET | `/api/v4/recaps/{recap_id}` | Get a specific recap | `recaps.yaml` |
| DELETE | `/api/v4/recaps/{recap_id}` | Delete a recap | `recaps.yaml` |
| POST | `/api/v4/recaps/{recap_id}/read` | Mark a recap as read | `recaps.yaml` |
| POST | `/api/v4/recaps/{recap_id}/regenerate` | Regenerate a recap | `recaps.yaml` |
| GET | `/api/v4/remotecluster` | Get a list of remote clusters. | `remoteclusters.yaml` |
| POST | `/api/v4/remotecluster` | Create a new remote cluster. | `remoteclusters.yaml` |
| GET | `/api/v4/remotecluster/{remote_id}` | Get a remote cluster. | `remoteclusters.yaml` |
| PATCH | `/api/v4/remotecluster/{remote_id}` | Patch a remote cluster. | `remoteclusters.yaml` |
| DELETE | `/api/v4/remotecluster/{remote_id}` | Delete a remote cluster. | `remoteclusters.yaml` |
| POST | `/api/v4/remotecluster/{remote_id}/generate_invite` | Generate invite code. | `remoteclusters.yaml` |
| POST | `/api/v4/remotecluster/accept_invite` | Accept a remote cluster invite code. | `remoteclusters.yaml` |
| GET | `/api/v4/reports/users` | Get a list of paged and sorted users for admin reporting purposes | `reports.yaml` |
| GET | `/api/v4/reports/users/count` | Gets the full count of users that match the filter. | `reports.yaml` |
| POST | `/api/v4/reports/users/export` | Starts a job to export the users to a report file. | `reports.yaml` |
| POST | `/api/v4/reports/posts` | Get posts for reporting and compliance purposes using cursor-based pagination | `reports.yaml` |
| GET | `/api/v4/roles` | Get a list of all the roles | `roles.yaml` |
| GET | `/api/v4/roles/{role_id}` | Get a role | `roles.yaml` |
| GET | `/api/v4/roles/name/{role_name}` | Get a role | `roles.yaml` |
| PUT | `/api/v4/roles/{role_id}/patch` | Patch a role | `roles.yaml` |
| POST | `/api/v4/roles/names` | Get a list of roles by name | `roles.yaml` |
| GET | `/api/v4/saml/metadata` | Get metadata | `saml.yaml` |
| POST | `/api/v4/saml/metadatafromidp` | Get metadata from Identity Provider | `saml.yaml` |
| POST | `/api/v4/saml/certificate/idp` | Upload IDP certificate | `saml.yaml` |
| DELETE | `/api/v4/saml/certificate/idp` | Remove IDP certificate | `saml.yaml` |
| POST | `/api/v4/saml/certificate/public` | Upload public certificate | `saml.yaml` |
| DELETE | `/api/v4/saml/certificate/public` | Remove public certificate | `saml.yaml` |
| POST | `/api/v4/saml/certificate/private` | Upload private key | `saml.yaml` |
| DELETE | `/api/v4/saml/certificate/private` | Remove private key | `saml.yaml` |
| GET | `/api/v4/saml/certificate/status` | Get certificate status | `saml.yaml` |
| POST | `/api/v4/saml/reset_auth_data` | Reset AuthData to Email | `saml.yaml` |
| POST | `/api/v4/posts/schedule` | Creates a scheduled post | `scheduled_post.yaml` |
| GET | `/api/v4/posts/scheduled/team/{team_id}` | Gets all scheduled posts for a user for the specified team.. | `scheduled_post.yaml` |
| PUT | `/api/v4/posts/schedule/{scheduled_post_id}` | Update a scheduled post | `scheduled_post.yaml` |
| DELETE | `/api/v4/posts/schedule/{scheduled_post_id}` | Delete a scheduled post | `scheduled_post.yaml` |
| GET | `/api/v4/schemes` | Get the schemes. | `schemes.yaml` |
| POST | `/api/v4/schemes` | Create a scheme | `schemes.yaml` |
| GET | `/api/v4/schemes/{scheme_id}` | Get a scheme | `schemes.yaml` |
| DELETE | `/api/v4/schemes/{scheme_id}` | Delete a scheme | `schemes.yaml` |
| PUT | `/api/v4/schemes/{scheme_id}/patch` | Patch a scheme | `schemes.yaml` |
| GET | `/api/v4/schemes/{scheme_id}/teams` | Get a page of teams which use this scheme. | `schemes.yaml` |
| GET | `/api/v4/schemes/{scheme_id}/channels` | Get a page of channels which use this scheme. | `schemes.yaml` |
| GET | `/api/v4/terms_of_service` | Get latest terms of service | `service_terms.yaml` |
| POST | `/api/v4/terms_of_service` | Creates a new terms of service | `service_terms.yaml` |
| GET | `/api/v4/sharedchannels/{team_id}` | Get all shared channels for team. | `sharedchannels.yaml` |
| GET | `/api/v4/remotecluster/{remote_id}/sharedchannelremotes` | Get shared channel remotes by remote cluster. | `sharedchannels.yaml` |
| GET | `/api/v4/sharedchannels/remote_info/{remote_id}` | Get remote cluster info by ID for user. | `sharedchannels.yaml` |
| POST | `/api/v4/remotecluster/{remote_id}/channels/{channel_id}/invite` | Invites a remote cluster to a channel. | `sharedchannels.yaml` |
| POST | `/api/v4/remotecluster/{remote_id}/channels/{channel_id}/uninvite` | Uninvites a remote cluster to a channel. | `sharedchannels.yaml` |
| GET | `/api/v4/sharedchannels/{channel_id}/remotes` | Get remote clusters for a shared channel | `sharedchannels.yaml` |
| GET | `/api/v4/sharedchannels/users/{user_id}/can_dm/{other_user_id}` | Check if user can DM another user in shared channels context | `sharedchannels.yaml` |
| GET | `/api/v4/users/{user_id}/status` | Get user status | `status.yaml` |
| PUT | `/api/v4/users/{user_id}/status` | Update user status | `status.yaml` |
| POST | `/api/v4/users/status/ids` | Get user statuses by id | `status.yaml` |
| PUT | `/api/v4/users/{user_id}/status/custom` | Update user custom status | `status.yaml` |
| DELETE | `/api/v4/users/{user_id}/status/custom` | Unsets user custom status | `status.yaml` |
| DELETE | `/api/v4/users/{user_id}/status/custom/recent` | Delete user's recent custom status | `status.yaml` |
| POST | `/api/v4/users/{user_id}/status/custom/recent/delete` | Delete user's recent custom status | `status.yaml` |
| GET | `/api/v4/system/timezones` | Retrieve a list of supported timezones | `system.yaml` |
| GET | `/api/v4/system/ping` | Check system health | `system.yaml` |
| GET | `/api/v4/system/notices/{teamId}` | Get notices for logged in user in specified team | `system.yaml` |
| PUT | `/api/v4/system/notices/view` | Update notices as 'viewed' | `system.yaml` |
| POST | `/api/v4/database/recycle` | Recycle database connections | `system.yaml` |
| POST | `/api/v4/email/test` | Send a test email | `system.yaml` |
| POST | `/api/v4/notifications/test` | Send a test notification | `system.yaml` |
| POST | `/api/v4/site_url/test` | Checks the validity of a Site URL | `system.yaml` |
| POST | `/api/v4/file/s3_test` | Test AWS S3 connection | `system.yaml` |
| GET | `/api/v4/config` | Get configuration | `system.yaml` |
| PUT | `/api/v4/config` | Update configuration | `system.yaml` |
| POST | `/api/v4/config/reload` | Reload configuration | `system.yaml` |
| GET | `/api/v4/config/client` | Get client configuration | `system.yaml` |
| GET | `/api/v4/config/environment` | Get configuration made through environment variables | `system.yaml` |
| PUT | `/api/v4/config/patch` | Patch configuration | `system.yaml` |
| POST | `/api/v4/license` | Upload license file | `system.yaml` |
| DELETE | `/api/v4/license` | Remove license file | `system.yaml` |
| GET | `/api/v4/license/client` | Get client license | `system.yaml` |
| GET | `/api/v4/license/load_metric` | Get license load metric | `system.yaml` |
| GET | `/api/v4/license/renewal` | Request the license renewal link | `system.yaml` |
| POST | `/api/v4/trial-license` | Request and install a trial license for your server | `system.yaml` |
| GET | `/api/v4/trial-license/prev` | Get last trial license used | `system.yaml` |
| GET | `/api/v4/audits` | Get audits | `system.yaml` |
| POST | `/api/v4/caches/invalidate` | Invalidate all the caches | `system.yaml` |
| GET | `/api/v4/logs` | Get logs | `system.yaml` |
| POST | `/api/v4/logs` | Add log message | `system.yaml` |
| GET | `/api/v4/analytics/old` | Get analytics | `system.yaml` |
| POST | `/api/v4/server_busy` | Set the server busy (high load) flag | `system.yaml` |
| GET | `/api/v4/server_busy` | Get server busy expiry time. | `system.yaml` |
| DELETE | `/api/v4/server_busy` | Clears the server busy (high load) flag | `system.yaml` |
| POST | `/api/v4/notifications/ack` | Acknowledge receiving of a notification | `system.yaml` |
| GET | `/api/v4/redirect_location` | Get redirect location | `system.yaml` |
| GET | `/api/v4/image` | Get an image by url | `system.yaml` |
| POST | `/api/v4/upgrade_to_enterprise` | Executes an inplace upgrade from Team Edition to Enterprise Edition | `system.yaml` |
| GET | `/api/v4/upgrade_to_enterprise/status` | Get the current status for the inplace upgrade from Team Edition to Enterprise Edition | `system.yaml` |
| GET | `/api/v4/upgrade_to_enterprise/allowed` | Check if the user is allowed to upgrade to Enterprise Edition | `system.yaml` |
| POST | `/api/v4/restart` | Restart the system after an upgrade from Team Edition to Enterprise Edition | `system.yaml` |
| POST | `/api/v4/integrity` | Perform a database integrity check | `system.yaml` |
| GET | `/api/v4/system/support_packet` | Download a zip file which contains helpful and useful information for troubleshooting your mattermost instance. | `system.yaml` |
| POST | `/api/v4/teams` | Create a team | `teams.yaml` |
| GET | `/api/v4/teams` | Get teams | `teams.yaml` |
| GET | `/api/v4/teams/{team_id}` | Get a team | `teams.yaml` |
| PUT | `/api/v4/teams/{team_id}` | Update a team | `teams.yaml` |
| DELETE | `/api/v4/teams/{team_id}` | Delete a team | `teams.yaml` |
| PUT | `/api/v4/teams/{team_id}/patch` | Patch a team | `teams.yaml` |
| PUT | `/api/v4/teams/{team_id}/privacy` | Update teams's privacy | `teams.yaml` |
| POST | `/api/v4/teams/{team_id}/restore` | Restore a team | `teams.yaml` |
| GET | `/api/v4/teams/name/{name}` | Get a team by name | `teams.yaml` |
| POST | `/api/v4/teams/search` | Search teams | `teams.yaml` |
| GET | `/api/v4/teams/name/{name}/exists` | Check if team exists | `teams.yaml` |
| GET | `/api/v4/users/{user_id}/teams` | Get a user's teams | `teams.yaml` |
| GET | `/api/v4/teams/{team_id}/members` | Get team members | `teams.yaml` |
| POST | `/api/v4/teams/{team_id}/members` | Add user to team | `teams.yaml` |
| POST | `/api/v4/teams/members/invite` | Add user to team from invite | `teams.yaml` |
| POST | `/api/v4/teams/{team_id}/members/batch` | Add multiple users to team | `teams.yaml` |
| GET | `/api/v4/users/{user_id}/teams/members` | Get team members for a user | `teams.yaml` |
| GET | `/api/v4/teams/{team_id}/members/{user_id}` | Get a team member | `teams.yaml` |
| DELETE | `/api/v4/teams/{team_id}/members/{user_id}` | Remove user from team | `teams.yaml` |
| POST | `/api/v4/teams/{team_id}/members/ids` | Get team members by ids | `teams.yaml` |
| GET | `/api/v4/teams/{team_id}/stats` | Get a team stats | `teams.yaml` |
| POST | `/api/v4/teams/{team_id}/regenerate_invite_id` | Regenerate the Invite ID from a Team | `teams.yaml` |
| GET | `/api/v4/teams/{team_id}/image` | Get the team icon | `teams.yaml` |
| POST | `/api/v4/teams/{team_id}/image` | Sets the team icon | `teams.yaml` |
| DELETE | `/api/v4/teams/{team_id}/image` | Remove the team icon | `teams.yaml` |
| PUT | `/api/v4/teams/{team_id}/members/{user_id}/roles` | Update a team member roles | `teams.yaml` |
| PUT | `/api/v4/teams/{team_id}/members/{user_id}/schemeRoles` | Update the scheme-derived roles of a team member. | `teams.yaml` |
| GET | `/api/v4/users/{user_id}/teams/unread` | Get team unreads for a user | `teams.yaml` |
| GET | `/api/v4/users/{user_id}/teams/{team_id}/unread` | Get unreads for a team | `teams.yaml` |
| POST | `/api/v4/teams/{team_id}/invite/email` | Invite users to the team by email | `teams.yaml` |
| POST | `/api/v4/teams/{team_id}/invite-guests/email` | Invite guests to the team by email | `teams.yaml` |
| DELETE | `/api/v4/teams/invites/email` | Invalidate active email invitations | `teams.yaml` |
| POST | `/api/v4/teams/{team_id}/import` | Import a Team from other application | `teams.yaml` |
| GET | `/api/v4/teams/invite/{invite_id}` | Get invite info for a team | `teams.yaml` |
| PUT | `/api/v4/teams/{team_id}/scheme` | Set a team's scheme | `teams.yaml` |
| GET | `/api/v4/teams/{team_id}/members_minus_group_members` | Team members minus group members. | `teams.yaml` |
| POST | `/api/v4/uploads` | Create an upload | `uploads.yaml` |
| GET | `/api/v4/uploads/{upload_id}` | Get an upload session | `uploads.yaml` |
| POST | `/api/v4/uploads/{upload_id}` | Perform a file upload | `uploads.yaml` |
| GET | `/api/v4/usage/posts` | Get current usage of posts | `usage.yaml` |
| GET | `/api/v4/usage/storage` | Get the total file storage usage for the instance in bytes. | `usage.yaml` |
| POST | `/api/v4/users/login` | Login to Mattermost server | `users.yaml` |
| POST | `/api/v4/users/login/cws` | Auto-Login to Mattermost server using CWS token | `users.yaml` |
| POST | `/api/v4/users/login/sso/code-exchange` | Exchange SSO login code for session tokens | `users.yaml` |
| POST | `/oauth/intune` | Login with Microsoft Intune MAM | `users.yaml` |
| POST | `/api/v4/users/logout` | Logout from the Mattermost server | `users.yaml` |
| POST | `/api/v4/users` | Create a user | `users.yaml` |
| GET | `/api/v4/users` | Get users | `users.yaml` |
| DELETE | `/api/v4/users` | Permanent delete all users | `users.yaml` |
| POST | `/api/v4/users/ids` | Get users by ids | `users.yaml` |
| POST | `/api/v4/users/group_channels` | Get users by group channels ids | `users.yaml` |
| POST | `/api/v4/users/usernames` | Get users by usernames | `users.yaml` |
| POST | `/api/v4/users/search` | Search users | `users.yaml` |
| GET | `/api/v4/users/autocomplete` | Autocomplete users | `users.yaml` |
| GET | `/api/v4/users/known` | Get user IDs of known users | `users.yaml` |
| GET | `/api/v4/users/stats` | Get total count of users in the system | `users.yaml` |
| GET | `/api/v4/users/stats/filtered` | Get total count of users in the system matching the specified filters | `users.yaml` |
| GET | `/api/v4/users/{user_id}` | Get a user | `users.yaml` |
| PUT | `/api/v4/users/{user_id}` | Update a user | `users.yaml` |
| DELETE | `/api/v4/users/{user_id}` | Deactivate a user account. | `users.yaml` |
| PUT | `/api/v4/users/{user_id}/patch` | Patch a user | `users.yaml` |
| PUT | `/api/v4/users/{user_id}/roles` | Update a user's roles | `users.yaml` |
| PUT | `/api/v4/users/{user_id}/active` | Activate or deactivate a user | `users.yaml` |
| GET | `/api/v4/users/{user_id}/image` | Get user's profile image | `users.yaml` |
| POST | `/api/v4/users/{user_id}/image` | Set user's profile image | `users.yaml` |
| DELETE | `/api/v4/users/{user_id}/image` | Delete user's profile image | `users.yaml` |
| GET | `/api/v4/users/{user_id}/image/default` | Return user's default (generated) profile image | `users.yaml` |
| GET | `/api/v4/users/username/{username}` | Get a user by username | `users.yaml` |
| POST | `/api/v4/users/password/reset` | Reset password | `users.yaml` |
| PUT | `/api/v4/users/{user_id}/mfa` | Update a user's MFA | `users.yaml` |
| POST | `/api/v4/users/{user_id}/mfa/generate` | Generate MFA secret | `users.yaml` |
| POST | `/api/v4/users/{user_id}/demote` | Demote a user to a guest | `users.yaml` |
| POST | `/api/v4/users/{user_id}/promote` | Promote a guest to user | `users.yaml` |
| POST | `/api/v4/users/{user_id}/convert_to_bot` | Convert a user into a bot | `users.yaml` |
| POST | `/api/v4/users/mfa` | Check MFA | `users.yaml` |
| PUT | `/api/v4/users/{user_id}/password` | Update a user's password | `users.yaml` |
| POST | `/api/v4/users/password/reset/send` | Send password reset email | `users.yaml` |
| GET | `/api/v4/users/email/{email}` | Get a user by email | `users.yaml` |
| GET | `/api/v4/users/{user_id}/sessions` | Get user's sessions | `users.yaml` |
| POST | `/api/v4/users/{user_id}/sessions/revoke` | Revoke a user session | `users.yaml` |
| POST | `/api/v4/users/{user_id}/sessions/revoke/all` | Revoke all active sessions for a user | `users.yaml` |
| PUT | `/api/v4/users/sessions/device` | Attach mobile device and extra props to the session object | `users.yaml` |
| GET | `/api/v4/users/{user_id}/audits` | Get user's audits | `users.yaml` |
| POST | `/api/v4/users/{user_id}/email/verify/member` | Verify user email by ID | `users.yaml` |
| POST | `/api/v4/users/email/verify` | Verify user email | `users.yaml` |
| POST | `/api/v4/users/email/verify/send` | Send verification email | `users.yaml` |
| POST | `/api/v4/users/login/switch` | Switch login method | `users.yaml` |
| POST | `/api/v4/users/login/type` | Get login authentication type | `users.yaml` |
| POST | `/api/v4/users/{user_id}/tokens` | Create a user access token | `users.yaml` |
| GET | `/api/v4/users/{user_id}/tokens` | Get user access tokens | `users.yaml` |
| GET | `/api/v4/users/tokens` | Get user access tokens | `users.yaml` |
| POST | `/api/v4/users/tokens/revoke` | Revoke a user access token | `users.yaml` |
| GET | `/api/v4/users/tokens/{token_id}` | Get a user access token | `users.yaml` |
| POST | `/api/v4/users/tokens/disable` | Disable personal access token | `users.yaml` |
| POST | `/api/v4/users/tokens/enable` | Enable personal access token | `users.yaml` |
| POST | `/api/v4/users/tokens/search` | Search tokens | `users.yaml` |
| PUT | `/api/v4/users/{user_id}/auth` | Update a user's authentication method | `users.yaml` |
| POST | `/api/v4/users/{user_id}/terms_of_service` | Records user action when they accept or decline custom terms of service | `users.yaml` |
| GET | `/api/v4/users/{user_id}/terms_of_service` | Fetches user's latest terms of service action if the latest action was for acceptance. | `users.yaml` |
| POST | `/api/v4/users/sessions/revoke/all` | Revoke all sessions from all users. | `users.yaml` |
| POST | `/api/v4/users/{user_id}/typing` | Publish a user typing websocket event. | `users.yaml` |
| GET | `/api/v4/users/{user_id}/uploads` | Get uploads for a user | `users.yaml` |
| GET | `/api/v4/users/{user_id}/channel_members` | Get all channel members from all teams for a user | `users.yaml` |
| POST | `/api/v4/users/migrate_auth/ldap` | Migrate user accounts authentication type to LDAP. | `users.yaml` |
| POST | `/api/v4/users/migrate_auth/saml` | Migrate user accounts authentication type to SAML. | `users.yaml` |
| GET | `/api/v4/users/{user_id}/teams/{team_id}/threads` | Get all threads that user is following | `users.yaml` |
| GET | `/api/v4/users/{user_id}/teams/{team_id}/threads/mention_counts` | Get all unread mention counts from followed threads, per-channel | `users.yaml` |
| PUT | `/api/v4/users/{user_id}/teams/{team_id}/threads/read` | Mark all threads that user is following as read | `users.yaml` |
| PUT | `/api/v4/users/{user_id}/teams/{team_id}/threads/{thread_id}/read/{timestamp}` | Mark a thread that user is following read state to the timestamp | `users.yaml` |
| POST | `/api/v4/users/{user_id}/teams/{team_id}/threads/{thread_id}/set_unread/{post_id}` | Mark a thread that user is following as unread based on a post id | `users.yaml` |
| PUT | `/api/v4/users/{user_id}/teams/{team_id}/threads/{thread_id}/following` | Start following a thread | `users.yaml` |
| DELETE | `/api/v4/users/{user_id}/teams/{team_id}/threads/{thread_id}/following` | Stop following a thread | `users.yaml` |
| GET | `/api/v4/users/{user_id}/teams/{team_id}/threads/{thread_id}` | Get a thread followed by the user | `users.yaml` |
| GET | `/api/v4/users/{user_id}/data_retention/team_policies` | Get the policies which are applied to a user's teams | `users.yaml` |
| GET | `/api/v4/users/{user_id}/data_retention/channel_policies` | Get the policies which are applied to a user's channels | `users.yaml` |
| GET | `/api/v4/users/invalid_emails` | Get users with invalid emails | `users.yaml` |
| POST | `/api/v4/users/{user_id}/reset_failed_attempts` | Reset the failed password attempts for a user | `users.yaml` |
| POST | `/api/v4/hooks/incoming` | Create an incoming webhook | `webhooks.yaml` |
| GET | `/api/v4/hooks/incoming` | List incoming webhooks | `webhooks.yaml` |
| GET | `/api/v4/hooks/incoming/{hook_id}` | Get an incoming webhook | `webhooks.yaml` |
| DELETE | `/api/v4/hooks/incoming/{hook_id}` | Delete an incoming webhook | `webhooks.yaml` |
| PUT | `/api/v4/hooks/incoming/{hook_id}` | Update an incoming webhook | `webhooks.yaml` |
| POST | `/api/v4/hooks/outgoing` | Create an outgoing webhook | `webhooks.yaml` |
| GET | `/api/v4/hooks/outgoing` | List outgoing webhooks | `webhooks.yaml` |
| GET | `/api/v4/hooks/outgoing/{hook_id}` | Get an outgoing webhook | `webhooks.yaml` |
| DELETE | `/api/v4/hooks/outgoing/{hook_id}` | Delete an outgoing webhook | `webhooks.yaml` |
| PUT | `/api/v4/hooks/outgoing/{hook_id}` | Update an outgoing webhook | `webhooks.yaml` |
| POST | `/api/v4/hooks/outgoing/{hook_id}/regen_token` | Regenerate the token for the outgoing webhook. | `webhooks.yaml` |
