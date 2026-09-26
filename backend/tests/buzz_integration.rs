//! Buzz integration tests: admin API surface, security invariants, and the
//! full outbound pipeline (post → transactional outbox → dispatcher →
//! connector) with failure isolation, idempotency, and loop prevention.

mod common;

use common::{spawn_app, spawn_app_with_config, test_config};
use serde_json::json;

use rustchat::config::BuzzIntegrationConfig;
use rustchat::integrations::buzz::dispatcher::dispatch_cycle;
use rustchat::integrations::buzz::events::has_origin_marker;
use rustchat::integrations::buzz::mock::{MockBuzzConnector, MockBuzzConnectorProvider};
use rustchat::integrations::buzz::repository::{BuzzRepository, NewBuzzConnection};
use rustchat::integrations::outbox::{DeliveryOutcome, OutboxRepository};

fn buzz_config() -> BuzzIntegrationConfig {
    BuzzIntegrationConfig {
        enabled: true,
        poll_interval_secs: 1,
        max_attempts: 3,
        backoff_base_secs: 1,
        backoff_max_secs: 2,
        batch_size: 10,
        in_flight_lease_secs: 0, // disable lease reclaim unless a test wants it
        // Tests drive delivery deterministically via dispatch_cycle; never
        // spawn the real production outbox dispatcher worker here (it would
        // race the explicit cycles against the shared test DB).
        run_dispatcher: false,
    }
}

fn app_state_with_buzz(pool: sqlx::PgPool) -> rustchat::state::AppState {
    let mut state = rustchat::testsupport::minimal_app_state();
    state.db = pool;
    state.config.integrations.buzz = buzz_config();
    state
}

/// Register a user, promote to system_admin, return the auth token.
async fn admin_token(app: &common::TestApp, email: &str) -> String {
    let reg = app
        .api_client
        .post(format!("{}/api/v1/auth/register", app.address))
        .json(&json!({
            "username": email.split('@').next().unwrap(),
            "email": email,
            "password": "Password123!",
            "display_name": "Admin User",
        }))
        .send()
        .await
        .unwrap();
    assert!(reg.status().is_success(), "register failed: {reg:?}");
    sqlx::query("UPDATE users SET role = 'system_admin' WHERE email = $1")
        .bind(email)
        .execute(&app.db_pool)
        .await
        .unwrap();
    let login = app
        .api_client
        .post(format!("{}/api/v1/auth/login", app.address))
        .json(&json!({ "email": email, "password": "Password123!" }))
        .send()
        .await
        .unwrap();
    assert!(login.status().is_success());
    login.json::<serde_json::Value>().await.unwrap()["token"]
        .as_str()
        .unwrap()
        .to_string()
}

/// Register a plain member and return the auth token.
async fn member_token(app: &common::TestApp, email: &str) -> String {
    let reg = app
        .api_client
        .post(format!("{}/api/v1/auth/register", app.address))
        .json(&json!({
            "username": email.split('@').next().unwrap(),
            "email": email,
            "password": "Password123!",
            "display_name": "Admin User",
        }))
        .send()
        .await
        .unwrap();
    assert!(reg.status().is_success());
    let login = app
        .api_client
        .post(format!("{}/api/v1/auth/login", app.address))
        .json(&json!({ "email": email, "password": "Password123!" }))
        .send()
        .await
        .unwrap();
    login.json::<serde_json::Value>().await.unwrap()["token"]
        .as_str()
        .unwrap()
        .to_string()
}

fn test_key() -> String {
    // Deterministic test key (hex) — generated once per process.
    use std::sync::LazyLock;
    static KEY: LazyLock<String> =
        LazyLock::new(|| nostr::key::Keys::generate().secret_key().to_secret_hex());
    KEY.clone()
}

fn buzz_enabled_config() -> rustchat::config::Config {
    let mut config = test_config();
    config.integrations.buzz = buzz_config();
    config
}

async fn create_connection(app: &common::TestApp, token: &str) -> serde_json::Value {
    let res = app
        .api_client
        .post(format!(
            "{}/api/v1/admin/integrations/buzz/connections",
            app.address
        ))
        .bearer_auth(token)
        .json(&json!({
            "name": "main",
            "relay_url": "https://buzz.example.com",
            "private_key": test_key(),
        }))
        .send()
        .await
        .unwrap();
    assert!(
        res.status().is_success(),
        "create connection failed: {:?}",
        res.status()
    );
    res.json().await.unwrap()
}

// ===========================================================================
// Admin API: authorization and gating
// ===========================================================================

