use sea_orm_migration::{prelude::*, schema::*};
use crate::m20260717_011703_create_libraries::Libraries;
use crate::m20260717_011703_create_users::Users;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Shelves::Table)
                    .if_not_exists()
                    .col(uuid(Shelves::Id).primary_key())
                    .col(uuid(Shelves::LibraryId).not_null())
                    .col(string(Shelves::Name).not_null())
                    .col(string_null(Shelves::Description))
                    .col(string(Shelves::Kind).not_null())
                    .col(json_null(Shelves::RuleAst))
                    .col(uuid(Shelves::OwnerId).not_null())
                    .col(timestamp_with_time_zone(Shelves::CreatedAt).not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_shelves_library")
                            .from(Shelves::Table, Shelves::LibraryId)
                            .to(Libraries::Table, Libraries::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_shelves_user")
                            .from(Shelves::Table, Shelves::OwnerId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Shelves::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
pub enum Shelves {
    Table,
    Id,
    LibraryId,
    Name,
    Description,
    Kind,
    RuleAst,
    OwnerId,
    CreatedAt,
}
