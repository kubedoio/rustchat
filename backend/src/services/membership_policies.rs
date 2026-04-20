use crate::api::AppState;
use crate::error::ApiResult;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use std::collections::HashSet;

use uuid::Uuid;

/// Source types for policy applicability
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "VARCHAR", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum PolicySourceType {
    AllUsers,
    AuthService,
    Group,
    Role,
    Org,
}

/// Scope types for policies
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "VARCHAR", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum PolicyScopeType {
    Global,
    Team,
}

/// Target types for policy targets
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "VARCHAR", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum PolicyTargetType {
    Team,
    Channel,
}

/// Role modes for policy targets
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "VARCHAR", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum RoleMode {
    Member,
    Admin,
}

/// Origin types for membership tracking
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "VARCHAR", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum MembershipOrigin {
    Manual,
    Policy,
    Invite,
    Sync,
    Default,
}

/// Membership type for origin tracking
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "VARCHAR", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum MembershipType {
    Team,
    Channel,
}

/// Auto membership policy
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct AutoMembershipPolicy {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub scope_type: PolicyScopeType,
    pub team_id: Option<Uuid>,
    pub source_type: PolicySourceType,
    pub source_config: serde_json::Value,
    pub enabled: bool,
    pub priority: i32,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Policy target (team or channel)
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct AutoMembershipPolicyTarget {
    pub id: Uuid,
    pub policy_id: Uuid,
    pub target_type: PolicyTargetType,
    pub target_id: Uuid,
    pub role_mode: RoleMode,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Policy audit log entry
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct AutoMembershipPolicyAudit {
    pub id: Uuid,
    pub policy_id: Option<Uuid>,
    pub run_id: Uuid,
    pub user_id: Uuid,
    pub target_type: PolicyTargetType,
    pub target_id: Uuid,
    pub action: String,
    pub status: String,
    pub error_message: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Membership origin record
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct MembershipOriginRecord {
    pub id: Uuid,
    pub membership_type: MembershipType,
    pub membership_id: Uuid,
    pub origin: MembershipOrigin,
    pub policy_id: Option<Uuid>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Create policy request
#[derive(Debug, Clone, Deserialize)]
pub struct CreatePolicyRequest {
    pub name: String,
    pub description: Option<String>,
    pub scope_type: PolicyScopeType,
    pub team_id: Option<Uuid>,
    pub source_type: PolicySourceType,
    #[serde(default)]
    pub source_config: serde_json::Value,
    pub enabled: bool,
    #[serde(default)]
    pub priority: i32,
    pub targets: Vec<CreatePolicyTarget>,
}

/// Create policy target request
#[derive(Debug, Clone, Deserialize)]
pub struct CreatePolicyTarget {
    pub target_type: PolicyTargetType,
    pub target_id: Uuid,
    #[serde(default = "default_role_mode")]
    pub role_mode: RoleMode,
}

fn default_role_mode() -> RoleMode {
    RoleMode::Member
}

/// Update policy request
#[derive(Debug, Clone, Deserialize)]
pub struct UpdatePolicyRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub enabled: Option<bool>,
    pub priority: Option<i32>,
    pub source_config: Option<serde_json::Value>,
    pub targets: Option<Vec<CreatePolicyTarget>>,
}

/// Policy with targets
#[derive(Debug, Clone, Serialize)]
pub struct PolicyWithTargets {
    #[serde(flatten)]
    pub policy: AutoMembershipPolicy,
    pub targets: Vec<AutoMembershipPolicyTarget>,
}

/// Policy repository for database operations
pub struct PolicyRepository<'a> {
    db: &'a sqlx::PgPool,
}

impl<'a> PolicyRepository<'a> {
    pub fn new(db: &'a sqlx::PgPool) -> Self {
        Self { db }
    }

    /// Create a new policy with targets
    pub async fn create_policy(&self, req: CreatePolicyRequest) -> ApiResult<PolicyWithTargets> {
        // Validate: at least one target is required
        if req.targets.is_empty() {
            return Err(crate::error::AppError::Validation(
                "At least one target is required".to_string(),
            ));
        }

        // Validate: check for duplicate targets in request
        let mut seen = std::collections::HashSet::new();
        for target in &req.targets {
            let key = (target.target_type, target.target_id);
            if !seen.insert(key) {
                return Err(crate::error::AppError::Validation(format!(
                    "Duplicate target: {:?} {}",
                    target.target_type, target.target_id
                )));
            }
        }

        let mut tx = self.db.begin().await?;

        // Validate: team_id must exist for team-scoped policies
        if let Some(team_id) = req.team_id {
            let team_exists: bool =
                sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM teams WHERE id = $1)")
                    .bind(team_id)
                    .fetch_one(&mut *tx)
                    .await?;
            if !team_exists {
                return Err(crate::error::AppError::Validation(format!(
                    "Team {} does not exist",
                    team_id
                )));
            }
        }

        // Validate: all targets must exist
        for target in &req.targets {
            match target.target_type {
                PolicyTargetType::Team => {
                    let exists: bool =
                        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM teams WHERE id = $1)")
                            .bind(target.target_id)
                            .fetch_one(&mut *tx)
                            .await?;
                    if !exists {
                        return Err(crate::error::AppError::Validation(format!(
                            "Target team {} does not exist",
                            target.target_id
                        )));
                    }
                }
                PolicyTargetType::Channel => {
                    let exists: bool =
                        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM channels WHERE id = $1)")
                            .bind(target.target_id)
                            .fetch_one(&mut *tx)
                            .await?;
                    if !exists {
                        return Err(crate::error::AppError::Validation(format!(
                            "Target channel {} does not exist",
                            target.target_id
                        )));
                    }
                }
            }
        }

        // Insert policy
        let policy: AutoMembershipPolicy = sqlx::query_as(
            r#"
            INSERT INTO auto_membership_policies 
                (name, description, scope_type, team_id, source_type, source_config, enabled, priority)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING *
            "#,
        )
        .bind(&req.name)
        .bind(&req.description)
        .bind(req.scope_type)
        .bind(req.team_id)
        .bind(req.source_type)
        .bind(&req.source_config)
        .bind(req.enabled)
        .bind(req.priority)
        .fetch_one(&mut *tx)
        .await?;

        // Insert targets
        let mut targets = Vec::new();
        for target_req in req.targets {
            let target: AutoMembershipPolicyTarget = sqlx::query_as(
                r#"
                INSERT INTO auto_membership_policy_targets 
                    (policy_id, target_type, target_id, role_mode)
                VALUES ($1, $2, $3, $4)
                RETURNING *
                "#,
            )
            .bind(policy.id)
            .bind(target_req.target_type)
            .bind(target_req.target_id)
            .bind(target_req.role_mode)
            .fetch_one(&mut *tx)
            .await?;
            targets.push(target);
        }

        tx.commit().await?;

        Ok(PolicyWithTargets { policy, targets })
    }

    /// Get policy by ID with targets
    pub async fn get_policy(&self, policy_id: Uuid) -> ApiResult<Option<PolicyWithTargets>> {
        let policy: Option<AutoMembershipPolicy> =
            sqlx::query_as("SELECT * FROM auto_membership_policies WHERE id = $1")
                .bind(policy_id)
                .fetch_optional(self.db)
                .await?;

        match policy {
            Some(policy) => {
                let targets: Vec<AutoMembershipPolicyTarget> = sqlx::query_as(
                    "SELECT * FROM auto_membership_policy_targets WHERE policy_id = $1",
                )
                .bind(policy_id)
                .fetch_all(self.db)
                .await?;

                Ok(Some(PolicyWithTargets { policy, targets }))
            }
            None => Ok(None),
        }
    }

    /// List policies with optional filters
    pub async fn list_policies(
        &self,
        scope_type: Option<PolicyScopeType>,
        team_id: Option<Uuid>,
        enabled: Option<bool>,
    ) -> ApiResult<Vec<PolicyWithTargets>> {
        let mut query = String::from("SELECT * FROM auto_membership_policies WHERE 1=1");

        if scope_type.is_some() {
            query.push_str(" AND scope_type = $1");
        }
        if team_id.is_some() {
            query.push_str(&format!(
                " AND team_id = ${}",
                if scope_type.is_some() { 2 } else { 1 }
            ));
        }
        if enabled.is_some() {
            let param_num = 1 + scope_type.is_some() as i32 + team_id.is_some() as i32;
            query.push_str(&format!(" AND enabled = ${}", param_num));
        }

        query.push_str(" ORDER BY priority DESC, created_at ASC");

        let mut q = sqlx::query_as(&query);

        if let Some(st) = scope_type {
            q = q.bind(st);
        }
        if let Some(ti) = team_id {
            q = q.bind(ti);
        }
        if let Some(e) = enabled {
            q = q.bind(e);
        }

        let policies: Vec<AutoMembershipPolicy> = q.fetch_all(self.db).await?;

        // Fetch targets for each policy
        let mut result = Vec::new();
        for policy in policies {
            let targets: Vec<AutoMembershipPolicyTarget> =
                sqlx::query_as("SELECT * FROM auto_membership_policy_targets WHERE policy_id = $1")
                    .bind(policy.id)
                    .fetch_all(self.db)
                    .await?;

            result.push(PolicyWithTargets { policy, targets });
        }

        Ok(result)
    }

    /// Update policy
    pub async fn update_policy(
        &self,
        policy_id: Uuid,
        req: UpdatePolicyRequest,
    ) -> ApiResult<Option<PolicyWithTargets>> {
        let mut tx = self.db.begin().await?;

        // Build dynamic update query
        let mut updates = Vec::new();
        let mut param_idx = 1;

        if let Some(_name) = &req.name {
            updates.push(format!("name = ${}", param_idx));
            param_idx += 1;
        }
        if req.description.is_some() {
            updates.push(format!("description = ${}", param_idx));
            param_idx += 1;
        }
        if let Some(_enabled) = req.enabled {
            updates.push(format!("enabled = ${}", param_idx));
            param_idx += 1;
        }
        if let Some(_priority) = req.priority {
            updates.push(format!("priority = ${}", param_idx));
            param_idx += 1;
        }
        if req.source_config.is_some() {
            updates.push(format!("source_config = ${}", param_idx));
            param_idx += 1;
        }

        if !updates.is_empty() {
            let query = format!(
                "UPDATE auto_membership_policies SET {} WHERE id = ${} RETURNING *",
                updates.join(", "),
                param_idx
            );

            let mut q = sqlx::query_as(&query);

            if let Some(name) = &req.name {
                q = q.bind(name);
            }
            if let Some(desc) = &req.description {
                q = q.bind(desc);
            }
            if let Some(enabled) = req.enabled {
                q = q.bind(enabled);
            }
            if let Some(priority) = req.priority {
                q = q.bind(priority);
            }
            if let Some(config) = &req.source_config {
                q = q.bind(config);
            }
            q = q.bind(policy_id);

            let _: Option<AutoMembershipPolicy> = q.fetch_optional(&mut *tx).await?;
        }

        // Update targets if provided
        if let Some(targets) = req.targets {
            // Validate: at least one target is required
            if targets.is_empty() {
                return Err(crate::error::AppError::Validation(
                    "At least one target is required".to_string(),
                ));
            }

            // Validate: check for duplicate targets
            let mut seen = std::collections::HashSet::new();
            for target in &targets {
                let key = (target.target_type, target.target_id);
                if !seen.insert(key) {
                    return Err(crate::error::AppError::Validation(format!(
                        "Duplicate target: {:?} {}",
                        target.target_type, target.target_id
                    )));
                }
            }

            // Validate: all targets must exist
            for target in &targets {
                match target.target_type {
                    PolicyTargetType::Team => {
                        let exists: bool =
                            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM teams WHERE id = $1)")
                                .bind(target.target_id)
                                .fetch_one(&mut *tx)
                                .await?;
                        if !exists {
                            return Err(crate::error::AppError::Validation(format!(
                                "Target team {} does not exist",
                                target.target_id
                            )));
                        }
                    }
                    PolicyTargetType::Channel => {
                        let exists: bool = sqlx::query_scalar(
                            "SELECT EXISTS(SELECT 1 FROM channels WHERE id = $1)",
                        )
                        .bind(target.target_id)
                        .fetch_one(&mut *tx)
                        .await?;
                        if !exists {
                            return Err(crate::error::AppError::Validation(format!(
                                "Target channel {} does not exist",
                                target.target_id
                            )));
                        }
                    }
                }
            }

            // Delete existing targets
            sqlx::query("DELETE FROM auto_membership_policy_targets WHERE policy_id = $1")
                .bind(policy_id)
                .execute(&mut *tx)
                .await?;

            // Insert new targets
            for target_req in targets {
                sqlx::query(
                    r#"
                    INSERT INTO auto_membership_policy_targets 
                        (policy_id, target_type, target_id, role_mode)
                    VALUES ($1, $2, $3, $4)
                    "#,
                )
                .bind(policy_id)
                .bind(target_req.target_type)
                .bind(target_req.target_id)
                .bind(target_req.role_mode)
                .execute(&mut *tx)
                .await?;
            }
        }

        tx.commit().await?;

        self.get_policy(policy_id).await
    }

    /// Delete policy
    pub async fn delete_policy(&self, policy_id: Uuid) -> ApiResult<bool> {
        let result = sqlx::query("DELETE FROM auto_membership_policies WHERE id = $1")
            .bind(policy_id)
            .execute(self.db)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Get applicable policies for a user
    pub async fn get_applicable_policies(
        &self,
        user_id: Uuid,
        team_id: Option<Uuid>,
    ) -> ApiResult<Vec<PolicyWithTargets>> {
        // Get user's auth provider and identity attributes.
        let user_info: Option<(Option<String>, String, Option<Uuid>)> =
            sqlx::query_as("SELECT auth_provider, role, org_id FROM users WHERE id = $1")
                .bind(user_id)
                .fetch_optional(self.db)
                .await?;

        let (auth_provider, user_role, user_org_id) = match user_info {
            Some(info) => info,
            None => return Ok(Vec::new()),
        };

        // Get all enabled global policies
        let global_policies = self
            .list_policies(Some(PolicyScopeType::Global), None, Some(true))
            .await?;

        // Get team-specific policies if team_id provided
        let team_policies = if let Some(tid) = team_id {
            self.list_policies(Some(PolicyScopeType::Team), Some(tid), Some(true))
                .await?
        } else {
            Vec::new()
        };

        // Combine and filter by source type applicability
        let mut applicable = Vec::new();

        for policy in global_policies.into_iter().chain(team_policies) {
            let is_applicable = match policy.policy.source_type {
                PolicySourceType::AllUsers => true,
                PolicySourceType::AuthService => {
                    // Backwards compatible key handling: accept auth_provider or auth_service.
                    let config_auth = policy
                        .policy
                        .source_config
                        .get("auth_provider")
                        .and_then(|v| v.as_str())
                        .or_else(|| {
                            policy
                                .policy
                                .source_config
                                .get("auth_service")
                                .and_then(|v| v.as_str())
                        });
                    config_auth.is_none() || config_auth == auth_provider.as_deref()
                }
                PolicySourceType::Group => {
                    // Support both group UUIDs (for synced/internal groups) and group names
                    // Group names are matched against the groups.display_name column
                    let group_ids = extract_uuid_values(
                        &policy.policy.source_config,
                        &["group_ids", "group_id"],
                    );
                    let group_names = extract_string_values(
                        &policy.policy.source_config,
                        &["group_names", "names"],
                    );

                    if group_ids.is_empty() && group_names.is_empty() {
                        false
                    } else {
                        // Check by UUID if provided
                        let has_id_match = if !group_ids.is_empty() {
                            let group_ids_vec: Vec<Uuid> = group_ids.into_iter().collect();
                            sqlx::query_scalar(
                                "SELECT EXISTS(SELECT 1 FROM group_members WHERE user_id = $1 AND group_id = ANY($2))",
                            )
                            .bind(user_id)
                            .bind(&group_ids_vec)
                            .fetch_one(self.db)
                            .await?
                        } else {
                            false
                        };

                        // Check by name if provided (for OIDC groups not synced to UUIDs)
                        let has_name_match = if !group_names.is_empty() {
                            let names_vec: Vec<String> = group_names.into_iter().collect();
                            sqlx::query_scalar(
                                r#"
                                SELECT EXISTS(
                                    SELECT 1 FROM group_members gm
                                    JOIN groups g ON g.id = gm.group_id
                                    WHERE gm.user_id = $1 
                                    AND LOWER(g.display_name) = ANY($2)
                                )
                                "#,
                            )
                            .bind(user_id)
                            .bind(
                                names_vec
                                    .iter()
                                    .map(|n| n.to_lowercase())
                                    .collect::<Vec<_>>(),
                            )
                            .fetch_one(self.db)
                            .await?
                        } else {
                            false
                        };

                        has_id_match || has_name_match
                    }
                }
                PolicySourceType::Role => {
                    let roles =
                        extract_string_values(&policy.policy.source_config, &["roles", "role"]);
                    if roles.is_empty() {
                        false
                    } else {
                        roles.contains(&user_role)
                    }
                }
                PolicySourceType::Org => {
                    let org_ids =
                        extract_uuid_values(&policy.policy.source_config, &["org_ids", "org_id"]);
                    if org_ids.is_empty() {
                        false
                    } else if let Some(org_id) = user_org_id {
                        org_ids.contains(&org_id)
                    } else {
                        false
                    }
                }
            };

            if is_applicable {
                applicable.push(policy);
            }
        }

        // Sort by priority (highest first)
        applicable.sort_by_key(|a| std::cmp::Reverse(a.policy.priority));

        Ok(applicable)
    }
}

