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
					.table(Authors::Table)
					.if_not_exists()
					.col(uuid(Authors::Id).primary_key())
					.col(uuid(Authors::LibraryId).not_null())
					.col(string(Authors::Name).not_null())
					.col(string_null(Authors::SortName))
					.col(string_null(Authors::Bio))
					.col(string_null(Authors::BirthDate))
					.col(string_null(Authors::DeathDate))
					.col(uuid_null(Authors::PhotoAssetId))
					.col(timestamp_with_time_zone(Authors::CreatedAt).not_null())
					.col(timestamp_with_time_zone(Authors::UpdatedAt).not_null())
					.foreign_key(
						ForeignKey::create()
							.name("fk_auth_library")
							.from(Authors::Table, Authors::LibraryId)
							.to(Libraries::Table, Libraries::Id)
							.on_delete(ForeignKeyAction::Cascade),
					)
					.to_owned(),
			)
			.await
	}

	async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		manager.drop_table(Table::drop().table(Authors::Table).to_owned()).await
	}
}

#[derive(DeriveIden)]
pub enum Authors {
	Table,
	Id,
	LibraryId,
	Name,
	SortName,
	Bio,
	BirthDate,
	DeathDate,
	PhotoAssetId,
	CreatedAt,
	UpdatedAt,
}
