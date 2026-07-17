use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum RuleOperator {
	#[serde(rename = "equals")]
	Equals,
	#[serde(rename = "not_equals")]
	NotEquals,
	#[serde(rename = "contains")]
	Contains,
	#[serde(rename = "greater_than")]
	GreaterThan,
	#[serde(rename = "less_than")]
	LessThan,
	#[serde(rename = "within_last")]
	WithinLast,
	#[serde(rename = "includes_any")]
	IncludesAny,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum RuleField {
	#[serde(rename = "title")]
	Title,
	#[serde(rename = "author")]
	Author,
	#[serde(rename = "language")]
	Language,
	#[serde(rename = "publisher")]
	Publisher,
	#[serde(rename = "isbn")]
	Isbn,
	#[serde(rename = "series")]
	Series,
	#[serde(rename = "page_count")]
	PageCount,
	#[serde(rename = "read_status")]
	ReadStatus,
	#[serde(rename = "rating")]
	Rating,
	#[serde(rename = "date_added")]
	DateAdded,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CombinationOp {
	#[serde(rename = "and")]
	And,
	#[serde(rename = "or")]
	Or,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RuleNode {
	Leaf {
		field: RuleField,
		op: RuleOperator,
		value: serde_json::Value,
	},
	Group {
		operator: CombinationOp,
		rules: Vec<RuleNode>,
	},
}
