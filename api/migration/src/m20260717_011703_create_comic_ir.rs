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
					.table(ComicIr::Table)
					.if_not_exists()
					.col(uuid(ComicIr::Id).primary_key())
					.col(uuid(ComicIr::BookId).unique_key().not_null())
					.col(integer(ComicIr::PageCount).not_null())
					.col(json(ComicIr::PageManifest).not_null())
					.col(timestamp_with_time_zone(ComicIr::CreatedAt).not_null())
					.foreign_key(
						ForeignKey::create()
							.name("fk_comic_ir_book")
							.from(ComicIr::Table, ComicIr::BookId)
							.to(Books::Table, Books::Id)
							.on_delete(ForeignKeyAction::Cascade),
					)
					.to_owned(),
			)
			.await
	}

	async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		manager.drop_table(Table::drop().table(ComicIr::Table).to_owned()).await
	}
}

#[derive(DeriveIden)]
pub enum ComicIr {
	Table,
	Id,
	BookId,
	PageCount,
	PageManifest,
	CreatedAt,
}
