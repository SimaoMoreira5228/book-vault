use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "linguistic_annotations")]
pub struct Model {
	#[sea_orm(primary_key, auto_increment = false)]
	pub id: Uuid,
	pub book_id: Uuid,
	pub language: String,
	pub section_id: Uuid,
	pub block_index: i64,
	pub char_start: i64,
	pub char_end: i64,
	pub surface_form: String,
	pub lemma: String,
	pub reading: Option<String>,
	pub pos: Option<String>,
	pub frequency_rank: Option<i64>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
	#[sea_orm(
		belongs_to = "super::books::Entity",
		from = "Column::BookId",
		to = "super::books::Column::Id",
		on_update = "NoAction",
		on_delete = "Cascade"
	)]
	Books,
}

impl Related<super::books::Entity> for Entity {
	fn to() -> RelationDef {
		Relation::Books.def()
	}
}

impl ActiveModelBehavior for ActiveModel {}
