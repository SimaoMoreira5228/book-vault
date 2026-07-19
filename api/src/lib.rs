use std::sync::Arc;

use axum::Router;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

mod config;
pub mod cover;
pub mod db;
mod error;

pub mod auth;
pub mod export;
pub mod ingest;
pub mod ir;
pub mod jobs;
pub mod koreader;
pub mod language;
pub mod metadata;
pub mod opds;
pub mod routes;
pub mod search;
pub mod shelves;
pub mod storage;

pub use config::Config;
pub use config::EmailConfig;
pub use error::AppError;

pub type SharedState = Arc<AppState>;

pub struct AppState {
	pub config: Config,
	pub db: sea_orm::DatabaseConnection,
	pub storage: Arc<dyn storage::StorageProvider>,
	pub metadata_service: metadata::service::MetadataService,
	pub rate_limiter: auth::rate_limit::RateLimiter,
	pub search_engine: search::engine::SearchEngine,
	pub dictionary_service: language::dictionary::DictionaryService,
}

pub fn build_router(state: SharedState) -> Router {
	use axum::http::{HeaderValue, Method};
	use tower_http::cors::AllowOrigin;

	let cors = if state.config.cors.allowed_origin == "*" {
		CorsLayer::very_permissive()
	} else {
		let origin: HeaderValue = state.config.cors.allowed_origin.clone().parse().unwrap();
		CorsLayer::new()
			.allow_origin(AllowOrigin::list([origin]))
			.allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE, Method::OPTIONS])
			.allow_headers(Any)
			.allow_credentials(true)
			.max_age(std::time::Duration::from_secs(86400))
	};

	Router::new()
		.nest("/api/v1", routes::build_routes())
		.nest("/api/v1", koreader::routes())
		.nest("/api/v1/admin", routes::kobo::admin_routes())
		.nest("/api/kobo", routes::kobo::routes())
		.nest("/opds", opds::routes())
		.layer(TraceLayer::new_for_http())
		.layer(cors)
		.with_state(state)
}

pub async fn init_tracing(config: &Config) {
	use tracing_subscriber::layer::SubscriberExt;
	use tracing_subscriber::util::SubscriberInitExt;

	let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(&config.logging.level));
	tracing_subscriber::registry()
		.with(filter)
		.with(tracing_subscriber::fmt::layer())
		.init();
}
