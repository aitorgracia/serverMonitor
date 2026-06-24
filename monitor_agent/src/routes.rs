use std::sync::Arc;
use axum::{
    Router,
    routing::{get, post},
    extract::{State, Query, Path},
    http::{StatusCode, HeaderMap},
    Json,
    middleware::{self, Next},
    response::Response,
};
use axum::http::Request;
use serde::Deserialize;
use chrono::Utc;

use crate::AppState;
use crate::db::{SnapshotRow, get_history};
use crate::metrics::collect;

// --- AUTH MIDDLEWARE ---

type ApiError = (StatusCode, Json<serde_json::Value>);

async fn auth_middleware(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    request: Request<axum::body::Body>,
    next: Next,
) -> Result<Response, ApiError> {
    let path = request.uri().path();
    if path == "/health" || path == "/test" {
        return Ok(next.run(request).await);
    }

    let expected = format!("Bearer {}", state.config.api_key);
    let provided  = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if provided != expected {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": "Unauthorized"})),
        ));
    }

    Ok(next.run(request).await)
}

// --- HANDLERS ---

async fn get_current(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let (cpu_total, ram_used_gb, ram_total_gb, services) = collect(&state.config);

    Json(serde_json::json!({
        "timestamp":    Utc::now().timestamp(),
        "cpu_total":    cpu_total,
        "ram_used_gb":  ram_used_gb,
        "ram_total_gb": ram_total_gb,
        "services":     services,
    }))
}

#[derive(Deserialize)]
struct HistoryQuery {
    hours: Option<u64>,
}

async fn get_history_handler(
    State(state): State<Arc<AppState>>,
    Query(params): Query<HistoryQuery>,
) -> Result<Json<Vec<SnapshotRow>>, ApiError> {
    let hours    = params.hours.unwrap_or(6).min(state.config.history_hours);
    let since_ts = Utc::now().timestamp() - (hours as i64 * 3600);

    let db = state.db.lock().await;
    match get_history(&db, since_ts) {
        Ok(rows) => Ok(Json(rows)),
        Err(e)   => {
            tracing::error!("Error leyendo historial: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("Error interno: {}", e)})),
            ))
        }
    }
}

async fn service_start(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    match crate::metrics::start_service(&state.config, &name) {
        Ok(msg) => Ok(Json(serde_json::json!({"status": "ok", "message": msg}))),
        Err(e) => {
            tracing::error!("Error iniciando servicio: {}", e);
            Err((
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": e})),
            ))
        }
    }
}

async fn service_stop(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    match crate::metrics::stop_service(&state.config, &name) {
        Ok(msg) => Ok(Json(serde_json::json!({"status": "ok", "message": msg}))),
        Err(e) => {
            tracing::error!("Error deteniendo servicio: {}", e);
            Err((
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": e})),
            ))
        }
    }
}

async fn health() -> &'static str {
    "ok"
}

async fn test_post() -> &'static str {
    "post ok"
}

async fn foo_param() -> &'static str {
    "foo/{x} ok"
}

async fn bar_start() -> &'static str {
    "bar/x/start ok"
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tokio::sync::Mutex;
    use axum::body::Body;
    use http::Request;
    use tower::ServiceExt;

    fn test_state() -> Arc<AppState> {
        let config = crate::config::Config {
            poll_interval_secs: 30,
            history_hours: 24,
            api_key: "test-api-key".into(),
            services: vec![
                crate::config::ServiceConfig {
                    name: "ts.service".into(),
                    display_name: "TeamSpeak".into(),
                },
            ],
        };
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        crate::db::init(":memory:").ok();
        // init() opens a file path, so manually create tables
        conn.execute_batch("
            CREATE TABLE IF NOT EXISTS snapshots (
                id           INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp    INTEGER NOT NULL,
                cpu_total    REAL    NOT NULL,
                ram_used_gb  REAL    NOT NULL,
                ram_total_gb REAL    NOT NULL
            );
            CREATE TABLE IF NOT EXISTS service_snapshots (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                snapshot_id INTEGER NOT NULL REFERENCES snapshots(id) ON DELETE CASCADE,
                name        TEXT    NOT NULL,
                display_name TEXT   NOT NULL,
                running     INTEGER NOT NULL,
                cpu_usage   REAL    NOT NULL,
                memory_mb   INTEGER NOT NULL
            );
        ").unwrap();

        Arc::new(AppState {
            db: Arc::new(Mutex::new(conn)),
            config: Arc::new(config),
        })
    }

    #[tokio::test]
    async fn test_health_returns_ok() {
        let state = test_state();
        let app = router(state);
        let response = app
            .oneshot(Request::builder().uri("/health").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_metrics_without_auth_returns_401() {
        let state = test_state();
        let app = router(state);
        let response = app
            .oneshot(Request::builder().uri("/metrics").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_metrics_with_wrong_auth_returns_401() {
        let state = test_state();
        let app = router(state);
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/metrics")
                    .header("Authorization", "Bearer wrong-key")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_metrics_with_valid_auth_returns_200() {
        let state = test_state();
        let app = router(state);
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/metrics")
                    .header("Authorization", "Bearer test-api-key")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_history_without_auth_returns_401() {
        let state = test_state();
        let app = router(state);
        let response = app
            .oneshot(Request::builder().uri("/metrics/history").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_service_start_without_auth_returns_401() {
        let state = test_state();
        let app = router(state);
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/services/ts.service/start")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_history_with_valid_auth_returns_200() {
        let state = test_state();
        let app = router(state);
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/metrics/history")
                    .header("Authorization", "Bearer test-api-key")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }
}

// --- ROUTER ---

pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/test", post(test_post))
        .route("/metrics", get(get_current))
        .route("/metrics/history", get(get_history_handler))
        .route("/foo/{x}", post(foo_param))
        .route("/bar/y/start", post(bar_start))
        .route("/services/{name}/start", post(service_start))
        .route("/services/{name}/stop",  post(service_stop))
        .layer(middleware::from_fn_with_state(state.clone(), auth_middleware))
        .with_state(state)
}
