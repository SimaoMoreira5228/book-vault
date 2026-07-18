use sea_orm_migration::prelude::*;
use sea_orm_migration::schema::*;

use crate::m20260717_011703_create_books::Books;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
	async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		manager
			.create_table(
				Table::create()
					.table(LinguisticAnnotations::Table)
					.if_not_exists()
					.col(uuid(LinguisticAnnotations::Id).primary_key())
					.col(uuid(LinguisticAnnotations::BookId).not_null())
					.col(string(LinguisticAnnotations::Language).not_null())
					.col(uuid(LinguisticAnnotations::SectionId).not_null())
					.col(integer(LinguisticAnnotations::BlockIndex).not_null())
					.col(integer(LinguisticAnnotations::CharStart).not_null())
					.col(integer(LinguisticAnnotations::CharEnd).not_null())
					.col(string(LinguisticAnnotations::SurfaceForm).not_null())
					.col(string(LinguisticAnnotations::Lemma).not_null())
					.col(string_null(LinguisticAnnotations::Reading))
					.col(string_null(LinguisticAnnotations::Pos))
					.col(integer_null(LinguisticAnnotations::FrequencyRank))
					.foreign_key(
						ForeignKey::create()
							.name("fk_ling_ann_book")
							.from(LinguisticAnnotations::Table, LinguisticAnnotations::BookId)
							.to(Books::Table, Books::Id)
							.on_delete(ForeignKeyAction::Cascade),
					)
					.to_owned(),
			)
			.await
	}

	async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		manager
			.drop_table(Table::drop().table(LinguisticAnnotations::Table).to_owned())
			.await
	}
}

#[derive(DeriveIden)]
pub enum LinguisticAnnotations {
	Table,
	Id,
	BookId,
	Language,
	SectionId,
	BlockIndex,
	CharStart,
	CharEnd,
	SurfaceForm,
	Lemma,
	Reading,
	Pos,
	FrequencyRank,
}
