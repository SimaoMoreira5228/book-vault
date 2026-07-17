use sea_orm_migration::prelude::*;
use sea_orm_migration::schema::*;

use crate::m20260717_011703_create_books::Books;
use crate::m20260717_011706_create_shelves::Shelves;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
	async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		manager
			.create_table(
				Table::create()
					.table(ShelfEntries::Table)
					.if_not_exists()
					.col(uuid(ShelfEntries::ShelfId).not_null())
					.col(uuid(ShelfEntries::BookId).not_null())
					.col(integer(ShelfEntries::Position).not_null())
					.col(timestamp_with_time_zone(ShelfEntries::AddedAt).not_null())
					.primary_key(Index::create().col(ShelfEntries::ShelfId).col(ShelfEntries::BookId))
					.foreign_key(
						ForeignKey::create()
							.name("fk_se_shelf")
							.from(ShelfEntries::Table, ShelfEntries::ShelfId)
							.to(Shelves::Table, Shelves::Id)
							.on_delete(ForeignKeyAction::Cascade),
					)
					.foreign_key(
						ForeignKey::create()
							.name("fk_se_book")
							.from(ShelfEntries::Table, ShelfEntries::BookId)
							.to(Books::Table, Books::Id)
							.on_delete(ForeignKeyAction::Cascade),
					)
					.to_owned(),
			)
			.await
	}

	async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		manager.drop_table(Table::drop().table(ShelfEntries::Table).to_owned()).await
	}
}

#[derive(DeriveIden)]
pub enum ShelfEntries {
	Table,
	ShelfId,
	BookId,
	Position,
	AddedAt,
}
