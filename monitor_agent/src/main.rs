use std::sync::Arc;
use std::net::SocketAddr;
use axum::Router;
use tokio::sync::Mutex;
use tower_http::cors::CorsLayer;
use tracing_subscriber;

use monitor_agent::config;
use monitor_agent::db;
use monitor_agent::metrics;
use monitor_agent::routes;
use monitor_agent::AppState;

fn bind_reuse(addr: SocketAddr) -> tokio::net::TcpListener {
    let socket = socket2::Socket::new(
        socket2::Domain::IPV4,
        socket2::Type::STREAM,
        Some(socket2::Protocol::TCP),
    ).expect("No se pudo crear el socket");
    socket.set_reuse_address(true).expect("No se pudo setear SO_REUSEADDR");
    socket.bind(&addr.into()).expect(&format!("No se pudo bindear al puerto {}", addr.port()));
    socket.listen(1024).expect("No se pudo poner el socket en modo listen");
    tokio::net::TcpListener::from_std(socket.into()).expect("No se pudo convertir a TcpListener tokio")
}

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

    let addr: SocketAddr = "0.0.0.0:3000".parse().expect("Dirección inválida");
    tracing::info!("Agente escuchando en http://{}/metrics", addr);

    let listener = bind_reuse(addr);
    axum::serve(listener, app).await.unwrap();
}
