use std::sync::Arc;
use tokio::sync::Mutex;
use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
};
use tower::ServiceExt;

use monitor_agent::config::{Config, ServiceConfig};
use monitor_agent::db;
use monitor_agent::routes;
use monitor_agent::AppState;

fn test_app() -> Router {
    let config = Arc::new(Config {
        poll_interval_secs: 30,
        history_hours: 24,
        api_key: "integration-test-key".into(),
        services: vec![
            ServiceConfig {
                name: "test.service".into(),
                display_name: "Test Service".into(),
            },
        ],
    });

    let conn = db::init(":memory:").unwrap_or_else(|_| {
        // If :memory: doesn't work via init(), create manually
        rusqlite::Connection::open_in_memory().unwrap()
    });
    let db = Arc::new(Mutex::new(conn));

    let state = Arc::new(AppState {
        db,
        config,
    });

    routes::router(state)
}

fn auth_header() -> &'static str {
    "Bearer integration-test-key"
}

#[tokio::test]
async fn test_health_endpoint() {
    let app = test_app();
    let res = app
        .oneshot(Request::builder().uri("/health").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_metrics_endpoint_full() {
    let app = test_app();
    let res = app
        .oneshot(
            Request::builder()
                .uri("/metrics")
                .header("Authorization", auth_header())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_history_with_custom_hours() {
    let app = test_app();
    let res = app
        .oneshot(
            Request::builder()
                .uri("/metrics/history?hours=2")
                .header("Authorization", auth_header())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_unauthorized_access() {
    let app = test_app();

    let endpoints = vec![
        "/metrics",
        "/metrics/history",
        "/services/test.service/start",
        "/services/test.service/stop",
    ];

    for endpoint in endpoints {
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(endpoint)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(
            res.status(),
            StatusCode::UNAUTHORIZED,
            "Expected 401 for {}",
            endpoint
        );
    }
}

#[tokio::test]
async fn test_invalid_token_rejected() {
    let app = test_app();
    let res = app
        .oneshot(
            Request::builder()
                .uri("/metrics")
                .header("Authorization", "Bearer wrong-token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}
