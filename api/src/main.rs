use std::net::SocketAddr;
use std::sync::Arc;

use book_vault::{AppState, Config, SharedState, build_router};
use tracing::info;

#[tokio::main]
async fn main() {
	let config = Config::load();
	book_vault::init_tracing(&config).await;

	let db = book_vault::db::connect(&config.database.url)
		.await
		.expect("Failed to connect to database");

	book_vault::db::run_migrations(&db).await.expect("Failed to run migrations");

	let storage: Arc<dyn book_vault::storage::StorageProvider> = match config.storage.provider.as_str() {
		"s3" => {
			info!("Using S3 storage provider");
			Arc::new(
				book_vault::storage::S3Provider::new(&config.storage.s3)
					.await
					.expect("Failed to initialize S3 provider"),
			)
		}
		_ => {
			info!("Using local filesystem storage provider");
			Arc::new(book_vault::storage::LocalFsProvider::new(std::path::PathBuf::from(
				&config.storage.base_path,
			)))
		}
	};

	let engine = book_vault::search::engine::SearchEngine::new();
	engine.rebuild(&db);

	let dictionary_provider: Option<Box<dyn book_vault::language::dictionary::DictionaryProvider>> =
		Some(Box::new(book_vault::language::dictionary::FreeDictionaryProvider));

	let translation_provider: Option<Box<dyn book_vault::language::dictionary::TranslationProvider>> = {
		let lt_url = &config.integrations.hosted_services.libretranslate_url;
		if lt_url.is_empty() {
			None
		} else {
			Some(Box::new(book_vault::language::dictionary::LibreTranslateProvider {
				api_url: lt_url.clone(),
			}))
		}
	};

	let state: SharedState = Arc::new(AppState {
		metadata_service: book_vault::metadata::service::MetadataService::new(&config),
		config: config.clone(),
		db,
		storage,
		rate_limiter: book_vault::auth::rate_limit::RateLimiter::new(5, 900),
		search_engine: engine,
		dictionary_provider,
		translation_provider,
	});

	let worker_state = state.clone();
	tokio::spawn(async move {
		let worker = book_vault::jobs::worker::JobWorker::new(worker_state);
		worker.run_forever().await;
	});

	let app = build_router(state);

	let addr: SocketAddr = format!("[::]:{}", config.server.port)
		.parse()
		.expect("Invalid server address");

	info!("BookVault listening on {}", addr);
	let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
	axum::serve(listener, app).await.unwrap();
}
