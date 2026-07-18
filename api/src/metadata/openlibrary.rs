use async_trait::async_trait;
use serde::Deserialize;

use crate::AppError;
use crate::metadata::{MetadataProvider, MetadataQuery, ProspectiveMetadata};

pub struct OpenLibraryProvider {
	client: reqwest::Client,
}

impl Default for OpenLibraryProvider {
	fn default() -> Self {
		Self::new()
	}
}

impl OpenLibraryProvider {
	pub fn new() -> Self {
		Self {
			client: reqwest::Client::builder()
				.user_agent("BookVault/0.1")
				.build()
				.expect("Failed to build reqwest client"),
		}
	}
}

#[async_trait]
impl MetadataProvider for OpenLibraryProvider {
	fn id(&self) -> &'static str {
		"openlibrary"
	}

	async fn search(&self, query: &MetadataQuery) -> Result<Vec<ProspectiveMetadata>, AppError> {
		let url = build_search_url(query);
		let resp = self
			.client
			.get(&url)
			.send()
			.await
			.map_err(|e| AppError::Internal(format!("OpenLibrary search failed: {e}")))?;

		let body: OpenLibrarySearchResponse = resp
			.json()
			.await
			.map_err(|e| AppError::Internal(format!("OpenLibrary parse failed: {e}")))?;

		let mut results = Vec::new();
		for doc in body.docs.into_iter().take(10) {
			let provider_id = doc.key.trim_start_matches("/works/").to_string();

			let desc = if !provider_id.is_empty() {
				fetch_description(&self.client, &provider_id).await
			} else {
				None
			};

			let cover_url = doc
				.cover_i
				.map(|id| format!("https://covers.openlibrary.org/b/id/{id}-L.jpg"));

			let (isbn10, isbn13) = extract_isbns(doc.isbn.as_ref());

			results.push(ProspectiveMetadata {
				provider: "openlibrary".to_string(),
				provider_id,
				title: Some(doc.title),
				authors: doc.author_name.unwrap_or_default(),
				description: desc,
				isbn10,
				isbn13,
				page_count: doc.number_of_pages_median,
				genres: doc.subject.unwrap_or_default(),
				cover_url,
				published_date: doc.first_publish_year.map(|y| y.to_string()),
				publisher: doc.publisher.and_then(|p| p.into_iter().next()),
				rating: None,
				subtitle: doc.subtitle,
			});
		}

		Ok(results)
	}
}

fn build_search_url(query: &MetadataQuery) -> String {
	let mut params = Vec::new();
	if let Some(ref isbn) = query.isbn {
		let cleaned: String = isbn.chars().filter(|c| c.is_ascii_digit()).collect();
		params.push(format!("isbn={cleaned}"));
	} else {
		if let Some(ref title) = query.title {
			params.push(format!("title={}", urlencode(title)));
		}
		if let Some(ref author) = query.author {
			params.push(format!("author={}", urlencode(author)));
		}
		params.push("limit=10".to_string());
	}
	format!("https://openlibrary.org/search.json?{}", params.join("&"))
}

fn urlencode(s: &str) -> String {
	urlencoding(s)
}

fn urlencoding(s: &str) -> String {
	s.replace(' ', "+")
}

async fn fetch_description(client: &reqwest::Client, work_id: &str) -> Option<String> {
	let url = format!("https://openlibrary.org/works/{work_id}.json");
	let resp = client.get(&url).send().await.ok()?;
	let body: serde_json::Value = resp.json().await.ok()?;

	let desc = body.get("description")?;
	match desc {
		serde_json::Value::String(s) => Some(s.clone()),
		serde_json::Value::Object(m) => m.get("value").and_then(|v| v.as_str()).map(|s| s.to_string()),
		_ => None,
	}
}

fn extract_isbns(isbns: Option<&serde_json::Value>) -> (Option<String>, Option<String>) {
	let values: Vec<String> = match isbns {
		Some(serde_json::Value::String(s)) => s.split(',').map(|s| s.trim().trim_matches('"').to_string()).collect(),
		Some(serde_json::Value::Array(arr)) => arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect(),
		_ => return (None, None),
	};

	let mut isbn10 = None;
	let mut isbn13 = None;

	for item in &values {
		let clean: String = item.chars().filter(|c| c.is_ascii_digit()).collect();
		if clean.len() == 13 {
			isbn13 = Some(clean);
		} else if clean.len() == 10 {
			isbn10 = Some(clean);
		}
	}

	(isbn10, isbn13)
}

#[derive(Debug, Deserialize)]
struct OpenLibrarySearchResponse {
	docs: Vec<OpenLibraryDoc>,
}

#[derive(Debug, Deserialize)]
struct OpenLibraryDoc {
	key: String,
	title: String,
	subtitle: Option<String>,
	author_name: Option<Vec<String>>,
	first_publish_year: Option<i32>,
	isbn: Option<serde_json::Value>,
	cover_i: Option<i64>,
	publisher: Option<Vec<String>>,
	subject: Option<Vec<String>>,
	number_of_pages_median: Option<i32>,
}
