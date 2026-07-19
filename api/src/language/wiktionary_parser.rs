use parse_wiki_text_2::{Configuration, Node, Parameter};

#[derive(Debug, Clone)]
pub struct ParsedDefinition {
	pub part_of_speech: Option<String>,
	pub sense_label: Option<String>,
	pub definition: String,
	pub examples: Vec<String>,
}

pub struct WiktionaryParser {
	config: Configuration,
}

impl Default for WiktionaryParser {
	fn default() -> Self {
		Self::new()
	}
}

impl WiktionaryParser {
	pub fn new() -> Self {
		Self { config: Configuration::default() }
	}

	pub fn subdomain_for(&self, lang: &str) -> &str {
		match lang {
			"pt" => "pt",
			"it" => "it",
			"es" => "es",
			"fr" => "fr",
			"de" => "de",
			_ => "en",
		}
	}

	pub fn parse(&self, wikitext: &str, lang: &str) -> Vec<ParsedDefinition> {
		let result = match self.config.parse(wikitext) {
			Ok(r) => r,
			Err(_) => return Vec::new(),
		};
		match lang {
			"pt" => self.parse_pt(&result.nodes),
			"it" => self.parse_it(&result.nodes),
			"de" => self.parse_de(&result.nodes),
			"es" => self.parse_es(&result.nodes),
			"fr" => self.parse_fr(&result.nodes),
			_ => Vec::new(),
		}
	}

	pub fn parse_en_wiktionary(&self, wikitext: &str, word_lang: &str) -> Vec<ParsedDefinition> {
		let result = match self.config.parse(wikitext) {
			Ok(r) => r,
			Err(_) => return Vec::new(),
		};
		let lang_name = match word_lang {
			"en" => "English",
			"pt" => "Portuguese",
			"es" => "Spanish",
			"fr" => "French",
			"de" => "German",
			"it" => "Italian",
			_ => "English",
		};

		let mut definitions = Vec::new();
		let mut in_lang_section = false;
		let mut i = 0;
		while i < result.nodes.len() {
			match &result.nodes[i] {
				Node::Heading { level, nodes, .. } if *level == 2 => {
					let text = collect_text(nodes);
					in_lang_section = text == lang_name;
				}
				Node::Heading { level, nodes, .. } if in_lang_section && (*level == 3 || *level == 4) => {
					let text = collect_text(nodes);
					if is_en_pos(&text) {
						if let Some(defs) = self.collect_defs_after(&result.nodes, &mut i) {
							for d in defs {
								definitions.push(ParsedDefinition {
									part_of_speech: Some(text.clone()),
									definition: d.text,
									sense_label: d.sense_label,
									examples: d.examples,
								});
							}
						}
						continue;
					}
				}
				_ => {}
			}
			i += 1;
		}
		definitions
	}

	fn collect_defs_after(&self, nodes: &[Node], i: &mut usize) -> Option<Vec<ExtractedDef>> {
		let start_heading = match &nodes[*i] {
			Node::Heading { level, .. } => *level,
			_ => return None,
		};
		*i += 1;
		let mut defs = Vec::new();
		while *i < nodes.len() {
			if let Node::Heading { level, .. } = &nodes[*i] {
				if *level <= start_heading {
					break;
				}
			}
			if let Node::OrderedList { items, .. } = &nodes[*i] {
				for item in items {
					if let Some(def) = extract_definition_from_item(&item.nodes) {
						defs.push(def);
					}
				}
			}
			*i += 1;
		}
		if defs.is_empty() { None } else { Some(defs) }
	}

	fn parse_pt(&self, nodes: &[Node]) -> Vec<ParsedDefinition> {
		self.parse_standard_l2_pos(nodes, "pt", &["Substantivo", "Verbo", "Adjetivo", "Advérbio", "Preposição", "Conjunção", "Interjeição", "Pronome", "Artigo", "Numeral", "Forma verbal"])
	}

