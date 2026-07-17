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
					.table(Sessions::Table)
					.if_not_exists()
					.col(uuid(Sessions::Id).primary_key())
					.col(uuid(Sessions::UserId).not_null())
					.col(binary(Sessions::TokenHash).not_null().unique_key())
					.col(string_null(Sessions::UserAgent))
					.col(string_null(Sessions::IpAddress))
					.col(timestamp_with_time_zone(Sessions::CreatedAt).not_null())
					.col(timestamp_with_time_zone(Sessions::LastSeenAt).not_null())
					.col(timestamp_with_time_zone(Sessions::ExpiresAt).not_null())
					.col(timestamp_with_time_zone_null(Sessions::RevokedAt))
					.foreign_key(
						ForeignKey::create()
							.name("fk_sessions_user")
							.from(Sessions::Table, Sessions::UserId)
							.to(Users::Table, Users::Id)
							.on_delete(ForeignKeyAction::Cascade),
					)
					.to_owned(),
			)
			.await?;

		manager
			.create_index(
				IndexCreateStatement::new()
					.name("idx_sessions_token_hash")
					.table(Sessions::Table)
					.col(Sessions::TokenHash)
					.to_owned(),
			)
			.await?;

		manager
			.create_index(
				IndexCreateStatement::new()
					.name("idx_sessions_user_id")
					.table(Sessions::Table)
					.col(Sessions::UserId)
					.to_owned(),
			)
			.await
	}

	async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		manager.drop_table(Table::drop().table(Sessions::Table).to_owned()).await
	}
}

#[derive(DeriveIden)]
pub enum Sessions {
	Table,
	Id,
	UserId,
	TokenHash,
	UserAgent,
	IpAddress,
	CreatedAt,
	LastSeenAt,
	ExpiresAt,
	RevokedAt,
}
