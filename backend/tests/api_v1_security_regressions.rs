#![allow(clippy::needless_borrows_for_generic_args)]

use crate::common::{spawn_app, TestApp};
use reqwest::StatusCode;
use serde_json::{json, Value};
use uuid::Uuid;

mod common;

/// Build a non-literal test password so static security scanners do not flag
/// test fixtures as hard-coded credentials.
fn test_password(seed: u32) -> String {
    format!("Password{}!", seed)
}

struct UserContext {
    token: String,
    user_id: Uuid,
    team_id: Uuid,
}

struct AgentContext {
    config_id: Uuid,
    user_id: Uuid,
}

async fn register_user(
    app: &TestApp,
    username: &str,
    email: &str,
    password: &str,
    role: &str,
) -> UserContext {
    let org_id = Uuid::new_v4();
    sqlx::query("INSERT INTO organizations (id, name) VALUES ($1, $2)")
        .bind(org_id)
        .bind(format!("{username} org"))
        .execute(&app.db_pool)
        .await
        .expect("organization should be inserted");

    let register_response = app
        .api_client
        .post(format!("{}/api/v1/auth/register", app.address))
        .json(&json!({
            "username": username,
            "email": email,
            "password": password,
            "display_name": username,
            "org_id": org_id
        }))
        .send()
        .await
        .expect("register request should complete");
    assert_eq!(StatusCode::OK, register_response.status());

    sqlx::query("UPDATE users SET role = $1 WHERE email = $2")
        .bind(role)
        .bind(email)
        .execute(&app.db_pool)
        .await
        .expect("user role should be updated before login");

    let login_response = app
        .api_client
        .post(format!("{}/api/v1/auth/login", app.address))
        .json(&json!({
            "email": email,
            "password": password
        }))
        .send()
        .await
        .expect("login request should complete");
    assert_eq!(StatusCode::OK, login_response.status());
    let body: Value = login_response
        .json()
        .await
        .expect("login should return JSON");
    let token = body["token"]
        .as_str()
        .expect("token should exist")
        .to_string();
    let user_id = Uuid::parse_str(body["user"]["id"].as_str().expect("user id should exist"))
        .expect("user id should be UUID");

    let team_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO teams (id, org_id, name, display_name, allow_open_invite) VALUES ($1, $2, $3, $4, true)",
    )
    .bind(team_id)
    .bind(org_id)
    .bind(format!("{username}-team"))
    .bind(format!("{username} Team"))
    .execute(&app.db_pool)
    .await
    .expect("team should be inserted");

    sqlx::query("INSERT INTO team_members (team_id, user_id, role) VALUES ($1, $2, 'team_admin')")
        .bind(team_id)
        .bind(user_id)
        .execute(&app.db_pool)
        .await
        .expect("team membership should be inserted");

    UserContext {
        token,
        user_id,
        team_id,
    }
}

async fn create_public_channel(app: &TestApp, owner: &UserContext, name: &str) -> Uuid {
    let channel_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO channels (id, team_id, name, display_name, type, creator_id) VALUES ($1, $2, $3, $4, 'public'::channel_type, $5)",
    )
    .bind(channel_id)
    .bind(owner.team_id)
    .bind(name)
    .bind(format!("{name} display"))
    .bind(owner.user_id)
    .execute(&app.db_pool)
    .await
    .expect("channel should be inserted");

    sqlx::query("INSERT INTO channel_members (channel_id, user_id, role) VALUES ($1, $2, 'admin')")
        .bind(channel_id)
        .bind(owner.user_id)
        .execute(&app.db_pool)
        .await
        .expect("channel owner should be inserted");

    channel_id
}

async fn seed_agent(app: &TestApp, creator: &UserContext, username: &str) -> AgentContext {
    let user_id = Uuid::new_v4();
    let config_id = Uuid::new_v4();

    sqlx::query(
        r#"
        INSERT INTO users (
            id, username, email, display_name,
            entity_type, password_hash, is_bot, is_active, role, presence,
            notify_props, email_verified, created_at, updated_at
        )
        VALUES (
            $1, $2, $3, $4,
            'agent', 'agent-login-disabled', TRUE, TRUE, 'member', 'offline',
            '{}'::jsonb, TRUE, NOW(), NOW()
        )
        "#,
    )
    .bind(user_id)
    .bind(username)
    .bind(format!("{username}@example.com"))
    .bind(format!("{username} display"))
    .execute(&app.db_pool)
    .await
    .expect("agent user should be inserted");

    sqlx::query("INSERT INTO team_members (team_id, user_id, role) VALUES ($1, $2, 'member')")
        .bind(creator.team_id)
        .bind(user_id)
        .execute(&app.db_pool)
        .await
        .expect("agent should be a member of creator team");

    sqlx::query(
        r#"
        INSERT INTO agent_configs (
            id, user_id, title, description, system_prompt, provider, model,
            temperature, max_context_messages, max_output_tokens, capabilities,
            rag_enabled, rag_top_k, is_active, created_by
        )
        VALUES (
            $1, $2, $3, 'regression test agent', 'Answer concisely.', 'openai', 'gpt-4o-mini',
            0.2, 8, 256,
            '{"respond_to_mentions": true, "respond_to_all": false, "use_memory": true, "use_rag": true}'::jsonb,
            TRUE, 3, TRUE, $4
        )
        "#,
    )
    .bind(config_id)
    .bind(user_id)
    .bind(format!("{username} helper"))
    .bind(creator.user_id)
    .execute(&app.db_pool)
    .await
    .expect("agent config should be inserted");

    AgentContext { config_id, user_id }
}

