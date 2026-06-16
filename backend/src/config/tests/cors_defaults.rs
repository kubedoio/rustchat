use super::*;
use crate::api::build_cors_layer;
use axum::{body::Body, http::Method, http::Request, routing::get, Router};
use tower::ServiceExt;

const TEST_JWT_SECRET: &str = "7M@xQ2vL9!bN4#pR6$hT8%yU1^cD3&kJ5*zW0+fS"; // pragma: allowlist secret
const TEST_ENCRYPTION_KEY: &str = "3P!nV7@qL2#xR9$gT4%kY6^dM1&hC8*zB5+uF0wJ"; // pragma: allowlist secret

enum Override {
    Str(&'static str),
    Bool(bool),
}

fn load_config(overrides: &[(&'static str, Override)]) -> Config {
    let mut builder = config::Config::builder()
        .set_override("database_url", "postgres://localhost/rustchat")
        .unwrap()
        .set_override("jwt_secret", TEST_JWT_SECRET)
        .unwrap()
        .set_override("encryption_key", TEST_ENCRYPTION_KEY)
        .unwrap();

    for (key, value) in overrides {
        builder = match value {
            Override::Str(s) => builder.set_override(*key, *s).unwrap(),
            Override::Bool(b) => builder.set_override(*key, *b).unwrap(),
        };
    }

    builder.build().unwrap().try_deserialize().unwrap()
}

async fn preflight(config: &Config, origin: &str) -> axum::http::Response<Body> {
    let cors = build_cors_layer(config);
    let app = Router::new().route("/", get(|| async {})).layer(cors);
    let request = Request::builder()
        .method(Method::OPTIONS)
        .uri("/")
        .header("Origin", origin)
        .header("Access-Control-Request-Method", "GET")
        .body(Body::empty())
        .unwrap();

    app.oneshot(request).await.unwrap()
}

#[test]
fn default_environment_is_production_and_dev_cors_disabled() {
    let config = load_config(&[]);
    assert_eq!(config.environment, "production");
    assert!(!config.allow_dev_cors);
}

#[tokio::test]
async fn allow_dev_cors_true_allows_any_origin() {
    let config = load_config(&[
        ("environment", Override::Str("production")),
        ("allow_dev_cors", Override::Bool(true)),
    ]);

    let response = preflight(&config, "https://attacker.example").await;
    assert_eq!(
        response
            .headers()
            .get("access-control-allow-origin")
            .unwrap(),
        "*"
    );
}

#[tokio::test]
async fn allow_dev_cors_false_blocks_cross_origin() {
    let config = load_config(&[
        ("environment", Override::Str("development")),
        ("allow_dev_cors", Override::Bool(false)),
    ]);

    let response = preflight(&config, "https://attacker.example").await;
    assert!(response
        .headers()
        .get("access-control-allow-origin")
        .is_none());
}

#[tokio::test]
async fn configured_origins_take_precedence_over_dev_cors() {
    let config = load_config(&[
        ("environment", Override::Str("production")),
        ("allow_dev_cors", Override::Bool(false)),
        (
            "cors_allowed_origins",
            Override::Str("https://app.example.com, https://admin.example.com"),
        ),
    ]);

    let allowed = preflight(&config, "https://app.example.com").await;
    assert_eq!(
        allowed
            .headers()
            .get("access-control-allow-origin")
            .unwrap(),
        "https://app.example.com"
    );

    let disallowed = preflight(&config, "https://attacker.example").await;
    assert!(disallowed
        .headers()
        .get("access-control-allow-origin")
        .is_none());
}