	fn parse_fr(&self, nodes: &[Node]) -> Vec<ParsedDefinition> {
		let mut defs = Vec::new();
		let mut i = 0;
		while i < nodes.len() {
			let pos = match &nodes[i] {
				Node::Heading { level: 3, nodes, .. } => extract_fr_pos(nodes),
				_ => None,
			};
			if let Some(pos_name) = pos {
				i += 1;
				while i < nodes.len() {
					if let Node::Heading { level, .. } = &nodes[i] {
						if *level <= 3 { break; }
					}
					if let Node::OrderedList { items, .. } = &nodes[i] {
						for item in items {
							if let Some(def) = extract_definition_from_item(&item.nodes) {
								defs.push(ParsedDefinition {
									part_of_speech: Some(pos_name.clone()),
									definition: def.text,
									sense_label: def.sense_label,
									examples: def.examples,
								});
							}
						}
					}
					i += 1;
				}
				continue;
			}
			i += 1;
		}
		defs
	}

	fn parse_it(&self, nodes: &[Node]) -> Vec<ParsedDefinition> {
		let mut defs = Vec::new();
		let mut i = 0;
		while i < nodes.len() {
			let pos = match &nodes[i] {
				Node::Template { name, .. } => extract_it_pos(name),
				_ => None,
			};
			if let Some(pos_name) = pos {
				i += 1;
				let mut collected = Vec::new();
				while i < nodes.len() {
					match &nodes[i] {
						Node::Template { name, .. } => {
							if extract_it_pos(name).is_some() { break; }
						}
						Node::Heading { .. } => break,
						Node::OrderedList { items, .. } => {
							for item in items {
								if let Some(def) = extract_definition_from_item(&item.nodes) {
									collected.push(def);
								}
							}
						}
						_ => {}
					}
					i += 1;
				}
				for c in collected {
					defs.push(ParsedDefinition {
						part_of_speech: Some(pos_name.clone()),
						definition: c.text,
						sense_label: c.sense_label,
						examples: c.examples,
					});
				}
				continue;
			}
			i += 1;
		}
		defs
	}

	fn parse_es(&self, nodes: &[Node]) -> Vec<ParsedDefinition> {
		let mut defs = Vec::new();
		let mut i = 0;
		while i < nodes.len() {
			let pos = match &nodes[i] {
				Node::Heading { level, nodes, .. } if *level == 3 || *level == 4 => extract_es_pos(nodes),
				_ => None,
			};
			if let Some(pos_name) = pos {
				i += 1;
				while i < nodes.len() {
					if let Node::Heading { level, .. } = &nodes[i] {
						if *level <= 3 { break; }
					}
					if let Node::DefinitionList { items, .. } = &nodes[i] {
						for item in items {
							if matches!(item.type_, parse_wiki_text_2::DefinitionListItemType::Term) {
								let text = collect_def_text_es(&item.nodes);
								let cleaned = clean_def_text(&text);
								if !cleaned.is_empty() && cleaned != "1" && cleaned != "2" {
									let examples = extract_es_examples(&item.nodes);
									defs.push(ParsedDefinition {
										part_of_speech: Some(pos_name.clone()),
										definition: cleaned,
										sense_label: None,
										examples,
									});
								}
							}
						}
					}
					i += 1;
				}
				continue;
			}
			i += 1;
		}
		defs
	}

	fn parse_de(&self, nodes: &[Node]) -> Vec<ParsedDefinition> {
		let mut defs = Vec::new();
		let mut i = 0;
		while i < nodes.len() {
			let pos = match &nodes[i] {
				Node::Heading { level: 3, nodes, .. } => extract_de_pos(nodes),
				_ => None,
			};
			if let Some(pos_name) = pos {
				i += 1;
				while i < nodes.len() {
					if let Node::Heading { level, .. } = &nodes[i] {
						if *level <= 3 { break; }
					}
					if let Node::DefinitionList { items, .. } = &nodes[i] {
						for item in items {
							if matches!(item.type_, parse_wiki_text_2::DefinitionListItemType::Details) {
								let text = collect_text(&item.nodes);
								let cleaned = clean_de_def_text(&text);
								if !cleaned.is_empty() {
									defs.push(ParsedDefinition {
										part_of_speech: Some(pos_name.clone()),
										definition: cleaned,
										sense_label: None,
										examples: Vec::new(),
									});
								}
							}
						}
					}
					i += 1;
				}
				continue;
			}
			i += 1;
		}
		defs
	}

