use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::{Channel, Team, TeamMember, TeamMemberResponse};

pub struct TeamRepository<'a> {
    pool: &'a PgPool,
}

impl<'a> TeamRepository<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    /// List all teams.
    pub async fn list_teams(&self) -> Result<Vec<Team>, sqlx::Error> {
        sqlx::query_as::<_, Team>(
            r#"
            SELECT t.* FROM teams t
            ORDER BY t.name
            "#,
        )
        .fetch_all(self.pool)
        .await
    }

    /// Get a team by ID.
    pub async fn get_team_by_id(&self, id: Uuid) -> Result<Option<Team>, sqlx::Error> {
        sqlx::query_as::<_, Team>("SELECT * FROM teams WHERE id = $1")
            .bind(id)
            .fetch_optional(self.pool)
            .await
    }

    /// Create a new team.
    pub async fn create_team(
        &self,
        id: Uuid,
        org_id: Uuid,
        name: &str,
        display_name: Option<&str>,
        description: Option<&str>,
    ) -> Result<Team, sqlx::Error> {
        sqlx::query_as::<_, Team>(
            r#"
            INSERT INTO teams (id, org_id, name, display_name, description)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(org_id)
        .bind(name)
        .bind(display_name)
        .bind(description)
        .fetch_one(self.pool)
        .await
    }

    /// Update a team with optional fields.
    pub async fn update_team(
        &self,
        id: Uuid,
        name: Option<&str>,
        display_name: Option<&str>,
        description: Option<&str>,
        is_public: Option<bool>,
        allow_open_invite: Option<bool>,
    ) -> Result<Team, sqlx::Error> {
        sqlx::query_as::<_, Team>(
            r#"
            UPDATE teams SET
                name = COALESCE($1, name),
                display_name = COALESCE($2, display_name),
                description = COALESCE($3, description),
                is_public = COALESCE($4, is_public),
                allow_open_invite = COALESCE($5, allow_open_invite),
                updated_at = NOW()
            WHERE id = $6
            RETURNING *
            "#,
        )
        .bind(name)
        .bind(display_name)
        .bind(description)
        .bind(is_public)
        .bind(allow_open_invite)
        .bind(id)
        .fetch_one(self.pool)
        .await
    }

    /// Delete a team.
    pub async fn delete_team(&self, id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM teams WHERE id = $1")
            .bind(id)
            .execute(self.pool)
            .await?;
        Ok(())
    }

    /// List all teams a user belongs to.
    pub async fn list_teams_for_user(&self, user_id: Uuid) -> Result<Vec<Team>, sqlx::Error> {
        sqlx::query_as::<_, Team>(
            r#"
            SELECT t.* FROM teams t
            INNER JOIN team_members tm ON t.id = tm.team_id
            WHERE tm.user_id = $1
            ORDER BY t.name
            "#,
        )
        .bind(user_id)
        .fetch_all(self.pool)
        .await
    }

    /// Check if a user is a member of a team.
    pub async fn is_team_member(&self, team_id: Uuid, user_id: Uuid) -> Result<bool, sqlx::Error> {
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM team_members WHERE team_id = $1 AND user_id = $2)",
        )
        .bind(team_id)
        .bind(user_id)
        .fetch_one(self.pool)
        .await?;
        Ok(exists)
    }

    /// Get a specific team membership row.
    pub async fn get_team_member(
        &self,
        team_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<TeamMember>, sqlx::Error> {
        sqlx::query_as::<_, TeamMember>(
            "SELECT * FROM team_members WHERE team_id = $1 AND user_id = $2",
        )
        .bind(team_id)
        .bind(user_id)
        .fetch_optional(self.pool)
        .await
    }

    /// List team members with user details.
    pub async fn list_team_members(
        &self,
        team_id: Uuid,
    ) -> Result<Vec<TeamMemberResponse>, sqlx::Error> {
        sqlx::query_as::<_, TeamMemberResponse>(
            r#"
            SELECT tm.team_id, tm.user_id, tm.role, tm.created_at,
                   u.username, u.display_name, u.avatar_url, u.presence
            FROM team_members tm
            JOIN users u ON tm.user_id = u.id
            WHERE tm.team_id = $1
            ORDER BY u.username
            "#,
        )
        .bind(team_id)
        .fetch_all(self.pool)
        .await
    }

    /// Add a member to a team.
    pub async fn add_team_member(
        &self,
        team_id: Uuid,
        user_id: Uuid,
        role: &str,
    ) -> Result<TeamMember, sqlx::Error> {
        sqlx::query_as::<_, TeamMember>(
            r#"
            INSERT INTO team_members (team_id, user_id, role)
            VALUES ($1, $2, $3)
            RETURNING *
            "#,
        )
        .bind(team_id)
        .bind(user_id)
        .bind(role)
        .fetch_one(self.pool)
        .await
    }

    /// Remove a member from a team.
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

    /// Update a team member's role.
    pub async fn update_team_member_role(
        &self,
        team_id: Uuid,
        user_id: Uuid,
        role: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE team_members SET role = $1 WHERE team_id = $2 AND user_id = $3")
            .bind(role)
            .bind(team_id)
            .bind(user_id)
            .execute(self.pool)
            .await?;
        Ok(())
    }

    /// Get a member's role in a team.
    pub async fn get_member_role(
        &self,
        team_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<String>, sqlx::Error> {
        sqlx::query_scalar("SELECT role FROM team_members WHERE team_id = $1 AND user_id = $2")
            .bind(team_id)
            .bind(user_id)
            .fetch_optional(self.pool)
            .await
    }

    /// Update a team member's role and return the updated member.
    pub async fn update_team_member_role_returning(
        &self,
        team_id: Uuid,
        user_id: Uuid,
        role: &str,
    ) -> Result<Option<crate::models::TeamMember>, sqlx::Error> {
        sqlx::query_as::<_, crate::models::TeamMember>(
            "UPDATE team_members SET role = $1 WHERE team_id = $2 AND user_id = $3 RETURNING *",
        )
        .bind(role)
        .bind(team_id)
        .bind(user_id)
        .fetch_optional(self.pool)
        .await
    }

    /// List user IDs of all members in a team.
    pub async fn list_team_member_ids(&self, team_id: Uuid) -> Result<Vec<Uuid>, sqlx::Error> {
        sqlx::query_scalar("SELECT user_id FROM team_members WHERE team_id = $1")
            .bind(team_id)
            .fetch_all(self.pool)
            .await
    }

    /// List team members with user presence data.
    pub async fn list_team_members_with_presence(
        &self,
        team_id: Uuid,
    ) -> Result<Vec<(Uuid, Uuid, String, Option<String>)>, sqlx::Error> {
        sqlx::query_as(
            r#"
            SELECT tm.team_id, tm.user_id, tm.role, u.presence
            FROM team_members tm
            JOIN users u ON tm.user_id = u.id
            WHERE tm.team_id = $1
            ORDER BY u.username
            "#,
        )
        .bind(team_id)
        .fetch_all(self.pool)
        .await
    }

    /// List channels in a team that a user is a member of.
    pub async fn list_team_channels(
        &self,
        team_id: Uuid,
        user_id: Uuid,
    ) -> Result<Vec<Channel>, sqlx::Error> {
        sqlx::query_as::<_, Channel>(
            r#"
            SELECT c.* FROM channels c
            INNER JOIN channel_members cm ON c.id = cm.channel_id
            WHERE c.team_id = $1 AND cm.user_id = $2
            ORDER BY c.name
            "#,
        )
        .bind(team_id)
        .bind(user_id)
        .fetch_all(self.pool)
        .await
    }

    /// List all public teams.
    pub async fn list_public_teams(&self) -> Result<Vec<Team>, sqlx::Error> {
        sqlx::query_as::<_, Team>(
            r#"
            SELECT t.* FROM teams t
            WHERE t.is_public = true
            ORDER BY t.name
            "#,
        )
        .fetch_all(self.pool)
        .await
    }

    /// Remove a user from all channels in a team.
    pub async fn remove_user_from_team_channels(
        &self,
        user_id: Uuid,
        team_id: Uuid,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            DELETE FROM channel_members
            WHERE user_id = $1 AND channel_id IN (
                SELECT id FROM channels WHERE team_id = $2
            )
            "#,
        )
        .bind(user_id)
        .bind(team_id)
        .execute(self.pool)
        .await?;
        Ok(())
    }
}

