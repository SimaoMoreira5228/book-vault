use book_vault::{build_router, AppState, Config, SharedState};
use std::sync::Arc;
use std::net::SocketAddr;
use tracing::info;

#[tokio::main]
async fn main() {
    let config = Config::load();

    book_vault::init_tracing(&config).await;

    let db = book_vault::db::connect(&config.database.url)
        .await
        .expect("Failed to connect to database");

    book_vault::db::run_migrations(&db)
        .await
        .expect("Failed to run migrations");

    let storage = Arc::new(book_vault::storage::LocalFsProvider::new(
        std::path::PathBuf::from(&config.storage.base_path),
    ));

    let state: SharedState = Arc::new(AppState { config, db, storage });
    let app = build_router(state);

    let addr: SocketAddr = format!("[::]:{}", 8080)
        .parse()
        .expect("Invalid server address");

    info!("BookVault listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