	fn parse_standard_l2_pos(&self, all_nodes: &[Node], lang: &str, pos_names: &[&str]) -> Vec<ParsedDefinition> {
		let mut definitions = Vec::new();
		let mut i = 0;
		while i < all_nodes.len() {
			let pos = match &all_nodes[i] {
				Node::Heading { level: 2, nodes, .. } => {
					let text = collect_text(nodes);
					if pos_names.iter().any(|n| text.eq_ignore_ascii_case(n)) {
						Some(map_pos_name(lang, &text))
					} else {
						None
					}
				}
				_ => None,
			};
			if let Some(pos_name) = pos {
				if is_skip_section(&pos_name) {
					i += 1;
					continue;
				}
				i += 1;
				let mut collected = Vec::new();
				while i < all_nodes.len() {
					match &all_nodes[i] {
						Node::Heading { level: 2, nodes, .. } => {
							let t = collect_text(nodes);
							if pos_names.iter().any(|n| t.eq_ignore_ascii_case(n)) || is_skip_section(&t) {
								break;
							}
							break;
						}
						Node::Heading { level: 3, .. } => {
							if is_skip_section(&collect_text_from_heading(&all_nodes[i])) {
								let end_pos = all_nodes[i].end();
								i += 1;
								while i < all_nodes.len() && all_nodes[i].start() < end_pos { i += 1; }
								continue;
							}
							break;
						}
						Node::OrderedList { items, .. } => {
							for item in items {
								let text = collect_text(&item.nodes);
								let trimmed = text.trim();
								if trimmed.is_empty() || trimmed.starts_with('*') { continue; }
								let examples = collect_examples(&item.nodes);
								let cleaned = clean_def_text(&text);
								if !cleaned.is_empty() {
									let mapped_pos = map_pos_name(lang, &pos_name);
									collected.push(ParsedDefinition {
										part_of_speech: Some(mapped_pos),
										sense_label: None,
										definition: cleaned,
										examples,
									});
								}
							}
						}
						_ => {}
					}
					i += 1;
				}
				definitions.extend(collected);
				continue;
			}
			i += 1;
		}
		definitions
	}
}

fn collect_text(nodes: &[Node]) -> String {
	let mut s = String::new();
	for node in nodes {
		match node {
			Node::Text { value, .. } => s.push_str(value),
			Node::Link { target, text, .. } => {
				let display = collect_text(text);
				if !display.is_empty() { s.push_str(&display); }
				else { s.push_str(target); }
			}
			Node::CharacterEntity { character, .. } => s.push(*character),
			_ => {}
		}
	}
	s
}

fn collect_text_from_heading(node: &Node) -> String {
	match node {
		Node::Heading { nodes, .. } => collect_text(nodes),
		_ => String::new(),
	}
}

fn map_pos_name(lang: &str, name: &str) -> String {
	let lower = name.to_lowercase().trim().to_string();
	match (lang, lower.as_str()) {
		("pt", "substantivo") => "Noun",
		("pt", "verbo") | ("pt", "forma verbal") => "Verb",
		("pt", "adjetivo") => "Adjective",
		("pt", "advérbio") | ("pt", "adverbio") => "Adverb",
		("pt", "preposição") | ("pt", "preposicao") => "Preposition",
		("pt", "conjunção") | ("pt", "conjuncao") => "Conjunction",
		("pt", "interjeição") | ("pt", "interjeicao") => "Interjection",
		("pt", "pronome") => "Pronoun",
		("pt", "artigo") => "Determiner",
		("pt", "numeral") => "Numeral",
		_ => name,
	}.to_string()
}

fn is_en_pos(text: &str) -> bool {
	let t = text.trim();
	matches!(t, "Noun" | "Verb" | "Adjective" | "Adverb" | "Preposition" | "Conjunction" | "Interjection" | "Pronoun" | "Determiner" | "Numeral")
}

fn is_skip_section(text: &str) -> bool {
	let t = text.trim().to_lowercase();
	matches!(t.as_str(),
		"aumentativos" | "diminutivos" | "tradução" | "ver também"
		| "etimologia" | "no wikcionário" | "na wikipedia" | "no wikiquote"
		| "ligações externas" | "grafia histórica" | "references"
		| "further reading" | "quotations" | "descendants" | "pronunciation"
		| "etymology" | "anagrams" | "derived terms" | "related terms"
		| "see also" | "translations" | "coordinate terms" | "synonyms"
	)
}

