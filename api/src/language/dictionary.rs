use async_trait::async_trait;
use serde::{Deserialize, Serialize};

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
		Self { providers: Vec::new(), translator: None }
	}

	pub fn register(&mut self, provider: Box<dyn DictionaryProvider>) {
		self.providers.push(provider);
	}

	pub fn set_translator(&mut self, translator: Box<dyn TranslationProvider>) {
		self.translator = Some(translator);
	}

	pub fn find_provider(&self, lang: &str) -> Option<&Box<dyn DictionaryProvider>> {
		for p in &self.providers {
			if p.supports(lang) {
				return Some(p);
			}
		}
		self.providers.iter().find(|p| p.supports("en"))
	}
}

pub struct FreeDictionaryProvider;

#[async_trait]
impl DictionaryProvider for FreeDictionaryProvider {
	fn id(&self) -> &'static str { "freedictionary" }
	fn supports(&self, lang: &str) -> bool { lang == "en" || lang.starts_with("en-") }

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
		struct FreeApiResponse { word: String, phonetic: Option<String>, meanings: Vec<Meaning> }
		#[derive(Deserialize)]
		struct Meaning { part_of_speech: String, definitions: Vec<Def> }
		#[derive(Deserialize)]
		struct Def { definition: String, example: Option<String> }

		let data: Vec<FreeApiResponse> = resp.json().await.unwrap_or_default();
		if data.is_empty() { return Ok(vec![fallback_entry(query)]); }

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

pub struct WiktionaryProvider;

fn clean_html(html: &str) -> String {
	use scraper::Html;
	Html::parse_fragment(html)
		.root_element()
		.text()
		.collect::<Vec<_>>()
		.join(" ")
		.split_whitespace()
		.collect::<Vec<_>>()
		.join(" ")
		.trim()
		.to_string()
}

fn extract_lemma_from_form_of(definition: &str) -> Option<String> {
	let lower = definition.to_lowercase();
	if lower.contains("of") {
		if let Some(idx) = lower.rfind("of ") {
			let after = &lower[idx + 3..];
			let word = after.trim_end_matches('.').trim().to_string();
			if !word.is_empty() && word.len() > 1 {
				return Some(word);
			}
		}
	}
	None
}

fn extract_examples(def: &serde_json::Value) -> Vec<String> {
	let mut examples = Vec::new();
	if let Some(parsed) = def.get("parsedExamples").and_then(|v| v.as_array()) {
		for e in parsed {
			if let Some(text) = e["example"].as_str() {
				let clean = clean_html(text);
				if !clean.is_empty() { examples.push(clean); }
			}
		}
	}

	if examples.is_empty() {
		if let Some(raw) = def.get("examples").and_then(|v| v.as_array()) {
			for e in raw {
				if let Some(text) = e.as_str() {
					let clean = clean_html(text);
					if !clean.is_empty() { examples.push(clean); }
				}
			}
		}
	}
	examples
}

#[async_trait]
impl DictionaryProvider for WiktionaryProvider {
	fn id(&self) -> &'static str { "wiktionary" }
	fn supports(&self, _lang: &str) -> bool { true }

	async fn lookup(&self, query: &DictionaryQuery) -> Result<Vec<DictionaryEntry>, crate::AppError> {
		let url = format!("https://en.wiktionary.org/api/rest_v1/page/definition/{}", urlencoding(&query.word));
		let client = reqwest::Client::builder()
			.user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/150.0.0.0 Safari/537.36")
			.timeout(std::time::Duration::from_secs(8))
			.build()
			.map_err(|e| crate::AppError::Internal(format!("HTTP client: {}", e)))?;

		let resp = match client.get(&url).send().await {
			Ok(r) if r.status().is_success() => r,
			Ok(r) if r.status().as_u16() == 429 => return Err(crate::AppError::Internal("Wiktionary rate limited".into())),
			_ => return Ok(Vec::new()),
		};

		let body: serde_json::Value = match resp.json().await {
			Ok(v) => v,
			Err(_) => return Ok(Vec::new()),
		};

		let mut entries = Vec::new();
		let mut extracted_lemma: Option<String> = None;
		let lang = &query.language[..query.language.len().min(2)];

		for &try_lang in &[lang, "en"] {
			if let Some(defs) = body.get(try_lang).and_then(|v| v.as_array()) {
				for def in defs {
					let pos = def["partOfSpeech"].as_str().unwrap_or("").to_string();
					let pos_opt = if pos.is_empty() { None } else { Some(pos) };
					if let Some(defs_arr) = def["definitions"].as_array() {
						for d in defs_arr {
							let raw = d["definition"].as_str().unwrap_or("");
							if raw.is_empty() { continue; }
							let definition = clean_html(raw);
							if definition.is_empty() { continue; }

							if extracted_lemma.is_none() {
								extracted_lemma = extract_lemma_from_form_of(&definition);
							}

							let examples = extract_examples(d);
							entries.push(DictionaryEntry {
								word: query.word.clone(),
								lemma: query.word.to_lowercase(),
								sense_label: pos_opt.clone(),
								sense_id: None,
								part_of_speech: pos_opt.clone(),
								definition,
								example_sentences: examples,
								pronunciation: None,
								frequency_rank: None,
							});
						}
					}
				}
				if !entries.is_empty() { break; }
			}
		}

		if let Some(base) = extracted_lemma {
			if base != query.word.to_lowercase() {
				let base_q = DictionaryQuery {
					word: base,
					context: query.context.clone(),
					language: query.language.clone(),
				};
				if let Ok(mut base_entries) = self.lookup(&base_q).await {
					if !base_entries.is_empty() {
						for e in &mut base_entries { e.word = query.word.clone(); }
						let mut merged = base_entries;
						merged.extend(entries);
						return Ok(merged);
					}
				}
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
	fn id(&self) -> &'static str { "libretranslate" }
	fn supports(&self, _from: &str, _to: &str) -> bool { true }

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

		if !resp.status().is_success() { return Ok(None); }

		#[derive(Deserialize)]
		struct TranslateResponse { translated_text: String }
		let data: TranslateResponse = resp.json().await.unwrap_or(TranslateResponse { translated_text: String::new() });
		if data.translated_text.is_empty() { Ok(None) } else { Ok(Some(data.translated_text)) }
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
