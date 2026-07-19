pub mod admin;
pub mod assets;
pub mod authors;
pub mod bookmarks;
pub mod books;
pub mod email;
pub mod events;
pub mod export_routes;
pub mod jobs;
pub mod kobo;
pub mod metadata_routes;
pub mod progress;
pub mod read;
pub mod search;
pub mod series;
pub mod shelves;
pub mod studio;

use axum::Router;
use axum::routing::get;

pub fn build_routes() -> Router<crate::SharedState> {
	Router::new()
		.nest("/auth", super::auth::routes::routes())
		.nest("/books", books::routes())
		.nest("/books", read::routes())
		.nest("/books", export_routes::routes())
		.nest("/shelves", shelves::routes())
		.nest("/books", assets::routes())
		.nest("/search", search::routes())
		.nest("/books", progress::book_routes())
		.nest("/books", metadata_routes::routes())
		.nest("/annotations", progress::annotation_routes())
		.nest("/books", studio::routes())
		.nest("/books", email::book_routes())
		.nest("/email", email::status_routes())
		.nest("/books", super::language::routes())
		.nest("/vocabulary", super::language::vocab_routes())
		.nest("/revisions", studio::routes())
		.nest("/events", events::routes())
		.nest("/series", series::series_routes())
		.nest("/authors", authors::author_routes())
		.nest("/bookmarks", bookmarks::bookmark_routes())
		.nest("/jobs", jobs::routes())
		.nest("/admin", admin::routes())
		.route("/health", get(health_check))
}

async fn health_check() -> &'static str {
	"OK"
}
