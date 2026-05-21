//! Playbook repository for centralized query patterns
//!
//! This module centralizes playbook, checklist, task, run, and status update queries
//! previously scattered across api/playbooks.rs.

use sqlx::PgPool;
use uuid::Uuid;

use crate::error::{ApiResult, AppError};
use crate::models::{
    ChecklistWithTasks, Playbook, PlaybookChecklist, PlaybookFull, PlaybookRun, PlaybookTask,
    RunProgress, RunStatusUpdate, RunTask,
};

/// Repository for playbook-related database operations
#[derive(Debug, Clone)]
pub struct PlaybookRepository<'a> {
    pool: &'a PgPool,
}

impl<'a> PlaybookRepository<'a> {
    /// Create a new PlaybookRepository instance
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    // ============ Playbooks ============

    /// List playbooks accessible to a user in a team
    pub async fn list_playbooks(&self, team_id: Uuid, user_id: Uuid) -> ApiResult<Vec<Playbook>> {
        let playbooks = sqlx::query_as::<_, Playbook>(
            r#"
            SELECT * FROM playbooks
            WHERE team_id = $1
              AND is_archived = false
              AND (
                is_public = true
                OR created_by = $2
                OR ($2 = ANY(member_ids))
              )
            ORDER BY name
            "#,
        )
        .bind(team_id)
        .bind(user_id)
        .fetch_all(self.pool)
        .await?;

        Ok(playbooks)
    }

    /// Create a new playbook
    #[allow(clippy::too_many_arguments)]
    pub async fn create_playbook(
        &self,
        team_id: Uuid,
        created_by: Uuid,
        name: &str,
        description: &Option<String>,
        icon: &Option<String>,
        is_public: bool,
        create_channel_on_run: bool,
        channel_name_template: &Option<String>,
    ) -> ApiResult<Playbook> {
        let playbook = sqlx::query_as::<_, Playbook>(
            r#"
            INSERT INTO playbooks (team_id, created_by, name, description, icon, is_public, create_channel_on_run, channel_name_template)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING *
            "#,
        )
        .bind(team_id)
        .bind(created_by)
        .bind(name)
        .bind(description)
        .bind(icon)
        .bind(is_public)
        .bind(create_channel_on_run)
        .bind(channel_name_template)
        .fetch_one(self.pool)
        .await?;

        Ok(playbook)
    }

