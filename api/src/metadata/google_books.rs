use async_trait::async_trait;
use serde::Deserialize;

use crate::AppError;
use crate::metadata::{MetadataProvider, MetadataQuery, ProspectiveMetadata};

pub struct GoogleBooksProvider {
	client: reqwest::Client,
}

impl Default for GoogleBooksProvider {
	fn default() -> Self {
		Self::new()
	}
}

impl GoogleBooksProvider {
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
impl MetadataProvider for GoogleBooksProvider {
	fn id(&self) -> &'static str {
		"google_books"
	}

	async fn search(&self, query: &MetadataQuery) -> Result<Vec<ProspectiveMetadata>, AppError> {
		let url = build_google_url(query);
		let resp = self
			.client
			.get(&url)
			.send()
			.await
			.map_err(|e| AppError::Internal(format!("Google Books search failed: {e}")))?;

		let body: GoogleBooksResponse = resp
			.json()
			.await
			.map_err(|e| AppError::Internal(format!("Google Books parse failed: {e}")))?;

		let items = body.items.unwrap_or_default();
		let mut results = Vec::new();

		for item in items.into_iter().take(10) {
			let info = item.volume_info;
			let (isbn10, isbn13) = extract_google_isbns(&info.industry_identifiers);

			let cover_url = info
				.image_links
				.as_ref()
				.and_then(|l| l.thumbnail.clone())
				.map(|u| u.replace("http://", "https://"))
				.map(|u| {
					if u.starts_with("http://") {
						u.replacen("http://", "https://", 1)
					} else {
						u
					}
				});

			results.push(ProspectiveMetadata {
				provider: "google_books".to_string(),
				provider_id: item.id,
				title: info.title,
				authors: info.authors.unwrap_or_default(),
				description: info.description,
				isbn10,
				isbn13,
				page_count: info.page_count,
				genres: info.categories.unwrap_or_default(),
				cover_url,
				published_date: info.published_date,
				publisher: info.publisher,
				rating: info.average_rating,
				subtitle: info.subtitle,
			});
		}

		Ok(results)
	}
}

fn build_google_url(query: &MetadataQuery) -> String {
	let q = if let Some(ref isbn) = query.isbn {
		let cleaned: String = isbn.chars().filter(|c| c.is_ascii_digit()).collect();
		format!("isbn:{cleaned}")
	} else {
		let mut parts = Vec::new();
		if let Some(ref title) = query.title {
			parts.push(format!("intitle:{}", urlencode(title)));
		}
		if let Some(ref author) = query.author {
			parts.push(format!("inauthor:{}", urlencode(author)));
		}
		parts.join("+")
	};

	format!("https://www.googleapis.com/books/v1/volumes?q={}&maxResults=10", q)
}

fn urlencode(s: &str) -> String {
	s.replace(' ', "+")
}

fn extract_google_isbns(identifiers: &Option<Vec<IndustryIdentifier>>) -> (Option<String>, Option<String>) {
	let identifiers = match identifiers {
		Some(v) => v,
		None => return (None, None),
	};

	let mut isbn10 = None;
	let mut isbn13 = None;

	for id in identifiers {
		match id.type_.as_str() {
			"ISBN_10" => isbn10 = Some(id.identifier.clone()),
			"ISBN_13" => isbn13 = Some(id.identifier.clone()),
			_ => {}
		}
	}

	(isbn10, isbn13)
}

#[derive(Debug, Deserialize)]
struct GoogleBooksResponse {
	items: Option<Vec<GoogleBookItem>>,
}

#[derive(Debug, Deserialize)]
struct GoogleBookItem {
	id: String,
	volume_info: VolumeInfo,
}

#[derive(Debug, Deserialize)]
struct VolumeInfo {
	title: Option<String>,
	subtitle: Option<String>,
	authors: Option<Vec<String>>,
	description: Option<String>,
	published_date: Option<String>,
	page_count: Option<i32>,
	industry_identifiers: Option<Vec<IndustryIdentifier>>,
	categories: Option<Vec<String>>,
	publisher: Option<String>,
	image_links: Option<ImageLinks>,
	average_rating: Option<f32>,
}

#[derive(Debug, Deserialize)]
struct IndustryIdentifier {
	#[serde(rename = "type")]
	type_: String,
	identifier: String,
}

#[derive(Debug, Deserialize)]
struct ImageLinks {
	thumbnail: Option<String>,
}