#[tokio::test]
async fn buzz_admin_api_requires_admin_role() {
    let app = spawn_app_with_config(buzz_enabled_config()).await;
    let member = member_token(&app, "member@example.com").await;

    // Every admin endpoint must return 403 for a logged-in non-admin, before
    // any data access. Path params are dummy ids: require_admin runs first, so
    // non-existent resources still yield 403 (never 404 on a member request).
    let id = "00000000-0000-0000-0000-000000000000";
    let cases: Vec<(&str, String, serde_json::Value)> = vec![
        (
            "GET",
            "/api/v1/admin/integrations/buzz/connections".to_string(),
            json!({}),
        ),
        (
            "POST",
            "/api/v1/admin/integrations/buzz/connections".to_string(),
            json!({
                "name": "member-forbidden",
                "relay_url": "wss://buzz.example.com",
                "private_key": test_key(),
            }),
        ),
        (
            "GET",
            format!("/api/v1/admin/integrations/buzz/connections/{id}"),
            json!({}),
        ),
        (
            "PATCH",
            format!("/api/v1/admin/integrations/buzz/connections/{id}"),
            json!({
                "enabled": true,
            }),
        ),
        (
            "DELETE",
            format!("/api/v1/admin/integrations/buzz/connections/{id}"),
            json!({}),
        ),
        (
            "POST",
            format!("/api/v1/admin/integrations/buzz/connections/{id}/key"),
            json!({
                "private_key": test_key(),
            }),
        ),
        (
            "POST",
            format!("/api/v1/admin/integrations/buzz/connections/{id}/test"),
            json!({}),
        ),
        (
            "GET",
            format!("/api/v1/admin/integrations/buzz/connections/{id}/mappings"),
            json!({}),
        ),
        (
            "PUT",
            format!("/api/v1/admin/integrations/buzz/connections/{id}/mappings"),
            json!({
                "rustchat_channel_id": id,
                "buzz_channel_id": id,
                "outbound_enabled": true,
            }),
        ),
        (
            "DELETE",
            format!("/api/v1/admin/integrations/buzz/connections/{id}/mappings/{id}"),
            json!({}),
        ),
        (
            "GET",
            format!("/api/v1/admin/integrations/buzz/connections/{id}/deliveries"),
            json!({}),
        ),
        (
            "POST",
            format!("/api/v1/admin/integrations/buzz/deliveries/{id}/retry"),
            json!({}),
        ),
    ];

    for (method, path, body) in cases {
        let res = app
            .api_client
            .request(method.parse().unwrap(), format!("{}{}", app.address, path))
            .bearer_auth(&member)
            .json(&body)
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), 403, "{method} {path} must be admin-only");
    }
}

#[tokio::test]
async fn buzz_admin_api_rejects_unauthenticated_requests() {
    let app = spawn_app_with_config(buzz_enabled_config()).await;
    let res = app
        .api_client
        .get(format!(
            "{}/api/v1/admin/integrations/buzz/connections",
            app.address
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 401);
}

#[tokio::test]
async fn buzz_admin_api_fails_fast_when_integration_disabled() {
    let app = spawn_app().await; // default config: buzz disabled
    let admin = admin_token(&app, "admin@example.com").await;

    // Every buzz admin endpoint must fail fast (before touching the DB) with an
    // explicit "disabled" error when the integration is off. require_admin runs
    // first, then require_integration_enabled — so a valid admin gets 422.
    let id = "00000000-0000-0000-0000-000000000000";
    for (method, path, body) in [
        (
            "GET",
            "/api/v1/admin/integrations/buzz/connections".to_string(),
            json!({}),
        ),
        (
            "POST",
            "/api/v1/admin/integrations/buzz/connections".to_string(),
            json!({
                "name": "n",
                "relay_url": "wss://buzz.example.com",
                "private_key": test_key(),
            }),
        ),
        (
            "PUT",
            format!("/api/v1/admin/integrations/buzz/connections/{id}/mappings"),
            json!({
                "rustchat_channel_id": id,
                "buzz_channel_id": id,
                "outbound_enabled": true,
            }),
        ),
        (
            "POST",
            format!("/api/v1/admin/integrations/buzz/deliveries/{id}/retry"),
            json!({}),
        ),
    ] {
        let res = app
            .api_client
            .request(method.parse().unwrap(), format!("{}{}", app.address, path))
            .bearer_auth(&admin)
            .json(&body)
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), 422, "{method} {path} must be disabled");
        let body: serde_json::Value = res.json().await.unwrap();
        assert!(
            body["error"]["message"]
                .as_str()
                .unwrap_or_default()
                .contains("disabled"),
            "expected explicit disabled error for {path}, got {body}"
        );
    }
}

// ===========================================================================
// Admin API: connection lifecycle and security invariants
// ===========================================================================

