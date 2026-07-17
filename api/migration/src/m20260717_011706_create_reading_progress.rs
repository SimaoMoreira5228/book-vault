use sea_orm_migration::{prelude::*, schema::*};
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
                    .table(ReadingProgress::Table)
                    .if_not_exists()
                    .col(uuid(ReadingProgress::Id).primary_key())
                    .col(uuid(ReadingProgress::UserId).not_null())
                    .col(uuid(ReadingProgress::BookId).not_null())
                    .col(uuid(ReadingProgress::SectionId).not_null())
                    .col(integer(ReadingProgress::BlockIndex).not_null())
                    .col(integer(ReadingProgress::CharOffset).not_null())
                    .col(double(ReadingProgress::Percentage).not_null())
                    .col(timestamp_with_time_zone(ReadingProgress::UpdatedAt).not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_rp_user")
                            .from(ReadingProgress::Table, ReadingProgress::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_rp_book")
                            .from(ReadingProgress::Table, ReadingProgress::BookId)
                            .to(Books::Table, Books::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                IndexCreateStatement::new()
                    .name("idx_rp_user_book")
                    .table(ReadingProgress::Table)
                    .col(ReadingProgress::UserId)
                    .col(ReadingProgress::BookId)
                    .unique()
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(ReadingProgress::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
pub enum ReadingProgress {
    Table,
    Id,
    UserId,
    BookId,
    SectionId,
    BlockIndex,
    CharOffset,
    Percentage,
    UpdatedAt,
}
