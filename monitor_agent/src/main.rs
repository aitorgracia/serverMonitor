use std::sync::Arc;
use axum::Router;
use tokio::sync::Mutex;
use tower_http::cors::CorsLayer;
use tracing_subscriber;

use monitor_agent::config;
use monitor_agent::db;
use monitor_agent::metrics;
use monitor_agent::routes;
use monitor_agent::AppState;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let config = Arc::new(config::load("config.toml").expect("No se pudo cargar config.toml"));

    let conn = db::init("monitor.db").expect("No se pudo inicializar la base de datos");
    let db   = Arc::new(Mutex::new(conn));

    let state = Arc::new(AppState {
        db:     db.clone(),
        config: config.clone(),
    });

    {
        let state   = state.clone();
        let interval = config.poll_interval_secs;
        tokio::spawn(async move {
            metrics::snapshot_loop(state, interval).await;
        });
    }

    let app = Router::new()
        .merge(routes::router(state.clone()))
        .layer(CorsLayer::permissive());

    let addr = "0.0.0.0:3000";
    tracing::info!("Agente escuchando en http://{}/metrics", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
