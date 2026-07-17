use std::collections::{HashMap, HashSet};

use dashmap::DashMap;
use tracing::info;
use uuid::Uuid;

use crate::ir::BookIr;
use crate::ir::block::Block;

#[derive(Clone)]
pub struct SearchHit {
	pub book_id: Uuid,
	pub section_id: Uuid,
	pub block_index: u32,
	pub text: String,
	pub score: f64,
	pub snippet: String,
}

pub struct SearchEngine {
	word_index: DashMap<String, Vec<(Uuid, Uuid, u32)>>,
	trigram_index: DashMap<String, HashSet<String>>,
	texts: DashMap<(Uuid, Uuid, u32), String>,
}

impl Default for SearchEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl SearchEngine {
	pub fn new() -> Self {
		Self {
			word_index: DashMap::new(),
			trigram_index: DashMap::new(),
			texts: DashMap::new(),
		}
	}

	pub fn index_book(&self, book_id: Uuid, ir: &BookIr) {
		for section in &ir.spine {
			for (idx, block) in section.blocks.iter().enumerate() {
				let text = extract_block_text(block);
				if text.is_empty() {
					continue;
				}
				let key = (book_id, section.id, idx as u32);
				let words = tokenize(&text);
				for word in &words {
					self.word_index
						.entry(word.clone())
						.or_default()
						.push((book_id, section.id, idx as u32));
					for tri in trigrams(word) {
						self.trigram_index.entry(tri).or_default().insert(word.clone());
					}
				}
				self.texts.insert(key, text);
			}
		}
		info!("Indexed book {}: {} sections", book_id, ir.spine.len());
	}

	pub fn remove_book(&self, book_id: &Uuid) {
		let keys: Vec<_> = self
			.texts
			.iter()
			.filter(|e| e.key().0 == *book_id)
			.map(|e| *e.key())
			.collect();
		for k in &keys {
			self.texts.remove(k);
		}

		let empty_words: Vec<String> = self
			.word_index
			.iter_mut()
			.filter_map(|mut entry| {
				entry.retain(|(bid, _, _)| bid != book_id);
				if entry.is_empty() { Some(entry.key().clone()) } else { None }
			})
			.collect();
		for w in &empty_words {
			self.word_index.remove(w);
			let empty_tris: Vec<String> = self
				.trigram_index
				.iter_mut()
				.filter_map(|mut te| {
					te.remove(w);
					if te.is_empty() { Some(te.key().clone()) } else { None }
				})
				.collect();
			for t in &empty_tris {
				self.trigram_index.remove(t);
			}
		}
	}

	pub fn search(&self, query: &str, limit: usize) -> Vec<SearchHit> {
		let query_terms = tokenize(query);
		if query_terms.is_empty() {
			return Vec::new();
		}

		let mut scores: HashMap<(Uuid, Uuid, u32), (f64, Vec<usize>)> = HashMap::new();

		for term in &query_terms {
			let lower = term.to_lowercase();

			if let Some(exact) = self.word_index.get(&lower) {
				for &(bid, sid, bidx) in exact.iter() {
					let e = scores.entry((bid, sid, bidx)).or_insert((0.0, Vec::new()));
					e.0 += 1.0;
					if let Some(txt) = self.texts.get(&(bid, sid, bidx)) {
						if let Some(pos) = txt.to_lowercase().find(&lower) {
							e.1.push(pos);
						}
					}
				}
			}

			for entry in self.word_index.iter() {
				if entry.key().starts_with(&lower) && *entry.key() != lower {
					for &(bid, sid, bidx) in entry.value().iter() {
						let e = scores.entry((bid, sid, bidx)).or_insert((0.0, Vec::new()));
						e.0 += 0.8;
						if let Some(txt) = self.texts.get(&(bid, sid, bidx)) {
							if let Some(pos) = txt.to_lowercase().find(entry.key()) {
								e.1.push(pos);
							}
						}
					}
				}
			}

			let term_tris: HashSet<String> = trigrams(&lower).into_iter().collect();
			if !term_tris.is_empty() {
				let mut candidates: HashMap<String, f64> = HashMap::new();
				for tri in &term_tris {
					if let Some(words) = self.trigram_index.get(tri) {
						for w in words.value().iter() {
							*candidates.entry(w.clone()).or_insert(0.0) += 1.0;
						}
					}
				}
				for (candidate, overlap) in &candidates {
					if *candidate == lower {
						continue;
					}
					let cand_tris: HashSet<String> = trigrams(candidate).into_iter().collect();
					let union = term_tris.union(&cand_tris).count() as f64;
					let similarity = if union > 0.0 { overlap / union } else { 0.0 };
					if similarity >= 0.3 {
						if let Some(fuzzy) = self.word_index.get(candidate) {
							for &(bid, sid, bidx) in fuzzy.iter() {
								let e = scores.entry((bid, sid, bidx)).or_insert((0.0, Vec::new()));
								e.0 += similarity * 0.6;
								if let Some(txt) = self.texts.get(&(bid, sid, bidx)) {
									if let Some(pos) = txt.to_lowercase().find(candidate.as_str()) {
										e.1.push(pos);
									}
								}
							}
						}
					}
				}
			}
		}

		let mut results: Vec<SearchHit> = scores
			.into_iter()
			.filter_map(|((bid, sid, bidx), (score, positions))| {
				let avg_pos = if positions.is_empty() {
					0
				} else {
					positions.iter().sum::<usize>() / positions.len()
				};
				self.texts.get(&(bid, sid, bidx)).map(|txt| {
					let snippet = generate_snippet(&txt, avg_pos, 150);
					SearchHit {
						book_id: bid,
						section_id: sid,
						block_index: bidx,
						text: txt.clone(),
						score,
						snippet,
					}
				})
			})
			.collect();

		results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
		results.truncate(limit);
		results
	}

