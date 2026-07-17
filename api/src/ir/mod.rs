pub mod block;
pub mod span;

use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Section {
	pub id: Uuid,
	pub title: Option<String>,
	pub sequence_index: u32,
	pub blocks: Vec<block::Block>,
}

#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct BookIr {
	pub version: u8,
	pub spine: Vec<Section>,
}
