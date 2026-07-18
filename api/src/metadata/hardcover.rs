use async_trait::async_trait;

use crate::AppError;
use crate::metadata::{MetadataProvider, MetadataQuery, ProspectiveMetadata};

pub struct HardcoverProvider {
	client: reqwest::Client,
	api_key: String,
}

impl HardcoverProvider {
	pub fn new(api_key: &str) -> Self {
		Self {
			client: reqwest::Client::builder()
				.user_agent("BookVault/0.1")
				.build()
				.expect("reqwest client"),
			api_key: api_key.to_string(),
		}
	}

	async fn graphql(&self, query: &str) -> Result<serde_json::Value, AppError> {
		let resp = self
			.client
			.post("https://api.hardcover.app/v1/graphql")
			.header("Authorization", format!("Bearer {}", self.api_key))
			.header("Content-Type", "application/json")
			.json(&serde_json::json!({ "query": query }))
			.send()
			.await
			.map_err(|e| AppError::Internal(format!("Hardcover request failed: {e}")))?;

		if !resp.status().is_success() {
			let status = resp.status();
			let body = resp.text().await.unwrap_or_default();
			return Err(AppError::Internal(format!("Hardcover HTTP {status}: {body}")));
		}

		resp.json()
			.await
			.map_err(|e| AppError::Internal(format!("Hardcover JSON parse: {e}")))
	}

	fn convert(data: &serde_json::Value) -> Option<ProspectiveMetadata> {
		let title = data["title"].as_str()?;
		let cover_url = data["cover"]["url"].as_str().map(|s| s.to_string());
		let publisher = data["publisher"].as_str().map(|s| s.to_string());
		let page_count = data["pages"].as_i64().map(|c| c as i32);

		let published_date = data["publication_date"]
			.as_str()
			.map(|s| if s.len() >= 10 { s[..10].to_string() } else { s.to_string() });

		let rating = data["ratings"]["average"].as_f64().map(|r| r as f32);

		let description = data["description"].as_str().map(strip_html);

		let authors: Vec<String> = data["authors"]
			.as_array()
			.map(|arr| arr.iter().filter_map(|a| a["name"].as_str().map(|s| s.to_string())).collect())
			.unwrap_or_default();

		let provider_id = data["id"].as_u64().map(|id| id.to_string()).unwrap_or_default();

		Some(ProspectiveMetadata {
			provider: "hardcover".to_string(),
			provider_id,
			title: Some(title.to_string()),
			authors,
			description,
			isbn10: None,
			isbn13: None,
			page_count,
			genres: Vec::new(),
			cover_url,
			published_date,
			publisher,
			rating,
			subtitle: data["subtitle"].as_str().map(|s| s.to_string()),
		})
	}
}

#[async_trait]
impl MetadataProvider for HardcoverProvider {
	fn id(&self) -> &'static str {
		"hardcover"
	}

	async fn search(&self, query: &MetadataQuery) -> Result<Vec<ProspectiveMetadata>, AppError> {
		if self.api_key.is_empty() {
			return Ok(Vec::new());
		}

		let search_term = if let Some(ref isbn) = query.isbn {
			isbn.clone()
		} else {
			let mut parts = Vec::new();
			if let Some(ref title) = query.title {
				parts.push(title.clone());
			}
			if let Some(ref author) = query.author {
				parts.push(author.clone());
			}
			parts.join(" ")
		};

		let search_query = format!(
			r#"{{
				books(
					limit: 5,
					where: {{ title: {{ _ilike: "%{search_term}%" }} }}
				) {{
					id title slug subtitle description language {{ code }}
					authors {{ name }}
					ratings {{ average }}
					pages publication_date publisher
					cover {{ url }}
				}}
			}}"#,
		);

		let result = self.graphql(&search_query).await?;
		let books = result["data"]["books"].as_array().map(|a| a.to_vec()).unwrap_or_default();

		if books.is_empty() {
			return Ok(Vec::new());
		}

		let mut metadata = Vec::new();
		for book in &books {
			if let Some(meta) = Self::convert(book) {
				metadata.push(meta);
				if metadata.len() >= 3 {
					break;
				}
			}
		}

		Ok(metadata)
	}
}

fn strip_html(text: &str) -> String {
	let mut out = String::new();
	let mut in_tag = false;
	for ch in text.chars() {
		match ch {
			'<' => in_tag = true,
			'>' if in_tag => in_tag = false,
			_ if !in_tag => out.push(ch),
			_ => {}
		}
	}
	out.trim().to_string()
}
