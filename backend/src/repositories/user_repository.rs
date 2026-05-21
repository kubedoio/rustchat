use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::User;

pub struct UserRepository<'a> {
    pool: &'a PgPool,
}

impl<'a> UserRepository<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    /// Get a user by ID that has not been soft-deleted.
    pub async fn get_by_id(&self, id: Uuid) -> Result<Option<User>, sqlx::Error> {
        sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1 AND deleted_at IS NULL")
            .bind(id)
            .fetch_optional(self.pool)
            .await
    }

    /// Get a user by ID without checking deleted_at.
    pub async fn get_by_id_unchecked(&self, id: Uuid) -> Result<Option<User>, sqlx::Error> {
        sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
            .bind(id)
            .fetch_optional(self.pool)
            .await
    }

    /// Get a user by exact username match.
    pub async fn get_by_username(&self, username: &str) -> Result<Option<User>, sqlx::Error> {
        sqlx::query_as::<_, User>("SELECT * FROM users WHERE username = $1")
            .bind(username)
            .fetch_optional(self.pool)
            .await
    }

    /// Get a user by email.
    pub async fn get_by_email(&self, email: &str) -> Result<Option<User>, sqlx::Error> {
        sqlx::query_as::<_, User>("SELECT * FROM users WHERE email = $1 AND deleted_at IS NULL")
            .bind(email)
            .fetch_optional(self.pool)
            .await
    }

    /// Get an active user by email.
    pub async fn get_active_by_email(&self, email: &str) -> Result<Option<User>, sqlx::Error> {
        sqlx::query_as::<_, User>(
            "SELECT * FROM users WHERE email = $1 AND is_active = true AND deleted_at IS NULL",
        )
        .bind(email)
        .fetch_optional(self.pool)
        .await
    }

    /// Create a new user.
    pub async fn create_user(
        &self,
        username: &str,
        email: &str,
        password_hash: &Option<String>,
        display_name: &Option<String>,
        org_id: Option<Uuid>,
        is_active: bool,
    ) -> Result<User, sqlx::Error> {
        sqlx::query_as::<_, User>(
            r#"
            INSERT INTO users (username, email, password_hash, display_name, org_id, role, is_active, email_verified)
            VALUES ($1, $2, $3, $4, $5, 'member', $6, false)
            RETURNING *
            "#,
        )
        .bind(username)
        .bind(email)
        .bind(password_hash)
        .bind(display_name)
        .bind(org_id)
        .bind(is_active)
        .fetch_one(self.pool)
        .await
    }

    /// Update a user's last login timestamp.
    pub async fn update_last_login(&self, id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE users SET last_login_at = NOW() WHERE id = $1")
            .bind(id)
            .execute(self.pool)
            .await
            .map(|_| ())
    }

    /// Get username by user ID.
    pub async fn get_username(&self, id: Uuid) -> Result<Option<String>, sqlx::Error> {
        sqlx::query_scalar("SELECT username FROM users WHERE id = $1")
            .bind(id)
            .fetch_optional(self.pool)
            .await
    }

    /// Get multiple users by IDs that have not been soft-deleted.
    pub async fn get_by_ids(&self, ids: &[Uuid]) -> Result<Vec<User>, sqlx::Error> {
        sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = ANY($1) AND deleted_at IS NULL")
            .bind(ids)
            .fetch_all(self.pool)
            .await
    }

    /// List users with optional org filter and optional search term.
    pub async fn list_users(
        &self,
        org_id: Option<Uuid>,
        search_term: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<User>, sqlx::Error> {
        match (org_id, search_term) {
            (None, Some(term)) => {
                let like = format!("%{}%", term);
                sqlx::query_as::<_, User>(
                    "SELECT * FROM users WHERE deleted_at IS NULL AND (username ILIKE $1 OR display_name ILIKE $1) ORDER BY created_at DESC LIMIT $2 OFFSET $3"
                )
                .bind(like)
                .bind(limit)
                .bind(offset)
                .fetch_all(self.pool)
                .await
            }
            (None, None) => {
                sqlx::query_as::<_, User>(
                    "SELECT * FROM users WHERE deleted_at IS NULL ORDER BY created_at DESC LIMIT $1 OFFSET $2"
                )
                .bind(limit)
                .bind(offset)
                .fetch_all(self.pool)
                .await
            }
            (Some(org_id), Some(term)) => {
                let like = format!("%{}%", term);
                sqlx::query_as::<_, User>(
                    "SELECT * FROM users WHERE org_id = $1 AND deleted_at IS NULL AND (username ILIKE $2 OR display_name ILIKE $2) ORDER BY created_at DESC LIMIT $3 OFFSET $4"
                )
                .bind(org_id)
                .bind(like)
                .bind(limit)
                .bind(offset)
                .fetch_all(self.pool)
                .await
            }
            (Some(org_id), None) => {
                sqlx::query_as::<_, User>(
                    "SELECT * FROM users WHERE org_id = $1 AND deleted_at IS NULL ORDER BY created_at DESC LIMIT $2 OFFSET $3"
                )
                .bind(org_id)
                .bind(limit)
                .bind(offset)
                .fetch_all(self.pool)
                .await
            }
        }
    }

    /// Search active users by username or email (ILIKE match).
    pub async fn search_active(&self, query: &str, limit: i64) -> Result<Vec<User>, sqlx::Error> {
        let like = format!("%{}%", query);
        sqlx::query_as::<_, User>(
            "SELECT * FROM users WHERE (username ILIKE $1 OR email ILIKE $1) AND is_active = true ORDER BY username ASC LIMIT $2"
        )
        .bind(like)
        .bind(limit)
        .fetch_all(self.pool)
        .await
    }

    /// Search active team members by username or email (ILIKE match).
    pub async fn search_team_members(
        &self,
        team_id: Uuid,
        query: &str,
        limit: i64,
    ) -> Result<Vec<User>, sqlx::Error> {
        let like = format!("%{}%", query);
        sqlx::query_as::<_, User>(
            r#"
            SELECT u.*
            FROM users u
            JOIN team_members tm ON u.id = tm.user_id
            WHERE tm.team_id = $1
              AND (u.username ILIKE $2 OR u.email ILIKE $2)
              AND u.is_active = true
            ORDER BY u.username ASC
            LIMIT $3
            "#,
        )
        .bind(team_id)
        .bind(like)
        .bind(limit)
        .fetch_all(self.pool)
        .await
    }

    /// Search active channel members by username or email (ILIKE match).
    pub async fn search_channel_members(
        &self,
        channel_id: Uuid,
        query: &str,
        limit: i64,
    ) -> Result<Vec<User>, sqlx::Error> {
        let like = format!("%{}%", query);
        sqlx::query_as::<_, User>(
            r#"
            SELECT u.*
            FROM users u
            JOIN channel_members cm ON u.id = cm.user_id
            WHERE cm.channel_id = $1
              AND (u.username ILIKE $2 OR u.email ILIKE $2)
              AND u.is_active = true
            ORDER BY u.username ASC
            LIMIT $3
            "#,
        )
        .bind(channel_id)
        .bind(like)
        .bind(limit)
        .fetch_all(self.pool)
        .await
    }

    /// Update a user's username.
    pub async fn update_username(&self, id: Uuid, username: &str) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE users SET username = $1 WHERE id = $2")
            .bind(username)
            .bind(id)
            .execute(self.pool)
            .await
            .map(|_| ())
    }

    /// Update a user's custom status JSON.
    pub async fn update_custom_status(
        &self,
        id: Uuid,
        custom_status: &serde_json::Value,
    ) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE users SET custom_status = $1 WHERE id = $2")
            .bind(custom_status)
            .bind(id)
            .execute(self.pool)
            .await
            .map(|_| ())
    }

    /// Update a user's display name.
    pub async fn update_display_name(
        &self,
        id: Uuid,
        display_name: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE users SET display_name = $1 WHERE id = $2")
            .bind(display_name)
            .bind(id)
            .execute(self.pool)
            .await
            .map(|_| ())
    }

    /// Update a user's avatar URL.
    pub async fn update_avatar_url(&self, id: Uuid, avatar_url: &str) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE users SET avatar_url = $1 WHERE id = $2")
            .bind(avatar_url)
            .bind(id)
            .execute(self.pool)
            .await
            .map(|_| ())
    }

    /// Update a user's password hash.
    pub async fn update_password_hash(
        &self,
        id: Uuid,
        password_hash: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE users SET password_hash = $1, updated_at = NOW() WHERE id = $2")
            .bind(password_hash)
            .bind(id)
            .execute(self.pool)
            .await
            .map(|_| ())
    }

    /// Update a user's role.
    pub async fn update_role(&self, id: Uuid, role: &str) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE users SET role = $1 WHERE id = $2")
            .bind(role)
            .bind(id)
            .execute(self.pool)
            .await
            .map(|_| ())
    }

    /// Update a user's active status.
    pub async fn update_active(&self, id: Uuid, active: bool) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE users SET is_active = $1 WHERE id = $2")
            .bind(active)
            .bind(id)
            .execute(self.pool)
            .await
            .map(|_| ())
    }

    /// Mark a user as a bot.
    pub async fn update_is_bot(&self, id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE users SET is_bot = true WHERE id = $1")
            .bind(id)
            .execute(self.pool)
            .await
            .map(|_| ())
    }

    /// Update a user's presence and presence_manual fields.
    pub async fn update_presence(
        &self,
        id: Uuid,
        presence: &str,
        presence_manual: bool,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE users
            SET presence = $2, presence_manual = $3
            WHERE id = $1
            "#,
        )
        .bind(id)
        .bind(presence)
        .bind(presence_manual)
        .execute(self.pool)
        .await
        .map(|_| ())
    }

    /// Update a user's status fields (status_text, status_emoji, status_expires_at, custom_status).
    pub async fn update_status_fields(
        &self,
        id: Uuid,
        status_text: Option<&str>,
        status_emoji: Option<&str>,
        status_expires_at: Option<DateTime<Utc>>,
        custom_status: Value,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE users
            SET status_text = $2,
                status_emoji = $3,
                status_expires_at = $4,
                custom_status = $5,
                updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(id)
        .bind(status_text)
        .bind(status_emoji)
        .bind(status_expires_at)
        .bind(custom_status)
        .execute(self.pool)
        .await
        .map(|_| ())
    }

    /// Clear a user's status fields.
    pub async fn clear_status(&self, id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE users SET status_text = NULL, status_emoji = NULL, status_expires_at = NULL, custom_status = 'null'::jsonb, updated_at = NOW() WHERE id = $1"
        )
        .bind(id)
        .execute(self.pool)
        .await
        .map(|_| ())
    }

    /// Clear a user's status fields and return the updated presence data.
    pub async fn clear_status_returning(
        &self,
        id: Uuid,
    ) -> Result<
        (
            String,
            bool,
            Option<chrono::DateTime<Utc>>,
            Option<String>,
            Option<String>,
            Option<chrono::DateTime<Utc>>,
        ),
        sqlx::Error,
    > {
        sqlx::query_as::<_, (String, bool, Option<chrono::DateTime<Utc>>, Option<String>, Option<String>, Option<chrono::DateTime<Utc>>)>(
            "UPDATE users SET status_text = NULL, status_emoji = NULL, status_expires_at = NULL, custom_status = 'null'::jsonb, updated_at = NOW() WHERE id = $1 RETURNING presence, COALESCE(presence_manual, false), last_login_at, status_text, status_emoji, status_expires_at"
        )
        .bind(id)
        .fetch_one(self.pool)
        .await
    }

    /// Get user status fields (presence, manual, last_login_at, status_text, status_emoji, status_expires_at).
    pub async fn get_user_status_fields(
        &self,
        id: Uuid,
    ) -> Result<
        Option<(
            String,
            bool,
            Option<chrono::DateTime<Utc>>,
            Option<String>,
            Option<String>,
            Option<chrono::DateTime<Utc>>,
        )>,
        sqlx::Error,
    > {
        sqlx::query_as::<_, (String, bool, Option<chrono::DateTime<Utc>>, Option<String>, Option<String>, Option<chrono::DateTime<Utc>>)>(
            "SELECT presence, COALESCE(presence_manual, false), last_login_at, status_text, status_emoji, status_expires_at FROM users WHERE id = $1"
        )
        .bind(id)
        .fetch_optional(self.pool)
        .await
    }

    /// Get presence values for multiple users by IDs.
    pub async fn get_presences_by_ids(
        &self,
        ids: &[Uuid],
    ) -> Result<Vec<(Uuid, String)>, sqlx::Error> {
        sqlx::query_as::<_, (Uuid, String)>(
            r#"
            SELECT id, presence
            FROM users
            WHERE id = ANY($1)
            "#,
        )
        .bind(ids)
        .fetch_all(self.pool)
        .await
    }

    /// Upsert a thread membership for a user.
    pub async fn upsert_thread_membership(
        &self,
        user_id: Uuid,
        post_id: Uuid,
        last_read_at: DateTime<Utc>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(r#"
            INSERT INTO thread_memberships (user_id, post_id, last_read_at, unread_replies_count, mention_count)
            VALUES ($1, $2, $3, 0, 0)
            ON CONFLICT (user_id, post_id) DO UPDATE SET
                last_read_at = $3,
                unread_replies_count = 0,
                mention_count = 0,
                updated_at = NOW()
        "#)
        .bind(user_id)
        .bind(post_id)
        .bind(last_read_at)
        .execute(self.pool)
        .await
        .map(|_| ())
    }

    // ============ Preferences ============

    /// Get or create user preferences.
    pub async fn get_or_create_preferences(
        &self,
        user_id: Uuid,
    ) -> Result<crate::models::UserPreferences, sqlx::Error> {
        let prefs = sqlx::query_as::<_, crate::models::UserPreferences>(
            "SELECT * FROM user_preferences WHERE user_id = $1",
        )
        .bind(user_id)
        .fetch_optional(self.pool)
        .await?;

        match prefs {
            Some(p) => Ok(p),
            None => {
                sqlx::query_as::<_, crate::models::UserPreferences>(
                    r#"INSERT INTO user_preferences (user_id) VALUES ($1) RETURNING *"#,
                )
                .bind(user_id)
                .fetch_one(self.pool)
                .await
            }
        }
    }

    /// Upsert user preferences.
    pub async fn upsert_preferences(
        &self,
        user_id: Uuid,
        payload: &crate::models::UpdatePreferences,
    ) -> Result<crate::models::UserPreferences, sqlx::Error> {
        sqlx::query_as::<_, crate::models::UserPreferences>(
            r#"
            INSERT INTO user_preferences (
                user_id, notify_desktop, notify_push, notify_email, notify_sounds,
                dnd_enabled, message_display, sidebar_behavior, time_format, mention_keywords,
                collapsed_reply_threads, use_military_time, teammate_name_display,
                availability_status_visible, show_last_active_time, timezone,
                link_previews_enabled, image_previews_enabled, click_to_reply,
                channel_display_mode, quick_reactions_enabled, emoji_picker_enabled, language,
                group_unread_channels, limit_visible_dms_gms,
                send_on_ctrl_enter, enable_post_formatting, enable_join_leave_messages,
                enable_performance_debugging, unread_scroll_position, sync_drafts
            )
            VALUES (
                $1, COALESCE($2, 'all'), COALESCE($3, 'all'), COALESCE($4, 'none'), COALESCE($5, true),
                COALESCE($6, false), COALESCE($7, 'standard'), COALESCE($8, 'unreads_first'), COALESCE($9, '12h'), $10,
                $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22, $23,
                $24, $25, $26, $27, $28, $29, $30, $31
            )
            ON CONFLICT (user_id) DO UPDATE SET
                notify_desktop = COALESCE($2, user_preferences.notify_desktop),
                notify_push = COALESCE($3, user_preferences.notify_push),
                notify_email = COALESCE($4, user_preferences.notify_email),
                notify_sounds = COALESCE($5, user_preferences.notify_sounds),
                dnd_enabled = COALESCE($6, user_preferences.dnd_enabled),
                message_display = COALESCE($7, user_preferences.message_display),
                sidebar_behavior = COALESCE($8, user_preferences.sidebar_behavior),
                time_format = COALESCE($9, user_preferences.time_format),
                mention_keywords = COALESCE($10, user_preferences.mention_keywords),
                collapsed_reply_threads = COALESCE($11, user_preferences.collapsed_reply_threads),
                use_military_time = COALESCE($12, user_preferences.use_military_time),
                teammate_name_display = COALESCE($13, user_preferences.teammate_name_display),
                availability_status_visible = COALESCE($14, user_preferences.availability_status_visible),
                show_last_active_time = COALESCE($15, user_preferences.show_last_active_time),
                timezone = COALESCE($16, user_preferences.timezone),
                link_previews_enabled = COALESCE($17, user_preferences.link_previews_enabled),
                image_previews_enabled = COALESCE($18, user_preferences.image_previews_enabled),
                click_to_reply = COALESCE($19, user_preferences.click_to_reply),
                channel_display_mode = COALESCE($20, user_preferences.channel_display_mode),
                quick_reactions_enabled = COALESCE($21, user_preferences.quick_reactions_enabled),
                emoji_picker_enabled = COALESCE($22, user_preferences.emoji_picker_enabled),
                language = COALESCE($23, user_preferences.language),
                group_unread_channels = COALESCE($24, user_preferences.group_unread_channels),
                limit_visible_dms_gms = COALESCE($25, user_preferences.limit_visible_dms_gms),
                send_on_ctrl_enter = COALESCE($26, user_preferences.send_on_ctrl_enter),
                enable_post_formatting = COALESCE($27, user_preferences.enable_post_formatting),
                enable_join_leave_messages = COALESCE($28, user_preferences.enable_join_leave_messages),
                enable_performance_debugging = COALESCE($29, user_preferences.enable_performance_debugging),
                unread_scroll_position = COALESCE($30, user_preferences.unread_scroll_position),
                sync_drafts = COALESCE($31, user_preferences.sync_drafts),
                updated_at = NOW()
            RETURNING *
            "#
        )
        .bind(user_id)
        .bind(&payload.notify_desktop)
        .bind(&payload.notify_push)
        .bind(&payload.notify_email)
        .bind(payload.notify_sounds)
        .bind(payload.dnd_enabled)
        .bind(&payload.message_display)
        .bind(&payload.sidebar_behavior)
        .bind(&payload.time_format)
        .bind(&payload.mention_keywords)
        .bind(payload.collapsed_reply_threads)
        .bind(payload.use_military_time)
        .bind(&payload.teammate_name_display)
        .bind(payload.availability_status_visible)
        .bind(payload.show_last_active_time)
        .bind(&payload.timezone)
        .bind(payload.link_previews_enabled)
        .bind(payload.image_previews_enabled)
        .bind(payload.click_to_reply)
        .bind(&payload.channel_display_mode)
        .bind(payload.quick_reactions_enabled)
        .bind(payload.emoji_picker_enabled)
        .bind(&payload.language)
        .bind(&payload.group_unread_channels)
        .bind(&payload.limit_visible_dms_gms)
        .bind(payload.send_on_ctrl_enter)
        .bind(payload.enable_post_formatting)
        .bind(payload.enable_join_leave_messages)
        .bind(payload.enable_performance_debugging)
        .bind(&payload.unread_scroll_position)
        .bind(payload.sync_drafts)
        .fetch_one(self.pool)
        .await
    }

    // ============ Status Presets ============

    /// List status presets for a user (including defaults).
    pub async fn list_status_presets(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<crate::models::StatusPreset>, sqlx::Error> {
        sqlx::query_as::<_, crate::models::StatusPreset>(
            "SELECT * FROM status_presets WHERE user_id IS NULL OR user_id = $1 ORDER BY is_default DESC, sort_order"
        )
        .bind(user_id)
        .fetch_all(self.pool)
        .await
    }

    /// Create a custom status preset.
    pub async fn create_status_preset(
        &self,
        user_id: Uuid,
        emoji: &str,
        text: &str,
        duration_minutes: Option<i32>,
    ) -> Result<crate::models::StatusPreset, sqlx::Error> {
        sqlx::query_as::<_, crate::models::StatusPreset>(
            r#"
            INSERT INTO status_presets (user_id, emoji, text, duration_minutes, sort_order)
            VALUES ($1, $2, $3, $4, (SELECT COALESCE(MAX(sort_order), 0) + 1 FROM status_presets WHERE user_id = $1))
            RETURNING *
            "#
        )
        .bind(user_id)
        .bind(emoji)
        .bind(text)
        .bind(duration_minutes)
        .fetch_one(self.pool)
        .await
    }

    /// Delete a custom status preset.
    pub async fn delete_status_preset(
        &self,
        preset_id: Uuid,
        user_id: Uuid,
    ) -> Result<u64, sqlx::Error> {
        sqlx::query(
            "DELETE FROM status_presets WHERE id = $1 AND user_id = $2 AND is_default = false",
        )
        .bind(preset_id)
        .bind(user_id)
        .execute(self.pool)
        .await
        .map(|r| r.rows_affected())
    }

    // ============ Channel Notifications ============

    /// Get channel notification settings for a user.
    pub async fn get_channel_notification(
        &self,
        user_id: Uuid,
        channel_id: Uuid,
    ) -> Result<Option<crate::models::ChannelNotificationSetting>, sqlx::Error> {
        sqlx::query_as::<_, crate::models::ChannelNotificationSetting>(
            "SELECT * FROM channel_notification_settings WHERE user_id = $1 AND channel_id = $2",
        )
        .bind(user_id)
        .bind(channel_id)
        .fetch_optional(self.pool)
        .await
    }

    /// Upsert channel notification settings for a user.
    pub async fn upsert_channel_notification(
        &self,
        user_id: Uuid,
        channel_id: Uuid,
        payload: &crate::models::UpdateChannelNotification,
    ) -> Result<crate::models::ChannelNotificationSetting, sqlx::Error> {
        sqlx::query_as::<_, crate::models::ChannelNotificationSetting>(
            r#"
            INSERT INTO channel_notification_settings (user_id, channel_id, notify_level, is_muted, mute_until)
            VALUES ($1, $2, COALESCE($3, 'default'), COALESCE($4, false), $5)
            ON CONFLICT (user_id, channel_id) DO UPDATE SET
                notify_level = COALESCE($3, channel_notification_settings.notify_level),
                is_muted = COALESCE($4, channel_notification_settings.is_muted),
                mute_until = COALESCE($5, channel_notification_settings.mute_until),
                updated_at = NOW()
            RETURNING *
            "#
        )
        .bind(user_id)
        .bind(channel_id)
        .bind(&payload.notify_level)
        .bind(payload.is_muted)
        .bind(payload.mute_until)
        .fetch_one(self.pool)
        .await
    }

    /// Seed default Mattermost preferences for a new user.
    pub async fn seed_default_preferences(&self, user_id: Uuid) -> Result<(), sqlx::Error> {
        let default_theme = serde_json::json!({
            "sidebarBg": "#1e1e2e",
            "sidebarText": "#cdd6f4",
            "sidebarUnreadText": "#f38ba8",
            "sidebarTextHoverBg": "#313244",
            "sidebarTextActiveBorder": "#89b4fa",
            "sidebarTextActiveColor": "#89b4fa",
            "sidebarHeaderBg": "#181825",
            "sidebarHeaderTextColor": "#cdd6f4",
            "onlineIndicator": "#a6e3a1",
            "awayIndicator": "#f9e2af",
            "dndIndicator": "#f38ba8",
            "mentionBg": "#f38ba8",
            "mentionColor": "#1e1e2e",
            "centerChannelBg": "#1e1e2e",
            "centerChannelColor": "#cdd6f4",
            "newMessageSeparator": "#89b4fa",
            "linkColor": "#89b4fa",
            "buttonBg": "#89b4fa",
            "buttonColor": "#1e1e2e",
            "errorTextColor": "#da6c6e",
            "mentionHighlightBg": "#0d6e6e",
            "mentionHighlightLink": "#a4f4f4",
            "codeTheme": "monokai"
        })
        .to_string();

        sqlx::query(
            r#"
            INSERT INTO mattermost_preferences (user_id, category, name, value)
            VALUES ($1, 'theme', '', $2)
            ON CONFLICT (user_id, category, name) DO NOTHING
            "#,
        )
        .bind(user_id)
        .bind(default_theme)
        .execute(self.pool)
        .await?;

        let display_prefs = [
            ("use_military_time", "false"),
            ("timezone", "Auto"),
            ("collapsed_reply_threads", "on"),
        ];

        for (name, value) in display_prefs {
            sqlx::query(
                r#"
                INSERT INTO mattermost_preferences (user_id, category, name, value)
                VALUES ($1, 'display_settings', $2, $3)
                ON CONFLICT (user_id, category, name) DO NOTHING
                "#,
            )
            .bind(user_id)
            .bind(name)
            .bind(value)
            .execute(self.pool)
            .await?;
        }

        let notify_prefs = [
            ("desktop", "mention"),
            ("push", "mention"),
            ("email", "true"),
        ];

        for (name, value) in notify_prefs {
            sqlx::query(
                r#"
                INSERT INTO mattermost_preferences (user_id, category, name, value)
                VALUES ($1, 'notifications', $2, $3)
                ON CONFLICT (user_id, category, name) DO NOTHING
                "#,
            )
            .bind(user_id)
            .bind(name)
            .bind(value)
            .execute(self.pool)
            .await?;
        }

        sqlx::query(
            r#"
            INSERT INTO mattermost_preferences (user_id, category, name, value)
            VALUES ($1, 'sidebar_settings', 'show_unread_section', 'true')
            ON CONFLICT (user_id, category, name) DO NOTHING
            "#,
        )
        .bind(user_id)
        .execute(self.pool)
        .await?;

        Ok(())
    }

    /// Get a user's display name, falling back to username.
    pub async fn get_display_name_or_username(&self, id: Uuid) -> Result<String, sqlx::Error> {
        let name: Option<String> = sqlx::query_scalar(
            "SELECT COALESCE(NULLIF(display_name, ''), username) FROM users WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(self.pool)
        .await?;

        Ok(name.unwrap_or_else(|| "Direct Message".to_string()))
    }

    /// Get status snapshot fields for multiple users by IDs.
    pub async fn get_statuses_by_ids(
        &self,
        ids: &[Uuid],
    ) -> Result<Vec<(Uuid, String, bool, Option<DateTime<Utc>>)>, sqlx::Error> {
        sqlx::query_as::<_, (Uuid, String, bool, Option<DateTime<Utc>>)>(
            r#"
            SELECT id, presence, COALESCE(presence_manual, false), last_login_at
            FROM users
            WHERE id = ANY($1)
            "#,
        )
        .bind(ids)
        .fetch_all(self.pool)
        .await
    }

    /// Clear expired custom status for a single user if needed. Returns whether a row was updated.
    pub async fn clear_expired_custom_status_if_needed(
        &self,
        id: Uuid,
    ) -> Result<bool, sqlx::Error> {
        let result = sqlx::query(
            r#"
            UPDATE users
            SET status_text = NULL,
                status_emoji = NULL,
                status_expires_at = NULL,
                custom_status = 'null'::jsonb
            WHERE id = $1
              AND status_expires_at IS NOT NULL
              AND status_expires_at <= NOW()
            "#,
        )
        .bind(id)
        .execute(self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Clear expired custom statuses for multiple users. Returns rows affected.
    pub async fn clear_expired_custom_statuses_for_users(
        &self,
        ids: &[Uuid],
    ) -> Result<i64, sqlx::Error> {
        let result = sqlx::query(
            r#"
            UPDATE users
            SET status_text = NULL,
                status_emoji = NULL,
                status_expires_at = NULL,
                custom_status = 'null'::jsonb
            WHERE id = ANY($1)
              AND status_expires_at IS NOT NULL
              AND status_expires_at <= NOW()
            "#,
        )
        .bind(ids)
        .execute(self.pool)
        .await?;

        Ok(result.rows_affected() as i64)
    }

    /// Clear all expired custom statuses and return affected user IDs.
    pub async fn clear_expired_custom_statuses(&self) -> Result<Vec<Uuid>, sqlx::Error> {
        let rows: Vec<(Uuid,)> = sqlx::query_as(
            r#"
            UPDATE users
            SET status_text = NULL,
                status_emoji = NULL,
                status_expires_at = NULL,
                custom_status = 'null'::jsonb
            WHERE status_expires_at IS NOT NULL
              AND status_expires_at <= NOW()
            RETURNING id
            "#,
        )
        .fetch_all(self.pool)
        .await?;

        Ok(rows.into_iter().map(|(id,)| id).collect())
    }

    /// Get recent custom statuses from preferences.
    pub async fn get_recent_custom_status_value(
        &self,
        user_id: Uuid,
    ) -> Result<Option<String>, sqlx::Error> {
        sqlx::query_scalar(
            r#"
            SELECT value FROM mattermost_preferences
            WHERE user_id = $1 AND category = 'display_settings' AND name = 'recent_custom_status'
            "#,
        )
        .bind(user_id)
        .fetch_optional(self.pool)
        .await
    }

    /// Save recent custom statuses to preferences.
    pub async fn save_recent_custom_status_value(
        &self,
        user_id: Uuid,
        value: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO mattermost_preferences (user_id, category, name, value)
            VALUES ($1, 'display_settings', 'recent_custom_status', $2)
            ON CONFLICT (user_id, category, name) DO UPDATE SET value = $2
            "#,
        )
        .bind(user_id)
        .bind(value)
        .execute(self.pool)
        .await?;

        Ok(())
    }

    /// Get active users by IDs.
    pub async fn get_active_by_ids(&self, ids: &[Uuid]) -> Result<Vec<User>, sqlx::Error> {
        sqlx::query_as::<_, User>(
            "SELECT * FROM users WHERE id = ANY($1) AND is_active = true AND deleted_at IS NULL",
        )
        .bind(ids)
        .fetch_all(self.pool)
        .await
    }

    /// Get users by usernames.
    pub async fn get_by_usernames(&self, usernames: &[String]) -> Result<Vec<User>, sqlx::Error> {
        sqlx::query_as::<_, User>("SELECT * FROM users WHERE username = ANY($1)")
            .bind(usernames)
            .fetch_all(self.pool)
            .await
    }

    /// Check if a user exists.
    pub async fn exists(&self, id: Uuid) -> Result<bool, sqlx::Error> {
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM users WHERE id = $1)")
            .bind(id)
            .fetch_one(self.pool)
            .await
    }

    /// List active members of a channel with pagination.
    pub async fn list_channel_members_paginated(
        &self,
        channel_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<User>, sqlx::Error> {
        sqlx::query_as::<_, User>(
            r#"
            SELECT u.*
            FROM users u
            JOIN channel_members cm ON u.id = cm.user_id
            WHERE cm.channel_id = $1 AND u.is_active = true
            ORDER BY u.username ASC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(channel_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(self.pool)
        .await
    }

    /// Get IDs of all users that share a channel with the given user.
    pub async fn get_known_user_ids(&self, user_id: Uuid) -> Result<Vec<Uuid>, sqlx::Error> {
        sqlx::query_scalar(
            r#"
            SELECT DISTINCT cm2.user_id
            FROM channel_members cm
            JOIN channel_members cm2 ON cm.channel_id = cm2.channel_id
            WHERE cm.user_id = $1 AND cm2.user_id != $1
            "#,
        )
        .bind(user_id)
        .fetch_all(self.pool)
        .await
    }

    /// Get all preferences for a user.
    pub async fn get_preferences(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<(Uuid, String, String, String)>, sqlx::Error> {
        sqlx::query_as(
            "SELECT user_id, category, name, value FROM mattermost_preferences WHERE user_id = $1",
        )
        .bind(user_id)
        .fetch_all(self.pool)
        .await
    }

    /// Get preferences for a user by category.
    pub async fn get_preferences_by_category(
        &self,
        user_id: Uuid,
        category: &str,
    ) -> Result<Vec<(Uuid, String, String, String)>, sqlx::Error> {
        sqlx::query_as(
            "SELECT user_id, category, name, value FROM mattermost_preferences WHERE user_id = $1 AND category = $2",
        )
        .bind(user_id)
        .bind(category)
        .fetch_all(self.pool)
        .await
    }

    /// Get a single preference by user, category, and name.
    pub async fn get_preference(
        &self,
        user_id: Uuid,
        category: &str,
        name: &str,
    ) -> Result<Option<(Uuid, String, String, String)>, sqlx::Error> {
        sqlx::query_as(
            "SELECT user_id, category, name, value FROM mattermost_preferences WHERE user_id = $1 AND category = $2 AND name = $3",
        )
        .bind(user_id)
        .bind(category)
        .bind(name)
        .fetch_optional(self.pool)
        .await
    }

    /// Upsert a single preference.
    pub async fn upsert_preference(
        &self,
        user_id: Uuid,
        category: &str,
        name: &str,
        value: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO mattermost_preferences (user_id, category, name, value)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (user_id, category, name)
            DO UPDATE SET value = $4
            "#,
        )
        .bind(user_id)
        .bind(category)
        .bind(name)
        .bind(value)
        .execute(self.pool)
        .await?;

        Ok(())
    }

    /// Delete a single preference.
    pub async fn delete_preference(
        &self,
        user_id: Uuid,
        category: &str,
        name: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "DELETE FROM mattermost_preferences WHERE user_id = $1 AND category = $2 AND name = $3",
        )
        .bind(user_id)
        .bind(category)
        .bind(name)
        .execute(self.pool)
        .await?;

        Ok(())
    }

    /// List active members of a team with pagination.
    pub async fn list_team_members_paginated(
        &self,
        team_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<User>, sqlx::Error> {
        sqlx::query_as::<_, User>(
            r#"
            SELECT u.*
            FROM users u
            JOIN team_members tm ON u.id = tm.user_id
            WHERE tm.team_id = $1 AND u.is_active = true
            ORDER BY u.username ASC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(team_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(self.pool)
        .await
    }

    /// List active members of a team who are not members of a channel with pagination.
    pub async fn list_team_members_not_in_channel_paginated(
        &self,
        team_id: Uuid,
        channel_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<User>, sqlx::Error> {
        sqlx::query_as::<_, User>(
            r#"
            SELECT u.*
            FROM users u
            JOIN team_members tm ON u.id = tm.user_id
            LEFT JOIN channel_members cm ON u.id = cm.user_id AND cm.channel_id = $2
            WHERE tm.team_id = $1 AND cm.user_id IS NULL AND u.is_active = true
            ORDER BY u.username ASC
            LIMIT $3 OFFSET $4
            "#,
        )
        .bind(team_id)
        .bind(channel_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(self.pool)
        .await
    }

    /// List users who are not members of a team with pagination.
    pub async fn list_users_not_in_team_paginated(
        &self,
        org_id: Option<Uuid>,
        team_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<User>, sqlx::Error> {
        match org_id {
            Some(org_id) => {
                sqlx::query_as::<_, User>(
                    r#"
                    SELECT u.*
                    FROM users u
                    WHERE u.org_id = $1
                      AND u.deleted_at IS NULL
                      AND NOT EXISTS (
                          SELECT 1
                          FROM team_members tm
                          WHERE tm.team_id = $2 AND tm.user_id = u.id
                      )
                    ORDER BY u.created_at DESC
                    LIMIT $3 OFFSET $4
                    "#,
                )
                .bind(org_id)
                .bind(team_id)
                .bind(limit)
                .bind(offset)
                .fetch_all(self.pool)
                .await
            }
            None => {
                sqlx::query_as::<_, User>(
                    r#"
                    SELECT u.*
                    FROM users u
                    WHERE u.deleted_at IS NULL
                      AND NOT EXISTS (
                          SELECT 1
                          FROM team_members tm
                          WHERE tm.team_id = $1 AND tm.user_id = u.id
                      )
                    ORDER BY u.created_at DESC
                    LIMIT $2 OFFSET $3
                    "#,
                )
                .bind(team_id)
                .bind(limit)
                .bind(offset)
                .fetch_all(self.pool)
                .await
            }
        }
    }

    /// List users who do not belong to any team with pagination.
    pub async fn list_users_without_team_paginated(
        &self,
        org_id: Option<Uuid>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<User>, sqlx::Error> {
        match org_id {
            Some(org_id) => {
                sqlx::query_as::<_, User>(
                    r#"
                    SELECT u.*
                    FROM users u
                    WHERE u.org_id = $1
                      AND u.deleted_at IS NULL
                      AND NOT EXISTS (
                          SELECT 1 FROM team_members tm WHERE tm.user_id = u.id
                      )
                    ORDER BY u.created_at DESC
                    LIMIT $2 OFFSET $3
                    "#,
                )
                .bind(org_id)
                .bind(limit)
                .bind(offset)
                .fetch_all(self.pool)
                .await
            }
            None => {
                sqlx::query_as::<_, User>(
                    r#"
                    SELECT u.*
                    FROM users u
                    WHERE u.deleted_at IS NULL
                      AND NOT EXISTS (
                          SELECT 1 FROM team_members tm WHERE tm.user_id = u.id
                      )
                    ORDER BY u.created_at DESC
                    LIMIT $1 OFFSET $2
                    "#,
                )
                .bind(limit)
                .bind(offset)
                .fetch_all(self.pool)
                .await
            }
        }
    }

    /// Get username, avatar_url, and email for a user
    pub async fn get_username_avatar_email(
        &self,
        id: Uuid,
    ) -> Result<(String, Option<String>, String), sqlx::Error> {
        sqlx::query_as::<_, (String, Option<String>, String)>(
            "SELECT username, avatar_url, email FROM users WHERE id = $1",
        )
        .bind(id)
        .fetch_one(self.pool)
        .await
    }

    /// Get user IDs and usernames by username list
    pub async fn get_ids_by_usernames(
        &self,
        usernames: &[&str],
    ) -> Result<Vec<(Uuid, String)>, sqlx::Error> {
        sqlx::query_as::<_, (Uuid, String)>(
            "SELECT id, username FROM users WHERE username = ANY($1)",
        )
        .bind(usernames)
        .fetch_all(self.pool)
        .await
    }

    /// Get a user's role by ID
    pub async fn get_role_by_id(&self, id: Uuid) -> Result<String, sqlx::Error> {
        sqlx::query_scalar("SELECT role FROM users WHERE id = $1")
            .bind(id)
            .fetch_one(self.pool)
            .await
    }

    /// Check if a role has a specific permission
    pub async fn has_permission(&self, role: &str, permission: &str) -> Result<bool, sqlx::Error> {
        sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM role_permissions WHERE role = $1 AND permission_id = $2)",
        )
        .bind(role)
        .bind(permission)
        .fetch_one(self.pool)
        .await
    }

    /// Get the first bot user ID
    pub async fn get_bot_user_id(&self) -> Result<Option<Uuid>, sqlx::Error> {
        sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE is_bot = true LIMIT 1")
            .fetch_optional(self.pool)
            .await
    }
}
