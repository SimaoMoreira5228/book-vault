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
					.table(VocabularyEntries::Table)
					.if_not_exists()
					.col(uuid(VocabularyEntries::Id).primary_key())
					.col(uuid(VocabularyEntries::UserId).not_null())
					.col(string(VocabularyEntries::Language).not_null())
					.col(string(VocabularyEntries::Lemma).not_null())
					.col(string_null(VocabularyEntries::SenseLabel))
					.col(string_null(VocabularyEntries::SenseId))
					.col(string_null(VocabularyEntries::Definition))
					.col(string(VocabularyEntries::State).not_null().default("unknown"))
					.col(timestamp_with_time_zone(VocabularyEntries::FirstSeenAt).not_null())
					.col(timestamp_with_time_zone_null(VocabularyEntries::LastReviewedAt))
					.col(timestamp_with_time_zone_null(VocabularyEntries::SrsDueAt))
					.col(integer_null(VocabularyEntries::SrsIntervalDays))
					.col(double_null(VocabularyEntries::SrsEaseFactor))
					.col(string_null(VocabularyEntries::SentenceSnippet))
					.col(string_null(VocabularyEntries::ContextSentence))
					.col(string_null(VocabularyEntries::Source))
					.col(integer_null(VocabularyEntries::FrequencyRankSense))
					.foreign_key(
						ForeignKey::create()
							.name("fk_vocab_user")
							.from(VocabularyEntries::Table, VocabularyEntries::UserId)
							.to(Users::Table, Users::Id)
							.on_delete(ForeignKeyAction::Cascade),
					)
					.to_owned(),
			)
			.await?;

		manager
			.create_index(
				Index::create()
					.name("idx_vocab_user_lang_sense")
					.table(VocabularyEntries::Table)
					.col(VocabularyEntries::UserId)
					.col(VocabularyEntries::Language)
					.col(VocabularyEntries::Lemma)
					.col(VocabularyEntries::SenseId)
					.unique()
					.to_owned(),
			)
			.await
	}

	async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
		manager
			.drop_table(Table::drop().table(VocabularyEntries::Table).to_owned())
			.await
	}
}

#[derive(DeriveIden)]
pub enum VocabularyEntries {
	Table,
	Id,
	UserId,
	Language,
	Lemma,
	SenseLabel,
	SenseId,
	Definition,
	State,
	FirstSeenAt,
	LastReviewedAt,
	SrsDueAt,
	SrsIntervalDays,
	SrsEaseFactor,
	SentenceSnippet,
	ContextSentence,
	Source,
	FrequencyRankSense,
}
