use sea_orm_migration::prelude::*;
use sea_orm_migration::schema::*;

use crate::m20260717_011703_create_books::Books;
use crate::m20260717_011703_create_users::Users;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
	async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		manager
			.create_table(
				Table::create()
					.table(Bookmarks::Table)
					.if_not_exists()
					.col(uuid(Bookmarks::Id).primary_key())
					.col(uuid(Bookmarks::UserId).not_null())
					.col(uuid(Bookmarks::BookId).not_null())
					.col(uuid(Bookmarks::SectionId).not_null())
					.col(integer(Bookmarks::BlockIndex).not_null())
					.col(string_null(Bookmarks::Title))
					.col(string_null(Bookmarks::Note))
					.col(timestamp_with_time_zone(Bookmarks::CreatedAt).not_null())
					.foreign_key(
						ForeignKey::create()
							.name("fk_bm_user")
							.from(Bookmarks::Table, Bookmarks::UserId)
							.to(Users::Table, Users::Id)
							.on_delete(ForeignKeyAction::Cascade),
					)
					.foreign_key(
						ForeignKey::create()
							.name("fk_bm_book")
							.from(Bookmarks::Table, Bookmarks::BookId)
							.to(Books::Table, Books::Id)
							.on_delete(ForeignKeyAction::Cascade),
					)
					.to_owned(),
			)
			.await
	}

	async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		manager.drop_table(Table::drop().table(Bookmarks::Table).to_owned()).await
	}
}

#[derive(DeriveIden)]
pub enum Bookmarks {
	Table,
	Id,
	UserId,
	BookId,
	SectionId,
	BlockIndex,
	Title,
	Note,
	CreatedAt,
}
