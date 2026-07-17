use std::path::PathBuf;
use std::sync::Arc;

use axum::Router;
use axum::response::IntoResponse;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

mod config;
pub mod cover;
pub mod db;
mod error;

pub mod auth;
pub mod export;
pub mod ingest;
pub mod ir;
pub mod jobs;
pub mod metadata;
pub mod routes;
pub mod search;
pub mod shelves;
pub mod storage;

pub use config::Config;
pub use error::AppError;

pub type SharedState = Arc<AppState>;

pub struct AppState {
	pub config: Config,
	pub db: sea_orm::DatabaseConnection,
	pub storage: Arc<dyn storage::StorageProvider>,
	pub metadata_service: metadata::service::MetadataService,
	pub rate_limiter: auth::rate_limit::RateLimiter,
	pub search_engine: search::engine::SearchEngine,
}

pub fn build_router(state: SharedState) -> Router {
	use axum::http::{HeaderValue, Method};
	use axum::routing::any;
	use tower_http::cors::AllowOrigin;

	let cors = if state.config.cors.allowed_origin == "*" {
		CorsLayer::permissive()
	} else {
		let origin: HeaderValue = state.config.cors.allowed_origin.clone().parse().unwrap();
		CorsLayer::new()
			.allow_origin(AllowOrigin::list([origin]))
			.allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE, Method::OPTIONS])
			.allow_headers(Any)
			.allow_credentials(true)
			.max_age(std::time::Duration::from_secs(86400))
	};

	let mut router = Router::new()
		.nest("/api/v1", routes::build_routes())
		.layer(TraceLayer::new_for_http())
		.layer(cors);

	if let Ok(web_dir_str) = std::env::var("BOOKVAULT_WEB_DIR") {
		let web_dir = PathBuf::from(&web_dir_str);
		if web_dir.exists() {
			tracing::info!("Serving web build from: {}", web_dir_str);
			let web_dir2 = web_dir.clone();
			router = router
				.nest_service("/", ServeDir::new(&web_dir).precompressed_gzip())
				.fallback(any(move || {
					let web_dir = web_dir2.clone();
					async move {
						let index_path = web_dir.join("index.html");
						match tokio::fs::read_to_string(&index_path).await {
							Ok(html) => (
								axum::http::StatusCode::OK,
								[(axum::http::header::CONTENT_TYPE, "text/html")],
								html,
							)
								.into_response(),
							Err(_) => (axum::http::StatusCode::NOT_FOUND, "Not found").into_response(),
						}
					}
				}));
		}
	}

	router.with_state(state)
}

pub async fn init_tracing(config: &Config) {
	let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(&config.logging.level));

	tracing_subscriber::registry()
		.with(filter)
		.with(tracing_subscriber::fmt::layer())
		.init();
}
