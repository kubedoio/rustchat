//! Playbooks API endpoints

use axum::{
    extract::{Path, Query, State},
    routing::{delete, get, post, put},
    Json, Router,
};
use uuid::Uuid;

use super::AppState;
use crate::auth::AuthUser;
use crate::error::{ApiResult, AppError};
use crate::models::{
    ChecklistWithTasks, CreateChecklist, CreatePlaybook, CreateStatusUpdate, CreateTask, Playbook,
    PlaybookChecklist, PlaybookFull, PlaybookRun, PlaybookTask, RunProgress, RunStatusUpdate,
    RunTask, RunWithTasks, StartRun, UpdatePlaybook, UpdateRun, UpdateRunTask,
};

#[derive(serde::Deserialize)]
pub struct TeamQuery {
    team_id: Uuid,
}

/// Build playbooks routes
pub fn router() -> Router<AppState> {
    Router::new()
        // Playbooks CRUD
        .route("/playbooks", get(list_playbooks))
        .route("/playbooks", post(create_playbook))
        .route("/playbooks/{id}", get(get_playbook))
        .route("/playbooks/{id}", put(update_playbook))
        .route("/playbooks/{id}", delete(delete_playbook))
        // Checklists
        .route(
            "/playbooks/{playbook_id}/checklists",
            post(create_checklist),
        )
        .route(
            "/playbooks/{playbook_id}/checklists/{id}",
            delete(delete_checklist),
        )
        // Tasks
        .route("/checklists/{checklist_id}/tasks", post(create_task))
        .route("/tasks/{id}", put(update_task))
        .route("/tasks/{id}", delete(delete_task))
        // Runs
        .route("/runs", get(list_runs))
        .route("/runs", post(start_run))
        .route("/runs/{id}", get(get_run))
        .route("/runs/{id}", put(update_run))
        .route("/runs/{id}/finish", post(finish_run))
        // Run tasks
        .route("/runs/{run_id}/tasks/{task_id}", put(update_run_task))
        // Status updates
        .route("/runs/{run_id}/updates", get(list_status_updates))
        .route("/runs/{run_id}/updates", post(create_status_update))
}

// ============ Playbooks ============

async fn list_playbooks(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<TeamQuery>,
) -> ApiResult<Json<Vec<Playbook>>> {
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
    .bind(query.team_id)
    .bind(auth.user_id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(playbooks))
}

async fn create_playbook(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<TeamQuery>,
    Json(payload): Json<CreatePlaybook>,
) -> ApiResult<Json<Playbook>> {
    let playbook = sqlx::query_as::<_, Playbook>(
        r#"
        INSERT INTO playbooks (team_id, created_by, name, description, icon, is_public, create_channel_on_run, channel_name_template)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        RETURNING *
        "#
    )
    .bind(query.team_id)
    .bind(auth.user_id)
    .bind(&payload.name)
    .bind(&payload.description)
    .bind(&payload.icon)
    .bind(payload.is_public.unwrap_or(false))
    .bind(payload.create_channel_on_run.unwrap_or(true))
    .bind(&payload.channel_name_template)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(playbook))
}