fn extract_fr_pos(nodes: &[Node]) -> Option<String> {
	for node in nodes {
		if let Node::Template { name, parameters, .. } = node {
			let name_text = collect_text(name).to_lowercase().trim().to_string();
			if name_text == "s" && parameters.len() >= 2 {
				let pos = collect_text(&parameters[0].value).to_lowercase().trim().to_string();
				return match pos.as_str() {
					"nom" => Some("Noun".to_string()),
					"verbe" => Some("Verb".to_string()),
					"adjectif" => Some("Adjective".to_string()),
					"adverbe" => Some("Adverb".to_string()),
					"interjection" => Some("Interjection".to_string()),
					"pronom" => Some("Pronoun".to_string()),
					"préposition" | "preposition" => Some("Preposition".to_string()),
					"conjonction" => Some("Conjunction".to_string()),
					_ => None,
				};
			}
		}
	}
	None
}

fn extract_it_pos(name_nodes: &[Node]) -> Option<String> {
	let name = collect_text(name_nodes).trim().to_lowercase();
	match name.as_str() {
		"-sost-" | "sostantivo" => Some("Noun".to_string()),
		"-verb-" => Some("Verb".to_string()),
		"-agg-" | "aggettivo" => Some("Adjective".to_string()),
		"-avv-" | "avverbio" => Some("Adverb".to_string()),
		"-pron-" | "pronome" => Some("Pronoun".to_string()),
		"-prep-" | "preposizione" => Some("Preposition".to_string()),
		"-cong-" | "congiunzione" => Some("Conjunction".to_string()),
		"-inter-" | "interiezione" => Some("Interjection".to_string()),
		"-art-" | "articolo" => Some("Determiner".to_string()),
		"-num-" | "numero" => Some("Numeral".to_string()),
		_ => None,
	}
}

fn collect_def_text_es(nodes: &[Node]) -> String {
	let mut s = String::new();
	for node in nodes {
		match node {
			Node::Text { value, .. } => s.push_str(value),
			Node::Link { target, text, .. } => {
				let display = collect_text(text);
				if !display.is_empty() { s.push_str(&display); }
				else { s.push_str(target); }
			}
			Node::CharacterEntity { character, .. } => s.push(*character),
			Node::Template { name, parameters, .. } => {
				let t = collect_text(name).to_lowercase().trim().to_string();
				if t == "impropia" || t == "plm" || t == "csem" || t == "l" || t == "etimología" || t == "uso" {
					if let Some(val) = param_value(parameters, 0) {
						s.push_str(&val);
					}
				}
			}
			_ => {}
		}
	}
	s
}

fn extract_es_pos(nodes: &[Node]) -> Option<String> {
	for node in nodes {
		if let Node::Template { name, .. } = node {
			let name_text = collect_text(name).to_lowercase().trim().to_string();
			if name_text == "sustantivo" || name_text == "nombre" || name_text.contains("sustantivo") {
				return Some("Noun".to_string());
			}
			if name_text == "verbo" || name_text.contains("verbo") {
				return Some("Verb".to_string());
			}
			if name_text == "adjetivo" {
				return Some("Adjective".to_string());
			}
			if name_text == "adverbio" {
				return Some("Adverb".to_string());
			}
			if name_text == "preposición" || name_text == "preposicion" {
				return Some("Preposition".to_string());
			}
			if name_text == "conjunción" || name_text == "conjuncion" {
				return Some("Conjunction".to_string());
			}
			if name_text == "interjección" || name_text == "interjeccion" {
				return Some("Interjection".to_string());
			}
			if name_text == "pronombre" {
				return Some("Pronoun".to_string());
			}
		}
	}

	let text = collect_text(nodes).to_lowercase();
	if text.contains("sustantivo") { Some("Noun".to_string()) }
	else { None }
}

fn extract_de_pos(nodes: &[Node]) -> Option<String> {
	for node in nodes {
		if let Node::Template { name, parameters, .. } = node {
			let name_text = collect_text(name).to_lowercase().trim().to_string();
			if name_text == "wortart" {
				if let Some(pos) = param_value(parameters, 0) {
					return match pos.to_lowercase().as_str() {
						"substantiv" => Some("Noun".to_string()),
						"verb" => Some("Verb".to_string()),
						"adjektiv" => Some("Adjective".to_string()),
						"adverb" => Some("Adverb".to_string()),
						"präposition" | "preposition" => Some("Preposition".to_string()),
						"konjunktion" => Some("Conjunction".to_string()),
						"interjektion" => Some("Interjection".to_string()),
						"pronomen" => Some("Pronoun".to_string()),
						_ => None,
					};
				}
			}
		}
	}
	None
}

