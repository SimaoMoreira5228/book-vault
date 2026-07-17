pub use sea_orm_migration::prelude::*;

mod m20250717_add_author_id_to_books;
mod m20250717_create_authors;
mod m20250717_create_bookmarks;
mod m20260717_011703_create_assets;
mod m20260717_011703_create_book_ir;
mod m20260717_011703_create_books;
mod m20260717_011703_create_comic_ir;
mod m20260717_011703_create_libraries;
mod m20260717_011703_create_sessions;
mod m20260717_011703_create_users;
mod m20260717_011706_create_annotations;
mod m20260717_011706_create_book_revisions;
mod m20260717_011706_create_job_queue;
mod m20260717_011706_create_reading_progress;
mod m20260717_011706_create_shelf_entries;
mod m20260717_011706_create_shelves;
mod m20260717_011707_add_keep_source;
mod m20260717_011708_create_book_metadata;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
	fn migrations() -> Vec<Box<dyn MigrationTrait>> {
		vec![
			Box::new(m20260717_011703_create_users::Migration),
			Box::new(m20260717_011703_create_sessions::Migration),
			Box::new(m20260717_011703_create_libraries::Migration),
			Box::new(m20260717_011703_create_books::Migration),
			Box::new(m20260717_011703_create_assets::Migration),
			Box::new(m20260717_011703_create_book_ir::Migration),
			Box::new(m20260717_011703_create_comic_ir::Migration),
			Box::new(m20260717_011706_create_shelves::Migration),
			Box::new(m20260717_011706_create_shelf_entries::Migration),
			Box::new(m20260717_011706_create_reading_progress::Migration),
			Box::new(m20260717_011706_create_annotations::Migration),
			Box::new(m20260717_011706_create_book_revisions::Migration),
			Box::new(m20260717_011706_create_job_queue::Migration),
			Box::new(m20260717_011707_add_keep_source::Migration),
			Box::new(m20260717_011708_create_book_metadata::Migration),
			Box::new(m20250717_create_bookmarks::Migration),
			Box::new(m20250717_create_authors::Migration),
			Box::new(m20250717_add_author_id_to_books::Migration),
		]
	}
}
