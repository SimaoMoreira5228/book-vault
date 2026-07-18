use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "vocab_example_sentences")]
pub struct Model {
	#[sea_orm(primary_key, auto_increment = false)]
	pub id: Uuid,
	pub vocabulary_entry_id: Uuid,
	pub sentence: String,
	pub source: String,
	pub book_id: Option<Uuid>,
	pub book_title: Option<String>,
	pub created_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
	#[sea_orm(
		belongs_to = "super::vocabulary_entries::Entity",
		from = "Column::VocabularyEntryId",
		to = "super::vocabulary_entries::Column::Id",
		on_update = "NoAction",
		on_delete = "Cascade"
	)]
	VocabularyEntries,
}

impl ActiveModelBehavior for ActiveModel {}
