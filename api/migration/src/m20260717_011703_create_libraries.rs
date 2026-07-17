use sea_orm_migration::prelude::*;
use sea_orm_migration::schema::*;

use crate::m20260717_011703_create_users::Users;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
	async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		manager
			.create_table(
				Table::create()
					.table(Libraries::Table)
					.if_not_exists()
					.col(uuid(Libraries::Id).primary_key())
					.col(string(Libraries::Name).not_null())
					.col(string_null(Libraries::Description))
					.col(uuid(Libraries::OwnerId).not_null())
					.col(timestamp_with_time_zone(Libraries::CreatedAt).not_null())
					.foreign_key(
						ForeignKey::create()
							.name("fk_libraries_owner")
							.from(Libraries::Table, Libraries::OwnerId)
							.to(Users::Table, Users::Id)
							.on_delete(ForeignKeyAction::Cascade),
					)
					.to_owned(),
			)
			.await
	}

	async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		manager.drop_table(Table::drop().table(Libraries::Table).to_owned()).await
	}
}

#[derive(DeriveIden)]
pub enum Libraries {
	Table,
	Id,
	Name,
	Description,
	OwnerId,
	CreatedAt,
}
