    extract::{State},
    Json,
};
use crate::api::AppState;
use crate::api::v4::extractors::MmAuthUser;
use crate::error::{ApiResult};
use crate::mattermost_compat::{models as mm};

pub async fn get_audits(
    State(state): State<AppState>,
    _auth: MmAuthUser,
) -> ApiResult<Json<Vec<mm::Audit>>> {
    let audits: Vec<mm::Audit> = sqlx::query_as(
        r#"
        SELECT id::text, 
               (extract(epoch from created_at)*1000)::int8 as create_at,
               actor_user_id::text as user_id,
               action,
               metadata::text as extra_info,
               actor_ip as ip_address,
               '' as session_id
        FROM audit_logs
        ORDER BY created_at DESC
        LIMIT 100
        "#
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(audits))
}
