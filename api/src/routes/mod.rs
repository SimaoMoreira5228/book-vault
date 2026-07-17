pub mod admin;
pub mod books;
pub mod events;
pub mod export_routes;
pub mod read;
pub mod search;
pub mod shelves;

use axum::{routing::get, Router};

pub fn build_routes() -> Router<crate::SharedState> {
    Router::new()
        .nest("/auth", super::auth::routes::routes())
        .nest("/books", books::routes())
        .nest("/books", read::routes())
        .nest("/books", export_routes::routes())
        .nest("/shelves", shelves::routes())
        .nest("/search", search::routes())
        .nest("/events", events::routes())
        .nest("/admin", admin::routes())
        .route("/health", get(health_check))
}

async fn health_check() -> &'static str {
    "OK"
}
