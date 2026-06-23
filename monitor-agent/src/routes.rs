use std::sync::Arc;
use axum::{
    Router,
    routing::get,
    extract::{State, Query},
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

async fn auth_middleware(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    request: Request<axum::body::Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let expected = format!("Bearer {}", state.config.api_key);
    let provided  = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if provided != expected {
        return Err(StatusCode::UNAUTHORIZED);
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
) -> Result<Json<Vec<SnapshotRow>>, StatusCode> {
    let hours    = params.hours.unwrap_or(6).min(state.config.history_hours);
    let since_ts = Utc::now().timestamp() - (hours as i64 * 3600);

    let db = state.db.lock().await;
    match get_history(&db, since_ts) {
        Ok(rows) => Ok(Json(rows)),
        Err(e)   => {
            tracing::error!("Error leyendo historial: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn health() -> &'static str {
    "ok"
}

// --- ROUTER ---

pub fn router(state: Arc<AppState>) -> Router {
    let protected = Router::new()
        .route("/metrics",         get(get_current))
        .route("/metrics/history", get(get_history_handler))
        .layer(middleware::from_fn_with_state(state.clone(), auth_middleware));

    Router::new()
        .route("/health", get(health))  // sin auth, para monitorización básica
        .merge(protected)
        .with_state(state)
}
