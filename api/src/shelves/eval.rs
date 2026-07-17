use sea_orm::{ColumnTrait, Condition};

use crate::db::entities::books;
use crate::shelves::rule::{CombinationOp, RuleField, RuleNode, RuleOperator};

fn str_val(v: &serde_json::Value) -> Option<String> {
	v.as_str().map(|s| s.to_string())
}

fn num_val(v: &serde_json::Value) -> Option<i64> {
	v.as_i64().or_else(|| v.as_f64().map(|f| f as i64))
}

fn build_leaf_condition(field: &RuleField, op: &RuleOperator, value: &serde_json::Value) -> Condition {
	match field {
		RuleField::Title => text_condition(books::Column::Title, op, value),
		RuleField::Author => text_condition(books::Column::Author, op, value),
		RuleField::Language => text_condition(books::Column::Language, op, value),
		RuleField::Publisher => text_condition(books::Column::Publisher, op, value),
		RuleField::Isbn => text_condition(books::Column::Isbn, op, value),
		RuleField::Series => text_condition(books::Column::Series, op, value),
		RuleField::PageCount => num_condition(books::Column::PageCount, op, value),
		RuleField::ReadStatus => text_condition(books::Column::ReadStatus, op, value),
		RuleField::Rating => num_condition(books::Column::Rating, op, value),
		RuleField::DateAdded => date_condition(books::Column::CreatedAt, op, value),
	}
}

fn text_condition(col: impl ColumnTrait, op: &RuleOperator, value: &serde_json::Value) -> Condition {
	let v = str_val(value);
	match op {
		RuleOperator::Equals => Condition::all().add_option(v.map(|x| col.eq(x))),
		RuleOperator::NotEquals => Condition::all().add_option(v.map(|x| col.ne(x))),
		RuleOperator::Contains => Condition::all().add_option(v.map(|x| col.like(format!("%{}%", x)))),
		RuleOperator::IncludesAny => {
			let arr = value.as_array();
			Condition::all().add_option(arr.map(|vals| {
				let likes: Vec<sea_orm::Condition> = vals
					.iter()
					.filter_map(|v| v.as_str())
					.map(|s| Condition::any().add(col.like(format!("%{}%", s))))
					.collect();
				likes.into_iter().fold(Condition::any(), |acc, c| acc.add(c))
			}))
		}
		_ => Condition::all(),
	}
}

fn num_condition(col: impl ColumnTrait, op: &RuleOperator, value: &serde_json::Value) -> Condition {
	let n = num_val(value);
	match op {
		RuleOperator::Equals => Condition::all().add_option(n.map(|x| col.eq(x))),
		RuleOperator::NotEquals => Condition::all().add_option(n.map(|x| col.ne(x))),
		RuleOperator::GreaterThan => Condition::all().add_option(n.map(|x| col.gt(x))),
		RuleOperator::LessThan => Condition::all().add_option(n.map(|x| col.lt(x))),
		_ => Condition::all(),
	}
}

fn date_condition(col: impl ColumnTrait, op: &RuleOperator, value: &serde_json::Value) -> Condition {
	match op {
		RuleOperator::WithinLast => {
			let days = num_val(value).unwrap_or(30).abs();
			let cutoff: chrono::DateTime<chrono::FixedOffset> = (chrono::Utc::now() - chrono::Duration::days(days)).into();
			Condition::all().add(col.gte(cutoff))
		}
		_ => Condition::all(),
	}
}

fn evaluate_node(node: &RuleNode) -> Condition {
	match node {
		RuleNode::Leaf { field, op, value } => build_leaf_condition(field, op, value),
		RuleNode::Group { operator, rules } => {
			let conditions: Vec<Condition> = rules.iter().map(evaluate_node).collect();
			match operator {
				CombinationOp::And => {
					let mut combined = Condition::all();
					for c in conditions {
						combined = combined.add(c);
					}
					combined
				}
				CombinationOp::Or => {
					let mut combined = Condition::any();
					for c in conditions {
						combined = combined.add(c);
					}
					combined
				}
			}
		}
	}
}

pub fn build_condition(rule_ast: &serde_json::Value) -> Result<Condition, crate::AppError> {
	let root: RuleNode = serde_json::from_value(rule_ast.clone())
		.map_err(|e| crate::AppError::Internal(format!("Invalid rule AST: {}", e)))?;
	Ok(evaluate_node(&root))
}
