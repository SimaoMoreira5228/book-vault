use sea_orm_migration::{prelude::*, schema::*};
use crate::m20260717_011703_create_books::Books;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Books::Table)
                    .add_column_if_not_exists(
                        ColumnDef::new(Alias::new("keep_source")).boolean().default(true)
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Books::Table)
                    .add_column_if_not_exists(
                        ColumnDef::new(Alias::new("sequence_index")).integer().default(0)
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Books::Table)
                    .drop_column(Alias::new("keep_source"))
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Books::Table)
                    .drop_column(Alias::new("sequence_index"))
                    .to_owned(),
            )
            .await
    }
}
