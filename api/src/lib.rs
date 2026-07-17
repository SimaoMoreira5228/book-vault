use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

mod config;
mod error;
pub mod db;

pub mod auth;
pub mod ir;
pub mod storage;
pub mod ingest;
pub mod metadata;
pub mod export;
pub mod routes;
pub mod jobs;
pub mod search;
pub mod shelves;

use std::sync::Arc;
use axum::Router;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

pub use config::Config;
pub use error::AppError;

pub type SharedState = Arc<AppState>;

pub struct AppState {
    pub config: Config,
    pub db: sea_orm::DatabaseConnection,
    pub storage: Arc<dyn storage::StorageProvider>,
    pub metadata_service: metadata::service::MetadataService,
}

pub fn build_router(state: SharedState) -> Router {
    Router::new()
        .nest("/api/v1", routes::build_routes())
        .with_state(state)
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
}

pub async fn init_tracing(config: &Config) {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(&config.logging.level));

    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer())
        .init();
}
