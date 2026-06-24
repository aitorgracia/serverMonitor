pub mod config;
pub mod db;
pub mod metrics;
pub mod routes;

use std::sync::Arc;
use tokio::sync::Mutex;

pub struct AppState {
    pub db:     Arc<Mutex<rusqlite::Connection>>,
    pub config: Arc<config::Config>,
}
