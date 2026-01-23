use crate::common::spawn_app;

mod common;

#[tokio::test]
async fn health_check_works() {
    let app = spawn_app().await;

    let response = app
        .api_client
        .get(&format!("{}/api/v1/health/live", &app.address))
        .send()
        .await
        .expect("Failed to execute request.");

    assert!(response.status().is_success());
}
