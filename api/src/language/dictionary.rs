use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::language::wiktionary_parser::ParsedDefinition;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DictionaryEntry {
	pub word: String,
	pub lemma: String,
	pub sense_label: Option<String>,
	pub sense_id: Option<String>,
	pub part_of_speech: Option<String>,
	pub definition: String,
	pub example_sentences: Vec<String>,
	pub pronunciation: Option<String>,
	pub frequency_rank: Option<u32>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DictionaryQuery {
	pub word: String,
	pub context: String,
	pub language: String,
	pub definition_language: String,
}

#[async_trait]
pub trait DictionaryProvider: Send + Sync {
	fn id(&self) -> &'static str;
	fn supports(&self, lang: &str) -> bool;
	async fn lookup(&self, query: &DictionaryQuery) -> Result<Vec<DictionaryEntry>, crate::AppError>;
}

#[async_trait]
pub trait TranslationProvider: Send + Sync {
	fn id(&self) -> &'static str;
	fn supports(&self, from: &str, to: &str) -> bool;
	async fn translate(&self, text: &str, from: &str, to: &str) -> Result<Option<String>, crate::AppError>;
}

pub struct DictionaryService {
	pub providers: Vec<Box<dyn DictionaryProvider>>,
	pub translator: Option<Box<dyn TranslationProvider>>,
}

impl Default for DictionaryService {
	fn default() -> Self {
		Self::new()
	}
}

impl DictionaryService {
	pub fn new() -> Self {
		Self {
			providers: Vec::new(),
			translator: None,
		}
	}

	pub fn register(&mut self, provider: Box<dyn DictionaryProvider>) {
		self.providers.push(provider);
	}

	pub fn set_translator(&mut self, translator: Box<dyn TranslationProvider>) {
		self.translator = Some(translator);
	}

	pub fn find_provider(&self, lang: &str) -> Option<&dyn DictionaryProvider> {
		for p in &self.providers {
			if p.supports(lang) {
				return Some(&**p);
			}
		}
		self.providers.iter().find(|p| p.supports("en")).map(|p| &**p)
	}
}

pub struct FreeDictionaryProvider;

#[async_trait]
impl DictionaryProvider for FreeDictionaryProvider {
	fn id(&self) -> &'static str {
		"freedictionary"
	}
	fn supports(&self, lang: &str) -> bool {
		lang == "en" || lang.starts_with("en-")
	}

	async fn lookup(&self, query: &DictionaryQuery) -> Result<Vec<DictionaryEntry>, crate::AppError> {
		let url = format!("https://api.dictionaryapi.dev/api/v2/entries/en/{}", urlencoding(&query.word));
		let client = reqwest::Client::builder()
			.timeout(std::time::Duration::from_secs(5))
			.build()
			.map_err(|e| crate::AppError::Internal(format!("HTTP client: {}", e)))?;

		let resp = match client.get(&url).send().await {
			Ok(r) => r,
			Err(_) => return Ok(vec![fallback_entry(query)]),
		};

		if !resp.status().is_success() {
			return Ok(vec![fallback_entry(query)]);
		}

		#[derive(Deserialize)]
		struct FreeApiResponse {
			word: String,
			phonetic: Option<String>,
			meanings: Vec<Meaning>,
		}
		#[derive(Deserialize)]
		struct Meaning {
			part_of_speech: String,
			definitions: Vec<Def>,
		}
		#[derive(Deserialize)]
		struct Def {
			definition: String,
			example: Option<String>,
		}

		let data: Vec<FreeApiResponse> = resp.json().await.unwrap_or_default();
		if data.is_empty() {
			return Ok(vec![fallback_entry(query)]);
		}

		let mut entries = Vec::new();
		for entry in &data {
			for meaning in &entry.meanings {
				for (i, defn) in meaning.definitions.iter().enumerate() {
					entries.push(DictionaryEntry {
						word: query.word.clone(),
						lemma: entry.word.to_lowercase(),
						sense_label: Some(format!("{} #{}", meaning.part_of_speech, i + 1)),
						sense_id: None,
						part_of_speech: Some(meaning.part_of_speech.clone()),
						definition: defn.definition.clone(),
						example_sentences: defn.example.clone().map(|e| vec![e]).unwrap_or_default(),
						pronunciation: entry.phonetic.clone(),
						frequency_rank: None,
					});
				}
			}
		}
		Ok(entries)
	}
}