fn clean_def_text(text: &str) -> String {
	text.split_whitespace()
		.collect::<Vec<_>>()
		.join(" ")
		.trim_matches(|c: char| c == ',' || c == ';' || c == ':' || c == '.')
		.trim()
		.to_string()
}

fn clean_de_def_text(text: &str) -> String {
	let text = text.trim();
	if let Some(rest) = text.strip_prefix('[') {
		if let Some(end) = rest.find(']') {
			let after = rest[end + 1..].trim();
			return clean_def_text(after);
		}
	}
	clean_def_text(text)
}

fn collect_examples(nodes: &[Node]) -> Vec<String> {
	let mut examples = Vec::new();
	for node in nodes {
		if let Node::Template { name, parameters, .. } = node {
			let tpl = collect_text(name).to_lowercase().trim().to_string();
			match tpl.as_str() {
				"quote-book" | "quote-journal" | "quote-web" | "quote-song" | "quote-text" | "ux" | "example" | "exemple" | "ejemplo" => {
					if let Some(p) = named_param(parameters, "passage")
						.or_else(|| named_param(parameters, "text"))
						.or_else(|| param_value(parameters, 0))
					{
						examples.push(p);
					}
				}
				_ => {}
			}
		}
	}
	examples
}

fn extract_es_examples(nodes: &[Node]) -> Vec<String> {
	let mut examples = Vec::new();
	for node in nodes {
		if let Node::Template { name, parameters, .. } = node {
			let tpl = collect_text(name).to_lowercase().trim().to_string();
			if tpl.as_str() == "ejemplo" {
				if let Some(p) = param_value(parameters, 0) {
					examples.push(p);
				}
			}
		}
	}
	examples
}

fn extract_definition_from_item(nodes: &[Node]) -> Option<ExtractedDef> {
	if nodes.is_empty() { return None; }
	let mut text_parts = Vec::new();
	let mut sense_label: Option<String> = None;
	let mut examples = Vec::new();

	for node in nodes {
		match node {
			Node::Template { name, parameters, .. } => {
				let tpl = collect_text(name).to_lowercase().trim().to_string();
				match tpl.as_str() {
					"gloss" | "gl" => {
						if let Some(val) = param_value(parameters, 0) { sense_label = Some(val); }
					}
					"lb" => {
						if parameters.len() >= 2 {
							if let Some(label) = param_value(parameters, 1) { sense_label = Some(label); }
						}
					}
					"m" | "mention" => {
						if let Some(val) = param_value(parameters, 0) { text_parts.push(val); }
					}
					"q" | "qualifier" => {
						if let Some(val) = param_value(parameters, 0) { text_parts.push(format!("({})", val)); }
					}
					"quote-book" | "quote-journal" | "quote-web" | "quote-song" | "quote-text" | "ux" | "example" | "exemple" | "ejemplo" => {
						if let Some(p) = named_param(parameters, "passage")
							.or_else(|| named_param(parameters, "text"))
							.or_else(|| param_value(parameters, 0))
						{ examples.push(p); }
					}
					_ => {
						if let Some(val) = param_value(parameters, 0) {
							text_parts.push(val);
						}
					}
				}
			}
			Node::Link { target, text, .. } => {
				let display = collect_text(text);
				if !display.is_empty() { text_parts.push(display); }
				else { text_parts.push(target.to_string()); }
			}
			Node::Text { value, .. } => {
				let v = value.trim();
				if !v.is_empty() { text_parts.push(v.to_string()); }
			}
			Node::CharacterEntity { character, .. } => { text_parts.push(character.to_string()); }
			_ => {}
		}
	}

	let text = text_parts.join(" ");
	let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
	if text.is_empty() { return None; }
	Some(ExtractedDef { text, sense_label, examples })
}

fn param_value(parameters: &[Parameter], index: usize) -> Option<String> {
	parameters.get(index).map(|p| collect_text(&p.value))
}

fn named_param(parameters: &[Parameter], name: &str) -> Option<String> {
	for p in parameters {
		if let Some(ref pname) = p.name {
			if collect_text(pname).trim().to_lowercase() == name {
				return Some(collect_text(&p.value));
			}
		}
	}
	None
}

struct ExtractedDef {
	text: String,
	sense_label: Option<String>,
	examples: Vec<String>,
}

use parse_wiki_text_2::Positioned;

#[cfg(test)]
mod tests {
	use super::*;

