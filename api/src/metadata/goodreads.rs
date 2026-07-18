use async_trait::async_trait;

use crate::AppError;
use crate::metadata::{MetadataProvider, MetadataQuery, ProspectiveMetadata};

pub struct GoodReadsProvider {
	client: reqwest::Client,
}

impl Default for GoodReadsProvider {
	fn default() -> Self {
		Self::new()
	}
}

impl GoodReadsProvider {
	pub fn new() -> Self {
		Self {
			client: reqwest::Client::builder()
				.user_agent(
					"Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/150.0.0.0 Safari/537.36",
				)
				.build()
				.expect("reqwest client"),
		}
	}

	fn extract_next_data(html: &str) -> Option<serde_json::Value> {
		let marker = r#"<script id="__NEXT_DATA__" type="application/json">"#;
		let start = html.find(marker)? + marker.len();
		let end = html[start..].find("</script>")? + start;
		serde_json::from_str(&html[start..end]).ok()
	}

	fn parse_book_from_state(apollo: &serde_json::Value, legacy_id: &str) -> Option<serde_json::Value> {
		let query_key = format!("getBookByLegacyId({{\"legacyId\":\"{legacy_id}\"}})");
		let book_ref = apollo["ROOT_QUERY"][&query_key]["__ref"].as_str()?;
		let book = apollo.get(book_ref)?;

		let title_uncut = book["titleComplete"].as_str().or_else(|| book["title"].as_str())?;
		let (title, _series_name, _series_pos) = parse_series_from_title(title_uncut);

		let authors: Vec<String> = book["primaryContributorEdge"]["node"]["name"]
			.as_str()
			.map(|n| vec![n.to_string()])
			.unwrap_or_default();

		let description = book["description"].as_str().map(|d| {
			let mut text = String::new();
			let mut in_tag = false;
			for ch in d.chars() {
				match ch {
					'<' => in_tag = true,
					'>' if in_tag => in_tag = false,
					_ if !in_tag => text.push(ch),
					_ => {}
				}
			}
			text.trim().to_string()
		});

		let cover_url = book["imageUrl"].as_str().map(|u| u.to_string());

		let publisher = book["details"]["publisher"]
			.as_str()
			.filter(|p| !p.is_empty())
			.map(|p| p.to_string());

		let page_count = book["details"]["numPages"].as_i64().map(|c| c as i32);

		let published_date = book["details"]["publicationTime"].as_i64().map(|ts| {
			let secs = ts / 1000;
			let days = secs / 86400;
			let y = 1970i64;
			let mut remaining = days;
			let mut year = 1970i64;
			loop {
				let days_in_year = if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) {
					366
				} else {
					365
				};
				if remaining < days_in_year {
					break;
				}
				remaining -= days_in_year;
				year += 1;
			}
			format!("{year}")
		});

		let rating = book["work"]["stats"]["averageRating"].as_f64().map(|r| r as f32);

		let isbn13 = book["details"]["isbn13"]
			.as_str()
			.filter(|s| !s.is_empty())
			.map(|s| s.to_string());
		let isbn10 = book["details"]["isbn"]
			.as_str()
			.filter(|s| !s.is_empty())
			.map(|s| s.to_string());

		let genres: Vec<String> = book["bookGenres"]
			.as_array()
			.map(|arr| {
				arr.iter()
					.filter_map(|g| g["genre"]["name"].as_str())
					.map(|s| s.to_string())
					.collect()
			})
			.unwrap_or_default();

		let mut result = serde_json::json!({
			"title": title,
			"authors": authors,
			"description": description,
			"cover_url": cover_url,
			"publisher": publisher,
			"page_count": page_count,
			"published_date": published_date,
			"rating": rating,
			"isbn13": isbn13,
			"isbn10": isbn10,
			"genres": genres,
		});

		if let Some(sub) = book["subtitle"].as_str() {
			result["subtitle"] = serde_json::Value::String(sub.to_string());
		}

		Some(result)
	}
}

