//! Playbooks API endpoints

use axum::{
    extract::{Path, Query, State},
    routing::{delete, get, post, put},
    Json, Router,
};
use uuid::Uuid;

use super::AppState;
use crate::auth::policy::permissions;
use crate::auth::AuthUser;
use crate::error::{ApiResult, AppError};
use crate::models::{
    CreateChecklist, CreatePlaybook, CreateStatusUpdate, CreateTask, Playbook, PlaybookChecklist,
    PlaybookFull, PlaybookRun, PlaybookTask, RunStatusUpdate, RunTask, RunWithTasks, StartRun,
    UpdatePlaybook, UpdateRun, UpdateRunTask,
};
use crate::repositories::{calculate_progress, PlaybookRepository};

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
    let repo = PlaybookRepository::new(&state.db);
    let playbooks = repo.list_playbooks(query.team_id, auth.user_id).await?;
    Ok(Json(playbooks))
}

async fn create_playbook(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<TeamQuery>,
    Json(payload): Json<CreatePlaybook>,
) -> ApiResult<Json<Playbook>> {
    let repo = PlaybookRepository::new(&state.db);
    let playbook = repo
        .create_playbook(
            query.team_id,
            auth.user_id,
            &payload.name,
            &payload.description,
            &payload.icon,
            payload.is_public.unwrap_or(false),
            payload.create_channel_on_run.unwrap_or(true),
            &payload.channel_name_template,
        )
        .await?;

    Ok(Json(playbook))
}