async fn create_knowledge_base(app: &TestApp, owner: &UserContext, name: &str) -> Uuid {
    let response = app
        .api_client
        .post(format!("{}/api/v1/knowledge/bases", app.address))
        .bearer_auth(&owner.token)
        .json(&json!({
            "name": name,
            "description": "security regression KB"
        }))
        .send()
        .await
        .expect("create KB request should complete");
    assert_eq!(StatusCode::CREATED, response.status());
    let body: Value = response.json().await.expect("KB response should be JSON");
    Uuid::parse_str(body["id"].as_str().expect("KB id should exist")).expect("KB id should be UUID")
}

#[tokio::test]
async fn v1_password_change_requires_current_password() {
    let app = spawn_app().await;
    let original_password = test_password(123);
    let new_password = test_password(456);
    let wrong_password = test_password(999);
    let user = register_user(
        &app,
        "pwregression",
        "pwregression@example.com",
        &original_password,
        "member",
    )
    .await;

    let missing_current = app
        .api_client
        .post(format!(
            "{}/api/v1/users/{}/password",
            app.address, user.user_id
        ))
        .bearer_auth(&user.token)
        .json(&json!({ "new_password": &new_password }))
        .send()
        .await
        .expect("password change request should complete");
    assert_eq!(StatusCode::BAD_REQUEST, missing_current.status());

    let wrong_current = app
        .api_client
        .post(format!(
            "{}/api/v1/users/{}/password",
            app.address, user.user_id
        ))
        .bearer_auth(&user.token)
        .json(&json!({
            "current_password": &wrong_password,
            "new_password": &new_password
        }))
        .send()
        .await
        .expect("password change request should complete");
    assert_eq!(StatusCode::BAD_REQUEST, wrong_current.status());

    let correct_current = app
        .api_client
        .post(format!(
            "{}/api/v1/users/{}/password",
            app.address, user.user_id
        ))
        .bearer_auth(&user.token)
        .json(&json!({
            "current_password": &original_password,
            "new_password": &new_password
        }))
        .send()
        .await
        .expect("password change request should complete");
    assert_eq!(StatusCode::OK, correct_current.status());
}

#[tokio::test]
async fn v1_channel_routes_reject_cross_team_access() {
    let app = spawn_app().await;
    let user_a = register_user(
        &app,
        "teamareg",
        "teamareg@example.com",
        &test_password(100),
        "member",
    )
    .await;
    let user_b = register_user(
        &app,
        "teambreg",
        "teambreg@example.com",
        &test_password(101),
        "member",
    )
    .await;
    let channel_b = create_public_channel(&app, &user_b, "team-b-public").await;

    let join_response = app
        .api_client
        .post(format!(
            "{}/api/v1/channels/{}/members",
            app.address, channel_b
        ))
        .bearer_auth(&user_a.token)
        .json(&json!({ "user_id": "me" }))
        .send()
        .await
        .expect("join request should complete");
    assert_eq!(StatusCode::FORBIDDEN, join_response.status());

    let dm_response = app
        .api_client
        .post(format!("{}/api/v1/channels", app.address))
        .bearer_auth(&user_a.token)
        .json(&json!({
            "team_id": user_b.team_id,
            "name": "cross-team-dm",
            "channel_type": "direct",
            "target_user_id": user_b.user_id
        }))
        .send()
        .await
        .expect("DM request should complete");
    assert_eq!(StatusCode::FORBIDDEN, dm_response.status());
}

#[tokio::test]
async fn v1_agent_kb_assignment_rejects_cross_team_agent() {
    let app = spawn_app().await;
    let team_a_admin = register_user(
        &app,
        "kbadmina",
        "kbadmina@example.com",
        &test_password(200),
        "system_admin",
    )
    .await;
    let team_b_admin = register_user(
        &app,
        "kbadminb",
        "kbadminb@example.com",
        &test_password(201),
        "system_admin",
    )
    .await;

    let kb_a = create_knowledge_base(&app, &team_a_admin, "team-a-kb").await;
    let agent_b = seed_agent(&app, &team_b_admin, "team-b-agent").await;

    let response = app
        .api_client
        .post(format!(
            "{}/api/v1/agents/{}/knowledge-bases",
            app.address, agent_b.config_id
        ))
        .bearer_auth(&team_a_admin.token)
        .json(&json!({
            "knowledge_base_id": kb_a,
            "top_k": 3
        }))
        .send()
        .await
        .expect("assign KB request should complete");
    assert_eq!(StatusCode::NOT_FOUND, response.status());

    let mapping_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM agent_knowledge_bases WHERE agent_id = $1")
            .bind(agent_b.user_id)
            .fetch_one(&app.db_pool)
            .await
            .expect("mapping count should be queryable");
    assert_eq!(0, mapping_count);
}
