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
					.table(Annotations::Table)
					.if_not_exists()
					.col(uuid(Annotations::Id).primary_key())
					.col(uuid(Annotations::UserId).not_null())
					.col(uuid(Annotations::BookId).not_null())
					.col(uuid(Annotations::SectionId).not_null())
					.col(integer(Annotations::BlockIndex).not_null())
					.col(integer(Annotations::StartOffset).not_null())
					.col(integer(Annotations::EndOffset).not_null())
					.col(string_null(Annotations::Color))
					.col(string_null(Annotations::Note))
					.col(timestamp_with_time_zone(Annotations::CreatedAt).not_null())
					.col(timestamp_with_time_zone(Annotations::UpdatedAt).not_null())
					.foreign_key(
						ForeignKey::create()
							.name("fk_ann_user")
							.from(Annotations::Table, Annotations::UserId)
							.to(Users::Table, Users::Id)
							.on_delete(ForeignKeyAction::Cascade),
					)
					.foreign_key(
						ForeignKey::create()
							.name("fk_ann_book")
							.from(Annotations::Table, Annotations::BookId)
							.to(Books::Table, Books::Id)
							.on_delete(ForeignKeyAction::Cascade),
					)
					.to_owned(),
			)
			.await
	}

	async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		manager.drop_table(Table::drop().table(Annotations::Table).to_owned()).await
	}
}

#[derive(DeriveIden)]
pub enum Annotations {
	Table,
	Id,
	UserId,
	BookId,
	SectionId,
	BlockIndex,
	StartOffset,
	EndOffset,
	Color,
	Note,
	CreatedAt,
	UpdatedAt,
}