async fn get_playbook(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<PlaybookFull>> {
    let repo = PlaybookRepository::new(&state.db);
    let full = repo
        .get_playbook_full(id, auth.user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Playbook not found or access denied".to_string()))?;

    Ok(Json(full))
}

async fn update_playbook(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdatePlaybook>,
) -> ApiResult<Json<Playbook>> {
    let repo = PlaybookRepository::new(&state.db);

    // Check ownership
    let current = repo
        .get_playbook_by_id(id)
        .await?
        .ok_or_else(|| AppError::NotFound("Playbook not found".to_string()))?;

    if !auth.can_access_owned(current.created_by, &permissions::ADMIN_FULL) {
        return Err(AppError::Forbidden(
            "Only the creator can edit this playbook".to_string(),
        ));
    }

    let playbook = repo
        .update_playbook(
            id,
            &payload.name,
            &payload.description,
            &payload.icon,
            payload.is_public,
            payload.create_channel_on_run,
            &payload.channel_name_template,
            &payload.keyword_triggers,
        )
        .await?;

    Ok(Json(playbook))
}

async fn delete_playbook(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    let repo = PlaybookRepository::new(&state.db);

    // Check ownership
    let created_by = repo
        .get_playbook_creator(id)
        .await?
        .ok_or_else(|| AppError::NotFound("Playbook not found".to_string()))?;

    if !auth.can_access_owned(created_by, &permissions::ADMIN_FULL) {
        return Err(AppError::Forbidden(
            "Only the creator can archive this playbook".to_string(),
        ));
    }

    repo.archive_playbook(id).await?;

    Ok(Json(serde_json::json!({"status": "archived"})))
}

// ============ Checklists ============

async fn create_checklist(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(playbook_id): Path<Uuid>,
    Json(payload): Json<CreateChecklist>,
) -> ApiResult<Json<PlaybookChecklist>> {
    let repo = PlaybookRepository::new(&state.db);
    repo.require_playbook_access(playbook_id, auth.user_id)
        .await?;

    let checklist = repo
        .create_checklist(playbook_id, &payload.name, payload.sort_order)
        .await?;

    Ok(Json(checklist))
}

async fn delete_checklist(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((playbook_id, id)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<serde_json::Value>> {
    let repo = PlaybookRepository::new(&state.db);
    repo.require_playbook_access(playbook_id, auth.user_id)
        .await?;

    repo.delete_checklist(id).await?;

    Ok(Json(serde_json::json!({"status": "deleted"})))
}

// ============ Tasks ============

async fn create_task(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(checklist_id): Path<Uuid>,
    Json(payload): Json<CreateTask>,
) -> ApiResult<Json<PlaybookTask>> {
    let repo = PlaybookRepository::new(&state.db);
    let playbook_id = repo.get_checklist_playbook_id(checklist_id).await?;
    repo.require_playbook_access(playbook_id, auth.user_id)
        .await?;

    let task = repo
        .create_task(
            checklist_id,
            &payload.title,
            &payload.description,
            payload.default_assignee_id,
            payload.due_after_minutes,
            &payload.slash_command,
            payload.sort_order,
        )
        .await?;

    Ok(Json(task))
}

async fn update_task(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(payload): Json<CreateTask>,
) -> ApiResult<Json<PlaybookTask>> {
    let repo = PlaybookRepository::new(&state.db);
    let playbook_id = repo.get_task_playbook_id(id).await?;
    repo.require_playbook_access(playbook_id, auth.user_id)
        .await?;

    let task = repo
        .update_task(
            id,
            &payload.title,
            &payload.description,
            payload.default_assignee_id,
            payload.due_after_minutes,
            &payload.slash_command,
        )
        .await?;

    Ok(Json(task))
}

async fn delete_task(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    let repo = PlaybookRepository::new(&state.db);
    let playbook_id = repo.get_task_playbook_id(id).await?;
    repo.require_playbook_access(playbook_id, auth.user_id)
        .await?;

    repo.delete_task(id).await?;

    Ok(Json(serde_json::json!({"status": "deleted"})))
}

// ============ Runs ============

async fn list_runs(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<TeamQuery>,
) -> ApiResult<Json<Vec<PlaybookRun>>> {
    let repo = PlaybookRepository::new(&state.db);
    let runs = repo.list_runs(query.team_id, auth.user_id).await?;
    Ok(Json(runs))
}

async fn start_run(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<TeamQuery>,
    Json(payload): Json<StartRun>,
) -> ApiResult<Json<RunWithTasks>> {
    let repo = PlaybookRepository::new(&state.db);

    // 1. Fetch Playbook to check settings
    let playbook = repo
        .get_playbook_by_id(payload.playbook_id)
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
            VALUES ($1, $2, $3, $4, $5::channel_type, $6)
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
    let run = repo
        .create_run(
            payload.playbook_id,
            query.team_id,
            &payload.name,
            payload.owner_id.unwrap_or(auth.user_id),
            channel_id,
            &payload.attributes,
        )
        .await?;

    // 4. Create run tasks from playbook tasks
    repo.create_run_tasks_from_playbook(run.id, payload.playbook_id)
        .await?;

    // 5. Fetch run tasks
    let tasks = repo.list_run_tasks(run.id).await?;
    let progress = calculate_progress(&tasks);

    Ok(Json(RunWithTasks {
        run,
        tasks,
        progress,
    }))
}

async fn get_run(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<RunWithTasks>> {
    let repo = PlaybookRepository::new(&state.db);
    let run = repo
        .get_run(id)
        .await?
        .ok_or_else(|| AppError::NotFound("Run not found".to_string()))?;

    repo.require_playbook_access(run.playbook_id, auth.user_id)
        .await?;

    let tasks = repo.list_run_tasks(id).await?;
    let progress = calculate_progress(&tasks);

    Ok(Json(RunWithTasks {
        run,
        tasks,
        progress,
    }))
}

async fn update_run(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateRun>,
) -> ApiResult<Json<PlaybookRun>> {
    let repo = PlaybookRepository::new(&state.db);

    // Resolve parent playbook and enforce access
    let playbook_id = repo
        .get_run_playbook_id(id)
        .await?
        .ok_or_else(|| AppError::NotFound("Run not found".to_string()))?;
    repo.require_playbook_access(playbook_id, auth.user_id)
        .await?;

    let run = repo
        .update_run(id, &payload.status, &payload.summary, &payload.attributes)
        .await?;

    Ok(Json(run))
}

async fn finish_run(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<PlaybookRun>> {
    let repo = PlaybookRepository::new(&state.db);

    // Resolve parent playbook and enforce access
    let playbook_id = repo
        .get_run_playbook_id(id)
        .await?
        .ok_or_else(|| AppError::NotFound("Run not found".to_string()))?;
    repo.require_playbook_access(playbook_id, auth.user_id)
        .await?;

    let run = repo.finish_run(id).await?;
    Ok(Json(run))
}

// ============ Run Tasks ============

async fn update_run_task(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path((run_id, task_id)): Path<(Uuid, Uuid)>,
    Json(payload): Json<UpdateRunTask>,
) -> ApiResult<Json<RunTask>> {
    let repo = PlaybookRepository::new(&state.db);

    let completed_at = if payload.status.as_deref() == Some("done") {
        Some(chrono::Utc::now())
    } else {
        None
    };

    let completed_by = if payload.status.as_deref() == Some("done") {
        Some(_auth.user_id)
    } else {
        None
    };

    let task = repo
        .update_run_task(
            run_id,
            task_id,
            &payload.status,
            payload.assignee_id,
            &payload.notes,
            completed_at,
            completed_by,
        )
        .await?;

    Ok(Json(task))
}

// ============ Status Updates ============

async fn list_status_updates(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(run_id): Path<Uuid>,
) -> ApiResult<Json<Vec<RunStatusUpdate>>> {
    let repo = PlaybookRepository::new(&state.db);

    let playbook_id = repo
        .get_run_playbook_id(run_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Run not found".to_string()))?;
    repo.require_playbook_access(playbook_id, auth.user_id)
        .await?;

    let updates = repo.list_status_updates(run_id).await?;
    Ok(Json(updates))
}

async fn create_status_update(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(run_id): Path<Uuid>,
    Json(payload): Json<CreateStatusUpdate>,
) -> ApiResult<Json<RunStatusUpdate>> {
    let repo = PlaybookRepository::new(&state.db);
    let update = repo
        .create_status_update(
            run_id,
            auth.user_id,
            &payload.message,
            payload.is_broadcast.unwrap_or(false),
        )
        .await?;

    Ok(Json(update))
}
