use async_trait::async_trait;

use crate::AppError;
use crate::metadata::{MetadataProvider, MetadataQuery, ProspectiveMetadata};

pub struct WikipediaProvider {
	client: reqwest::Client,
}

impl Default for WikipediaProvider {
	fn default() -> Self {
		Self::new()
	}
}

impl WikipediaProvider {
	pub fn new() -> Self {
		Self {
			client: reqwest::Client::builder()
				.user_agent("BookVault/0.1 (metadata enrichment)")
				.build()
				.expect("reqwest client"),
		}
	}

	fn urlencode(s: &str) -> String {
		s.split(' ').map(|part| urlencoding(part)).collect::<Vec<_>>().join("+")
	}

	async fn fetch_summary(&self, title: &str) -> Result<Option<ProspectiveMetadata>, AppError> {
		let safe_title = Self::urlencode(title);
		let url = format!("https://en.wikipedia.org/api/rest_v1/page/summary/{safe_title}");
		let resp = match self.client.get(&url).send().await {
			Ok(r) if r.status().is_success() => r,
			_ => return Ok(None),
		};
		let data: serde_json::Value = match resp.json().await {
			Ok(d) => d,
			Err(_) => return Ok(None),
		};

		let page_title = match data["title"].as_str() {
			Some(t) => t.to_string(),
			None => return Ok(None),
		};
		let extract = data["extract"].as_str().map(|s| s.to_string());
		let cover_url = data["thumbnail"]["source"].as_str().map(|s| s.to_string());
		let wiki_url = data["content_urls"]["desktop"]["page"].as_str().map(|s| s.to_string());
		let description = data["description"].as_str().map(|s| s.to_string());

		let page_id = data["pageid"].as_u64().unwrap_or(0);

		Ok(Some(ProspectiveMetadata {
			provider: "wikipedia".to_string(),
			provider_id: page_id.to_string(),
			title: Some(page_title.to_string()),
			authors: Vec::new(),
			description: extract.or(description),
			isbn10: None,
			isbn13: None,
			page_count: None,
			genres: Vec::new(),
			cover_url,
			published_date: None,
			publisher: None,
			rating: None,
			subtitle: None,
		}))
	}
}

#[async_trait]
impl MetadataProvider for WikipediaProvider {
	fn id(&self) -> &'static str {
		"wikipedia"
	}

	async fn search(&self, query: &MetadataQuery) -> Result<Vec<ProspectiveMetadata>, AppError> {
		let search_term = if let Some(ref title) = query.title {
			title.clone()
		} else if let Some(ref author) = query.author {
			author.clone()
		} else {
			return Ok(Vec::new());
		};

		let search_query = Self::urlencode(&search_term);
		let search_url = format!(
			"https://en.wikipedia.org/w/api.php?action=query&list=search&srsearch={search_query}&format=json&srlimit=5"
		);
		let resp = match self.client.get(&search_url).send().await {
			Ok(r) if r.status().is_success() => r,
			_ => return Ok(Vec::new()),
		};
		let search_data: serde_json::Value = match resp.json().await {
			Ok(d) => d,
			Err(_) => return Ok(Vec::new()),
		};

		let pages: Vec<(String, u64)> = search_data["query"]["search"]
			.as_array()
			.map(|arr| {
				arr.iter()
					.filter_map(|r| {
						let title = r["title"].as_str()?;
						let pageid = r["pageid"].as_u64().unwrap_or(0);
						Some((title.to_string(), pageid))
					})
					.collect()
			})
			.unwrap_or_default();

		if pages.is_empty() {
			return Ok(Vec::new());
		}

		if let Some((title, _)) = pages.first() {
			if let Ok(Some(meta)) = self.fetch_summary(title).await {
				return Ok(vec![meta]);
			}
		}

		Ok(Vec::new())
	}
}

fn urlencoding(s: &str) -> String {
	s.replace(' ', "%20")
		.replace('&', "%26")
		.replace('?', "%3F")
		.replace('=', "%3D")
		.replace('#', "%23")
}
