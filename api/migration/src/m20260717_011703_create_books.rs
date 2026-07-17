use sea_orm_migration::prelude::*;
use sea_orm_migration::schema::*;

use crate::m20260717_011703_create_libraries::Libraries;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
	async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		manager
			.create_table(
				Table::create()
					.table(Books::Table)
					.if_not_exists()
					.col(uuid(Books::Id).primary_key())
					.col(uuid(Books::LibraryId).not_null())
					.col(string(Books::Title).not_null())
					.col(string_null(Books::Author))
					.col(string_null(Books::Isbn))
					.col(string_null(Books::Language))
					.col(string_null(Books::Publisher))
					.col(string_null(Books::Series))
					.col(integer_null(Books::SeriesIndex))
					.col(integer_null(Books::PageCount))
					.col(string(Books::ReadStatus).default("unread"))
					.col(integer_null(Books::Rating))
					.col(string(Books::Format).not_null())
					.col(string_null(Books::SourceHash))
					.col(timestamp_with_time_zone(Books::CreatedAt).not_null())
					.col(timestamp_with_time_zone(Books::UpdatedAt).not_null())
					.foreign_key(
						ForeignKey::create()
							.name("fk_books_library")
							.from(Books::Table, Books::LibraryId)
							.to(Libraries::Table, Libraries::Id)
							.on_delete(ForeignKeyAction::Cascade),
					)
					.to_owned(),
			)
			.await
	}

	async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		manager.drop_table(Table::drop().table(Books::Table).to_owned()).await
	}
}

#[derive(DeriveIden)]
pub enum Books {
	Table,
	Id,
	LibraryId,
	Title,
	Author,
	Isbn,
	Language,
	Publisher,
	Series,
	SeriesIndex,
	PageCount,
	ReadStatus,
	Rating,
	Format,
	SourceHash,
	CreatedAt,
	UpdatedAt,
}
