use sea_orm_migration::prelude::*;
use sea_orm_migration::schema::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
	async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		manager
			.create_table(
				Table::create()
					.table(DictionaryCache::Table)
					.if_not_exists()
					.col(uuid(DictionaryCache::Id).primary_key())
					.col(string(DictionaryCache::Word).not_null())
					.col(string(DictionaryCache::Language).not_null())
					.col(string(DictionaryCache::ContextHash).not_null())
					.col(string(DictionaryCache::ResponseJson).not_null())
					.col(timestamp_with_time_zone(DictionaryCache::CreatedAt).not_null())
					.col(timestamp_with_time_zone(DictionaryCache::ExpiresAt).not_null())
					.to_owned(),
			)
			.await?;

		manager
			.create_index(
				Index::create()
					.name("idx_dict_cache_lookup")
					.table(DictionaryCache::Table)
					.col(DictionaryCache::Word)
					.col(DictionaryCache::Language)
					.col(DictionaryCache::ContextHash)
					.to_owned(),
			)
			.await
	}

	async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		manager
			.drop_table(Table::drop().table(DictionaryCache::Table).to_owned())
			.await
	}
}

#[derive(DeriveIden)]
pub enum DictionaryCache {
	Table,
	Id,
	Word,
	Language,
	ContextHash,
	ResponseJson,
	CreatedAt,
	ExpiresAt,
}
