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
					.table(VocabExampleSentences::Table)
					.if_not_exists()
					.col(uuid(VocabExampleSentences::Id).primary_key())
					.col(uuid(VocabExampleSentences::VocabularyEntryId).not_null())
					.col(string(VocabExampleSentences::Sentence).not_null())
					.col(string(VocabExampleSentences::Source).not_null().default("book"))
					.col(uuid_null(VocabExampleSentences::BookId))
					.col(string_null(VocabExampleSentences::BookTitle))
					.col(timestamp_with_time_zone(VocabExampleSentences::CreatedAt).not_null())
					.foreign_key(
						ForeignKey::create()
							.name("fk_vocab_sent_entry")
							.from(VocabExampleSentences::Table, VocabExampleSentences::VocabularyEntryId)
							.to(
								crate::m20260718_create_vocabulary_entries::VocabularyEntries::Table,
								crate::m20260718_create_vocabulary_entries::VocabularyEntries::Id,
							)
							.on_delete(ForeignKeyAction::Cascade),
					)
					.to_owned(),
			)
			.await
	}

	async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		manager
			.drop_table(Table::drop().table(VocabExampleSentences::Table).to_owned())
			.await
	}
}

#[derive(DeriveIden)]
pub enum VocabExampleSentences {
	Table,
	Id,
	VocabularyEntryId,
	Sentence,
	Source,
	BookId,
	BookTitle,
	CreatedAt,
}
