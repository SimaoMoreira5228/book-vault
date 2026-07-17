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
                    .table(Assets::Table)
                    .if_not_exists()
                    .col(uuid(Assets::Id).primary_key())
                    .col(uuid(Assets::BookId).not_null())
                    .col(string(Assets::Kind).not_null())
                    .col(string(Assets::MimeType).not_null())
                    .col(big_integer(Assets::SizeBytes).not_null())
                    .col(string(Assets::StoragePath).not_null())
                    .col(string(Assets::Sha256).not_null())
                    .col(timestamp_with_time_zone(Assets::CreatedAt).not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_assets_book")
                            .from(Assets::Table, Assets::BookId)
                            .to(Books::Table, Books::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Assets::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
pub enum Assets {
    Table,
    Id,
    BookId,
    Kind,
    MimeType,
    SizeBytes,
    StoragePath,
    Sha256,
    CreatedAt,
}