async fn get_playbook(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<PlaybookFull>> {
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
    .bind(auth.user_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound("Playbook not found or access denied".to_string()))?;

    let checklists = sqlx::query_as::<_, PlaybookChecklist>(
        "SELECT * FROM playbook_checklists WHERE playbook_id = $1 ORDER BY sort_order",
    )
    .bind(id)
    .fetch_all(&state.db)
    .await?;

    let mut checklists_with_tasks = Vec::new();
    for checklist in checklists {
        let tasks = sqlx::query_as::<_, PlaybookTask>(
            "SELECT * FROM playbook_tasks WHERE checklist_id = $1 ORDER BY sort_order",
        )
        .bind(checklist.id)
        .fetch_all(&state.db)
        .await?;

        checklists_with_tasks.push(ChecklistWithTasks { checklist, tasks });
    }

    Ok(Json(PlaybookFull {
        playbook,
        checklists: checklists_with_tasks,
    }))
}

async fn update_playbook(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdatePlaybook>,
) -> ApiResult<Json<Playbook>> {
    // Check ownership
    let current = sqlx::query_as::<_, Playbook>("SELECT * FROM playbooks WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("Playbook not found".to_string()))?;

    if current.created_by != auth.user_id && auth.role != "system_admin" {
        return Err(AppError::Forbidden(
            "Only the creator can edit this playbook".to_string(),
        ));
    }

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
    .bind(&payload.name)
    .bind(&payload.description)
    .bind(&payload.icon)
    .bind(payload.is_public)
    .bind(payload.create_channel_on_run)
    .bind(&payload.channel_name_template)
    .bind(&payload.keyword_triggers)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(playbook))
}

async fn delete_playbook(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    // Check ownership
    let current = sqlx::query!("SELECT created_by FROM playbooks WHERE id = $1", id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("Playbook not found".to_string()))?;

    if current.created_by != auth.user_id && auth.role != "system_admin" {
        return Err(AppError::Forbidden(
            "Only the creator can archive this playbook".to_string(),
        ));
    }

    sqlx::query("UPDATE playbooks SET is_archived = true WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    Ok(Json(serde_json::json!({"status": "archived"})))
}

// ============ Checklists ============

async fn create_checklist(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(playbook_id): Path<Uuid>,
    Json(payload): Json<CreateChecklist>,
) -> ApiResult<Json<PlaybookChecklist>> {
    let checklist = sqlx::query_as::<_, PlaybookChecklist>(
        r#"
        INSERT INTO playbook_checklists (playbook_id, name, sort_order)
        VALUES ($1, $2, COALESCE($3, (SELECT COALESCE(MAX(sort_order), 0) + 1 FROM playbook_checklists WHERE playbook_id = $1)))
        RETURNING *
        "#
    )
    .bind(playbook_id)
    .bind(&payload.name)
    .bind(payload.sort_order)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(checklist))
}

async fn delete_checklist(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path((_playbook_id, id)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<serde_json::Value>> {
    sqlx::query("DELETE FROM playbook_checklists WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    Ok(Json(serde_json::json!({"status": "deleted"})))
}

// ============ Tasks ============

async fn create_task(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(checklist_id): Path<Uuid>,
    Json(payload): Json<CreateTask>,
) -> ApiResult<Json<PlaybookTask>> {
    let task = sqlx::query_as::<_, PlaybookTask>(
        r#"
        INSERT INTO playbook_tasks (checklist_id, title, description, default_assignee_id, due_after_minutes, slash_command, sort_order)
        VALUES ($1, $2, $3, $4, $5, $6, COALESCE($7, (SELECT COALESCE(MAX(sort_order), 0) + 1 FROM playbook_tasks WHERE checklist_id = $1)))
        RETURNING *
        "#
    )
    .bind(checklist_id)
    .bind(&payload.title)
    .bind(&payload.description)
    .bind(payload.default_assignee_id)
    .bind(payload.due_after_minutes)
    .bind(&payload.slash_command)
    .bind(payload.sort_order)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(task))
}

async fn update_task(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(payload): Json<CreateTask>,
) -> ApiResult<Json<PlaybookTask>> {
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
    .bind(&payload.title)
    .bind(&payload.description)
    .bind(payload.default_assignee_id)
    .bind(payload.due_after_minutes)
    .bind(&payload.slash_command)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(task))
}

async fn delete_task(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    sqlx::query("DELETE FROM playbook_tasks WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    Ok(Json(serde_json::json!({"status": "deleted"})))
}

// ============ Runs ============

async fn list_runs(
    State(state): State<AppState>,
    _auth: AuthUser,
    Query(query): Query<TeamQuery>,
) -> ApiResult<Json<Vec<PlaybookRun>>> {
    let runs = sqlx::query_as::<_, PlaybookRun>(
        "SELECT * FROM playbook_runs WHERE team_id = $1 ORDER BY started_at DESC LIMIT 50",
    )
    .bind(query.team_id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(runs))
}

async fn start_run(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<TeamQuery>,
    Json(payload): Json<StartRun>,
) -> ApiResult<Json<RunWithTasks>> {
    // 1. Fetch Playbook to check settings
    let playbook = sqlx::query_as::<_, Playbook>("SELECT * FROM playbooks WHERE id = $1")
        .bind(payload.playbook_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("Playbook not found".to_string()))?;

    // 2. Determine Channel ID
    let mut channel_id = payload.channel_id;

    if channel_id.is_none() && playbook.create_channel_on_run {
        // Create a new channel
        let template = playbook
            .channel_name_template
            .unwrap_or_else(|| "run-{{date}}".to_string());
        let date_str = chrono::Utc::now().format("%Y%m%d-%H%M").to_string();
        let name = template
            .replace("{{date}}", &date_str)
            .replace("{{playbook_name}}", &playbook.name)
            .to_lowercase()
            .replace(" ", "-"); // Sanitize name

        let channel_name = format!("{}-{}", name, &Uuid::new_v4().simple().to_string()[0..6]); // Ensure uniqueness

        // Create channel
        let channel = sqlx::query_as::<_, crate::models::Channel>(
            r#"
            INSERT INTO channels (team_id, name, display_name, purpose, type, creator_id)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING *
            "#,
        )
        .bind(query.team_id)
        .bind(&channel_name)
        .bind(format!("Run: {}", payload.name))
        .bind(format!("Channel for playbook run: {}", payload.name))
        .bind(if playbook.is_public {
            "public"
        } else {
            "private"
        })
        .bind(auth.user_id)
        .fetch_one(&state.db)
        .await?;

        // Add creator/owner to channel
        sqlx::query(
            "INSERT INTO channel_members (channel_id, user_id, role) VALUES ($1, $2, 'admin')",
        )
        .bind(channel.id)
        .bind(auth.user_id)
        .execute(&state.db)
        .await?;

        channel_id = Some(channel.id);
    }

    // 3. Create the run
    let run = sqlx::query_as::<_, PlaybookRun>(
        r#"
        INSERT INTO playbook_runs (playbook_id, team_id, name, owner_id, channel_id, attributes)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING *
        "#,
    )
    .bind(payload.playbook_id)
    .bind(query.team_id)
    .bind(&payload.name)
    .bind(payload.owner_id.unwrap_or(auth.user_id))
    .bind(channel_id)
    .bind(&payload.attributes)
    .fetch_one(&state.db)
    .await?;

    // 4. Create run tasks from playbook tasks
    sqlx::query(
        r#"
        INSERT INTO run_tasks (run_id, task_id, assignee_id)
        SELECT $1, pt.id, pt.default_assignee_id
        FROM playbook_tasks pt
        JOIN playbook_checklists pc ON pt.checklist_id = pc.id
        WHERE pc.playbook_id = $2
        "#,
    )
    .bind(run.id)
    .bind(payload.playbook_id)
    .execute(&state.db)
    .await?;

    // 5. Fetch run tasks
    let tasks = sqlx::query_as::<_, RunTask>("SELECT * FROM run_tasks WHERE run_id = $1")
        .bind(run.id)
        .fetch_all(&state.db)
        .await?;

    let progress = calculate_progress(&tasks);

    Ok(Json(RunWithTasks {
        run,
        tasks,
        progress,
    }))
}

async fn get_run(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<RunWithTasks>> {
    let run = sqlx::query_as::<_, PlaybookRun>("SELECT * FROM playbook_runs WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("Run not found".to_string()))?;

    let tasks = sqlx::query_as::<_, RunTask>("SELECT * FROM run_tasks WHERE run_id = $1")
        .bind(id)
        .fetch_all(&state.db)
        .await?;

    let progress = calculate_progress(&tasks);

    Ok(Json(RunWithTasks {
        run,
        tasks,
        progress,
    }))
}

async fn update_run(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateRun>,
) -> ApiResult<Json<PlaybookRun>> {
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
    .bind(&payload.status)
    .bind(&payload.summary)
    .bind(&payload.attributes)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(run))
}

async fn finish_run(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<PlaybookRun>> {
    let run = sqlx::query_as::<_, PlaybookRun>(
        r#"
        UPDATE playbook_runs SET status = 'finished', finished_at = NOW(), updated_at = NOW()
        WHERE id = $1
        RETURNING *
        "#,
    )
    .bind(id)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(run))
}

// ============ Run Tasks ============

async fn update_run_task(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((run_id, task_id)): Path<(Uuid, Uuid)>,
    Json(payload): Json<UpdateRunTask>,
) -> ApiResult<Json<RunTask>> {
    let completed_at = if payload.status.as_deref() == Some("done") {
        Some(chrono::Utc::now())
    } else {
        None
    };

    let completed_by = if payload.status.as_deref() == Some("done") {
        Some(auth.user_id)
    } else {
        None
    };

    let task = sqlx::query_as::<_, RunTask>(
        r#"
        UPDATE run_tasks SET
            status = COALESCE($3, status),
            assignee_id = COALESCE($4, assignee_id),
            notes = COALESCE($5, notes),
            completed_at = COALESCE($6, completed_at),
            completed_by = COALESCE($7, completed_by),
            updated_at = NOW()
        WHERE run_id = $1 AND task_id = $2
        RETURNING *
        "#,
    )
    .bind(run_id)
    .bind(task_id)
    .bind(&payload.status)
    .bind(payload.assignee_id)
    .bind(&payload.notes)
    .bind(completed_at)
    .bind(completed_by)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(task))
}

// ============ Status Updates ============

async fn list_status_updates(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(run_id): Path<Uuid>,
) -> ApiResult<Json<Vec<RunStatusUpdate>>> {
    let updates = sqlx::query_as::<_, RunStatusUpdate>(
        "SELECT * FROM run_status_updates WHERE run_id = $1 ORDER BY created_at DESC",
    )
    .bind(run_id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(updates))
}

async fn create_status_update(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(run_id): Path<Uuid>,
    Json(payload): Json<CreateStatusUpdate>,
) -> ApiResult<Json<RunStatusUpdate>> {
    let update = sqlx::query_as::<_, RunStatusUpdate>(
        r#"
        INSERT INTO run_status_updates (run_id, author_id, message, is_broadcast)
        VALUES ($1, $2, $3, $4)
        RETURNING *
        "#,
    )
    .bind(run_id)
    .bind(auth.user_id)
    .bind(&payload.message)
    .bind(payload.is_broadcast.unwrap_or(false))
    .fetch_one(&state.db)
    .await?;

    Ok(Json(update))
}

// ============ Helpers ============

fn calculate_progress(tasks: &[RunTask]) -> RunProgress {
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
