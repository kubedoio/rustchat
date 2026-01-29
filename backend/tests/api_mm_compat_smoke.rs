use crate::common::spawn_app;

mod common;

#[tokio::test]
async fn mm_compat_smoke_test() {
    let app = spawn_app().await;

    // 1. Check Header
    let ping_res = app
        .api_client
        .get(format!("{}/api/v4/system/ping", &app.address))
        .send()
        .await
        .expect("Failed to execute request.");

    assert!(ping_res.headers().contains_key("x-mm-compat"));
    assert_eq!(ping_res.headers().get("x-mm-compat").unwrap(), "1");

    // 2. Check Ping Content
    let status = ping_res.json::<serde_json::Value>().await.unwrap();
    assert_eq!(status["version"], "9.5.0");
    assert_eq!(status["AndroidLatestVersion"], "");

    // 3. Check License
    let lic_res = app
        .api_client
        .get(format!("{}/api/v4/license/client?format=old", &app.address))
        .send()
        .await
        .unwrap();
    let lic = lic_res.json::<serde_json::Value>().await.unwrap();
    assert_eq!(lic["IsLicensed"], "false");

    // 4. Check Config
    let conf_res = app
        .api_client
        .get(format!("{}/api/v4/config/client?format=old", &app.address))
        .send()
        .await
        .unwrap();
    let conf = conf_res.json::<serde_json::Value>().await.unwrap();
    assert_eq!(conf["Version"], "9.5.0");
    assert!(conf.get("DiagnosticId").is_some());
}
