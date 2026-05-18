use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::channel::ChannelType;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct GroupRow {
    pub id: Uuid,
    pub name: Option<String>,
    pub display_name: String,
    pub description: String,
    pub source: String,
    pub remote_id: Option<String>,
    pub allow_reference: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct GroupListRow {
    pub id: Uuid,
    pub name: Option<String>,
    pub display_name: String,
    pub description: String,
    pub source: String,
    pub remote_id: Option<String>,
    pub allow_reference: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub has_syncables: bool,
    pub member_count: i64,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct GroupSyncableRow {
    pub group_id: Uuid,
    pub syncable_type: String,
    pub syncable_id: Uuid,
    pub auto_add: bool,
    pub scheme_admin: bool,
    pub create_at: DateTime<Utc>,
    pub update_at: DateTime<Utc>,
    pub delete_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct TeamMetaRow {
    pub id: Uuid,
    pub name: String,
    pub display_name: Option<String>,
    pub is_public: bool,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ChannelMetaRow {
    pub id: Uuid,
    pub name: String,
    pub display_name: Option<String>,
    pub channel_type: ChannelType,
    pub team_id: Uuid,
    pub team_name: String,
    pub team_display_name: Option<String>,
    pub team_is_public: bool,
}

#[derive(Debug, Clone, sqlx::FromRow, PartialEq, Eq, Hash)]
pub struct TrackedMembershipRow {
    pub target_type: String,
    pub target_id: Uuid,
    pub user_id: Uuid,
}

pub struct GroupRepository<'a> {
    pool: &'a PgPool,
}

impl<'a> GroupRepository<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    // --- Group CRUD ---

    pub async fn get_group_row_by_id(
        &self,
        group_id: Uuid,
    ) -> Result<Option<GroupRow>, sqlx::Error> {
        sqlx::query_as::<_, GroupRow>(
            r#"
            SELECT id, name, display_name, description, source, remote_id, allow_reference, created_at, updated_at, deleted_at
            FROM groups
            WHERE id = $1
              AND deleted_at IS NULL
            "#,
        )
        .bind(group_id)
        .fetch_optional(self.pool)
        .await
    }

    pub async fn get_group_row_by_id_unchecked(
        &self,
        group_id: Uuid,
    ) -> Result<Option<GroupRow>, sqlx::Error> {
        sqlx::query_as::<_, GroupRow>(
            r#"
            SELECT id, name, display_name, description, source, remote_id, allow_reference, created_at, updated_at, deleted_at
            FROM groups
            WHERE id = $1
            "#,
        )
        .bind(group_id)
        .fetch_optional(self.pool)
        .await
    }

    pub async fn get_group_list_by_id(
        &self,
        group_id: Uuid,
    ) -> Result<Option<GroupListRow>, sqlx::Error> {
        sqlx::query_as::<_, GroupListRow>(
            r#"
            SELECT
                g.id,
                g.name,
                g.display_name,
                g.description,
                g.source,
                g.remote_id,
                g.allow_reference,
                g.created_at,
                g.updated_at,
                g.deleted_at,
                EXISTS(
                    SELECT 1
                    FROM group_syncables gs
                    WHERE gs.group_id = g.id
                      AND gs.delete_at IS NULL
                ) AS has_syncables,
                (
                    SELECT COUNT(*)
                    FROM group_members gm
                    WHERE gm.group_id = g.id
                ) AS member_count
            FROM groups g
            WHERE g.id = $1
            "#,
        )
        .bind(group_id)
        .fetch_optional(self.pool)
        .await
    }

    pub async fn list_groups(&self) -> Result<Vec<GroupListRow>, sqlx::Error> {
        sqlx::query_as::<_, GroupListRow>(
            r#"
            SELECT
                g.id,
                g.name,
                g.display_name,
                g.description,
                g.source,
                g.remote_id,
                g.allow_reference,
                g.created_at,
                g.updated_at,
                g.deleted_at,
                EXISTS(
                    SELECT 1
                    FROM group_syncables gs
                    WHERE gs.group_id = g.id
                      AND gs.delete_at IS NULL
                ) AS has_syncables,
                (
                    SELECT COUNT(*)
                    FROM group_members gm
                    WHERE gm.group_id = g.id
                ) AS member_count
            FROM groups g
            WHERE g.deleted_at IS NULL
            ORDER BY g.display_name ASC
            "#,
        )
        .fetch_all(self.pool)
        .await
    }

    pub async fn list_groups_by_names(
        &self,
        names: Vec<String>,
    ) -> Result<Vec<GroupListRow>, sqlx::Error> {
        sqlx::query_as::<_, GroupListRow>(
            r#"
            SELECT
                g.id,
                g.name,
                g.display_name,
                g.description,
                g.source,
                g.remote_id,
                g.allow_reference,
                g.created_at,
                g.updated_at,
                g.deleted_at,
                EXISTS(
                    SELECT 1
                    FROM group_syncables gs
                    WHERE gs.group_id = g.id
                      AND gs.delete_at IS NULL
                ) AS has_syncables,
                (
                    SELECT COUNT(*)
                    FROM group_members gm
                    WHERE gm.group_id = g.id
                ) AS member_count
            FROM groups g
            WHERE g.deleted_at IS NULL
              AND g.name = ANY($1)
            ORDER BY g.display_name ASC
            "#,
        )
        .bind(names)
        .fetch_all(self.pool)
        .await
    }

    pub async fn create_group_with_members(
        &self,
        name: Option<&str>,
        display_name: &str,
        description: String,
        source: String,
        allow_reference: bool,
        user_ids: Vec<Uuid>,
    ) -> Result<GroupRow, sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        let created: GroupRow = sqlx::query_as::<_, GroupRow>(
            r#"
            INSERT INTO groups (name, display_name, description, source, remote_id, allow_reference)
            VALUES ($1, $2, $3, $4, NULL, $5)
            RETURNING id, name, display_name, description, source, remote_id, allow_reference, created_at, updated_at, deleted_at
            "#,
        )
        .bind(name)
        .bind(display_name)
        .bind(description)
        .bind(source)
        .bind(allow_reference)
        .fetch_one(&mut *tx)
        .await?;

        for user_id in user_ids {
            sqlx::query(
                "INSERT INTO group_members (group_id, user_id) VALUES ($1, $2) ON CONFLICT (group_id, user_id) DO NOTHING",
            )
            .bind(created.id)
            .bind(user_id)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;

        Ok(created)
    }

    pub async fn update_group(
        &self,
        group_id: Uuid,
        name: Option<&str>,
        display_name: Option<&str>,
        description: Option<String>,
        allow_reference: Option<bool>,
    ) -> Result<Option<GroupListRow>, sqlx::Error> {
        sqlx::query_as::<_, GroupListRow>(
            r#"
            UPDATE groups
            SET
                name = COALESCE($2, name),
                display_name = COALESCE($3, display_name),
                description = COALESCE($4, description),
                allow_reference = COALESCE($5, allow_reference),
                updated_at = NOW()
            WHERE id = $1
            RETURNING
                id,
                name,
                display_name,
                description,
                source,
                remote_id,
                allow_reference,
                created_at,
                updated_at,
                deleted_at,
                EXISTS(
                    SELECT 1
                    FROM group_syncables gs
                    WHERE gs.group_id = groups.id
                      AND gs.delete_at IS NULL
                ) AS has_syncables,
                (
                    SELECT COUNT(*)
                    FROM group_members gm
                    WHERE gm.group_id = groups.id
                ) AS member_count
            "#,
        )
        .bind(group_id)
        .bind(name)
        .bind(display_name)
        .bind(description)
        .bind(allow_reference)
        .fetch_optional(self.pool)
        .await
    }

    pub async fn soft_delete_group(
        &self,
        group_id: Uuid,
    ) -> Result<Option<GroupListRow>, sqlx::Error> {
        sqlx::query_as::<_, GroupListRow>(
            r#"
            UPDATE groups
            SET deleted_at = NOW(), updated_at = NOW()
            WHERE id = $1
            RETURNING
                id,
                name,
                display_name,
                description,
                source,
                remote_id,
                allow_reference,
                created_at,
                updated_at,
                deleted_at,
                EXISTS(
                    SELECT 1
                    FROM group_syncables gs
                    WHERE gs.group_id = groups.id
                      AND gs.delete_at IS NULL
                ) AS has_syncables,
                (
                    SELECT COUNT(*)
                    FROM group_members gm
                    WHERE gm.group_id = groups.id
                ) AS member_count
            "#,
        )
        .bind(group_id)
        .fetch_optional(self.pool)
        .await
    }

    pub async fn restore_group(
        &self,
        group_id: Uuid,
    ) -> Result<Option<GroupListRow>, sqlx::Error> {
        sqlx::query_as::<_, GroupListRow>(
            r#"
            UPDATE groups
            SET deleted_at = NULL, updated_at = NOW()
            WHERE id = $1
            RETURNING
                id,
                name,
                display_name,
                description,
                source,
                remote_id,
                allow_reference,
                created_at,
                updated_at,
                deleted_at,
                EXISTS(
                    SELECT 1
                    FROM group_syncables gs
                    WHERE gs.group_id = groups.id
                      AND gs.delete_at IS NULL
                ) AS has_syncables,
                (
                    SELECT COUNT(*)
                    FROM group_members gm
                    WHERE gm.group_id = groups.id
                ) AS member_count
            "#,
        )
        .bind(group_id)
        .fetch_optional(self.pool)
        .await
    }

    // --- Group members ---

    pub async fn list_group_members(
        &self,
        group_id: Uuid,
    ) -> Result<Vec<(Uuid, DateTime<Utc>)>, sqlx::Error> {
        sqlx::query_as(
            "SELECT user_id, created_at FROM group_members WHERE group_id = $1 ORDER BY created_at ASC",
        )
        .bind(group_id)
        .fetch_all(self.pool)
        .await
    }

    pub async fn add_group_member(
        &self,
        group_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<DateTime<Utc>>, sqlx::Error> {
        sqlx::query_scalar(
            r#"
            INSERT INTO group_members (group_id, user_id)
            VALUES ($1, $2)
            ON CONFLICT (group_id, user_id) DO NOTHING
            RETURNING created_at
            "#,
        )
        .bind(group_id)
        .bind(user_id)
        .fetch_optional(self.pool)
        .await
    }

    pub async fn remove_group_member(
        &self,
        group_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<DateTime<Utc>>, sqlx::Error> {
        sqlx::query_scalar(
            r#"
            DELETE FROM group_members
            WHERE group_id = $1
              AND user_id = $2
            RETURNING created_at
            "#,
        )
        .bind(group_id)
        .bind(user_id)
        .fetch_optional(self.pool)
        .await
    }

    pub async fn count_group_members(&self, group_id: Uuid) -> Result<i64, sqlx::Error> {
        sqlx::query_scalar("SELECT COUNT(*) FROM group_members WHERE group_id = $1")
            .bind(group_id)
            .fetch_one(self.pool)
            .await
    }

    pub async fn list_group_user_ids(&self, group_id: Uuid) -> Result<Vec<Uuid>, sqlx::Error> {
        sqlx::query_scalar("SELECT user_id FROM group_members WHERE group_id = $1")
            .bind(group_id)
            .fetch_all(self.pool)
            .await
    }

    // --- Group syncables ---

    pub async fn list_group_syncables(
        &self,
        group_id: Uuid,
    ) -> Result<Vec<GroupSyncableRow>, sqlx::Error> {
        sqlx::query_as::<_, GroupSyncableRow>(
            r#"
            SELECT group_id, syncable_type, syncable_id, auto_add, scheme_admin, create_at, update_at, delete_at
            FROM group_syncables
            WHERE group_id = $1
              AND delete_at IS NULL
            ORDER BY create_at ASC
            "#,
        )
        .bind(group_id)
        .fetch_all(self.pool)
        .await
    }

    pub async fn list_group_syncables_by_type(
        &self,
        group_id: Uuid,
        syncable_type: &str,
    ) -> Result<Vec<GroupSyncableRow>, sqlx::Error> {
        sqlx::query_as::<_, GroupSyncableRow>(
            r#"
            SELECT group_id, syncable_type, syncable_id, auto_add, scheme_admin, create_at, update_at, delete_at
            FROM group_syncables
            WHERE group_id = $1
              AND syncable_type = $2
              AND delete_at IS NULL
            ORDER BY create_at ASC
            "#,
        )
        .bind(group_id)
        .bind(syncable_type)
        .fetch_all(self.pool)
        .await
    }

    pub async fn get_group_syncable(
        &self,
        group_id: Uuid,
        syncable_type: &str,
        syncable_id: Uuid,
    ) -> Result<Option<GroupSyncableRow>, sqlx::Error> {
        sqlx::query_as::<_, GroupSyncableRow>(
            r#"
            SELECT group_id, syncable_type, syncable_id, auto_add, scheme_admin, create_at, update_at, delete_at
            FROM group_syncables
            WHERE group_id = $1
              AND syncable_type = $2
              AND syncable_id = $3
              AND delete_at IS NULL
            "#,
        )
        .bind(group_id)
        .bind(syncable_type)
        .bind(syncable_id)
        .fetch_optional(self.pool)
        .await
    }

    pub async fn upsert_group_syncable(
        &self,
        group_id: Uuid,
        syncable_type: &str,
        syncable_id: Uuid,
        auto_add: bool,
        scheme_admin: bool,
    ) -> Result<GroupSyncableRow, sqlx::Error> {
        sqlx::query_as::<_, GroupSyncableRow>(
            r#"
            INSERT INTO group_syncables (group_id, syncable_type, syncable_id, auto_add, scheme_admin, delete_at)
            VALUES ($1, $2, $3, $4, $5, NULL)
            ON CONFLICT (group_id, syncable_type, syncable_id)
            DO UPDATE SET
                auto_add = EXCLUDED.auto_add,
                scheme_admin = EXCLUDED.scheme_admin,
                update_at = NOW(),
                delete_at = NULL
            RETURNING group_id, syncable_type, syncable_id, auto_add, scheme_admin, create_at, update_at, delete_at
            "#,
        )
        .bind(group_id)
        .bind(syncable_type)
        .bind(syncable_id)
        .bind(auto_add)
        .bind(scheme_admin)
        .fetch_one(self.pool)
        .await
    }

    pub async fn patch_group_syncable(
        &self,
        group_id: Uuid,
        syncable_type: &str,
        syncable_id: Uuid,
        auto_add: Option<bool>,
        scheme_admin: Option<bool>,
    ) -> Result<Option<GroupSyncableRow>, sqlx::Error> {
        sqlx::query_as::<_, GroupSyncableRow>(
            r#"
            UPDATE group_syncables
            SET
                auto_add = COALESCE($4, auto_add),
                scheme_admin = COALESCE($5, scheme_admin),
                update_at = NOW()
            WHERE group_id = $1
              AND syncable_type = $2
              AND syncable_id = $3
              AND delete_at IS NULL
            RETURNING group_id, syncable_type, syncable_id, auto_add, scheme_admin, create_at, update_at, delete_at
            "#,
        )
        .bind(group_id)
        .bind(syncable_type)
        .bind(syncable_id)
        .bind(auto_add)
        .bind(scheme_admin)
        .fetch_optional(self.pool)
        .await
    }

    pub async fn delete_group_syncable(
        &self,
        group_id: Uuid,
        syncable_type: &str,
        syncable_id: Uuid,
    ) -> Result<u64, sqlx::Error> {
        let result = sqlx::query(
            r#"
            DELETE FROM group_syncables
            WHERE group_id = $1
              AND syncable_type = $2
              AND syncable_id = $3
            "#,
        )
        .bind(group_id)
        .bind(syncable_type)
        .bind(syncable_id)
        .execute(self.pool)
        .await?;
        Ok(result.rows_affected())
    }

    // --- Group-team/channel linking ---

    pub async fn get_team_meta(
        &self,
        team_id: Uuid,
    ) -> Result<Option<TeamMetaRow>, sqlx::Error> {
        sqlx::query_as::<_, TeamMetaRow>(
            r#"
            SELECT id, name, display_name, is_public
            FROM teams
            WHERE id = $1
            "#,
        )
        .bind(team_id)
        .fetch_optional(self.pool)
        .await
    }

    pub async fn get_channel_meta(
        &self,
        channel_id: Uuid,
    ) -> Result<Option<ChannelMetaRow>, sqlx::Error> {
        sqlx::query_as::<_, ChannelMetaRow>(
            r#"
            SELECT
                c.id,
                c.name,
                c.display_name,
                c.type as channel_type,
                t.id as team_id,
                t.name as team_name,
                t.display_name as team_display_name,
                t.is_public as team_is_public
            FROM channels c
            JOIN teams t ON t.id = c.team_id
            WHERE c.id = $1
            "#,
        )
        .bind(channel_id)
        .fetch_optional(self.pool)
        .await
    }

    pub async fn team_exists(&self, team_id: Uuid) -> Result<bool, sqlx::Error> {
        sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM teams WHERE id = $1 AND deleted_at IS NULL)",
        )
        .bind(team_id)
        .fetch_one(self.pool)
        .await
    }

    pub async fn channel_exists(&self, channel_id: Uuid) -> Result<bool, sqlx::Error> {
        sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM channels WHERE id = $1 AND deleted_at IS NULL)",
        )
        .bind(channel_id)
        .fetch_one(self.pool)
        .await
    }

    pub async fn get_channel_team_id(
        &self,
        channel_id: Uuid,
    ) -> Result<Option<Uuid>, sqlx::Error> {
        sqlx::query_scalar("SELECT team_id FROM channels WHERE id = $1")
            .bind(channel_id)
            .fetch_optional(self.pool)
            .await
    }

    pub async fn is_team_admin_or_owner(
        &self,
        team_id: Uuid,
        user_id: Uuid,
    ) -> Result<bool, sqlx::Error> {
        sqlx::query_scalar(
            r#"
            SELECT EXISTS(
                SELECT 1
                FROM team_members
                WHERE team_id = $1
                  AND user_id = $2
                  AND role IN ('admin', 'owner', 'team_admin')
            )
            "#,
        )
        .bind(team_id)
        .bind(user_id)
        .fetch_one(self.pool)
        .await
    }

    pub async fn is_channel_admin(
        &self,
        channel_id: Uuid,
        user_id: Uuid,
    ) -> Result<bool, sqlx::Error> {
        sqlx::query_scalar(
            r#"
            SELECT EXISTS(
                SELECT 1
                FROM channel_members
                WHERE channel_id = $1
                  AND user_id = $2
                  AND role IN ('admin', 'channel_admin')
            )
            "#,
        )
        .bind(channel_id)
        .bind(user_id)
        .fetch_one(self.pool)
        .await
    }

    // --- Group membership tracking ---

    pub async fn delete_group_syncable_membership(
        &self,
        group_id: Uuid,
        syncable_type: &str,
        syncable_id: Uuid,
        target_type: &str,
        target_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            DELETE FROM group_syncable_memberships
            WHERE group_id = $1
              AND syncable_type = $2
              AND syncable_id = $3
              AND target_type = $4
              AND target_id = $5
              AND user_id = $6
            "#,
        )
        .bind(group_id)
        .bind(syncable_type)
        .bind(syncable_id)
        .bind(target_type)
        .bind(target_id)
        .bind(user_id)
        .execute(self.pool)
        .await?;
        Ok(())
    }

    pub async fn has_other_syncable_memberships(
        &self,
        target_type: &str,
        target_id: Uuid,
        user_id: Uuid,
    ) -> Result<bool, sqlx::Error> {
        sqlx::query_scalar(
            r#"
            SELECT EXISTS(
                SELECT 1
                FROM group_syncable_memberships
                WHERE target_type = $1
                  AND target_id = $2
                  AND user_id = $3
            )
            "#,
        )
        .bind(target_type)
        .bind(target_id)
        .bind(user_id)
        .fetch_one(self.pool)
        .await
    }

    pub async fn remove_team_member(
        &self,
        team_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM team_members WHERE team_id = $1 AND user_id = $2")
            .bind(team_id)
            .bind(user_id)
            .execute(self.pool)
            .await?;
        Ok(())
    }

    pub async fn remove_channel_member(
        &self,
        channel_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM channel_members WHERE channel_id = $1 AND user_id = $2")
            .bind(channel_id)
            .bind(user_id)
            .execute(self.pool)
            .await?;
        Ok(())
    }

    pub async fn ensure_team_member(
        &self,
        team_id: Uuid,
        user_id: Uuid,
        role: &str,
    ) -> Result<u64, sqlx::Error> {
        let result = sqlx::query(
            "INSERT INTO team_members (team_id, user_id, role) VALUES ($1, $2, $3) ON CONFLICT (team_id, user_id) DO NOTHING",
        )
        .bind(team_id)
        .bind(user_id)
        .bind(role)
        .execute(self.pool)
        .await?;
        Ok(result.rows_affected())
    }

    pub async fn ensure_channel_member(
        &self,
        channel_id: Uuid,
        user_id: Uuid,
        role: &str,
    ) -> Result<u64, sqlx::Error> {
        let result = sqlx::query(
            "INSERT INTO channel_members (channel_id, user_id, role) VALUES ($1, $2, $3) ON CONFLICT (channel_id, user_id) DO NOTHING",
        )
        .bind(channel_id)
        .bind(user_id)
        .bind(role)
        .execute(self.pool)
        .await?;
        Ok(result.rows_affected())
    }

    pub async fn list_group_syncable_memberships(
        &self,
        group_id: Uuid,
        syncable_type: &str,
        syncable_id: Uuid,
    ) -> Result<Vec<TrackedMembershipRow>, sqlx::Error> {
        sqlx::query_as::<_, TrackedMembershipRow>(
            r#"
            SELECT target_type, target_id, user_id
            FROM group_syncable_memberships
            WHERE group_id = $1
              AND syncable_type = $2
              AND syncable_id = $3
            "#,
        )
        .bind(group_id)
        .bind(syncable_type)
        .bind(syncable_id)
        .fetch_all(self.pool)
        .await
    }

    pub async fn insert_group_syncable_membership(
        &self,
        group_id: Uuid,
        syncable_type: &str,
        syncable_id: Uuid,
        target_type: &str,
        target_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO group_syncable_memberships
                (group_id, syncable_type, syncable_id, target_type, target_id, user_id)
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT DO NOTHING
            "#,
        )
        .bind(group_id)
        .bind(syncable_type)
        .bind(syncable_id)
        .bind(target_type)
        .bind(target_id)
        .bind(user_id)
        .execute(self.pool)
        .await?;
        Ok(())
    }
}