// ------------------------------------------------------------------
// Team invite helpers
// ------------------------------------------------------------------

/// Row from the team_invite_tokens table.
#[derive(sqlx::FromRow)]
pub struct TeamInviteTokenRow {
    pub team_id: Uuid,
    pub expires_at: DateTime<Utc>,
    pub used_at: Option<DateTime<Utc>>,
}

impl<'a> TeamRepository<'a> {
    /// Get a team invite token row with a `FOR UPDATE` lock.
    pub async fn get_team_invite_token_for_update(
        &self,
        token: &str,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<Option<TeamInviteTokenRow>, sqlx::Error> {
        sqlx::query_as(
            r#"
            SELECT team_id, expires_at, used_at
            FROM team_invite_tokens
            WHERE token = $1
            FOR UPDATE
            "#,
        )
        .bind(token)
        .fetch_optional(&mut **tx)
        .await
    }

    /// Mark a team invite token as used.
    pub async fn mark_invite_token_used(
        &self,
        token: &str,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE team_invite_tokens SET used_at = NOW() WHERE token = $1")
            .bind(token)
            .execute(&mut **tx)
            .await?;
        Ok(())
    }

    /// Get a team by its `invite_id`.
    pub async fn get_team_by_invite_id(
        &self,
        invite_id: &str,
    ) -> Result<Option<Team>, sqlx::Error> {
        sqlx::query_as("SELECT * FROM teams WHERE invite_id = $1")
            .bind(invite_id)
            .fetch_optional(self.pool)
            .await
    }

    /// Upsert a team member (insert or update role on conflict).
    pub async fn upsert_team_member(
        &self,
        team_id: Uuid,
        user_id: Uuid,
        role: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO team_members (team_id, user_id, role)
            VALUES ($1, $2, $3)
            ON CONFLICT (team_id, user_id)
            DO UPDATE SET role = EXCLUDED.role
            "#,
        )
        .bind(team_id)
        .bind(user_id)
        .bind(role)
        .execute(self.pool)
        .await?;
        Ok(())
    }

