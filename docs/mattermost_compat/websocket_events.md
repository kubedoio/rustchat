# Mattermost WebSocket Events

This document tracks the WebSocket events that need to be supported for the mobile client.

| Event | Source Server Action | Expected Payload Fields | Status |
|---|---|---|---|
| `posted` | New post created | `event`, `data.post`, `data.channel_display_name`, `data.channel_name`, `data.channel_type`, `data.sender_name`, `data.team_id`, `broadcast.*`, `seq` | Partial (Missing channel/team info) |
| `typing` | User typing | `event`, `data.parent_id`, `data.user_id`, `broadcast.*`, `seq` | Implemented |
| `status_change` | User status changed | `event`, `data.user_id`, `data.status`, `broadcast.*`, `seq` | TODO |
| `channel_viewed` | User viewed channel | `event`, `data.channel_id`, `broadcast.*`, `seq` | TODO |
| `reaction_added` | Reaction added | `event`, `data.reaction` (JSON string), `broadcast.*`, `seq` | TODO |
| `reaction_removed` | Reaction removed | `event`, `data.reaction` (JSON string), `broadcast.*`, `seq` | TODO |
| `post_edited` | Post edited | `event`, `data.post` (JSON string), `broadcast.*`, `seq` | TODO |
| `post_deleted` | Post deleted | `event`, `data.post` (JSON string), `broadcast.*`, `seq` | TODO |
| `user_added` | User added to channel/team | `event`, `data.user_id`, `data.team_id`, `data.channel_id` | TODO |
| `user_removed` | User removed from channel/team | `event`, `data.user_id`, `data.remover_id` | TODO |