pub struct WiktionaryProvider {
	parser: crate::language::wiktionary_parser::WiktionaryParser,
}

impl Default for WiktionaryProvider {
	fn default() -> Self {
		Self::new()
	}
}

impl WiktionaryProvider {
	pub fn new() -> Self {
		Self {
			parser: crate::language::wiktionary_parser::WiktionaryParser::new(),
		}
	}

	async fn fetch_wikitext(subdomain: &str, word: &str) -> Result<String, crate::AppError> {
		let url = format!(
			"https://{}.wiktionary.org/w/rest.php/v1/page/{}",
			subdomain,
			Self::urlencode(word)
		);
		let client = reqwest::Client::builder()
			.user_agent(
				"Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/150.0.0.0 Safari/537.36",
			)
			.default_headers({
				let mut h = reqwest::header::HeaderMap::new();
				h.insert("Accept", "application/json".parse().unwrap());
				h.insert("Accept-Encoding", "gzip, deflate, br".parse().unwrap());
				h
			})
			.timeout(std::time::Duration::from_secs(8))
			.build()
			.map_err(|e| crate::AppError::Internal(format!("HTTP client: {}", e)))?;

		let resp = match client.get(&url).send().await {
			Ok(r) if r.status().is_success() => r,
			Ok(r) if r.status().as_u16() == 429 => {
				return Err(crate::AppError::Internal("Wiktionary rate limited".into()));
			}
			_ => return Err(crate::AppError::NotFound("Wiktionary page not found".into())),
		};

		let body: serde_json::Value = match resp.json().await {
			Ok(v) => v,
			Err(_) => return Err(crate::AppError::Internal("Failed to parse Wiktionary response".into())),
		};

		body["source"]
			.as_str()
			.map(|s| s.to_string())
			.ok_or_else(|| crate::AppError::Internal("No 'source' field in Wiktionary response".into()))
	}

		fn urlencode(s: &str) -> String {
		s.replace(' ', "%20").replace('&', "%26").replace('?', "%3F")
	}

	fn extract_base_lemma(definitions: &[ParsedDefinition]) -> Option<String> {
		for d in definitions {
			let lower = d.definition.to_lowercase();
			if let Some(idx) = lower.rfind("of ") {
				let after = &lower[idx + 3..];
				let word = after.trim_end_matches('.').trim().to_string();
				if !word.is_empty() && word.len() > 1 && !word.contains(' ') {
					return Some(word);
				}
			}
			if let Some(idx) = lower.rfind("de ") {
				let after = &lower[idx + 3..];
				let word = after.trim_end_matches('.').trim().to_string();
				if !word.is_empty() && word.len() > 1 && !word.contains(' ') && !lower.contains("de uma") && !lower.contains("de um") {
					let is_form_of = lower.contains("pessoa") || lower.contains("singular") || lower.contains("plural")
						|| lower.contains("presente") || lower.contains("pretérito") || lower.contains("indicativo")
						|| lower.contains("conjuntivo") || lower.contains("imperativo") || lower.contains("infinitivo")
						|| lower.contains("gerúndio") || lower.contains("particípio");
					if is_form_of {
						return Some(word);
					}
				}
			}
		}
		None
	}
}

