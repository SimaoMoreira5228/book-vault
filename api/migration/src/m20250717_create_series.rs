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
					.table(Series::Table)
					.if_not_exists()
					.col(uuid(Series::Id).primary_key())
					.col(uuid(Series::LibraryId).not_null())
					.col(string(Series::Name).not_null())
					.col(string_null(Series::Description))
					.col(timestamp_with_time_zone(Series::CreatedAt).not_null())
					.col(timestamp_with_time_zone(Series::UpdatedAt).not_null())
					.foreign_key(
						ForeignKey::create()
							.name("fk_series_lib")
							.from(Series::Table, Series::LibraryId)
							.to(Libraries::Table, Libraries::Id)
							.on_delete(ForeignKeyAction::Cascade),
					)
					.to_owned(),
			)
			.await
	}

	async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		manager.drop_table(Table::drop().table(Series::Table).to_owned()).await
	}
}

#[derive(DeriveIden)]
pub enum Series {
	Table,
	Id,
	LibraryId,
	Name,
	Description,
	CreatedAt,
	UpdatedAt,
}
