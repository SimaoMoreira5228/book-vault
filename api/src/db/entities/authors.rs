use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "authors")]
pub struct Model {
	#[sea_orm(primary_key, auto_increment = false)]
	pub id: Uuid,
	pub library_id: Uuid,
	pub name: String,
	pub sort_name: Option<String>,
	pub bio: Option<String>,
	pub birth_date: Option<String>,
	pub death_date: Option<String>,
	pub photo_asset_id: Option<Uuid>,
	pub created_at: DateTimeWithTimeZone,
	pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
	#[sea_orm(
		belongs_to = "super::libraries::Entity",
		from = "Column::LibraryId",
		to = "super::libraries::Column::Id",
		on_update = "NoAction",
		on_delete = "Cascade"
	)]
	Libraries,
}

impl Related<super::libraries::Entity> for Entity {
	fn to() -> RelationDef {
		Relation::Libraries.def()
	}
}

impl ActiveModelBehavior for ActiveModel {}
