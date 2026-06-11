#![allow(clippy::needless_borrows_for_generic_args)]

use crate::common::{spawn_app, TestApp};
use reqwest::StatusCode;
use serde_json::{json, Value};
use uuid::Uuid;

mod common;

struct AuthContext {
    token: String,
    user_id: Uuid,
    org_id: Uuid,
    team_id: Uuid,
}

struct AgentContext {
    config_id: Uuid,
    user_id: Uuid,
}

async fn register_user(app: &TestApp, username: &str, email: &str, role: &str) -> AuthContext {
    let org_id = Uuid::new_v4();
    sqlx::query("INSERT INTO organizations (id, name) VALUES ($1, $2)")
        .bind(org_id)
        .bind(format!("{username} org"))
        .execute(&app.db_pool)
        .await
        .expect("organization should be inserted");

    let payload = json!({
        "username": username,
        "email": email,
        "password": "Password123!",
        "display_name": username,
        "org_id": org_id
    });

    let register_response = app
        .api_client
        .post(format!("{}/api/v1/auth/register", app.address))
        .json(&payload)
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
            "password": "Password123!"
        }))
        .send()
        .await
        .expect("login request should complete");
    assert_eq!(StatusCode::OK, login_response.status());

    let body: Value = login_response
        .json()
        .await
        .expect("login response should be JSON");
    let token = body["token"]
        .as_str()
        .expect("login response should include token")
        .to_string();
    let user_id = Uuid::parse_str(
        body["user"]["id"]
            .as_str()
            .expect("login response should include user id"),
    )
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

    sqlx::query("INSERT INTO team_members (team_id, user_id, role) VALUES ($1, $2, $3)")
        .bind(team_id)
        .bind(user_id)
        .bind(if role.contains("team_admin") {
            "team_admin"
        } else {
            "member"
        })
        .execute(&app.db_pool)
        .await
        .expect("team membership should be inserted");

    AuthContext {
        token,
        user_id,
        org_id,
        team_id,
    }
}

async fn create_channel(app: &TestApp, auth: &AuthContext, name: &str) -> Uuid {
    let channel_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO channels (id, team_id, name, display_name, type, creator_id) VALUES ($1, $2, $3, $4, $5::channel_type, $6)",
    )
    .bind(channel_id)
    .bind(auth.team_id)
    .bind(name)
    .bind(format!("{name} display"))
    .bind("public")
    .bind(auth.user_id)
    .execute(&app.db_pool)
    .await
    .expect("channel should be inserted");

    sqlx::query(
        "INSERT INTO channel_members (channel_id, user_id, role, notify_props) VALUES ($1, $2, 'member', '{}'::jsonb)",
    )
    .bind(channel_id)
    .bind(auth.user_id)
    .execute(&app.db_pool)
    .await
    .expect("channel membership should be inserted");

    channel_id
}

async fn create_agent_via_api(app: &TestApp, token: &str, username: &str) -> AgentContext {
    let response = app
        .api_client
        .post(format!("{}/api/v1/agents", app.address))
        .bearer_auth(token)
        .json(&json!({
            "username": username,
            "email": format!("{username}@example.com"),
            "display_name": format!("{username} display"),
            "title": format!("{username} helper"),
            "description": "integration test agent",
            "system_prompt": "Answer concisely.",
            "provider": "openai",
            "model": "gpt-4o-mini",
            "api_token": "",
            "temperature": 0.2,
            "max_context_messages": 8,
            "max_output_tokens": 256,
            "capabilities": {
                "respond_to_mentions": true,
                "respond_to_all": false,
                "use_memory": true,
                "use_rag": false
            },
            "rag_enabled": false,
            "rag_top_k": 3
        }))
        .send()
        .await
        .expect("create agent request should complete");
    assert_eq!(StatusCode::OK, response.status());

    let body: Value = response
        .json()
        .await
        .expect("agent response should be JSON");
    AgentContext {
        config_id: Uuid::parse_str(body["id"].as_str().expect("agent id should be present"))
            .expect("agent id should be UUID"),
        user_id: Uuid::parse_str(
            body["user_id"]
                .as_str()
                .expect("agent user id should be present"),
        )
        .expect("agent user id should be UUID"),
    }
}