#[async_trait]
impl MetadataProvider for GoodReadsProvider {
	fn id(&self) -> &'static str {
		"goodreads"
	}

	async fn search(&self, query: &MetadataQuery) -> Result<Vec<ProspectiveMetadata>, AppError> {
		let term = if let Some(ref isbn) = query.isbn {
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

		let search_url = format!("https://www.goodreads.com/search?q={}", term.replace(' ', "+"));
		let resp = self
			.client
			.get(&search_url)
			.send()
			.await
			.map_err(|e| AppError::Internal(format!("GoodReads search failed: {e}")))?;
		let html = resp
			.text()
			.await
			.map_err(|e| AppError::Internal(format!("GoodReads read failed: {e}")))?;

		let mut results = Vec::new();
		let mut search_pos = 0;
		while let Some(row_start) = html[search_pos..].find("<tr") {
			let row_begin = search_pos + row_start;
			let row_end = match html[row_begin..].find("</tr>") {
				Some(e) => row_begin + e + 5,
				None => break,
			};
			let row_html = &html[row_begin..row_end];
			search_pos = row_end;

			let title_link_start = match row_html.find("class=\"bookTitle\"") {
				Some(s) => s,
				None => continue,
			};
			let href_start = row_html[..title_link_start].rfind("href=\"").map(|i| i + 6);
			let href = href_start.and_then(|s| {
				let end = row_html[s..].find('"')?;
				Some(row_html[s..s + end].to_string())
			});
			let (provider_id, numeric_id) = href
				.as_ref()
				.and_then(|u| u.split("/book/show/").nth(1))
				.map(|slug| {
					let cleaned = slug.split('?').next().unwrap_or(slug).to_string();
					let numeric: String = cleaned.chars().take_while(|c| c.is_ascii_digit()).collect();
					(cleaned, numeric)
				})
				.unwrap_or_default();
			if provider_id.is_empty() {
				continue;
			}

			let title = extract_field(row_html, r#"<span itemprop="name"#)
				.or_else(|| extract_field(row_html, r#"class="bookTitle">"#))
				.map(|s| s.to_string());

			let authors: Vec<String> = {
				let mut a = Vec::new();
				let mut pos = 0;
				while let Some(n_start) = row_html[pos..].find("class=\"authorName\"") {
					let span_start = match row_html[pos + n_start..].find("<span") {
						Some(s) => pos + n_start + s,
						None => break,
					};
					let text_start = match row_html[span_start..].find('>') {
						Some(s) => span_start + s + 1,
						None => break,
					};
					let text_end = match row_html[text_start..].find("</span>") {
						Some(e) => text_start + e,
						None => break,
					};
					let name = html_unescape(&row_html[text_start..text_end]).trim().to_string();
					if !name.is_empty() {
						a.push(name);
					}
					pos = text_end;
				}
				a
			};

			let rating = extract_field(row_html, r#"class="minirating""#).and_then(|r| {
				let cleaned = r.split("—").next().unwrap_or(&r).trim();
				cleaned.split_whitespace().next().and_then(|n| n.parse::<f32>().ok())
			});

			results.push(ProspectiveMetadata {
				provider: "goodreads".to_string(),
				provider_id,
				title,
				authors,
				description: None,
				isbn10: None,
				isbn13: None,
				page_count: None,
				genres: Vec::new(),
				cover_url: None,
				published_date: None,
				publisher: None,
				rating,
				subtitle: None,
			});
		}

		for meta in results.iter_mut().take(3) {
			let numeric: String = meta.provider_id.chars().take_while(|c| c.is_ascii_digit()).collect();
			if numeric.is_empty() {
				continue;
			}

			let url = format!("https://www.goodreads.com/book/show/{}", meta.provider_id);
			if let Ok(resp) = self.client.get(&url).send().await {
				if let Ok(body) = resp.text().await {
					if let Some(data) = Self::extract_next_data(&body) {
						let apollo = &data["props"]["pageProps"]["apolloState"];
						if let Some(detail) = Self::parse_book_from_state(apollo, &numeric) {
							if let Some(t) = detail["title"].as_str() {
								meta.title = Some(t.to_string());
							}
							if let Some(a) = detail["authors"].as_array() {
								meta.authors = a.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect();
							}
							if let Some(d) = detail["description"].as_str() {
								meta.description = Some(d.to_string());
							}
							if let Some(isbn) = detail["isbn13"].as_str() {
								meta.isbn13 = Some(isbn.to_string());
							}
							if let Some(isbn) = detail["isbn10"].as_str() {
								meta.isbn10 = Some(isbn.to_string());
							}
							if let Some(p) = detail["page_count"].as_i64() {
								meta.page_count = Some(p as i32);
							}
							if let Some(g) = detail["genres"].as_array() {
								meta.genres = g.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect();
							}
							if let Some(c) = detail["cover_url"].as_str() {
								meta.cover_url = Some(c.to_string());
							}
							if let Some(d) = detail["published_date"].as_str() {
								meta.published_date = Some(d.to_string());
							}
							if let Some(p) = detail["publisher"].as_str() {
								meta.publisher = Some(p.to_string());
							}
							if let Some(r) = detail["rating"].as_f64() {
								meta.rating = Some(r as f32);
							}
							if let Some(s) = detail["subtitle"].as_str() {
								meta.subtitle = Some(s.to_string());
							}
						}
					}
				}
			}
		}

		Ok(results)
	}
}

fn html_unescape(s: &str) -> String {
	s.replace("&amp;", "&")
		.replace("&lt;", "<")
		.replace("&gt;", ">")
		.replace("&quot;", "\"")
		.replace("&#39;", "'")
}

fn extract_field<'a>(html: &'a str, marker: &str) -> Option<&'a str> {
	let marker_pos = html.find(marker)?;
	let open = html[marker_pos..].find('>')? + marker_pos + 1;
	let close = html[open..].find('<')? + open;
	let text = html[open..close].trim();
	if text.is_empty() { None } else { Some(text) }
}

fn parse_series_from_title(title: &str) -> (String, Option<String>, Option<f32>) {
	if let Some(paren_start) = title.rfind(" (") {
		if let Some(paren_end) = title[paren_start..].find(')') {
			let inside = &title[paren_start + 2..paren_start + paren_end];
			let base_title = title[..paren_start].trim().to_string();
			if let Some(hash_pos) = inside.rfind(" #") {
				let series = inside[..hash_pos].trim().to_string();
				if let Ok(pos) = inside[hash_pos + 2..].trim().parse::<f32>() {
					return (base_title, Some(series), Some(pos));
				}
				return (base_title, Some(series), None);
			}
			return (base_title, None, None);
		}
	}
	(title.to_string(), None, None)
}
