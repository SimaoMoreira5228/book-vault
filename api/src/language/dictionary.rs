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
	async fn translate(&self, text: &str, from: &str, to: &str) -> Result<Option<String>, crate::AppError>;
}

pub struct FreeDictionaryProvider;

#[async_trait]
impl DictionaryProvider for FreeDictionaryProvider {
	fn id(&self) -> &'static str { "freedictionary" }
	fn supports(&self, lang: &str) -> bool { lang.starts_with("en") }

	async fn lookup(&self, query: &DictionaryQuery) -> Result<Vec<DictionaryEntry>, crate::AppError> {
		let url = format!("https://api.dictionaryapi.dev/api/v2/entries/en/{}", urlencoding(&query.word));
		let client = reqwest::Client::builder()
			.timeout(std::time::Duration::from_secs(5))
			.build()
			.map_err(|e| crate::AppError::Internal(format!("HTTP client: {}", e)))?;

		let resp = client.get(&url).send().await.map_err(|_| {
			crate::AppError::Internal("Dictionary API unreachable".into())
		})?;

		if !resp.status().is_success() {
			return Ok(vec![DictionaryEntry {
				word: query.word.clone(),
				lemma: query.word.to_lowercase(),
				sense_label: None,
				sense_id: None,
				part_of_speech: None,
				definition: "(no definition found)".to_string(),
				example_sentences: vec![query.context.clone()],
				pronunciation: None,
				frequency_rank: None,
			}]);
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

		if entries.is_empty() {
			entries.push(DictionaryEntry {
				word: query.word.clone(),
				lemma: query.word.to_lowercase(),
				sense_label: None,
				sense_id: None,
				part_of_speech: None,
				definition: "(no definition found)".to_string(),
				example_sentences: vec![query.context.clone()],
				pronunciation: None,
				frequency_rank: None,
			});
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

	async fn translate(&self, text: &str, from: &str, to: &str) -> Result<Option<String>, crate::AppError> {
		let url = format!("{}/translate", self.api_url.trim_end_matches('/'));
		let client = reqwest::Client::builder()
			.timeout(std::time::Duration::from_secs(10))
			.build()
			.map_err(|e| crate::AppError::Internal(format!("HTTP client: {}", e)))?;

		let body = serde_json::json!({
			"q": text,
			"source": from,
			"target": to,
		});

		let resp = client.post(&url).json(&body).send().await.map_err(|_| {
			crate::AppError::Internal("Translation API unreachable".into())
		})?;

		if !resp.status().is_success() {
			return Ok(None);
		}

		#[derive(Deserialize)]
		struct TranslateResponse { translated_text: String }
		let data: TranslateResponse = resp.json().await.unwrap_or(TranslateResponse { translated_text: String::new() });
		if data.translated_text.is_empty() { Ok(None) } else { Ok(Some(data.translated_text)) }
	}
}

fn urlencoding(s: &str) -> String {
	s.replace(' ', "%20").replace('&', "%26").replace('?', "%3F")
}
