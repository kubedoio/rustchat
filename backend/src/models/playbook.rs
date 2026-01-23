//! Playbooks models for workflow automation

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Playbook - a template for workflows
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct Playbook {
    pub id: Uuid,
    pub team_id: Uuid,
    pub created_by: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub icon: Option<String>,
    pub is_public: bool,
    pub member_ids: Option<Vec<Uuid>>,
    pub create_channel_on_run: bool,
    pub channel_name_template: Option<String>,
    pub default_owner_id: Option<Uuid>,
    pub webhook_enabled: bool,
    pub webhook_secret: Option<String>,
    pub keyword_triggers: Option<Vec<String>>,
    pub is_archived: bool,
    pub version: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// DTO for creating a playbook
#[derive(Debug, Clone, Deserialize)]
pub struct CreatePlaybook {
    pub name: String,
    pub description: Option<String>,
    pub icon: Option<String>,
    pub is_public: Option<bool>,
    pub create_channel_on_run: Option<bool>,
    pub channel_name_template: Option<String>,
}

/// DTO for updating a playbook
#[derive(Debug, Clone, Deserialize)]
pub struct UpdatePlaybook {
    pub name: Option<String>,
    pub description: Option<String>,
    pub icon: Option<String>,
    pub is_public: Option<bool>,
    pub create_channel_on_run: Option<bool>,
    pub channel_name_template: Option<String>,
    pub keyword_triggers: Option<Vec<String>>,
}

/// Playbook checklist (group of tasks)
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct PlaybookChecklist {
    pub id: Uuid,
    pub playbook_id: Uuid,
    pub name: String,
    pub sort_order: i32,
    pub created_at: DateTime<Utc>,
}

/// DTO for creating a checklist
#[derive(Debug, Clone, Deserialize)]
pub struct CreateChecklist {
    pub name: String,
    pub sort_order: Option<i32>,
}

/// Playbook task (item within a checklist)
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct PlaybookTask {
    pub id: Uuid,
    pub checklist_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub default_assignee_id: Option<Uuid>,
    pub assignee_role: Option<String>,
    pub due_after_minutes: Option<i32>,
    pub slash_command: Option<String>,
    pub webhook_url: Option<String>,
    pub condition_attribute: Option<String>,
    pub condition_value: Option<String>,
    pub sort_order: i32,
    pub created_at: DateTime<Utc>,
}

/// DTO for creating a task
#[derive(Debug, Clone, Deserialize)]
pub struct CreateTask {
    pub title: String,
    pub description: Option<String>,
    pub default_assignee_id: Option<Uuid>,
    pub due_after_minutes: Option<i32>,
    pub slash_command: Option<String>,
    pub sort_order: Option<i32>,
}

/// Playbook run (instance of playbook execution)
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct PlaybookRun {
    pub id: Uuid,
    pub playbook_id: Uuid,
    pub team_id: Uuid,
    pub channel_id: Option<Uuid>,
    pub name: String,
    pub owner_id: Uuid,
    pub status: String,
    pub attributes: Option<serde_json::Value>,
    pub summary: Option<String>,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// DTO for starting a run
#[derive(Debug, Clone, Deserialize)]
pub struct StartRun {
    pub playbook_id: Uuid,
    pub name: String,
    pub owner_id: Option<Uuid>,
    pub channel_id: Option<Uuid>,
    pub attributes: Option<serde_json::Value>,
}

/// DTO for updating a run
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateRun {
    pub status: Option<String>,
    pub summary: Option<String>,
    pub attributes: Option<serde_json::Value>,
}

/// Run task status
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct RunTask {
    pub id: Uuid,
    pub run_id: Uuid,
    pub task_id: Uuid,
    pub status: String,
    pub assignee_id: Option<Uuid>,
    pub completed_at: Option<DateTime<Utc>>,
    pub completed_by: Option<Uuid>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// DTO for updating run task
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateRunTask {
    pub status: Option<String>,
    pub assignee_id: Option<Uuid>,
    pub notes: Option<String>,
}

/// Run status update
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct RunStatusUpdate {
    pub id: Uuid,
    pub run_id: Uuid,
    pub author_id: Uuid,
    pub message: String,
    pub is_broadcast: bool,
    pub created_at: DateTime<Utc>,
}

/// DTO for creating status update
#[derive(Debug, Clone, Deserialize)]
pub struct CreateStatusUpdate {
    pub message: String,
    pub is_broadcast: Option<bool>,
}

/// Playbook with checklists and tasks for full view
#[derive(Debug, Clone, Serialize)]
pub struct PlaybookFull {
    #[serde(flatten)]
    pub playbook: Playbook,
    pub checklists: Vec<ChecklistWithTasks>,
}

/// Checklist with its tasks
#[derive(Debug, Clone, Serialize)]
pub struct ChecklistWithTasks {
    #[serde(flatten)]
    pub checklist: PlaybookChecklist,
    pub tasks: Vec<PlaybookTask>,
}

/// Run with task statuses for dashboard
#[derive(Debug, Clone, Serialize)]
pub struct RunWithTasks {
    #[serde(flatten)]
    pub run: PlaybookRun,
    pub tasks: Vec<RunTask>,
    pub progress: RunProgress,
}

/// Run progress summary
#[derive(Debug, Clone, Serialize)]
pub struct RunProgress {
    pub total: i32,
    pub completed: i32,
    pub in_progress: i32,
    pub pending: i32,
}