	macro_rules! fixture {
		($path:expr) => { include_str!(concat!("../../test_data/", $path)) };
	}

	macro_rules! check_len {
		($defs:expr, $min:expr) => {
			assert!($defs.len() >= $min, "{}: {} defs, expected >= {}", stringify!($defs), $defs.len(), $min);
		};
	}

	macro_rules! assert_has_pos {
		($defs:expr, $pos:expr) => {
			assert!($defs.iter().any(|d| d.part_of_speech.as_deref() == Some($pos)), "expected POS '{}', got {:?}", $pos, $defs.iter().map(|d| &d.part_of_speech).collect::<Vec<_>>());
		};
	}

	macro_rules! assert_has_def {
		($defs:expr, $needle:expr) => {
			assert!($defs.iter().any(|d| d.definition.to_lowercase().contains(&$needle.to_lowercase())), "expected '{}'", $needle);
		};
	}

	#[test]
	fn en_heart() {
		let parser = WiktionaryParser::new();
		let defs = parser.parse_en_wiktionary(fixture!("en/heart.txt"), "en");
		check_len!(defs, 3);
		assert_has_pos!(defs, "Noun");
		assert_has_def!(defs, "muscular");
	}

	#[test]
	fn en_run() {
		let parser = WiktionaryParser::new();
		let defs = parser.parse_en_wiktionary(fixture!("en/run.txt"), "en");
		check_len!(defs, 10);
		assert_has_pos!(defs, "Verb");
	}

	#[test]
	fn pt_coracao_immersion() {
		let parser = WiktionaryParser::new();
		let defs = parser.parse(fixture!("pt/coração.txt"), "pt");
		check_len!(defs, 3);
		assert_has_pos!(defs, "Noun");
		assert_has_def!(defs, "órgão");
	}

	#[test]
	fn pt_banco_immersion() {
		let parser = WiktionaryParser::new();
		let defs = parser.parse(fixture!("pt/banco.txt"), "pt");
		check_len!(defs, 5);
		assert_has_pos!(defs, "Noun");
		assert_has_def!(defs, "instituição");
	}

	#[test]
	fn pt_cantar_immersion() {
		let parser = WiktionaryParser::new();
		let defs = parser.parse(fixture!("pt/cantar.txt"), "pt");
		check_len!(defs, 2);
		assert_has_pos!(defs, "Verb");
	}

	#[test]
	fn pt_coracao_gloss() {
		let parser = WiktionaryParser::new();
		let defs = parser.parse_en_wiktionary(fixture!("en/coração.txt"), "pt");
		check_len!(defs, 3);
		assert_has_pos!(defs, "Noun");
		assert_has_def!(defs, "heart");
	}

	#[test]
	fn pt_banco_gloss() {
		let parser = WiktionaryParser::new();
		let defs = parser.parse_en_wiktionary(fixture!("en/banco.txt"), "pt");
		check_len!(defs, 3);
		assert_has_pos!(defs, "Noun");
		assert_has_def!(defs, "bank");
	}

	#[test]
	fn pt_empurro_immersion() {
		let parser = WiktionaryParser::new();
		let defs = parser.parse(fixture!("pt/empurro.txt"), "pt");
		check_len!(defs, 1);
		assert_has_pos!(defs, "Verb");
		assert_has_def!(defs, "primeira");
	}

	#[test]
	fn it_buongiorno_immersion() {
		let parser = WiktionaryParser::new();
		let defs = parser.parse(fixture!("it/buongiorno.txt"), "it");
		check_len!(defs, 1);
		assert_has_pos!(defs, "Noun");
		assert_has_def!(defs, "saluto");
	}

	#[test]
	fn it_banca_immersion() {
		let parser = WiktionaryParser::new();
		let defs = parser.parse(fixture!("it/banca.txt"), "it");
		check_len!(defs, 2);
		assert_has_pos!(defs, "Noun");
	}

	#[test]
	fn it_cuore_immersion() {
		let parser = WiktionaryParser::new();
		let defs = parser.parse(fixture!("it/cuore.txt"), "it");
		check_len!(defs, 2);
		assert_has_pos!(defs, "Noun");
	}

	#[test]
	fn it_parlare_immersion() {
		let parser = WiktionaryParser::new();
		let defs = parser.parse(fixture!("it/parlare.txt"), "it");
		check_len!(defs, 1);
		assert_has_pos!(defs, "Verb");
		assert_has_def!(defs, "parole");
	}

