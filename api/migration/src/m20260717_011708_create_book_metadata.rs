use sea_orm_migration::{prelude::*, schema::*};
use crate::m20260717_011703_create_books::Books;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(BookMetadata::Table)
                    .if_not_exists()
                    .col(uuid(BookMetadata::Id).primary_key())
                    .col(uuid(BookMetadata::BookId).not_null().unique_key())
                    .col(json(BookMetadata::ProviderIds))
                    .col(json(BookMetadata::LockedFields))
                    .col(json(BookMetadata::CachedMetadata))
                    .col(timestamp_with_time_zone_null(BookMetadata::LastRefreshedAt))
                    .col(timestamp_with_time_zone(BookMetadata::CreatedAt).not_null())
                    .col(timestamp_with_time_zone(BookMetadata::UpdatedAt).not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_book_metadata_book")
                            .from(BookMetadata::Table, BookMetadata::BookId)
                            .to(Books::Table, Books::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(MetadataCache::Table)
                    .if_not_exists()
                    .col(uuid(MetadataCache::Id).primary_key())
                    .col(string(MetadataCache::Provider).not_null())
                    .col(string(MetadataCache::QueryHash).not_null())
                    .col(json(MetadataCache::Response).not_null())
                    .col(timestamp_with_time_zone(MetadataCache::ExpiresAt).not_null())
                    .col(timestamp_with_time_zone(MetadataCache::CreatedAt).not_null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_metadata_cache_lookup")
                    .table(MetadataCache::Table)
                    .col(MetadataCache::Provider)
                    .col(MetadataCache::QueryHash)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(MetadataCache::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(BookMetadata::Table).to_owned()).await
    }
}

#[derive(DeriveIden)]
enum BookMetadata {
    Table,
    Id,
    BookId,
    ProviderIds,
    LockedFields,
    CachedMetadata,
    LastRefreshedAt,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum MetadataCache {
    Table,
    Id,
    Provider,
    QueryHash,
    Response,
    ExpiresAt,
    CreatedAt,
}