#[async_trait]
impl DictionaryProvider for WiktionaryProvider {
	fn id(&self) -> &'static str {
		"wiktionary"
	}
	fn supports(&self, _lang: &str) -> bool {
		true
	}

	async fn lookup(&self, query: &DictionaryQuery) -> Result<Vec<DictionaryEntry>, crate::AppError> {
		let word_lang = &query.language[..query.language.len().min(2)];
		let def_lang = &query.definition_language[..query.definition_language.len().min(2)];

		let is_immersion = word_lang == def_lang;

		let raw_defs = if is_immersion {
			let subdomain = self.parser.subdomain_for(word_lang);
			match Self::fetch_wikitext(subdomain, &query.word).await {
				Ok(wikitext) => self.parser.parse(&wikitext, word_lang),
				Err(_) => {
					if let Ok(en_wikitext) = Self::fetch_wikitext("en", &query.word).await {
						self.parser.parse_en_wiktionary(&en_wikitext, word_lang)
					} else {
						Vec::new()
					}
				}
			}
		} else {
			match Self::fetch_wikitext("en", &query.word).await {
				Ok(wikitext) => self.parser.parse_en_wiktionary(&wikitext, word_lang),
				Err(_) => Vec::new(),
			}
		};

		let mut entries: Vec<DictionaryEntry> = Vec::new();
		if let Some(base_lemma) = Self::extract_base_lemma(&raw_defs) {
			if base_lemma != query.word.to_lowercase() {
				let base_query = DictionaryQuery {
					word: base_lemma.clone(),
					context: query.context.clone(),
					language: query.language.clone(),
					definition_language: query.definition_language.clone(),
				};
				if let Ok(base_entries) = self.lookup(&base_query).await {
					for mut e in base_entries {
						e.word = query.word.clone();
						entries.push(e);
					}
				}
			}
		}

		if entries.is_empty() {
			for d in raw_defs {
				entries.push(DictionaryEntry {
					word: query.word.clone(),
					lemma: query.word.to_lowercase(),
					sense_label: d.sense_label,
					sense_id: None,
					part_of_speech: d.part_of_speech,
					definition: d.definition,
					example_sentences: d.examples,
					pronunciation: None,
					frequency_rank: None,
				});
			}
		}

		Ok(entries)
	}
}

pub struct LibreTranslateProvider {
	pub api_url: String,
}

#[async_trait]
impl TranslationProvider for LibreTranslateProvider {
	fn id(&self) -> &'static str {
		"libretranslate"
	}
	fn supports(&self, _from: &str, _to: &str) -> bool {
		true
	}

	async fn translate(&self, text: &str, from: &str, to: &str) -> Result<Option<String>, crate::AppError> {
		let url = format!("{}/translate", self.api_url.trim_end_matches('/'));
		let client = reqwest::Client::builder()
			.timeout(std::time::Duration::from_secs(10))
			.build()
			.map_err(|e| crate::AppError::Internal(format!("HTTP client: {}", e)))?;

		let body = serde_json::json!({ "q": text, "source": from, "target": to });
		let resp = match client.post(&url).json(&body).send().await {
			Ok(r) => r,
			Err(_) => return Ok(None),
		};

		if !resp.status().is_success() {
			return Ok(None);
		}

		#[derive(Deserialize)]
		struct TranslateResponse {
			translated_text: String,
		}
		let data: TranslateResponse = resp.json().await.unwrap_or(TranslateResponse {
			translated_text: String::new(),
		});
		if data.translated_text.is_empty() {
			Ok(None)
		} else {
			Ok(Some(data.translated_text))
		}
	}
}

fn fallback_entry(query: &DictionaryQuery) -> DictionaryEntry {
	DictionaryEntry {
		word: query.word.clone(),
		lemma: query.word.to_lowercase(),
		sense_label: None,
		sense_id: None,
		part_of_speech: None,
		definition: format!("(no definition found for \"{}\" in {})", query.word, query.language),
		example_sentences: vec![query.context.clone()],
		pronunciation: None,
		frequency_rank: None,
	}
}

fn urlencoding(s: &str) -> String {
	s.replace(' ', "%20").replace('&', "%26").replace('?', "%3F")
}