	#[test]
	fn it_cuore_gloss() {
		let parser = WiktionaryParser::new();
		let defs = parser.parse_en_wiktionary(fixture!("en/cuore.txt"), "it");
		check_len!(defs, 3);
		assert_has_pos!(defs, "Noun");
		assert_has_def!(defs, "heart");
	}

	#[test]
	fn de_wasser_immersion() {
		let parser = WiktionaryParser::new();
		let defs = parser.parse(fixture!("de/Wasser.txt"), "de");
		check_len!(defs, 3);
		assert_has_pos!(defs, "Noun");
		assert_has_def!(defs, "Flüssigkeit");
	}

	#[test]
	fn de_bank_immersion() {
		let parser = WiktionaryParser::new();
		let defs = parser.parse(fixture!("de/Bank.txt"), "de");
		check_len!(defs, 1);
		assert_has_pos!(defs, "Noun");
	}

	#[test]
	fn de_haus_immersion() {
		let parser = WiktionaryParser::new();
		let defs = parser.parse(fixture!("de/Haus.txt"), "de");
		check_len!(defs, 2);
		assert_has_pos!(defs, "Noun");
		assert_has_def!(defs, "Gebäude");
	}

	#[test]
	fn de_gehen_immersion() {
		let parser = WiktionaryParser::new();
		let defs = parser.parse(fixture!("de/gehen.txt"), "de");
		check_len!(defs, 2);
		assert_has_pos!(defs, "Verb");
	}

	#[test]
	fn de_wasser_gloss() {
		let parser = WiktionaryParser::new();
		let defs = parser.parse_en_wiktionary(fixture!("en/Wasser.txt"), "de");
		check_len!(defs, 2);
		assert_has_pos!(defs, "Noun");
		assert_has_def!(defs, "water");
	}

	#[test]
	fn es_banco_immersion() {
		let parser = WiktionaryParser::new();
		let defs = parser.parse(fixture!("es/banco.txt"), "es");
		check_len!(defs, 3);
		assert_has_pos!(defs, "Noun");
	}

	#[test]
	fn es_hola_immersion() {
		let parser = WiktionaryParser::new();
		let defs = parser.parse(fixture!("es/hola.txt"), "es");
		check_len!(defs, 1);
		assert_has_def!(defs, "saludo");
	}

	#[test]
	fn es_hablar_immersion() {
		let parser = WiktionaryParser::new();
		let defs = parser.parse(fixture!("es/hablar.txt"), "es");
		check_len!(defs, 3);
		assert_has_pos!(defs, "Verb");
		assert_has_def!(defs, "expresar");
	}

	#[test]
	fn es_casa_immersion() {
		let parser = WiktionaryParser::new();
		let defs = parser.parse(fixture!("es/casa.txt"), "es");
		check_len!(defs, 2);
		assert_has_pos!(defs, "Noun");
	}

	#[test]
	fn fr_banque_immersion() {
		let parser = WiktionaryParser::new();
		let defs = parser.parse(fixture!("fr/banque.txt"), "fr");
		check_len!(defs, 3);
		assert_has_pos!(defs, "Noun");
		assert_has_def!(defs, "argent");
	}

	#[test]
	fn fr_bonjour_immersion() {
		let parser = WiktionaryParser::new();
		let defs = parser.parse(fixture!("fr/bonjour.txt"), "fr");
		check_len!(defs, 1);
		assert_has_def!(defs, "salutation");
	}

	#[test]
	fn fr_maison_immersion() {
		let parser = WiktionaryParser::new();
		let defs = parser.parse(fixture!("fr/maison.txt"), "fr");
		check_len!(defs, 3);
		assert_has_pos!(defs, "Noun");
		assert_has_def!(defs, "bâtiment");
	}

	#[test]
	fn fr_manger_immersion() {
		let parser = WiktionaryParser::new();
		let defs = parser.parse(fixture!("fr/manger.txt"), "fr");
		check_len!(defs, 1);
		assert_has_pos!(defs, "Verb");
	}

	#[test]
	fn fr_maison_gloss() {
		let parser = WiktionaryParser::new();
		let defs = parser.parse_en_wiktionary(fixture!("en/maison.txt"), "fr");
		check_len!(defs, 2);
		assert_has_pos!(defs, "Noun");
		assert_has_def!(defs, "house");
	}
}
