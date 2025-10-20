use std::path::PathBuf;

use axum_test::TestServer;
use deepresearch_gui::config::{AppConfig, StorageBackend};
use deepresearch_gui::routes::build_router;
use deepresearch_gui::state::AppState;
use serde_json::json;
use tokio::time::{Duration, sleep, timeout};

fn base_config() -> AppConfig {
    AppConfig {
        listen_addr: "127.0.0.1:0".into(),
        max_concurrency: 2,
        default_enable_trace: true,
        assets_dir: PathBuf::from("crates/deepresearch-gui/web/dist"),
        gui_enabled: false,
        auth_token: None,
        storage: StorageBackend::InMemory,
        session_namespace: None,
        otel_endpoint: None,
    }
}

#[tokio::test]
async fn readiness_requires_gui_flag() {
    let disabled_config = base_config();
    let disabled_state = AppState::try_new(&disabled_config)
        .await
        .expect("state initialization failed");
    let disabled_router = build_router(disabled_state);
    let disabled_server = TestServer::new(disabled_router).unwrap();

    let response = disabled_server.get("/health/ready").await;
    assert_eq!(response.status_code(), 503);

    let mut enabled_config = base_config();
    enabled_config.gui_enabled = true;
    let enabled_state = AppState::try_new(&enabled_config)
        .await
        .expect("state initialization failed");
    let enabled_router = build_router(enabled_state);
    let enabled_server = TestServer::new(enabled_router).unwrap();

    let response = enabled_server.get("/health/ready").await;
    assert_eq!(response.status_code(), 200);
}

#[tokio::test]
async fn api_requires_bearer_token_when_configured() {
    let mut config = base_config();
    config.gui_enabled = true;
    config.auth_token = Some("secret".into());

    let state = AppState::try_new(&config)
        .await
        .expect("state initialization failed");
    let router = build_router(state);
    let server = TestServer::new(router).unwrap();

    // Missing token -> unauthorized
    let response = server.get("/api/sessions").await;
    assert_eq!(response.status_code(), 401);

    // Correct token -> ok (empty directory)
    let response = server
        .get("/api/sessions")
        .add_header("authorization", "Bearer secret")
        .await;
    assert_eq!(response.status_code(), 200);
    let body = response.json::<serde_json::Value>();
    assert!(body["sessions"].is_array());
}

#[tokio::test]
async fn session_stream_reports_completion() {
    let mut config = base_config();
    config.gui_enabled = true;

    let state = AppState::try_new(&config)
        .await
        .expect("state initialization failed");
    let shared_state = state.clone();
    let router = build_router(state);
    let server = TestServer::new(router).unwrap();

    let response = server
        .post("/api/sessions")
        .json(&json!({ "query": "How ready is the roadmap?" }))
        .await;
    assert_eq!(response.status_code(), 202);
    let body = response.json::<serde_json::Value>();
    let session_id = body["session_id"]
        .as_str()
        .expect("session id missing")
        .to_string();

    let status_path = format!("/api/sessions/{session_id}");
    let status = timeout(Duration::from_secs(5), async {
        loop {
            let response = server.get(&status_path).await;
            assert_eq!(response.status_code(), 200);
            let payload = response.json::<serde_json::Value>();
            if payload["state"] == "completed" {
                return payload;
            }
            sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("session did not complete in time");

    assert!(
        status["summary"]
            .as_str()
            .map(|s| !s.is_empty())
            .unwrap_or(false),
        "summary missing in status payload"
    );
    assert!(
        status["requires_manual"].is_boolean(),
        "status payload missing requires_manual flag"
    );

    let outcome = shared_state
        .session_service()
        .outcome(&session_id)
        .expect("session outcome missing");
    assert!(
        !outcome.summary.is_empty(),
        "outcome summary should not be empty"
    );

    let stream_path = format!("/api/sessions/{session_id}/stream");
    let stream_response = server.get(&stream_path).await;
    assert_eq!(stream_response.status_code(), 200);
    let body = stream_response.text();
    assert!(
        body.contains("event: completed"),
        "stream did not include completed event: {body}"
    );
    assert!(
        body.contains("\"kind\":\"completed\""),
        "stream payload missing completed kind: {body}"
    );
    assert!(
        body.contains("\"summary\""),
        "stream payload missing summary: {body}"
    );
    assert!(
        body.contains("\"requires_manual\":false"),
        "stream payload missing requires_manual indicator: {body}"
    );

    let trace_response = server
        .get(&format!("/api/sessions/{session_id}/trace"))
        .await;
    assert_eq!(trace_response.status_code(), 200);
    let trace_payload = trace_response.json::<serde_json::Value>();
    assert!(
        trace_payload["timeline"]
            .as_array()
            .map(|items| !items.is_empty())
            .unwrap_or(false),
        "timeline missing from trace payload"
    );
    assert!(
        trace_payload["task_metrics"]
            .as_array()
            .map(|items| !items.is_empty())
            .unwrap_or(false),
        "task metrics missing from trace payload"
    );
    assert!(trace_payload["artifacts"].is_object());
    assert!(trace_payload["requires_manual"].is_boolean());
}