fn extract_string_values(config: &serde_json::Value, keys: &[&str]) -> HashSet<String> {
    let mut out = HashSet::new();
    for key in keys {
        let Some(value) = config.get(*key) else {
            continue;
        };
        match value {
            serde_json::Value::String(raw) => {
                for part in raw.split(',') {
                    let trimmed = part.trim();
                    if !trimmed.is_empty() {
                        out.insert(trimmed.to_string());
                    }
                }
            }
            serde_json::Value::Array(items) => {
                for item in items {
                    if let Some(raw) = item.as_str() {
                        for part in raw.split(',') {
                            let trimmed = part.trim();
                            if !trimmed.is_empty() {
                                out.insert(trimmed.to_string());
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }
    out
}

fn extract_uuid_values(config: &serde_json::Value, keys: &[&str]) -> HashSet<Uuid> {
    extract_string_values(config, keys)
        .into_iter()
        .filter_map(|value| Uuid::parse_str(&value).ok())
        .collect()
}

/// Record membership origin
pub async fn record_membership_origin(
    db: &sqlx::PgPool,
    membership_type: MembershipType,
    membership_id: Uuid,
    origin: MembershipOrigin,
    policy_id: Option<Uuid>,
) -> ApiResult<()> {
    sqlx::query(
        r#"
        INSERT INTO membership_origins (membership_type, membership_id, origin, policy_id)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (membership_type, membership_id) 
        DO UPDATE SET 
            origin = EXCLUDED.origin,
            policy_id = EXCLUDED.policy_id,
            updated_at = NOW()
        "#,
    )
    .bind(membership_type)
    .bind(membership_id)
    .bind(origin)
    .bind(policy_id)
    .execute(db)
    .await?;

    Ok(())
}

/// Get membership origin
pub async fn get_membership_origin(
    db: &sqlx::PgPool,
    membership_type: MembershipType,
    membership_id: Uuid,
) -> ApiResult<Option<MembershipOrigin>> {
    let result: Option<(MembershipOrigin,)> = sqlx::query_as(
        "SELECT origin FROM membership_origins WHERE membership_type = $1 AND membership_id = $2",
    )
    .bind(membership_type)
    .bind(membership_id)
    .fetch_optional(db)
    .await?;

    Ok(result.map(|r| r.0))
}

/// Apply auto-membership policies for a user joining a team
pub async fn apply_auto_membership_for_team_join(
    state: &AppState,
    user_id: Uuid,
    team_id: Uuid,
    _trigger: &str,
) -> ApiResult<Vec<AutoMembershipPolicyAudit>> {
    let repo = PolicyRepository::new(&state.db);
    let policies = repo.get_applicable_policies(user_id, Some(team_id)).await?;

    let run_id = Uuid::new_v4();
    let mut audit_entries = Vec::new();

    for policy in policies {
        for target in &policy.targets {
            // Only process team or channel targets in this team context
            let should_apply = match target.target_type {
                PolicyTargetType::Team => target.target_id == team_id,
                PolicyTargetType::Channel => {
                    // Verify channel belongs to this team
                    let channel_team: Option<(Uuid,)> =
                        sqlx::query_as("SELECT team_id FROM channels WHERE id = $1")
                            .bind(target.target_id)
                            .fetch_optional(&state.db)
                            .await?;

                    channel_team.map(|ct| ct.0 == team_id).unwrap_or(false)
                }
            };

            if !should_apply {
                continue;
            }

            let (action, status, error_msg) = match target.target_type {
                PolicyTargetType::Team => {
                    // Add to team
                    // Note: team_members uses composite key (team_id, user_id), no id column
                    let result = sqlx::query(
                        r#"
                        INSERT INTO team_members (team_id, user_id, role)
                        VALUES ($1, $2, $3)
                        ON CONFLICT (team_id, user_id) DO NOTHING
                        "#,
                    )
                    .bind(target.target_id)
                    .bind(user_id)
                    .bind(match target.role_mode {
                        RoleMode::Member => "member",
                        RoleMode::Admin => "admin",
                    })
                    .execute(&state.db)
                    .await;

                    match result {
                        Ok(query_result) => {
                            if query_result.rows_affected() > 0 {
                                // Record membership origin using team_id as identifier
                                let _ = record_membership_origin(
                                    &state.db,
                                    MembershipType::Team,
                                    target.target_id, // Use team_id as membership identifier
                                    MembershipOrigin::Policy,
                                    Some(policy.policy.id),
                                )
                                .await;
                                ("add", "success", None)
                            } else {
                                ("skip", "success", Some("Already member".to_string()))
                            }
                        }
                        Err(e) => ("add", "failed", Some(e.to_string())),
                    }
                }
                PolicyTargetType::Channel => {
                    // Add to channel
                    let result = sqlx::query(
                        r#"
                        INSERT INTO channel_members (channel_id, user_id, role)
                        VALUES ($1, $2, $3)
                        ON CONFLICT (channel_id, user_id) DO NOTHING
                        RETURNING channel_id
                        "#,
                    )
                    .bind(target.target_id)
                    .bind(user_id)
                    .bind(match target.role_mode {
                        RoleMode::Member => "member",
                        RoleMode::Admin => "admin",
                    })
                    .fetch_optional(&state.db)
                    .await;

                    match result {
                        Ok(Some(_)) => {
                            // Get the composite key for membership_origins
                            // channel_members uses (channel_id, user_id) as PK
                            // We store channel_id as membership_id for channel membership
                            let _ = record_membership_origin(
                                &state.db,
                                MembershipType::Channel,
                                target.target_id, // Use channel_id as identifier
                                MembershipOrigin::Policy,
                                Some(policy.policy.id),
                            )
                            .await;
                            ("add", "success", None)
                        }
                        Ok(None) => ("skip", "success", Some("Already member".to_string())),
                        Err(e) => ("add", "failed", Some(e.to_string())),
                    }
                }
            };

            // Record audit entry
            let audit: AutoMembershipPolicyAudit = sqlx::query_as(
                r#"
                INSERT INTO auto_membership_policy_audit 
                    (policy_id, run_id, user_id, target_type, target_id, action, status, error_message)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                RETURNING *
                "#,
            )
            .bind(Some(policy.policy.id))
            .bind(run_id)
            .bind(user_id)
            .bind(target.target_type)
            .bind(target.target_id)
            .bind(action)
            .bind(status)
            .bind(error_msg)
            .fetch_one(&state.db)
            .await?;

            audit_entries.push(audit);
        }
    }

    Ok(audit_entries)
}

/// Get audit log for a policy
pub async fn get_policy_audit_log(
    db: &sqlx::PgPool,
    policy_id: Uuid,
    limit: i64,
    offset: i64,
) -> ApiResult<Vec<AutoMembershipPolicyAudit>> {
    let audits: Vec<AutoMembershipPolicyAudit> = sqlx::query_as(
        r#"
        SELECT * FROM auto_membership_policy_audit 
        WHERE policy_id = $1
        ORDER BY created_at DESC
        LIMIT $2 OFFSET $3
        "#,
    )
    .bind(policy_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(db)
    .await?;

    Ok(audits)
}

/// Get last run status for a policy
pub async fn get_policy_last_run_status(
    db: &sqlx::PgPool,
    policy_id: Uuid,
) -> ApiResult<Option<(i64, i64)>> {
    // Returns (success_count, failed_count) for last run
    let result: Option<(i64, i64)> = sqlx::query_as(
        r#"
        WITH last_run AS (
            SELECT run_id, MAX(created_at) as max_created
            FROM auto_membership_policy_audit
            WHERE policy_id = $1
            GROUP BY run_id
            ORDER BY max_created DESC
            LIMIT 1
        )
        SELECT 
            COUNT(*) FILTER (WHERE status = 'success') as success_count,
            COUNT(*) FILTER (WHERE status = 'failed') as failed_count
        FROM auto_membership_policy_audit
        WHERE policy_id = $1
        AND run_id = (SELECT run_id FROM last_run)
        "#,
    )
    .bind(policy_id)
    .fetch_optional(db)
    .await?;

    Ok(result)
}

/// Apply auto-membership policies for a newly registered user
/// This handles global policies that add users to teams and channels
pub async fn apply_auto_membership_for_new_user(
    state: &AppState,
    user_id: Uuid,
) -> ApiResult<Vec<AutoMembershipPolicyAudit>> {
    let repo = PolicyRepository::new(&state.db);

    // Get all enabled global policies that apply to this user
    let policies = repo.get_applicable_policies(user_id, None).await?;

    // Filter to only global policies (team-scoped policies don't apply to new users without teams)
    let global_policies: Vec<_> = policies
        .into_iter()
        .filter(|p| p.policy.scope_type == PolicyScopeType::Global)
        .collect();

    if global_policies.is_empty() {
        return Ok(Vec::new());
    }

    let run_id = Uuid::new_v4();
    let mut audit_entries = Vec::new();

    for policy in global_policies {
        for target in &policy.targets {
            let (action, status, error_msg) = match target.target_type {
                PolicyTargetType::Team => {
                    // Add user to team
                    // Note: team_members uses composite key (team_id, user_id), no id column
                    let result = sqlx::query(
                        r#"
                        INSERT INTO team_members (team_id, user_id, role)
                        VALUES ($1, $2, $3)
                        ON CONFLICT (team_id, user_id) DO NOTHING
                        "#,
                    )
                    .bind(target.target_id)
                    .bind(user_id)
                    .bind(match target.role_mode {
                        RoleMode::Member => "member",
                        RoleMode::Admin => "admin",
                    })
                    .execute(&state.db)
                    .await;

                    match result {
                        Ok(query_result) => {
                            if query_result.rows_affected() > 0 {
                                // Record membership origin using team_id as identifier
                                let _ = record_membership_origin(
                                    &state.db,
                                    MembershipType::Team,
                                    target.target_id, // Use team_id as membership identifier
                                    MembershipOrigin::Policy,
                                    Some(policy.policy.id),
                                )
                                .await;
                                ("add", "success", None)
                            } else {
                                ("skip", "success", Some("Already member".to_string()))
                            }
                        }
                        Err(e) => ("add", "failed", Some(e.to_string())),
                    }
                }
                PolicyTargetType::Channel => {
                    // For channel targets in global policies, we need to add the user to the channel
                    // This assumes the user is already a member of the team containing the channel
                    // or the channel is public/open
                    let result = sqlx::query(
                        r#"
                        INSERT INTO channel_members (channel_id, user_id, role)
                        VALUES ($1, $2, $3)
                        ON CONFLICT (channel_id, user_id) DO NOTHING
                        RETURNING channel_id
                        "#,
                    )
                    .bind(target.target_id)
                    .bind(user_id)
                    .bind(match target.role_mode {
                        RoleMode::Member => "member",
                        RoleMode::Admin => "admin",
                    })
                    .fetch_optional(&state.db)
                    .await;

                    match result {
                        Ok(Some(_)) => {
                            let _ = record_membership_origin(
                                &state.db,
                                MembershipType::Channel,
                                target.target_id,
                                MembershipOrigin::Policy,
                                Some(policy.policy.id),
                            )
                            .await;
                            ("add", "success", None)
                        }
                        Ok(None) => ("skip", "success", Some("Already member".to_string())),
                        Err(e) => ("add", "failed", Some(e.to_string())),
                    }
                }
            };

            // Record audit entry
            let audit: AutoMembershipPolicyAudit = sqlx::query_as(
                r#"
                INSERT INTO auto_membership_policy_audit 
                    (policy_id, run_id, user_id, target_type, target_id, action, status, error_message)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                RETURNING *
                "#,
            )
            .bind(Some(policy.policy.id))
            .bind(run_id)
            .bind(user_id)
            .bind(target.target_type)
            .bind(target.target_id)
            .bind(action)
            .bind(status)
            .bind(error_msg)
            .fetch_one(&state.db)
            .await?;

            audit_entries.push(audit);
        }
    }

    Ok(audit_entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn create_policy_request_deserializes_snake_case_variants() {
        let payload = json!({
            "name": "Team policy",
            "description": "Auto add users",
            "scope_type": "team",
            "team_id": "11111111-1111-1111-1111-111111111111",
            "source_type": "all_users",
            "source_config": {},
            "enabled": true,
            "priority": 0,
            "targets": [
                {
                    "target_type": "channel",
                    "target_id": "22222222-2222-2222-2222-222222222222",
                    "role_mode": "member"
                }
            ]
        });

        let parsed: CreatePolicyRequest =
            serde_json::from_value(payload).expect("snake_case create payload must deserialize");

        assert_eq!(parsed.scope_type, PolicyScopeType::Team);
        assert_eq!(parsed.source_type, PolicySourceType::AllUsers);
        assert_eq!(parsed.targets.len(), 1);
        assert_eq!(parsed.targets[0].target_type, PolicyTargetType::Channel);
        assert_eq!(parsed.targets[0].role_mode, RoleMode::Member);
    }

    #[test]
    fn policy_enum_serialization_is_snake_case() {
        assert_eq!(
            serde_json::to_value(PolicyScopeType::Global).expect("serialize scope"),
            json!("global")
        );
        assert_eq!(
            serde_json::to_value(PolicySourceType::AuthService).expect("serialize source"),
            json!("auth_service")
        );
        assert_eq!(
            serde_json::to_value(PolicyTargetType::Team).expect("serialize target"),
            json!("team")
        );
        assert_eq!(
            serde_json::to_value(RoleMode::Admin).expect("serialize role"),
            json!("admin")
        );
    }
}