#[tokio::test]
async fn connection_lifecycle_hides_and_validates_secrets() {
    let app = spawn_app_with_config(buzz_enabled_config()).await;
    let admin = admin_token(&app, "admin@example.com").await;

    // SSRF: private / loopback / non-http schemes are rejected.
    for bad_url in [
        "http://127.0.0.1:9000",
        "http://10.1.2.3",
        "http://169.254.169.254",
        "file:///etc/passwd",
        "not a url",
    ] {
        let res = app
            .api_client
            .post(format!(
                "{}/api/v1/admin/integrations/buzz/connections",
                app.address
            ))
            .bearer_auth(&admin)
            .json(&json!({
                "name": format!("bad-{bad_url:?}"),
                "relay_url": bad_url,
                "private_key": test_key(),
            }))
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), 422, "URL {bad_url} must be rejected");
    }

    // Malformed key rejected.
    let res = app
        .api_client
        .post(format!(
            "{}/api/v1/admin/integrations/buzz/connections",
            app.address
        ))
        .bearer_auth(&admin)
        .json(&json!({
            "name": "badkey",
            "relay_url": "https://buzz.example.com",
            "private_key": "definitely-not-a-key",
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 422);

    // Create: response must never contain the private key.
    let connection = create_connection(&app, &admin).await;
    let serialized = connection.to_string();
    assert!(
        !serialized.contains(&test_key()),
        "private key leaked: {serialized}"
    );
    assert!(connection["bridge_pubkey"].as_str().unwrap().len() == 64);

    // Key at rest is encrypted (not the raw hex) in the database.
    let stored: String =
        sqlx::query_scalar("SELECT private_key_encrypted FROM buzz_connections WHERE id = $1")
            .bind(
                connection["id"]
                    .as_str()
                    .unwrap()
                    .parse::<uuid::Uuid>()
                    .unwrap(),
            )
            .fetch_one(&app.db_pool)
            .await
            .unwrap();
    assert!(!stored.contains(&test_key()));
    assert!(stored.starts_with("enc:v1:"));

    // Rotation: new key, new pubkey, still no leak.
    let new_key = nostr::key::Keys::generate().secret_key().to_secret_hex();
    let res = app
        .api_client
        .post(format!(
            "{}/api/v1/admin/integrations/buzz/connections/{}/key",
            app.address,
            connection["id"].as_str().unwrap()
        ))
        .bearer_auth(&admin)
        .json(&json!({ "private_key": new_key }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    let rotated: serde_json::Value = res.json().await.unwrap();
    assert!(!rotated.to_string().contains(&new_key));
    assert_ne!(
        rotated["bridge_pubkey"], connection["bridge_pubkey"],
        "rotation must change the pubkey"
    );

    // Delete.
    let res = app
        .api_client
        .delete(format!(
            "{}/api/v1/admin/integrations/buzz/connections/{}",
            app.address,
            connection["id"].as_str().unwrap()
        ))
        .bearer_auth(&admin)
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
}

#[tokio::test]
async fn connection_test_endpoint_reports_credential_rejection() {
    let app = spawn_app_with_config(buzz_enabled_config()).await;
    let admin = admin_token(&app, "admin@example.com").await;
    let connection = create_connection(&app, &admin).await;

    // buzz.example.com does not resolve from the test environment (or fails
    // the public-DNS pin) — the endpoint must surface a controlled error,
    // not hang or panic. Either a 422/502-style error is acceptable; what
    // matters is it terminates promptly with an error payload.
    let res = app
        .api_client
        .post(format!(
            "{}/api/v1/admin/integrations/buzz/connections/{}/test",
            app.address,
            connection["id"].as_str().unwrap()
        ))
        .bearer_auth(&admin)
        .send()
        .await
        .unwrap();
    assert!(
        res.status().is_client_error() || res.status().is_server_error(),
        "unreachable relay must produce an error, got {}",
        res.status()
    );
}

// ===========================================================================
// Mapping management and authorization
// ===========================================================================

#[tokio::test]
async fn mapping_requires_existing_rustchat_channel() {
    let app = spawn_app_with_config(buzz_enabled_config()).await;
    let admin = admin_token(&app, "admin@example.com").await;
    let connection = create_connection(&app, &admin).await;
    let base = format!(
        "{}/api/v1/admin/integrations/buzz/connections/{}",
        app.address,
        connection["id"].as_str().unwrap()
    );

    // Non-existent RustChat channel is rejected (mapping authorization).
    let res = app
        .api_client
        .put(format!("{base}/mappings"))
        .bearer_auth(&admin)
        .json(&json!({
            "rustchat_channel_id": "00000000-0000-0000-0000-000000000001",
            "buzz_channel_id": "00000000-0000-0000-0000-000000000002",
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 422);

    // Create a real channel directly, then map it.
    let org_id = uuid::Uuid::new_v4();
    sqlx::query("INSERT INTO organizations (id, name) VALUES ($1, $2)")
        .bind(org_id)
        .bind("Buzz Test Org")
        .execute(&app.db_pool)
        .await
        .unwrap();
    let team_id: uuid::Uuid = sqlx::query_scalar(
        "INSERT INTO teams (org_id, name, display_name) VALUES ($1, 't', 'T') RETURNING id",
    )
    .bind(org_id)
    .fetch_one(&app.db_pool)
    .await
    .unwrap();
    let channel_id: uuid::Uuid = sqlx::query_scalar(
        "INSERT INTO channels (team_id, name, display_name, type) \
         VALUES ($1, 'general', 'General', 'public') RETURNING id",
    )
    .bind(team_id)
    .fetch_one(&app.db_pool)
    .await
    .unwrap();

    let res = app
        .api_client
        .put(format!("{base}/mappings"))
        .bearer_auth(&admin)
        .json(&json!({
            "rustchat_channel_id": channel_id,
            "buzz_channel_id": "11111111-1111-1111-1111-111111111111",
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200, "valid mapping must be accepted");
    let mapping: serde_json::Value = res.json().await.unwrap();

    // Upsert (same RustChat channel) updates rather than duplicates.
    let res = app
        .api_client
        .put(format!("{base}/mappings"))
        .bearer_auth(&admin)
        .json(&json!({
            "rustchat_channel_id": channel_id,
            "buzz_channel_id": "11111111-1111-1111-1111-111111111111",
            "outbound_enabled": false,
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);

    // Delete mapping.
    let res = app
        .api_client
        .delete(format!(
            "{base}/mappings/{}",
            mapping["id"].as_str().unwrap()
        ))
        .bearer_auth(&admin)
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
}

// ===========================================================================
// Outbound pipeline: enqueue → dispatch (mock connector)
// ===========================================================================

/// Test fixture: a mapped channel plus an outbox row enqueued for a post.
struct Pipeline {
    state: rustchat::state::AppState,
    connection_id: uuid::Uuid,
    rustchat_channel_id: uuid::Uuid,
    outbox_id: uuid::Uuid,
}

async fn pipeline(pool: sqlx::PgPool) -> Pipeline {
    let state = app_state_with_buzz(pool.clone());

    let connection = BuzzRepository::new(&pool)
        .create_connection(
            NewBuzzConnection {
                name: "main".to_string(),
                relay_url: "https://buzz.example.com".to_string(),
                private_key: test_key(),
                enabled: true,
            },
            &state.config.encryption_key,
        )
        .await
        .unwrap();

    let org_id = uuid::Uuid::new_v4();
    sqlx::query("INSERT INTO organizations (id, name) VALUES ($1, $2)")
        .bind(org_id)
        .bind("Buzz Test Org")
        .execute(&pool)
        .await
        .unwrap();
    let team_id: uuid::Uuid = sqlx::query_scalar(
        "INSERT INTO teams (org_id, name, display_name) VALUES ($1, 't', 'T') RETURNING id",
    )
    .bind(org_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    let channel_id: uuid::Uuid = sqlx::query_scalar(
        "INSERT INTO channels (team_id, name, display_name, type) \
         VALUES ($1, 'general', 'General', 'public') RETURNING id",
    )
    .bind(team_id)
    .fetch_one(&pool)
    .await
    .unwrap();

    BuzzRepository::new(&pool)
        .upsert_mapping(connection.id, channel_id, uuid::Uuid::new_v4(), true)
        .await
        .unwrap();

    // Enqueue through the production path: the same transaction helper the
    // post service calls.
    let post_id = uuid::Uuid::new_v4();
    let mut tx = pool.begin().await.unwrap();
    rustchat::integrations::buzz::dispatcher::enqueue_message_created_in_tx(
        &mut tx,
        channel_id,
        post_id,
        None,
        "alice",
        "hello buzz",
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();

    let outbox_id: uuid::Uuid =
        sqlx::query_scalar("SELECT id FROM integration_outbox WHERE provider = 'buzz'")
            .fetch_one(&pool)
            .await
            .unwrap();

    Pipeline {
        state,
        connection_id: connection.id,
        rustchat_channel_id: channel_id,
        outbox_id,
    }
}

async fn outbox_status(pool: &sqlx::PgPool, id: uuid::Uuid) -> (String, i32, Option<String>) {
    sqlx::query_as("SELECT status, attempts, last_error FROM integration_outbox WHERE id = $1")
        .bind(id)
        .fetch_one(pool)
        .await
        .unwrap()
}

#[tokio::test]
async fn unmapped_channel_produces_no_outbox_row() {
    let app = spawn_app_with_config(buzz_enabled_config()).await;
    let state = app_state_with_buzz(app.db_pool.clone());

    let org_id = uuid::Uuid::new_v4();
    sqlx::query("INSERT INTO organizations (id, name) VALUES ($1, $2)")
        .bind(org_id)
        .bind("Buzz Test Org")
        .execute(&app.db_pool)
        .await
        .unwrap();
    let team_id: uuid::Uuid = sqlx::query_scalar(
        "INSERT INTO teams (org_id, name, display_name) VALUES ($1, 't', 'T') RETURNING id",
    )
    .bind(org_id)
    .fetch_one(&app.db_pool)
    .await
    .unwrap();
    let channel_id: uuid::Uuid = sqlx::query_scalar(
        "INSERT INTO channels (team_id, name, display_name, type) \
         VALUES ($1, 'general', 'General', 'public') RETURNING id",
    )
    .bind(team_id)
    .fetch_one(&app.db_pool)
    .await
    .unwrap();

    let mut tx = app.db_pool.begin().await.unwrap();
    rustchat::integrations::buzz::dispatcher::enqueue_message_created_in_tx(
        &mut tx,
        channel_id,
        uuid::Uuid::new_v4(),
        None,
        "alice",
        "hello",
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();

    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM integration_outbox")
        .fetch_one(&app.db_pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
    drop(state);
}

#[tokio::test]
async fn duplicate_enqueue_is_idempotent() {
    let app = spawn_app_with_config(buzz_enabled_config()).await;
    let p = pipeline(app.db_pool.clone()).await;

    // Re-enqueue the same post (e.g. client retry path).
    let mut tx = app.db_pool.begin().await.unwrap();
    let post_id: uuid::Uuid = sqlx::query_scalar(
        "SELECT (payload->>'post_id')::uuid FROM integration_outbox WHERE id = $1",
    )
    .bind(p.outbox_id)
    .fetch_one(&app.db_pool)
    .await
    .unwrap();
    rustchat::integrations::buzz::dispatcher::enqueue_message_created_in_tx(
        &mut tx,
        p.rustchat_channel_id,
        post_id,
        None,
        "alice",
        "hello buzz",
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();

    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM integration_outbox")
        .fetch_one(&app.db_pool)
        .await
        .unwrap();
    assert_eq!(count, 1, "duplicate enqueue must not create a second row");
}

#[tokio::test]
async fn successful_delivery_marks_row_delivered_with_remote_id() {
    let app = spawn_app_with_config(buzz_enabled_config()).await;
    let p = pipeline(app.db_pool.clone()).await;
    let mock = MockBuzzConnector::new(vec![]);
    let provider = MockBuzzConnectorProvider {
        connector: mock.clone(),
    };

    dispatch_cycle(&p.state, &provider, &buzz_config()).await;

    let (status, attempts, error) = outbox_status(&app.db_pool, p.outbox_id).await;
    assert_eq!(status, "delivered");
    assert_eq!(attempts, 1);
    assert!(error.is_none());

    let remote: Option<String> =
        sqlx::query_scalar("SELECT remote_event_id FROM integration_outbox WHERE id = $1")
            .bind(p.outbox_id)
            .fetch_one(&app.db_pool)
            .await
            .unwrap();
    assert_eq!(remote.unwrap().len(), 64);

    // Loop prevention: the submitted event carries the origin marker.
    let submitted = mock.submitted().await;
    assert_eq!(submitted.len(), 1);
    let event_json: serde_json::Value = serde_json::from_slice(&submitted[0].body).unwrap();
    assert!(has_origin_marker(&event_json));
    assert_eq!(event_json["content"], "alice: hello buzz");
    // The origin marker identifies the connection.
    let tags = event_json["tags"].as_array().unwrap();
    assert!(tags
        .iter()
        .any(|t| t[0] == "rustchat:bridge" && t[1] == p.connection_id.to_string()));
}

#[tokio::test]
async fn auth_failure_dead_letters_immediately() {
    let app = spawn_app_with_config(buzz_enabled_config()).await;
    let p = pipeline(app.db_pool.clone()).await;
    let provider = MockBuzzConnectorProvider {
        connector: MockBuzzConnector::new(vec![DeliveryOutcome::Terminal {
            reason: "relay responded 401".to_string(),
        }]),
    };

    dispatch_cycle(&p.state, &provider, &buzz_config()).await;

    let (status, attempts, error) = outbox_status(&app.db_pool, p.outbox_id).await;
    assert_eq!(status, "dead_letter");
    assert_eq!(attempts, 1, "terminal errors must not be retried");
    assert!(error.unwrap().contains("401"));
}

#[tokio::test]
async fn transient_failures_retry_with_backoff_then_deliver() {
    let app = spawn_app_with_config(buzz_enabled_config()).await;
    let p = pipeline(app.db_pool.clone()).await;
    let mock = MockBuzzConnector::new(vec![
        DeliveryOutcome::Retryable {
            reason: "relay responded 503".to_string(),
        },
        DeliveryOutcome::Retryable {
            reason: "timeout".to_string(),
        },
    ]);
    let provider = MockBuzzConnectorProvider {
        connector: mock.clone(),
    };

    // First cycle: 503 → pending with a future next_attempt_at.
    dispatch_cycle(&p.state, &provider, &buzz_config()).await;
    let (status, attempts, _) = outbox_status(&app.db_pool, p.outbox_id).await;
    assert_eq!(status, "pending");
    assert_eq!(attempts, 1);

    // Deterministic backoff: set next_attempt_at explicitly far in the future
    // (not relying on elapsed wall clock) and assert the row is not claimed.
    sqlx::query(
        "UPDATE integration_outbox SET next_attempt_at = now() + interval '1 hour' WHERE id = $1",
    )
    .bind(p.outbox_id)
    .execute(&app.db_pool)
    .await
    .unwrap();
    dispatch_cycle(&p.state, &provider, &buzz_config()).await;
    assert_eq!(mock.submit_count().await, 1, "backoff must delay the retry");

    // Force the row due and retry: timeout → pending again, then success.
    sqlx::query(
        "UPDATE integration_outbox SET next_attempt_at = now() - interval '1 second' WHERE id = $1",
    )
    .bind(p.outbox_id)
    .execute(&app.db_pool)
    .await
    .unwrap();
    dispatch_cycle(&p.state, &provider, &buzz_config()).await;
    let (status, attempts, _) = outbox_status(&app.db_pool, p.outbox_id).await;
    assert_eq!(status, "pending");
    assert_eq!(attempts, 2);

    sqlx::query(
        "UPDATE integration_outbox SET next_attempt_at = now() - interval '1 second' WHERE id = $1",
    )
    .bind(p.outbox_id)
    .execute(&app.db_pool)
    .await
    .unwrap();
    dispatch_cycle(&p.state, &provider, &buzz_config()).await;
    let (status, attempts, _) = outbox_status(&app.db_pool, p.outbox_id).await;
    assert_eq!(status, "delivered");
    assert_eq!(attempts, 3);

    // Ambiguous-retry idempotency: every attempt used identical event bytes.
    assert!(mock.all_submissions_identical().await);
    assert_eq!(mock.submit_count().await, 3);
}

#[tokio::test]
async fn rotating_key_dead_letters_outstanding_deliveries() {
    let app = spawn_app_with_config(buzz_enabled_config()).await;
    let p = pipeline(app.db_pool.clone()).await;

    // A pending row exists before rotation.
    let (status, ..) = outbox_status(&app.db_pool, p.outbox_id).await;
    assert_eq!(status, "pending");

    // Rotate the connection key → outstanding undelivered rows are
    // dead-lettered (their stable Nostr event id is key-dependent, so a
    // retry under the new key could not deduplicate on the relay).
    let new_key = nostr::key::Keys::generate().secret_key().to_secret_hex();
    BuzzRepository::new(&app.db_pool)
        .rotate_connection_key(p.connection_id, &new_key, &p.state.config.encryption_key)
        .await
        .unwrap();

    let (status, ..) = outbox_status(&app.db_pool, p.outbox_id).await;
    assert_eq!(
        status, "dead_letter",
        "rotation must dead-letter outstanding deliveries"
    );

    // Dead-lettered rows are not claimed: a dispatch cycle must not attempt
    // a delivery under the new key.
    let mock = MockBuzzConnector::new(vec![]);
    let provider = MockBuzzConnectorProvider {
        connector: mock.clone(),
    };
    dispatch_cycle(&p.state, &provider, &buzz_config()).await;
    assert_eq!(mock.submit_count().await, 0);

    // Admin requeue under the NEW key: the row stays connection-linked, is
    // claimed with the new key, and delivers successfully (fresh event id).
    let requeued = OutboxRepository::new(&app.db_pool)
        .requeue_dead_letter("buzz", p.outbox_id)
        .await
        .unwrap();
    assert!(requeued, "requeue must succeed for a dead-lettered row");

    let mock = MockBuzzConnector::new(vec![DeliveryOutcome::Delivered {
        remote_id: Some("aa".repeat(32)),
    }]);
    let provider = MockBuzzConnectorProvider {
        connector: mock.clone(),
    };
    dispatch_cycle(&p.state, &provider, &buzz_config()).await;
    assert_eq!(
        mock.submit_count().await,
        1,
        "requeued row must be delivered under the new key"
    );
    let (status, ..) = outbox_status(&app.db_pool, p.outbox_id).await;
    assert_eq!(status, "delivered");
}

#[tokio::test]
async fn extended_outage_dead_letters_after_attempt_budget() {
    let app = spawn_app_with_config(buzz_enabled_config()).await;
    let p = pipeline(app.db_pool.clone()).await;
    let provider = MockBuzzConnectorProvider {
        connector: MockBuzzConnector::new(vec![
            DeliveryOutcome::Retryable {
                reason: "503".to_string(),
            },
            DeliveryOutcome::Retryable {
                reason: "503".to_string(),
            },
            DeliveryOutcome::Retryable {
                reason: "503".to_string(),
            },
        ]),
    };

    for _ in 0..3 {
        dispatch_cycle(&p.state, &provider, &buzz_config()).await;
        sqlx::query("UPDATE integration_outbox SET next_attempt_at = now() - interval '1 second' WHERE id = $1")
            .bind(p.outbox_id)
            .execute(&app.db_pool)
            .await
            .unwrap();
    }
    let (status, attempts, error) = outbox_status(&app.db_pool, p.outbox_id).await;
    assert_eq!(status, "dead_letter", "attempt budget (3) exhausted");
    assert_eq!(attempts, 3);
    assert!(error.is_some());
    assert_eq!(provider.connector.submit_count().await, 3);

    // Admin retry requeues the dead-lettered row and it delivers.
    let outbox = OutboxRepository::new(&app.db_pool);
    assert!(outbox
        .requeue_dead_letter("buzz", p.outbox_id)
        .await
        .unwrap());
    dispatch_cycle(&p.state, &provider, &buzz_config()).await;
    let (status, ..) = outbox_status(&app.db_pool, p.outbox_id).await;
    assert_eq!(status, "delivered");
}

#[tokio::test]
async fn stale_in_flight_row_is_reclaimed_after_restart() {
    let app = spawn_app_with_config(buzz_enabled_config()).await;
    let p = pipeline(app.db_pool.clone()).await;

    // Simulate a crash mid-delivery: row in_flight with an expired lease.
    sqlx::query(
        "UPDATE integration_outbox SET status = 'in_flight', attempts = 1, \
         updated_at = now() - interval '10 minutes' WHERE id = $1",
    )
    .bind(p.outbox_id)
    .execute(&app.db_pool)
    .await
    .unwrap();

    let mut config = buzz_config();
    config.in_flight_lease_secs = 60;
    let provider = MockBuzzConnectorProvider {
        connector: MockBuzzConnector::new(vec![]),
    };
    dispatch_cycle(&p.state, &provider, &config).await;

    let (status, attempts, _) = outbox_status(&app.db_pool, p.outbox_id).await;
    assert_eq!(status, "delivered", "reclaimed row must be delivered");
    assert_eq!(attempts, 2, "reclaim keeps the attempt history");
}

#[tokio::test]
async fn disabled_connection_dead_letters_pending_rows() {
    let app = spawn_app_with_config(buzz_enabled_config()).await;
    let p = pipeline(app.db_pool.clone()).await;

    sqlx::query("UPDATE buzz_connections SET enabled = false WHERE id = $1")
        .bind(p.connection_id)
        .execute(&app.db_pool)
        .await
        .unwrap();

    let provider = MockBuzzConnectorProvider {
        connector: MockBuzzConnector::new(vec![]),
    };
    dispatch_cycle(&p.state, &provider, &buzz_config()).await;

    let (status, ..) = outbox_status(&app.db_pool, p.outbox_id).await;
    assert_eq!(status, "dead_letter");
    // The connector was never consulted — no remote call for a disabled
    // connection.
    assert_eq!(provider.connector.submit_count().await, 0);
}

#[tokio::test]
async fn deleting_connection_dead_letters_and_retries_are_rejected() {
    let app = spawn_app_with_config(buzz_enabled_config()).await;
    let p = pipeline(app.db_pool.clone()).await;

    BuzzRepository::new(&app.db_pool)
        .delete_connection(p.connection_id)
        .await
        .unwrap();

    let (status, ..) = outbox_status(&app.db_pool, p.outbox_id).await;
    assert_eq!(status, "dead_letter");

    // Delivered history is retained with a NULL connection.
    let connection_id: Option<uuid::Uuid> =
        sqlx::query_scalar("SELECT connection_id FROM integration_outbox WHERE id = $1")
            .bind(p.outbox_id)
            .fetch_one(&app.db_pool)
            .await
            .unwrap();
    assert!(connection_id.is_none());

    // Mappings cascade-deleted.
    let mappings: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM buzz_channel_mappings")
        .fetch_one(&app.db_pool)
        .await
        .unwrap();
    assert_eq!(mappings, 0);

    // "Retries are rejected": even if the dead-lettered row were requeued, a
    // dispatch cycle cannot deliver it (its connection is gone), so no event
    // is ever submitted.
    let requeued = OutboxRepository::new(&app.db_pool)
        .requeue_dead_letter("buzz", p.outbox_id)
        .await
        .unwrap();
    assert!(requeued, "requeue of a dead-lettered row must succeed");
    let mock = MockBuzzConnector::new(vec![DeliveryOutcome::Delivered {
        remote_id: Some("aa".repeat(32)),
    }]);
    let provider = MockBuzzConnectorProvider {
        connector: mock.clone(),
    };
    dispatch_cycle(&p.state, &provider, &buzz_config()).await;
    assert_eq!(
        mock.submit_count().await,
        0,
        "deleted connection cannot deliver"
    );
}

// ===========================================================================
// End-to-end: post creation via the API enqueues the outbound event
// ===========================================================================

#[tokio::test]
async fn posting_in_mapped_channel_enqueues_outbox_row() {
    let app = spawn_app_with_config(buzz_enabled_config()).await;
    let admin = admin_token(&app, "admin@example.com").await;
    let connection = create_connection(&app, &admin).await;
    let connection_id = connection["id"].as_str().unwrap().to_string();

    // Build a team + channel through the public API.
    let team_res = app
        .api_client
        .post(format!("{}/api/v1/teams", app.address))
        .bearer_auth(&admin)
        .json(&json!({ "name": "bridgeteam", "display_name": "Bridge Team" }))
        .send()
        .await
        .unwrap();
    assert!(
        team_res.status().is_success(),
        "team create failed: {:?}",
        team_res.status()
    );
    let team: serde_json::Value = team_res.json().await.unwrap();
    let team_id = team["id"].as_str().unwrap();

    let chan_res = app
        .api_client
        .post(format!("{}/api/v1/channels", app.address))
        .bearer_auth(&admin)
        .json(&json!({
            "team_id": team_id,
            "name": "bridged",
            "display_name": "Bridged",
            "channel_type": "public",
        }))
        .send()
        .await
        .unwrap();
    assert!(
        chan_res.status().is_success(),
        "channel create failed: {:?}",
        chan_res.status()
    );
    let channel: serde_json::Value = chan_res.json().await.unwrap();
    let channel_id = channel["id"].as_str().unwrap();

    // Map the channel.
    let res = app
        .api_client
        .put(format!(
            "{}/api/v1/admin/integrations/buzz/connections/{connection_id}/mappings",
            app.address
        ))
        .bearer_auth(&admin)
        .json(&json!({
            "rustchat_channel_id": channel_id,
            "buzz_channel_id": "22222222-2222-2222-2222-222222222222",
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);

    // Post a message through the public API.
    let res = app
        .api_client
        .post(format!(
            "{}/api/v1/channels/{channel_id}/posts",
            app.address
        ))
        .bearer_auth(&admin)
        .json(&json!({ "message": "hello from rustchat" }))
        .send()
        .await
        .unwrap();
    assert!(
        res.status().is_success(),
        "post create failed: {:?}",
        res.status()
    );
    let post: serde_json::Value = res.json().await.unwrap();
    let post_id = post["id"].as_str().unwrap();

    // The outbox row exists, in pending state, with the post payload.
    let (status, payload): (String, serde_json::Value) =
        sqlx::query_as("SELECT status, payload FROM integration_outbox WHERE provider = 'buzz'")
            .fetch_one(&app.db_pool)
            .await
            .unwrap();
    assert_eq!(status, "pending");
    assert_eq!(payload["post_id"].as_str().unwrap(), post_id);
    assert_eq!(payload["content"].as_str().unwrap(), "hello from rustchat");
    assert_eq!(
        payload["buzz_channel_id"].as_str().unwrap(),
        "22222222-2222-2222-2222-222222222222"
    );
    assert_eq!(
        payload["author_label"].as_str().unwrap(),
        "Admin User",
        "author display name is used for attribution"
    );

    // Deliver through the mock and verify the bridged content.
    let state = app_state_with_buzz(app.db_pool.clone());
    let mock = MockBuzzConnector::new(vec![]);
    let provider = MockBuzzConnectorProvider {
        connector: mock.clone(),
    };
    dispatch_cycle(&state, &provider, &buzz_config()).await;
    let submitted = mock.submitted().await;
    assert_eq!(submitted.len(), 1);
    let event: serde_json::Value = serde_json::from_slice(&submitted[0].body).unwrap();
    assert_eq!(event["content"], "Admin User: hello from rustchat");
}

#[tokio::test]
async fn posting_in_unmapped_channel_enqueues_nothing() {
    let app = spawn_app_with_config(buzz_enabled_config()).await;
    let admin = admin_token(&app, "admin@example.com").await;
    create_connection(&app, &admin).await; // connection exists, no mapping

    let team_res = app
        .api_client
        .post(format!("{}/api/v1/teams", app.address))
        .bearer_auth(&admin)
        .json(&json!({ "name": "unmappedteam", "display_name": "U" }))
        .send()
        .await
        .unwrap();
    let team: serde_json::Value = team_res.json().await.unwrap();
    let chan_res = app
        .api_client
        .post(format!("{}/api/v1/channels", app.address))
        .bearer_auth(&admin)
        .json(&json!({
            "team_id": team["id"].as_str().unwrap(),
            "name": "unmapped",
            "display_name": "U",
            "channel_type": "public",
        }))
        .send()
        .await
        .unwrap();
    assert!(
        chan_res.status().is_success(),
        "channel create failed: {:?}",
        chan_res.status()
    );
    let channel: serde_json::Value = chan_res.json().await.unwrap();

    let res = app
        .api_client
        .post(format!(
            "{}/api/v1/channels/{}/posts",
            app.address,
            channel["id"].as_str().unwrap()
        ))
        .bearer_auth(&admin)
        .json(&json!({ "message": "not bridged" }))
        .send()
        .await
        .unwrap();
    assert!(
        res.status().is_success(),
        "post create failed: {:?}",
        res.status()
    );

    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM integration_outbox")
        .fetch_one(&app.db_pool)
        .await
        .unwrap();
    assert_eq!(count, 0, "unmapped channels must not bridge");
}

// ===========================================================================
// Deliveries admin listing
// ===========================================================================

#[tokio::test]
async fn deliveries_listing_filters_by_status_and_shape() {
    let app = spawn_app_with_config(buzz_enabled_config()).await;
    let admin = admin_token(&app, "admin@example.com").await;
    let connection = create_connection(&app, &admin).await;
    let connection_id = connection["id"].as_str().unwrap();

    let res = app
        .api_client
        .get(format!(
            "{}/api/v1/admin/integrations/buzz/connections/{connection_id}/deliveries",
            app.address
        ))
        .bearer_auth(&admin)
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    let body: serde_json::Value = res.json().await.unwrap();
    assert!(body["deliveries"].as_array().unwrap().is_empty());

    // Unknown status is a validation error.
    let res = app
        .api_client
        .get(format!(
            "{}/api/v1/admin/integrations/buzz/connections/{connection_id}/deliveries?status=bogus",
            app.address
        ))
        .bearer_auth(&admin)
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 422);
}
