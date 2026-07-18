use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "vocabulary_entries")]
pub struct Model {
	#[sea_orm(primary_key, auto_increment = false)]
	pub id: Uuid,
	pub user_id: Uuid,
	pub language: String,
	pub lemma: String,
	pub sense_label: Option<String>,
	pub sense_id: Option<String>,
	pub definition: Option<String>,
	pub state: String,
	pub first_seen_at: DateTimeWithTimeZone,
	pub last_reviewed_at: Option<DateTimeWithTimeZone>,
	pub srs_due_at: Option<DateTimeWithTimeZone>,
	pub srs_interval_days: Option<i64>,
	pub srs_ease_factor: Option<f64>,
	pub sentence_snippet: Option<String>,
	pub context_sentence: Option<String>,
	pub source: Option<String>,
	pub frequency_rank_sense: Option<i64>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
	#[sea_orm(
		belongs_to = "super::users::Entity",
		from = "Column::UserId",
		to = "super::users::Column::Id",
		on_update = "NoAction",
		on_delete = "Cascade"
	)]
	Users,
}

impl Related<super::users::Entity> for Entity {
	fn to() -> RelationDef {
		Relation::Users.def()
	}
}

impl ActiveModelBehavior for ActiveModel {}