async fn seed_agent(app: &TestApp, creator_id: Uuid, username: &str) -> AgentContext {
    let user_id = Uuid::new_v4();
    let config_id = Uuid::new_v4();

    sqlx::query(
        r#"
        INSERT INTO users (
            id, username, email, display_name,
            entity_type, api_key_hash, api_key_prefix,
            password_hash, is_bot, is_active, role, presence,
            notify_props, email_verified, created_at, updated_at
        )
        VALUES (
            $1, $2, $3, $4,
            'agent', NULL, NULL,
            'agent-login-disabled', TRUE, TRUE, 'member', 'offline',
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

    sqlx::query(
        r#"
        INSERT INTO agent_configs (
            id, user_id, title, description, system_prompt, provider, model,
            temperature, max_context_messages, max_output_tokens, capabilities,
            rag_enabled, rag_top_k, is_active, created_by
        )
        VALUES (
            $1, $2, $3, 'integration test agent', 'Answer concisely.', 'openai', 'gpt-4o-mini',
            0.2, 8, 256,
            '{"respond_to_mentions": true, "respond_to_all": false, "use_memory": true, "use_rag": false}'::jsonb,
            FALSE, 3, TRUE, $4
        )
        "#,
    )
    .bind(config_id)
    .bind(user_id)
    .bind(format!("{username} helper"))
    .bind(creator_id)
    .execute(&app.db_pool)
    .await
    .expect("agent config should be inserted");

    AgentContext { config_id, user_id }
}

async fn create_agent_post(
    app: &TestApp,
    channel_id: Uuid,
    agent_user_id: Uuid,
    from_agent: bool,
) -> Uuid {
    let post_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO posts (id, channel_id, user_id, message, props) VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(post_id)
    .bind(channel_id)
    .bind(agent_user_id)
    .bind("agent response")
    .bind(json!({ "from_agent": from_agent }))
    .execute(&app.db_pool)
    .await
    .expect("post should be inserted");
    post_id
}

#[tokio::test]
async fn api_v1_agents_create_contract() {
    let app = spawn_app().await;
    let admin = register_user(&app, "agentadmin", "agentadmin@example.com", "system_admin").await;

    let _agent = create_agent_via_api(&app, &admin.token, "createdagent").await;
}

#[tokio::test]
async fn api_v1_agents_read_update_delete_and_channel_assignment() {
    let app = spawn_app().await;
    let admin = register_user(
        &app,
        "agentcrudadmin",
        "agentcrudadmin@example.com",
        "system_admin",
    )
    .await;
    let channel_id = create_channel(&app, &admin, "agent-crud").await;

    let agent = seed_agent(&app, admin.user_id, "crudagent").await;

    let list_response = app
        .api_client
        .get(format!("{}/api/v1/agents", app.address))
        .bearer_auth(&admin.token)
        .send()
        .await
        .expect("list agents request should complete");
    assert_eq!(StatusCode::OK, list_response.status());
    let agents: Value = list_response.json().await.expect("list should be JSON");
    assert!(agents
        .as_array()
        .expect("agents should be an array")
        .iter()
        .any(|item| item["id"] == agent.config_id.to_string()));

    let get_response = app
        .api_client
        .get(format!("{}/api/v1/agents/{}", app.address, agent.config_id))
        .bearer_auth(&admin.token)
        .send()
        .await
        .expect("get agent request should complete");
    assert_eq!(StatusCode::OK, get_response.status());
    let detail: Value = get_response.json().await.expect("detail should be JSON");
    assert_eq!("crudagent", detail["username"]);

    let update_response = app
        .api_client
        .put(format!("{}/api/v1/agents/{}", app.address, agent.config_id))
        .bearer_auth(&admin.token)
        .json(&json!({
            "title": "Updated helper",
            "system_prompt": "Updated prompt.",
            "temperature": 0.4,
            "is_active": false
        }))
        .send()
        .await
        .expect("update agent request should complete");
    assert_eq!(StatusCode::OK, update_response.status());
    let updated: Value = update_response.json().await.expect("update should be JSON");
    assert_eq!("Updated helper", updated["title"]);
    assert_eq!(false, updated["is_active"]);

    let add_channel_response = app
        .api_client
        .post(format!(
            "{}/api/v1/agents/{}/channels/{}",
            app.address, agent.config_id, channel_id
        ))
        .bearer_auth(&admin.token)
        .send()
        .await
        .expect("assign channel request should complete");
    assert_eq!(StatusCode::OK, add_channel_response.status());

    let list_channels_response = app
        .api_client
        .get(format!(
            "{}/api/v1/agents/{}/channels",
            app.address, agent.config_id
        ))
        .bearer_auth(&admin.token)
        .send()
        .await
        .expect("list agent channels request should complete");
    assert_eq!(StatusCode::OK, list_channels_response.status());
    let channels: Value = list_channels_response
        .json()
        .await
        .expect("channels should be JSON");
    assert!(channels
        .as_array()
        .expect("channels should be an array")
        .iter()
        .any(|item| item["channel_id"] == channel_id.to_string()));

    let remove_channel_response = app
        .api_client
        .delete(format!(
            "{}/api/v1/agents/{}/channels/{}",
            app.address, agent.config_id, channel_id
        ))
        .bearer_auth(&admin.token)
        .send()
        .await
        .expect("remove channel request should complete");
    assert_eq!(StatusCode::OK, remove_channel_response.status());

    let delete_response = app
        .api_client
        .delete(format!("{}/api/v1/agents/{}", app.address, agent.config_id))
        .bearer_auth(&admin.token)
        .send()
        .await
        .expect("delete agent request should complete");
    assert_eq!(StatusCode::OK, delete_response.status());
}

#[tokio::test]
async fn api_v1_agents_reject_unauthenticated_and_malformed_requests() {
    let app = spawn_app().await;

    let unauthenticated = app
        .api_client
        .get(format!("{}/api/v1/agents", app.address))
        .send()
        .await
        .expect("unauthenticated request should complete");
    assert_eq!(StatusCode::UNAUTHORIZED, unauthenticated.status());

    let admin = register_user(
        &app,
        "agentvalidator",
        "agentvalidator@example.com",
        "system_admin",
    )
    .await;

    let malformed = app
        .api_client
        .post(format!("{}/api/v1/agents", app.address))
        .bearer_auth(&admin.token)
        .json(&json!({
            "username": "bad agent name",
            "email": "not-an-email",
            "title": "bad",
            "system_prompt": "bad",
            "provider": "openai",
            "model": "gpt-4o-mini"
        }))
        .send()
        .await
        .expect("malformed create request should complete");
    assert_eq!(StatusCode::BAD_REQUEST, malformed.status());
}

#[tokio::test]
async fn api_v1_agents_memories_feedback_stats_and_analytics() {
    let app = spawn_app().await;
    let admin = register_user(
        &app,
        "agentmetrics",
        "agentmetrics@example.com",
        "system_admin",
    )
    .await;
    let channel_id = create_channel(&app, &admin, "agent-metrics").await;
    let agent = seed_agent(&app, admin.user_id, "metricsagent").await;

    sqlx::query(
        "INSERT INTO agent_memories (agent_id, channel_id, memory_type, content, importance_score) VALUES ($1, $2, 'fact', $3, 0.9)",
    )
    .bind(agent.user_id)
    .bind(channel_id)
    .bind("The user prefers short answers.")
    .execute(&app.db_pool)
    .await
    .expect("memory should be inserted");

    let memories_response = app
        .api_client
        .get(format!(
            "{}/api/v1/agents/{}/memories",
            app.address, agent.config_id
        ))
        .bearer_auth(&admin.token)
        .send()
        .await
        .expect("list memories request should complete");
    assert_eq!(StatusCode::OK, memories_response.status());
    let memories: Value = memories_response
        .json()
        .await
        .expect("memories should be JSON");
    assert_eq!(
        1,
        memories.as_array().expect("memories should be array").len()
    );

    let missing_token_test = app
        .api_client
        .post(format!(
            "{}/api/v1/agents/{}/test",
            app.address, agent.config_id
        ))
        .bearer_auth(&admin.token)
        .json(&json!({ "message": "Hello test agent" }))
        .send()
        .await
        .expect("test agent request should complete");
    assert_eq!(StatusCode::BAD_REQUEST, missing_token_test.status());

    let post_id = create_agent_post(&app, channel_id, agent.user_id, true).await;

    let feedback_response = app
        .api_client
        .post(format!(
            "{}/api/v1/agents/posts/{}/feedback",
            app.address, post_id
        ))
        .bearer_auth(&admin.token)
        .json(&json!({
            "feedback_type": "positive",
            "comment": "useful"
        }))
        .send()
        .await
        .expect("submit feedback request should complete");
    assert_eq!(StatusCode::CREATED, feedback_response.status());

    let summary_response = app
        .api_client
        .get(format!(
            "{}/api/v1/agents/posts/{}/feedback",
            app.address, post_id
        ))
        .bearer_auth(&admin.token)
        .send()
        .await
        .expect("feedback summary request should complete");
    assert_eq!(StatusCode::OK, summary_response.status());
    let summary: Value = summary_response
        .json()
        .await
        .expect("summary should be JSON");
    assert_eq!(1, summary["positive_count"]);
    assert_eq!(0, summary["negative_count"]);

    let stats_response = app
        .api_client
        .get(format!(
            "{}/api/v1/agents/{}/feedback-stats",
            app.address, agent.user_id
        ))
        .bearer_auth(&admin.token)
        .send()
        .await
        .expect("feedback stats request should complete");
    assert_eq!(StatusCode::OK, stats_response.status());
    let stats: Value = stats_response.json().await.expect("stats should be JSON");
    assert_eq!(1, stats["total_positive"]);
    assert_eq!(1, stats["total_feedback"]);

    sqlx::query(
        "INSERT INTO agent_usage_logs (agent_id, channel_id, trigger_type, tokens_input, tokens_output, latency_ms, model) VALUES ($1, $2, 'mention', 10, 20, 30, 'gpt-4o-mini')",
    )
    .bind(agent.user_id)
    .bind(channel_id)
    .execute(&app.db_pool)
    .await
    .expect("usage log should be inserted");

    let analytics_response = app
        .api_client
        .get(format!(
            "{}/api/v1/agents/{}/analytics?days=7",
            app.address, agent.user_id
        ))
        .bearer_auth(&admin.token)
        .send()
        .await
        .expect("analytics request should complete");
    assert_eq!(StatusCode::OK, analytics_response.status());
    let analytics: Value = analytics_response
        .json()
        .await
        .expect("analytics should be JSON");
    assert_eq!(1, analytics["summary"]["total_invocations"]);
    assert_eq!(1, analytics["feedback_stats"]["total_feedback"]);

    let delete_feedback_response = app
        .api_client
        .delete(format!(
            "{}/api/v1/agents/posts/{}/feedback",
            app.address, post_id
        ))
        .bearer_auth(&admin.token)
        .send()
        .await
        .expect("delete feedback request should complete");
    assert_eq!(StatusCode::NO_CONTENT, delete_feedback_response.status());
}

#[tokio::test]
async fn api_v1_agents_feedback_rejects_non_members_and_non_agent_posts() {
    let app = spawn_app().await;
    let admin = register_user(
        &app,
        "agentfeedback",
        "agentfeedback@example.com",
        "system_admin",
    )
    .await;
    let outsider = register_user(&app, "outsider", "outsider@example.com", "member").await;
    assert_ne!(admin.org_id, outsider.org_id);

    let channel_id = create_channel(&app, &admin, "agent-feedback").await;
    let agent = seed_agent(&app, admin.user_id, "feedbackagent").await;
    let agent_post_id = create_agent_post(&app, channel_id, agent.user_id, true).await;

    let outsider_feedback = app
        .api_client
        .post(format!(
            "{}/api/v1/agents/posts/{}/feedback",
            app.address, agent_post_id
        ))
        .bearer_auth(&outsider.token)
        .json(&json!({ "feedback_type": "negative" }))
        .send()
        .await
        .expect("outsider feedback request should complete");
    assert_eq!(StatusCode::FORBIDDEN, outsider_feedback.status());

    let human_post_id = create_agent_post(&app, channel_id, admin.user_id, false).await;
    let non_agent_feedback = app
        .api_client
        .post(format!(
            "{}/api/v1/agents/posts/{}/feedback",
            app.address, human_post_id
        ))
        .bearer_auth(&admin.token)
        .json(&json!({ "feedback_type": "positive" }))
        .send()
        .await
        .expect("non-agent feedback request should complete");
    assert_eq!(StatusCode::BAD_REQUEST, non_agent_feedback.status());
}
