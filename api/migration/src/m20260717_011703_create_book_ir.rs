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
                    .table(BookIr::Table)
                    .if_not_exists()
                    .col(uuid(BookIr::Id).primary_key())
                    .col(uuid(BookIr::BookId).unique_key().not_null())
                    .col(tiny_integer(BookIr::Version).not_null())
                    .col(binary(BookIr::Payload).not_null())
                    .col(timestamp_with_time_zone(BookIr::CreatedAt).not_null())
                    .col(timestamp_with_time_zone(BookIr::UpdatedAt).not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_book_ir_book")
                            .from(BookIr::Table, BookIr::BookId)
                            .to(Books::Table, Books::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(BookIr::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
pub enum BookIr {
    Table,
    Id,
    BookId,
    Version,
    Payload,
    CreatedAt,
    UpdatedAt,
}
