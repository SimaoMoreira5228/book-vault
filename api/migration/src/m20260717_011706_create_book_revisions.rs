use sea_orm_migration::prelude::*;
use sea_orm_migration::schema::*;

use crate::m20260717_011703_create_books::Books;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
	async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		manager
			.create_table(
				Table::create()
					.table(BookRevisions::Table)
					.if_not_exists()
					.col(uuid(BookRevisions::Id).primary_key())
					.col(uuid(BookRevisions::BookId).not_null())
					.col(uuid(BookRevisions::SectionId).not_null())
					.col(json(BookRevisions::Snapshot).not_null())
					.col(integer(BookRevisions::Version).not_null())
					.col(timestamp_with_time_zone(BookRevisions::CreatedAt).not_null())
					.foreign_key(
						ForeignKey::create()
							.name("fk_revisions_book")
							.from(BookRevisions::Table, BookRevisions::BookId)
							.to(Books::Table, Books::Id)
							.on_delete(ForeignKeyAction::Cascade),
					)
					.to_owned(),
			)
			.await
	}

	async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		manager.drop_table(Table::drop().table(BookRevisions::Table).to_owned()).await
	}
}

#[derive(DeriveIden)]
pub enum BookRevisions {
	Table,
	Id,
	BookId,
	SectionId,
	Snapshot,
	Version,
	CreatedAt,
}