    /// Upsert a team member inside a transaction.
    pub async fn upsert_team_member_in_tx(
        &self,
        team_id: Uuid,
        user_id: Uuid,
        role: &str,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO team_members (team_id, user_id, role)
            VALUES ($1, $2, $3)
            ON CONFLICT (team_id, user_id)
            DO UPDATE SET role = EXCLUDED.role
            "#,
        )
        .bind(team_id)
        .bind(user_id)
        .bind(role)
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    /// Check whether a user exists and is not soft-deleted.
    pub async fn user_exists(&self, user_id: Uuid) -> Result<bool, sqlx::Error> {
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM users WHERE id = $1 AND deleted_at IS NULL)",
        )
        .bind(user_id)
        .fetch_one(self.pool)
        .await?;
        Ok(exists)
    }

    /// Get an active team invitation by user_id.
    pub async fn get_active_team_invitation_by_user(
        &self,
        team_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<(String, DateTime<Utc>)>, sqlx::Error> {
        sqlx::query_as(
            "SELECT token, expires_at FROM team_invitations \
             WHERE team_id = $1 AND user_id = $2 AND used = false AND expires_at > NOW()",
        )
        .bind(team_id)
        .bind(user_id)
        .fetch_optional(self.pool)
        .await
    }

    /// Upsert a team invitation for a specific user.
    pub async fn upsert_team_invitation_for_user(
        &self,
        team_id: Uuid,
        user_id: Uuid,
        invited_by: Uuid,
        token: &str,
        expires_at: DateTime<Utc>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO team_invitations \
             (team_id, user_id, invited_by, token, invitation_type, expires_at) \
             VALUES ($1, $2, $3, $4, 'member', $5) \
             ON CONFLICT (team_id, user_id) WHERE used = false \
             DO UPDATE SET token = EXCLUDED.token, expires_at = EXCLUDED.expires_at, updated_at = NOW()",
        )
        .bind(team_id)
        .bind(user_id)
        .bind(invited_by)
        .bind(token)
        .bind(expires_at)
        .execute(self.pool)
        .await?;
        Ok(())
    }

    /// Get an active team invitation by email.
    pub async fn get_active_team_invitation_by_email(
        &self,
        team_id: Uuid,
        email: &str,
    ) -> Result<Option<(String, DateTime<Utc>)>, sqlx::Error> {
        sqlx::query_as(
            "SELECT token, expires_at FROM team_invitations \
             WHERE team_id = $1 AND email = $2 AND used = false AND expires_at > NOW()",
        )
        .bind(team_id)
        .bind(email)
        .fetch_optional(self.pool)
        .await
    }

    /// Upsert a team invitation for an email address.
    pub async fn upsert_team_invitation_for_email(
        &self,
        team_id: Uuid,
        invited_by: Uuid,
        email: &str,
        token: &str,
        invitation_type: &str,
        expires_at: DateTime<Utc>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO team_invitations \
             (team_id, user_id, invited_by, email, token, invitation_type, expires_at) \
             VALUES ($1, NULL, $2, $3, $4, $5, $6) \
             ON CONFLICT (team_id, email) WHERE used = false \
             DO UPDATE SET token = EXCLUDED.token, expires_at = EXCLUDED.expires_at, updated_at = NOW()",
        )
        .bind(team_id)
        .bind(invited_by)
        .bind(email)
        .bind(token)
        .bind(invitation_type)
        .bind(expires_at)
        .execute(self.pool)
        .await?;
        Ok(())
    }

    /// Regenerate a team's `invite_id`.
    pub async fn regenerate_team_invite_id(
        &self,
        team_id: Uuid,
    ) -> Result<Option<String>, sqlx::Error> {
        sqlx::query_scalar(
            r#"
            UPDATE teams
            SET invite_id = replace(gen_random_uuid()::text, '-', '')
            WHERE id = $1
            RETURNING invite_id
            "#,
        )
        .bind(team_id)
        .fetch_optional(self.pool)
        .await
    }

    /// Get the first team a user belongs to (ordered by membership creation time).
    pub async fn get_first_team_for_user(
        &self,
        user_id: Uuid,
    ) -> Result<Option<Uuid>, sqlx::Error> {
        sqlx::query_scalar(
            "SELECT team_id FROM team_members WHERE user_id = $1 ORDER BY created_at ASC LIMIT 1",
        )
        .bind(user_id)
        .fetch_optional(self.pool)
        .await
    }

    /// Get a team ID by its name.
    pub async fn get_id_by_name(&self, name: &str) -> Result<Option<Uuid>, sqlx::Error> {
        sqlx::query_scalar("SELECT id FROM teams WHERE name = $1")
            .bind(name)
            .fetch_optional(self.pool)
            .await
    }

    /// Get a team by name.
    pub async fn get_team_by_name(&self, name: &str) -> Result<Option<Team>, sqlx::Error> {
        sqlx::query_as::<_, Team>("SELECT * FROM teams WHERE name = $1")
            .bind(name)
            .fetch_optional(self.pool)
            .await
    }

    /// Check if a team name exists.
    pub async fn team_name_exists(&self, name: &str) -> Result<bool, sqlx::Error> {
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM teams WHERE name = $1)")
            .bind(name)
            .fetch_one(self.pool)
            .await
    }

    /// Update a team's privacy setting.
    pub async fn update_team_privacy(
        &self,
        id: Uuid,
        privacy: &str,
    ) -> Result<Option<Team>, sqlx::Error> {
        sqlx::query_as::<_, Team>("UPDATE teams SET privacy = $1 WHERE id = $2 RETURNING *")
            .bind(privacy)
            .bind(id)
            .fetch_optional(self.pool)
            .await
    }

    /// Restore a soft-deleted team.
    pub async fn restore_team(&self, id: Uuid) -> Result<Option<Team>, sqlx::Error> {
        sqlx::query_as::<_, Team>("UPDATE teams SET deleted_at = NULL WHERE id = $1 RETURNING *")
            .bind(id)
            .execute(self.pool)
            .await?;

        // Fetch the restored team
        self.get_team_by_id(id).await
    }
}
