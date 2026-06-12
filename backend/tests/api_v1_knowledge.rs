#![allow(clippy::needless_borrows_for_generic_args)]

use crate::common::{spawn_app, TestApp};
use reqwest::{multipart, StatusCode};
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
}

async fn register_user(app: &TestApp, username: &str, email: &str, role: &str) -> AuthContext {
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
            "password": "Password123!",
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

async fn create_knowledge_base(app: &TestApp, token: &str, name: &str) -> Uuid {
    let response = app
        .api_client
        .post(format!("{}/api/v1/knowledge/bases", app.address))
        .bearer_auth(token)
        .json(&json!({
            "name": name,
            "description": "integration test knowledge base",
            "embedding_model": "text-embedding-3-small",
            "embedding_dimensions": 1536,
            "chunk_size": 512,
            "chunk_overlap": 64
        }))
        .send()
        .await
        .expect("create knowledge base request should complete");
    assert_eq!(StatusCode::CREATED, response.status());

    let body: Value = response
        .json()
        .await
        .expect("knowledge base should be JSON");
    Uuid::parse_str(body["id"].as_str().expect("knowledge base id should exist"))
        .expect("knowledge base id should be UUID")
}

async fn seed_agent(
    app: &TestApp,
    team_id: Uuid,
    creator_id: Uuid,
    username: &str,
) -> AgentContext {
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

    sqlx::query("INSERT INTO team_members (team_id, user_id, role) VALUES ($1, $2, 'member')")
        .bind(team_id)
        .bind(user_id)
        .execute(&app.db_pool)
        .await
        .expect("agent should be a member of the team");

    sqlx::query(
        r#"
        INSERT INTO agent_configs (
            id, user_id, title, description, system_prompt, provider, model,
            temperature, max_context_messages, max_output_tokens, capabilities,
            rag_enabled, rag_top_k, is_active, created_by
        )
        VALUES (
            $1, $2, $3, 'integration test agent', 'Use assigned knowledge bases.', 'openai', 'gpt-4o-mini',
            0.2, 8, 256,
            '{"respond_to_mentions": true, "respond_to_all": false, "use_memory": true, "use_rag": true}'::jsonb,
            TRUE, 3, TRUE, $4
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

    AgentContext { config_id }
}

async fn seed_document(app: &TestApp, auth: &AuthContext, kb_id: Uuid, title: &str) -> Uuid {
    let doc_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO knowledge_documents (
            id, knowledge_base_id, team_id, title, source_type, s3_key, s3_bucket,
            content_hash, mime_type, size_bytes, extracted_text, created_by
        )
        VALUES ($1, $2, $3, $4, 'upload', $5, 'test-bucket', $6, 'text/plain', 12, 'hello docs', $7)
        "#,
    )
    .bind(doc_id)
    .bind(kb_id)
    .bind(auth.team_id)
    .bind(title)
    .bind(format!("knowledge/{}/{}/{}", auth.team_id, kb_id, title))
    .bind(format!("{doc_id:x}"))
    .bind(auth.user_id)
    .execute(&app.db_pool)
    .await
    .expect("document should be inserted");
    doc_id
}

#[tokio::test]
async fn api_v1_knowledge_bases_crud_and_document_routes() {
    let app = spawn_app().await;
    let admin = register_user(
        &app,
        "knowledgeadmin",
        "knowledgeadmin@example.com",
        "team_admin",
    )
    .await;
    assert!(admin.org_id.to_string().len() > 30);

    let kb_id = create_knowledge_base(&app, &admin.token, "Runbooks").await;

    let list_response = app
        .api_client
        .get(format!("{}/api/v1/knowledge/bases", app.address))
        .bearer_auth(&admin.token)
        .send()
        .await
        .expect("list knowledge bases request should complete");
    assert_eq!(StatusCode::OK, list_response.status());
    let bases: Value = list_response.json().await.expect("bases should be JSON");
    assert!(bases
        .as_array()
        .expect("bases should be an array")
        .iter()
        .any(|item| item["id"] == kb_id.to_string()));

    let get_response = app
        .api_client
        .get(format!("{}/api/v1/knowledge/bases/{}", app.address, kb_id))
        .bearer_auth(&admin.token)
        .send()
        .await
        .expect("get knowledge base request should complete");
    assert_eq!(StatusCode::OK, get_response.status());
    let base: Value = get_response.json().await.expect("base should be JSON");
    assert_eq!("Runbooks", base["name"]);

    let update_response = app
        .api_client
        .put(format!("{}/api/v1/knowledge/bases/{}", app.address, kb_id))
        .bearer_auth(&admin.token)
        .json(&json!({
            "name": "Updated Runbooks",
            "description": "updated",
            "chunk_overlap": 32,
            "is_active": false
        }))
        .send()
        .await
        .expect("update knowledge base request should complete");
    assert_eq!(StatusCode::OK, update_response.status());
    let updated: Value = update_response
        .json()
        .await
        .expect("updated base should be JSON");
    assert_eq!("Updated Runbooks", updated["name"]);
    assert_eq!(false, updated["is_active"]);

    let unsupported_upload = app
        .api_client
        .post(format!(
            "{}/api/v1/knowledge/bases/{}/documents",
            app.address, kb_id
        ))
        .bearer_auth(&admin.token)
        .multipart(
            multipart::Form::new().part(
                "file",
                multipart::Part::bytes("not allowed".as_bytes().to_vec())
                    .file_name("blocked.bin")
                    .mime_str("application/octet-stream")
                    .expect("mime type should be valid"),
            ),
        )
        .send()
        .await
        .expect("upload document request should complete");
    assert_eq!(StatusCode::BAD_REQUEST, unsupported_upload.status());

    let doc_id = seed_document(&app, &admin, kb_id, "guide.txt").await;
    let list_docs_response = app
        .api_client
        .get(format!(
            "{}/api/v1/knowledge/bases/{}/documents",
            app.address, kb_id
        ))
        .bearer_auth(&admin.token)
        .send()
        .await
        .expect("list documents request should complete");
    assert_eq!(StatusCode::OK, list_docs_response.status());
    let docs: Value = list_docs_response
        .json()
        .await
        .expect("docs should be JSON");
    assert!(docs
        .as_array()
        .expect("docs should be an array")
        .iter()
        .any(|item| item["id"] == doc_id.to_string()));

    let get_doc_response = app
        .api_client
        .get(format!(
            "{}/api/v1/knowledge/documents/{}",
            app.address, doc_id
        ))
        .bearer_auth(&admin.token)
        .send()
        .await
        .expect("get document request should complete");
    assert_eq!(StatusCode::OK, get_doc_response.status());

    let delete_doc_response = app
        .api_client
        .delete(format!(
            "{}/api/v1/knowledge/documents/{}",
            app.address, doc_id
        ))
        .bearer_auth(&admin.token)
        .send()
        .await
        .expect("delete document request should complete");
    assert_eq!(StatusCode::NO_CONTENT, delete_doc_response.status());

    let delete_kb_response = app
        .api_client
        .delete(format!("{}/api/v1/knowledge/bases/{}", app.address, kb_id))
        .bearer_auth(&admin.token)
        .send()
        .await
        .expect("delete knowledge base request should complete");
    assert_eq!(StatusCode::NO_CONTENT, delete_kb_response.status());
}

#[tokio::test]
async fn api_v1_knowledge_sync_sources_agent_assignment_and_webhook() {
    let app = spawn_app().await;
    let admin = register_user(&app, "syncadmin", "syncadmin@example.com", "system_admin").await;
    let kb_id = create_knowledge_base(&app, &admin.token, "Synced Docs").await;
    let agent = seed_agent(&app, admin.team_id, admin.user_id, "knowledgeagent").await;

    let create_source_response = app
        .api_client
        .post(format!("{}/api/v1/knowledge/sync-sources", app.address))
        .bearer_auth(&admin.token)
        .json(&json!({
            "name": "RustShare",
            "source_type": "rustshare",
            "config": {
                "knowledge_base_id": kb_id,
                "folder_id": "folder-1",
                "base_url": "https://rustshare.example.test"
            },
            "sync_mode": "push",
            "sync_interval_minutes": 60
        }))
        .send()
        .await
        .expect("create sync source request should complete");
    assert_eq!(StatusCode::CREATED, create_source_response.status());
    let source: Value = create_source_response
        .json()
        .await
        .expect("sync source should be JSON");
    let source_id = Uuid::parse_str(source["id"].as_str().expect("source id should exist"))
        .expect("source id should be UUID");
    assert!(source.get("config_encrypted").is_none());

    let stored_config: (String,) =
        sqlx::query_as("SELECT config_encrypted FROM knowledge_sync_sources WHERE id = $1")
            .bind(source_id)
            .fetch_one(&app.db_pool)
            .await
            .expect("sync source config should be stored");
    assert!(stored_config.0.starts_with("enc:v1:"));
    let decrypted_config = rustchat::crypto::decrypt(&stored_config.0, "test-encryption-key")
        .expect("sync source config should decrypt");
    let decrypted_config: Value =
        serde_json::from_str(&decrypted_config).expect("sync source config should be JSON");
    assert_eq!(decrypted_config["knowledge_base_id"], kb_id.to_string());

    let list_sources_response = app
        .api_client
        .get(format!("{}/api/v1/knowledge/sync-sources", app.address))
        .bearer_auth(&admin.token)
        .send()
        .await
        .expect("list sync sources request should complete");
    assert_eq!(StatusCode::OK, list_sources_response.status());
    let sources: Value = list_sources_response
        .json()
        .await
        .expect("sources should be JSON");
    assert!(sources
        .as_array()
        .expect("sources should be an array")
        .iter()
        .any(|item| item["id"] == source_id.to_string()));

    let get_source_response = app
        .api_client
        .get(format!(
            "{}/api/v1/knowledge/sync-sources/{}",
            app.address, source_id
        ))
        .bearer_auth(&admin.token)
        .send()
        .await
        .expect("get sync source request should complete");
    assert_eq!(StatusCode::OK, get_source_response.status());

    let update_source_response = app
        .api_client
        .put(format!(
            "{}/api/v1/knowledge/sync-sources/{}",
            app.address, source_id
        ))
        .bearer_auth(&admin.token)
        .json(&json!({
            "name": "RustShare Updated",
            "sync_interval_minutes": 120,
            "is_active": false
        }))
        .send()
        .await
        .expect("update sync source request should complete");
    assert_eq!(StatusCode::OK, update_source_response.status());

    let assign_response = app
        .api_client
        .post(format!(
            "{}/api/v1/agents/{}/knowledge-bases",
            app.address, agent.config_id
        ))
        .bearer_auth(&admin.token)
        .json(&json!({
            "knowledge_base_id": kb_id,
            "top_k": 4,
            "relevance_threshold": 0.7
        }))
        .send()
        .await
        .expect("assign knowledge base request should complete");
    assert_eq!(StatusCode::OK, assign_response.status());

    let list_assignments_response = app
        .api_client
        .get(format!(
            "{}/api/v1/agents/{}/knowledge-bases",
            app.address, agent.config_id
        ))
        .bearer_auth(&admin.token)
        .send()
        .await
        .expect("list agent knowledge base request should complete");
    assert_eq!(StatusCode::OK, list_assignments_response.status());
    let assignments: Value = list_assignments_response
        .json()
        .await
        .expect("assignments should be JSON");
    assert!(assignments
        .as_array()
        .expect("assignments should be an array")
        .iter()
        .any(|item| item["knowledge_base_id"] == kb_id.to_string()));

    let unassign_response = app
        .api_client
        .delete(format!(
            "{}/api/v1/agents/{}/knowledge-bases/{}",
            app.address, agent.config_id, kb_id
        ))
        .bearer_auth(&admin.token)
        .send()
        .await
        .expect("unassign knowledge base request should complete");
    assert_eq!(StatusCode::NO_CONTENT, unassign_response.status());

    let webhook_response = app
        .api_client
        .post(format!("{}/api/v1/knowledge/sync/rustshare", app.address))
        .json(&json!({
            "event": "file.updated",
            "webhook_id": "webhook-1",
            "timestamp": "2026-06-10T20:00:00Z",
            "folder_id": "folder-1",
            "file": {
                "id": "file-1",
                "name": "guide.md",
                "mime_type": "text/markdown",
                "size_bytes": 42,
                "etag": "etag-1",
                "modified_at": "2026-06-10T20:00:00Z"
            }
        }))
        .send()
        .await
        .expect("rustshare webhook request should complete");
    assert_eq!(StatusCode::OK, webhook_response.status());

    let delete_source_response = app
        .api_client
        .delete(format!(
            "{}/api/v1/knowledge/sync-sources/{}",
            app.address, source_id
        ))
        .bearer_auth(&admin.token)
        .send()
        .await
        .expect("delete sync source request should complete");
    assert_eq!(StatusCode::NO_CONTENT, delete_source_response.status());
}