	pub fn rebuild(&self, db: &sea_orm::DatabaseConnection) {
		use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

		use crate::db::entities::book_ir as ir_entity;

		let rows = tokio::task::block_in_place(|| {
			tokio::runtime::Handle::current().block_on(async {
				ir_entity::Entity::find()
					.filter(ir_entity::Column::Payload.is_not_null())
					.all(db)
					.await
			})
		});

		match rows {
			Ok(rows) => {
				let mut count = 0u32;
				for row in &rows {
					if let Ok(ir) = rmp_serde::from_slice::<BookIr>(&row.payload) {
						self.index_book(row.book_id, &ir);
						count += 1;
					}
				}
				info!("Search engine rebuilt: {} books indexed", count);
			}
			Err(e) => {
				tracing::error!("Failed to rebuild search index: {e}");
			}
		}
	}
}

fn tokenize(text: &str) -> Vec<String> {
	text.to_lowercase()
		.split(|c: char| !c.is_alphanumeric() && c != '\'' && c != '-')
		.filter(|s| s.len() >= 2)
		.map(|s| s.to_string())
		.collect()
}

fn trigrams(word: &str) -> Vec<String> {
	let padded = format!("  {word} ");
	padded
		.as_bytes()
		.windows(3)
		.map(|w| String::from_utf8_lossy(w).to_string())
		.collect()
}

fn extract_block_text(block: &Block) -> String {
	match block {
		Block::Paragraph(spans) => spans.iter().map(|s| s.text.as_str()).collect(),
		Block::Heading { spans, .. } => spans.iter().map(|s| s.text.as_str()).collect(),
		Block::BlockQuote(blocks) => blocks.iter().map(extract_block_text).collect::<Vec<_>>().join(" "),
		Block::CodeBlock { content, .. } => content.clone(),
		Block::OrderedList(items) => items
			.iter()
			.flat_map(|item| item.iter().map(extract_block_text))
			.collect::<Vec<_>>()
			.join(" "),
		Block::UnorderedList(items) => items
			.iter()
			.flat_map(|item| item.iter().map(extract_block_text))
			.collect::<Vec<_>>()
			.join(" "),
		Block::Table { headers, rows, .. } => {
			let h: String = headers.iter().flat_map(|c| c.spans.iter().map(|s| s.text.as_str())).collect();
			let r: String = rows
				.iter()
				.flat_map(|row| row.iter().flat_map(|c| c.spans.iter().map(|s| s.text.as_str())))
				.collect();
			format!("{h} {r}")
		}
		Block::Image { alt, .. } => alt.clone().unwrap_or_default(),
		Block::HorizontalRule => String::new(),
		Block::Footnote { blocks, .. } => blocks.iter().map(extract_block_text).collect::<Vec<_>>().join(" "),
		Block::RawHtml { content } => strip_html(content),
	}
}

fn strip_html(html: &str) -> String {
	let mut out = String::with_capacity(html.len());
	let mut in_tag = false;
	for c in html.chars() {
		match c {
			'<' => in_tag = true,
			'>' => in_tag = false,
			_ if !in_tag => out.push(c),
			_ => {}
		}
	}
	out
}

fn generate_snippet(text: &str, position: usize, max_len: usize) -> String {
	if text.len() <= max_len {
		return text.to_string();
	}

	let mid = if position > max_len / 2 { position } else { max_len / 2 };
	let start = mid.saturating_sub(max_len / 2);
	let mut end = start + max_len;
	if end > text.len() {
		end = text.len();
	}
	let adj_start = if end - start < max_len {
		end.saturating_sub(max_len)
	} else {
		start
	};

	let s = adj_start.saturating_sub(15);
	let s = text[..s].rfind(char::is_whitespace).unwrap_or(adj_start);
	let e = (end + 15).min(text.len());
	let e = if e < text.len() {
		text[e..].find(char::is_whitespace).map(|p| e + p).unwrap_or(text.len())
	} else {
		text.len()
	};

	let prefix = if s > 0 { "…" } else { "" };
	let suffix = if e < text.len() { "…" } else { "" };
	format!("{prefix}{}{suffix}", &text[s..e])
}