    /// Get a playbook by ID if the user has access
    pub async fn get_playbook(&self, id: Uuid, user_id: Uuid) -> ApiResult<Option<Playbook>> {
        let playbook = sqlx::query_as::<_, Playbook>(
            r#"
            SELECT * FROM playbooks
            WHERE id = $1
              AND (
                is_public = true
                OR created_by = $2
                OR ($2 = ANY(member_ids))
              )
            "#,
        )
        .bind(id)
        .bind(user_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(playbook)
    }

    /// Get a playbook by ID without access check (for ownership verification)
    pub async fn get_playbook_by_id(&self, id: Uuid) -> ApiResult<Option<Playbook>> {
        let playbook = sqlx::query_as::<_, Playbook>("SELECT * FROM playbooks WHERE id = $1")
            .bind(id)
            .fetch_optional(self.pool)
            .await?;

        Ok(playbook)
    }

    /// Get playbook creator ID
    pub async fn get_playbook_creator(&self, id: Uuid) -> ApiResult<Option<Uuid>> {
        let created_by: Option<Uuid> =
            sqlx::query_scalar("SELECT created_by FROM playbooks WHERE id = $1")
                .bind(id)
                .fetch_optional(self.pool)
                .await?;

        Ok(created_by)
    }

    /// Update a playbook
    #[allow(clippy::too_many_arguments)]
    pub async fn update_playbook(
        &self,
        id: Uuid,
        name: &Option<String>,
        description: &Option<String>,
        icon: &Option<String>,
        is_public: Option<bool>,
        create_channel_on_run: Option<bool>,
        channel_name_template: &Option<String>,
        keyword_triggers: &Option<Vec<String>>,
    ) -> ApiResult<Playbook> {
        let playbook = sqlx::query_as::<_, Playbook>(
            r#"
            UPDATE playbooks SET
                name = COALESCE($2, name),
                description = COALESCE($3, description),
                icon = COALESCE($4, icon),
                is_public = COALESCE($5, is_public),
                create_channel_on_run = COALESCE($6, create_channel_on_run),
                channel_name_template = COALESCE($7, channel_name_template),
                keyword_triggers = COALESCE($8, keyword_triggers),
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(name)
        .bind(description)
        .bind(icon)
        .bind(is_public)
        .bind(create_channel_on_run)
        .bind(channel_name_template)
        .bind(keyword_triggers)
        .fetch_one(self.pool)
        .await?;

        Ok(playbook)
    }

    /// Archive (soft-delete) a playbook
    pub async fn archive_playbook(&self, id: Uuid) -> ApiResult<()> {
        sqlx::query("UPDATE playbooks SET is_archived = true WHERE id = $1")
            .bind(id)
            .execute(self.pool)
            .await?;

        Ok(())
    }

    /// Check if user has access to a playbook
    pub async fn has_playbook_access(&self, playbook_id: Uuid, user_id: Uuid) -> ApiResult<bool> {
        let exists: bool = sqlx::query_scalar(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM playbooks
                WHERE id = $1
                  AND (
                    is_public = true
                    OR created_by = $2
                    OR ($2 = ANY(member_ids))
                  )
            )
            "#,
        )
        .bind(playbook_id)
        .bind(user_id)
        .fetch_one(self.pool)
        .await?;

        Ok(exists)
    }

    /// Require playbook access, returning error if denied
    pub async fn require_playbook_access(&self, playbook_id: Uuid, user_id: Uuid) -> ApiResult<()> {
        if !self.has_playbook_access(playbook_id, user_id).await? {
            return Err(AppError::Forbidden(
                "You do not have access to this playbook".to_string(),
            ));
        }
        Ok(())
    }

    /// Get a full playbook with checklists and tasks
    pub async fn get_playbook_full(
        &self,
        id: Uuid,
        user_id: Uuid,
    ) -> ApiResult<Option<PlaybookFull>> {
        let playbook = match self.get_playbook(id, user_id).await? {
            Some(p) => p,
            None => return Ok(None),
        };

        let checklists = self.list_checklists(id).await?;
        let mut checklists_with_tasks = Vec::new();

        for checklist in checklists {
            let tasks = self.list_tasks_by_checklist(checklist.id).await?;
            checklists_with_tasks.push(ChecklistWithTasks { checklist, tasks });
        }

        Ok(Some(PlaybookFull {
            playbook,
            checklists: checklists_with_tasks,
        }))
    }

    // ============ Checklists ============

    /// List checklists for a playbook
    pub async fn list_checklists(&self, playbook_id: Uuid) -> ApiResult<Vec<PlaybookChecklist>> {
        let checklists = sqlx::query_as::<_, PlaybookChecklist>(
            "SELECT * FROM playbook_checklists WHERE playbook_id = $1 ORDER BY sort_order",
        )
        .bind(playbook_id)
        .fetch_all(self.pool)
        .await?;

        Ok(checklists)
    }

    /// Create a checklist
    pub async fn create_checklist(
        &self,
        playbook_id: Uuid,
        name: &str,
        sort_order: Option<i32>,
    ) -> ApiResult<PlaybookChecklist> {
        let checklist = sqlx::query_as::<_, PlaybookChecklist>(
            r#"
            INSERT INTO playbook_checklists (playbook_id, name, sort_order)
            VALUES ($1, $2, COALESCE($3, (SELECT COALESCE(MAX(sort_order), 0) + 1 FROM playbook_checklists WHERE playbook_id = $1)))
            RETURNING *
            "#,
        )
        .bind(playbook_id)
        .bind(name)
        .bind(sort_order)
        .fetch_one(self.pool)
        .await?;

        Ok(checklist)
    }

    /// Delete a checklist by ID
    pub async fn delete_checklist(&self, id: Uuid) -> ApiResult<()> {
        sqlx::query("DELETE FROM playbook_checklists WHERE id = $1")
            .bind(id)
            .execute(self.pool)
            .await?;

        Ok(())
    }

    /// Get playbook ID for a checklist
    pub async fn get_checklist_playbook_id(&self, checklist_id: Uuid) -> ApiResult<Uuid> {
        let playbook_id: Uuid =
            sqlx::query_scalar("SELECT playbook_id FROM playbook_checklists WHERE id = $1")
                .bind(checklist_id)
                .fetch_one(self.pool)
                .await?;

        Ok(playbook_id)
    }

    // ============ Tasks ============

    /// List tasks for a checklist
    pub async fn list_tasks_by_checklist(
        &self,
        checklist_id: Uuid,
    ) -> ApiResult<Vec<PlaybookTask>> {
        let tasks = sqlx::query_as::<_, PlaybookTask>(
            "SELECT * FROM playbook_tasks WHERE checklist_id = $1 ORDER BY sort_order",
        )
        .bind(checklist_id)
        .fetch_all(self.pool)
        .await?;

        Ok(tasks)
    }

    /// Create a task
    #[allow(clippy::too_many_arguments)]
    pub async fn create_task(
        &self,
        checklist_id: Uuid,
        title: &str,
        description: &Option<String>,
        default_assignee_id: Option<Uuid>,
        due_after_minutes: Option<i32>,
        slash_command: &Option<String>,
        sort_order: Option<i32>,
    ) -> ApiResult<PlaybookTask> {
        let task = sqlx::query_as::<_, PlaybookTask>(
            r#"
            INSERT INTO playbook_tasks (checklist_id, title, description, default_assignee_id, due_after_minutes, slash_command, sort_order)
            VALUES ($1, $2, $3, $4, $5, $6, COALESCE($7, (SELECT COALESCE(MAX(sort_order), 0) + 1 FROM playbook_tasks WHERE checklist_id = $1)))
            RETURNING *
            "#,
        )
        .bind(checklist_id)
        .bind(title)
        .bind(description)
        .bind(default_assignee_id)
        .bind(due_after_minutes)
        .bind(slash_command)
        .bind(sort_order)
        .fetch_one(self.pool)
        .await?;

        Ok(task)
    }

    /// Update a task
    pub async fn update_task(
        &self,
        id: Uuid,
        title: &str,
        description: &Option<String>,
        default_assignee_id: Option<Uuid>,
        due_after_minutes: Option<i32>,
        slash_command: &Option<String>,
    ) -> ApiResult<PlaybookTask> {
        let task = sqlx::query_as::<_, PlaybookTask>(
            r#"
            UPDATE playbook_tasks SET
                title = $2,
                description = COALESCE($3, description),
                default_assignee_id = $4,
                due_after_minutes = $5,
                slash_command = $6
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(title)
        .bind(description)
        .bind(default_assignee_id)
        .bind(due_after_minutes)
        .bind(slash_command)
        .fetch_one(self.pool)
        .await?;

        Ok(task)
    }

    /// Delete a task by ID
    pub async fn delete_task(&self, id: Uuid) -> ApiResult<()> {
        sqlx::query("DELETE FROM playbook_tasks WHERE id = $1")
            .bind(id)
            .execute(self.pool)
            .await?;

        Ok(())
    }

    /// Get playbook ID for a task (via checklist)
    pub async fn get_task_playbook_id(&self, task_id: Uuid) -> ApiResult<Uuid> {
        let playbook_id: Uuid = sqlx::query_scalar(
            r#"
            SELECT c.playbook_id
            FROM playbook_tasks t
            JOIN playbook_checklists c ON t.checklist_id = c.id
            WHERE t.id = $1
            "#,
        )
        .bind(task_id)
        .fetch_one(self.pool)
        .await?;

        Ok(playbook_id)
    }

    // ============ Runs ============

    /// List runs visible to a user in a team
    pub async fn list_runs(&self, team_id: Uuid, user_id: Uuid) -> ApiResult<Vec<PlaybookRun>> {
        let runs = sqlx::query_as::<_, PlaybookRun>(
            r#"
            SELECT pr.* FROM playbook_runs pr
            JOIN playbooks pb ON pr.playbook_id = pb.id
            WHERE pr.team_id = $1
              AND (
                pb.is_public = true
                OR pb.created_by = $2
                OR ($2 = ANY(pb.member_ids))
              )
            ORDER BY pr.started_at DESC
            LIMIT 50
            "#,
        )
        .bind(team_id)
        .bind(user_id)
        .fetch_all(self.pool)
        .await?;

        Ok(runs)
    }

    /// Get a run by ID
    pub async fn get_run(&self, id: Uuid) -> ApiResult<Option<PlaybookRun>> {
        let run = sqlx::query_as::<_, PlaybookRun>("SELECT * FROM playbook_runs WHERE id = $1")
            .bind(id)
            .fetch_optional(self.pool)
            .await?;

        Ok(run)
    }

    /// Get playbook ID for a run
    pub async fn get_run_playbook_id(&self, run_id: Uuid) -> ApiResult<Option<Uuid>> {
        let playbook_id: Option<Uuid> =
            sqlx::query_scalar("SELECT playbook_id FROM playbook_runs WHERE id = $1")
                .bind(run_id)
                .fetch_optional(self.pool)
                .await?;

        Ok(playbook_id)
    }

    /// Create a run
    pub async fn create_run(
        &self,
        playbook_id: Uuid,
        team_id: Uuid,
        name: &str,
        owner_id: Uuid,
        channel_id: Option<Uuid>,
        attributes: &Option<serde_json::Value>,
    ) -> ApiResult<PlaybookRun> {
        let run = sqlx::query_as::<_, PlaybookRun>(
            r#"
            INSERT INTO playbook_runs (playbook_id, team_id, name, owner_id, channel_id, attributes)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING *
            "#,
        )
        .bind(playbook_id)
        .bind(team_id)
        .bind(name)
        .bind(owner_id)
        .bind(channel_id)
        .bind(attributes)
        .fetch_one(self.pool)
        .await?;

        Ok(run)
    }

    /// Update a run
    pub async fn update_run(
        &self,
        id: Uuid,
        status: &Option<String>,
        summary: &Option<String>,
        attributes: &Option<serde_json::Value>,
    ) -> ApiResult<PlaybookRun> {
        let run = sqlx::query_as::<_, PlaybookRun>(
            r#"
            UPDATE playbook_runs SET
                status = COALESCE($2, status),
                summary = COALESCE($3, summary),
                attributes = COALESCE($4, attributes),
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(status)
        .bind(summary)
        .bind(attributes)
        .fetch_one(self.pool)
        .await?;

        Ok(run)
    }

    /// Finish a run
    pub async fn finish_run(&self, id: Uuid) -> ApiResult<PlaybookRun> {
        let run = sqlx::query_as::<_, PlaybookRun>(
            r#"
            UPDATE playbook_runs SET status = 'finished', finished_at = NOW(), updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .fetch_one(self.pool)
        .await?;

        Ok(run)
    }

    // ============ Run Tasks ============

    /// List run tasks for a run
    pub async fn list_run_tasks(&self, run_id: Uuid) -> ApiResult<Vec<RunTask>> {
        let tasks = sqlx::query_as::<_, RunTask>("SELECT * FROM run_tasks WHERE run_id = $1")
            .bind(run_id)
            .fetch_all(self.pool)
            .await?;

        Ok(tasks)
    }

    /// Update a run task
    #[allow(clippy::too_many_arguments)]
    pub async fn update_run_task(
        &self,
        run_id: Uuid,
        task_id: Uuid,
        status: &Option<String>,
        assignee_id: Option<Uuid>,
        notes: &Option<String>,
        completed_at: Option<chrono::DateTime<chrono::Utc>>,
        completed_by: Option<Uuid>,
    ) -> ApiResult<RunTask> {
        let task = sqlx::query_as::<_, RunTask>(
            r#"
            UPDATE run_tasks SET
                status = COALESCE($3, status),
                assignee_id = COALESCE($4, assignee_id),
                notes = COALESCE($5, notes),
                completed_at = CASE
                    WHEN $3 = 'done' THEN $6
                    WHEN $3 IS NOT NULL AND $3 != 'done' THEN NULL
                    ELSE completed_at
                END,
                completed_by = CASE
                    WHEN $3 = 'done' THEN $7
                    WHEN $3 IS NOT NULL AND $3 != 'done' THEN NULL
                    ELSE completed_by
                END,
                updated_at = NOW()
            WHERE run_id = $1 AND task_id = $2
            RETURNING *
            "#,
        )
        .bind(run_id)
        .bind(task_id)
        .bind(status)
        .bind(assignee_id)
        .bind(notes)
        .bind(completed_at)
        .bind(completed_by)
        .fetch_one(self.pool)
        .await?;

        Ok(task)
    }

    /// Create run tasks from playbook tasks
    pub async fn create_run_tasks_from_playbook(
        &self,
        run_id: Uuid,
        playbook_id: Uuid,
    ) -> ApiResult<()> {
        sqlx::query(
            r#"
            INSERT INTO run_tasks (run_id, task_id, assignee_id)
            SELECT $1, pt.id, pt.default_assignee_id
            FROM playbook_tasks pt
            JOIN playbook_checklists pc ON pt.checklist_id = pc.id
            WHERE pc.playbook_id = $2
            "#,
        )
        .bind(run_id)
        .bind(playbook_id)
        .execute(self.pool)
        .await?;

        Ok(())
    }

    // ============ Status Updates ============

    /// List status updates for a run
    pub async fn list_status_updates(&self, run_id: Uuid) -> ApiResult<Vec<RunStatusUpdate>> {
        let updates = sqlx::query_as::<_, RunStatusUpdate>(
            "SELECT * FROM run_status_updates WHERE run_id = $1 ORDER BY created_at DESC",
        )
        .bind(run_id)
        .fetch_all(self.pool)
        .await?;

        Ok(updates)
    }

    /// Create a status update
    pub async fn create_status_update(
        &self,
        run_id: Uuid,
        author_id: Uuid,
        message: &str,
        is_broadcast: bool,
    ) -> ApiResult<RunStatusUpdate> {
        let update = sqlx::query_as::<_, RunStatusUpdate>(
            r#"
            INSERT INTO run_status_updates (run_id, author_id, message, is_broadcast)
            VALUES ($1, $2, $3, $4)
            RETURNING *
            "#,
        )
        .bind(run_id)
        .bind(author_id)
        .bind(message)
        .bind(is_broadcast)
        .fetch_one(self.pool)
        .await?;

        Ok(update)
    }
}

/// Calculate run progress from tasks
pub fn calculate_progress(tasks: &[RunTask]) -> RunProgress {
    let total = tasks.len() as i32;
    let completed = tasks.iter().filter(|t| t.status == "done").count() as i32;
    let in_progress = tasks.iter().filter(|t| t.status == "in_progress").count() as i32;
    let pending = tasks.iter().filter(|t| t.status == "pending").count() as i32;

    RunProgress {
        total,
        completed,
        in_progress,
        pending,
    }
}
