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
					.table(KoboTokens::Table)
					.if_not_exists()
					.col(uuid(KoboTokens::Id).primary_key())
					.col(uuid(KoboTokens::UserId).not_null())
					.col(string(KoboTokens::Token).not_null())
					.col(string_null(KoboTokens::DeviceName))
					.col(timestamp_with_time_zone(KoboTokens::CreatedAt).not_null())
					.foreign_key(
						ForeignKey::create()
							.name("fk_kobo_token_user")
							.from(KoboTokens::Table, KoboTokens::UserId)
							.to(Users::Table, Users::Id)
							.on_delete(ForeignKeyAction::Cascade),
					)
					.to_owned(),
			)
			.await?;
		manager
			.create_index(
				Index::create()
					.name("idx_kobo_tokens_token")
					.table(KoboTokens::Table)
					.col(KoboTokens::Token)
					.unique()
					.to_owned(),
			)
			.await
	}

	async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		manager
			.drop_table(Table::drop().table(KoboTokens::Table).to_owned())
			.await
	}
}

#[derive(DeriveIden)]
pub enum KoboTokens {
	Table,
	Id,
	UserId,
	Token,
	DeviceName,
	CreatedAt,
}
