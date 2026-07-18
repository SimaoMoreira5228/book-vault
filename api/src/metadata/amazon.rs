use async_trait::async_trait;

use crate::AppError;
use crate::metadata::{MetadataProvider, MetadataQuery, ProspectiveMetadata};

pub struct AmazonProvider {
	client: reqwest::Client,
}

impl Default for AmazonProvider {
	fn default() -> Self {
		Self::new()
	}
}

impl AmazonProvider {
	pub fn new() -> Self {
		Self {
			client: reqwest::Client::builder()
				.user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
				.build()
				.expect("reqwest client"),
		}
	}

	fn extract_between(html: &str, before: &str, after: &str) -> Option<String> {
		let start = html.find(before)? + before.len();
		let end = html[start..].find(after)? + start;
		let val = html[start..end].trim();
		if val.is_empty() { None } else { Some(val.to_string()) }
	}

	fn extract_asin(html: &str) -> Option<String> {
		let patterns = [r#"data-asin="ASIN""#, r#"name="ASIN.amb" value=""#, r#""ASIN""#];
		for pattern in &patterns {
			if pattern.contains("ASIN") && !pattern.contains("value") {
				if let Some(val) = Self::extract_between(html, pattern, "\"") {
					if val.len() == 10 && val.chars().all(|c| c.is_alphanumeric()) {
						return Some(val);
					}
				}
			}
		}
		None
	}

	fn strip_html(&self, text: &str) -> String {
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

	fn parse_product_page(&self, html: &str, _isbn: Option<&str>) -> Option<ProspectiveMetadata> {
		let title = Self::extract_between(html, "<title>", "</title>").map(|t| {
			t.split(": Amazon.com")
				.next()
				.unwrap_or(&t)
				.split(':')
				.next()
				.unwrap_or(&t)
				.trim()
				.to_string()
		})?;

		let author_html = Self::extract_between(html, "class=\"author\"", "</div>")
			.or_else(|| Self::extract_between(html, "contributorNameID", "</a>").map(|s| self.strip_html(&s)))
			.map(|s| self.strip_html(&s));

		let authors: Vec<String> = author_html
			.map(|a| a.split(',').map(|s| s.trim().to_string()).collect())
			.unwrap_or_default();

		let rating = Self::extract_between(html, "\"averageRating\":\"", "\"")
			.or_else(|| Self::extract_between(html, "\"ratingValue\"", "\""))
			.and_then(|r| r.split('"').next_back().unwrap_or(&r).parse::<f32>().ok());

		let page_count = Self::extract_between(html, " pages", "<")
			.and_then(|s| {
				let s = s.trim();
				let end = s.rfind(' ').unwrap_or(s.len());
				s[..end].trim().parse::<i32>().ok()
			})
			.or_else(|| {
				Self::extract_between(html, "Page", "|").and_then(|s| s.split_whitespace().last()?.parse::<i32>().ok())
			});

		let isbn13 = Self::extract_between(html, "978", "<")
			.map(|s| {
				let digits: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
				if digits.len() >= 13 {
					digits[..13].to_string()
				} else {
					digits
				}
			})
			.filter(|s: &String| s.len() == 13);

		let publisher = Self::extract_between(html, "Publisher", "</li>")
			.or_else(|| Self::extract_between(html, "publisher", "</span>"))
			.map(|s| {
				let clean = s.replace("</span>", " ").replace("<span>", " ");
				self.strip_html(&clean)
			})
			.filter(|s: &String| !s.is_empty());

		let cover_url = Self::extract_between(html, "\"image\":\"", "\"")
			.or_else(|| Self::extract_between(html, "\"hiRes\"", "\""))
			.map(|u| u.replace("\\", ""));

		let provider_id = Self::extract_asin(html).unwrap_or_default();

		Some(ProspectiveMetadata {
			provider: "amazon".to_string(),
			provider_id,
			title: Some(title),
			authors,
			description: None,
			isbn10: None,
			isbn13,
			page_count,
			genres: Vec::new(),
			cover_url,
			published_date: None,
			publisher,
			rating,
			subtitle: None,
		})
	}
}

#[async_trait]
impl MetadataProvider for AmazonProvider {
	fn id(&self) -> &'static str {
		"amazon"
	}

	async fn search(&self, query: &MetadataQuery) -> Result<Vec<ProspectiveMetadata>, AppError> {
		let search_terms = if let Some(ref isbn) = query.isbn {
			let cleaned: String = isbn.chars().filter(|c| c.is_ascii_digit()).collect();
			vec![cleaned]
		} else {
			let mut terms = Vec::new();
			if let Some(ref title) = query.title {
				terms.push(title.clone());
			}
			if let Some(ref author) = query.author {
				terms.push(author.clone());
			}
			vec![terms.join("+")]
		};

		let mut results = Vec::new();

		for term in &search_terms {
			if term.len() >= 10 {
				let url = format!("https://www.amazon.com/dp/{term}");
				if let Ok(resp) = self.client.get(&url).send().await {
					if let Ok(body) = resp.text().await {
						if let Some(meta) = self.parse_product_page(&body, Some(term)) {
							results.push(meta);
							return Ok(results);
						}
					}
				}
			}
		}

		if query.title.is_some() || query.author.is_some() {
			let search_term = search_terms.join("+");
			let url = format!("https://www.amazon.com/s?k={search_term}&i=stripbooks");
			if let Ok(resp) = self.client.get(&url).send().await {
				if let Ok(body) = resp.text().await {
					let mut asin_pos = 0;
					while let Some(asin_start) = body[asin_pos..].find("/dp/") {
						let val_start = asin_start + 4;
						let asin_end = match body[val_start..].find('/').or_else(|| body[val_start..].find('?')) {
							Some(e) => val_start + e,
							None => break,
						};
						let asin = &body[val_start..asin_end];
						asin_pos = asin_end;

						if asin.len() == 10 && asin.chars().all(|c| c.is_alphanumeric()) {
							let dp_url = format!("https://www.amazon.com/dp/{asin}");
							if let Ok(d_resp) = self.client.get(&dp_url).send().await {
								if let Ok(d_body) = d_resp.text().await {
									if let Some(meta) = self.parse_product_page(&d_body, None) {
										results.push(meta);
										if results.len() >= 3 {
											break;
										}
									}
								}
							}
						}
					}
				}
			}
		}

		Ok(results)
	}
}
