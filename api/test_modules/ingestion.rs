use std::time::Duration;

use crate::common::TestApp;

const NETWORK_TIMEOUT: Duration = Duration::from_secs(30);
const JOB_TIMEOUT: Duration = Duration::from_secs(180);

async fn fetch(url: &str) -> Vec<u8> {
	match TestApp::try_download(url, NETWORK_TIMEOUT).await {
		Some(data) => {
			println!("Downloaded {url}: {} bytes", data.len());
			data
		}
		None => panic!("Failed to download {url}"),
	}
}

async fn ingest_and_read(app: &TestApp, url: &str, filename: &str) -> serde_json::Value {
	let data = fetch(url).await;
	let result = app.upload_file(filename, &data).await;
	let book_id = result["book_id"].as_str().unwrap().to_string();
	let job_id = result["job_id"].as_str().unwrap().to_string();
	let job = app.wait_for_job(&job_id, JOB_TIMEOUT).await;
	assert_eq!(job["status"], "completed", "job {job_id} failed: {:?}", job["error"]);
	let (status, read_data) = app.raw_get(&format!("/api/v1/books/{book_id}/read")).await;
	assert_eq!(status, 200, "read for {book_id} should return 200");
	read_data
}

fn collect_block_types(data: &serde_json::Value) -> Vec<String> {
	data["book"]["spine"]
		.as_array()
		.into_iter()
		.flatten()
		.flat_map(|s| s["blocks"].as_array().into_iter().flatten())
		.filter_map(|b| {
			if b.get("Paragraph").is_some() {
				Some("Paragraph".into())
			} else if b.get("Heading").is_some() {
				Some("Heading".into())
			} else if b.get("Image").is_some() {
				Some("Image".into())
			} else if b.get("CodeBlock").is_some() {
				Some("CodeBlock".into())
			} else if b.get("HorizontalRule").is_some() || b.as_str() == Some("HorizontalRule") {
				Some("HorizontalRule".into())
			} else {
				None
			}
		})
		.collect()
}

fn collect_text_with_marks(data: &serde_json::Value) -> Vec<(String, u16)> {
	data["book"]["spine"]
		.as_array()
		.into_iter()
		.flatten()
		.flat_map(|s| s["blocks"].as_array().into_iter().flatten())
		.flat_map(|b| {
			b.get("Paragraph")
				.or_else(|| b.get("Heading").and_then(|h| h.get("spans")))
				.and_then(|p| p.as_array())
				.into_iter()
				.flatten()
		})
		.filter_map(|s| {
			let text = s.get("text")?.as_str()?;
			let marks = s.get("marks")?.as_u64().unwrap_or(0) as u16;
			Some((text.to_string(), marks))
		})
		.collect()
}

#[tokio::test]
async fn variant_noimages() {
	let app = TestApp::new().await;
	let (_, _, _) = app.register_and_login("v_noimg").await;
	let data = ingest_and_read(&app, "https://www.gutenberg.org/ebooks/1513.epub.noimages", "r&j-noimg.epub").await;
	let types = collect_block_types(&data);
	assert!(types.contains(&"Paragraph".into()));
	assert!(types.contains(&"Heading".into()));
}

#[tokio::test]
async fn variant_with_images() {
	let app = TestApp::new().await;
	let (_, _, _) = app.register_and_login("v_img").await;
	let data = ingest_and_read(&app, "https://www.gutenberg.org/ebooks/1513.epub.images", "r&j-img.epub").await;
	let types = collect_block_types(&data);
	let image_count = types.iter().filter(|t| *t == "Image").count();
	println!("Images in IR: {image_count}");
	assert!(image_count >= 1);
}

#[tokio::test]
async fn variant_epub3() {
	let app = TestApp::new().await;
	let (_, _, _) = app.register_and_login("v_epub3").await;
	let data = ingest_and_read(&app, "https://www.gutenberg.org/ebooks/1513.epub3.images", "r&j-epub3.epub").await;
	let types = collect_block_types(&data);
	assert!(types.contains(&"Paragraph".into()));
}

#[tokio::test]
async fn book_pride_and_prejudice() {
	let app = TestApp::new().await;
	let (_, _, _) = app.register_and_login("book_pp").await;
	let data = ingest_and_read(&app, "https://www.gutenberg.org/ebooks/1342.epub.noimages", "pride.epub").await;
	let text = collect_text_with_marks(&data);
	let full: String = text.iter().map(|(t, _)| t.as_str()).collect::<Vec<_>>().join(" ");
	assert!(full.contains("Elizabeth"));
	assert!(full.contains("Darcy"));
}

#[tokio::test]
async fn book_romeo_and_juliet() {
	let app = TestApp::new().await;
	let (_, _, _) = app.register_and_login("book_rj").await;
	let data = ingest_and_read(&app, "https://www.gutenberg.org/ebooks/1513.epub.noimages", "romeo.epub").await;
	let text = collect_text_with_marks(&data);
	let full: String = text.iter().map(|(t, _)| t.as_str()).collect::<Vec<_>>().join(" ");
	assert!(full.contains("Romeo"));
	assert!(full.contains("Juliet"));
}

#[tokio::test]
async fn book_tom_sawyer() {
	let app = TestApp::new().await;
	let (_, _, _) = app.register_and_login("book_ts").await;
	let data = ingest_and_read(&app, "https://www.gutenberg.org/ebooks/74.epub.noimages", "tom.epub").await;
	let text = collect_text_with_marks(&data);
	let full: String = text.iter().map(|(t, _)| t.as_str()).collect::<Vec<_>>().join(" ");
	assert!(full.contains("Tom"));
	assert!(full.contains("Sawyer"));
}

#[tokio::test]
async fn book_frankenstein() {
	let app = TestApp::new().await;
	let (_, _, _) = app.register_and_login("book_fk").await;
	let data = ingest_and_read(&app, "https://www.gutenberg.org/ebooks/84.epub.noimages", "frankenstein.epub").await;
	let text = collect_text_with_marks(&data);
	let full: String = text.iter().map(|(t, _)| t.as_str()).collect::<Vec<_>>().join(" ");
	assert!(full.contains("Frankenstein"));
	assert!(full.contains("Victor"));
}

#[tokio::test]
async fn book_alice() {
	let app = TestApp::new().await;
	let (_, _, _) = app.register_and_login("book_alice").await;
	let data = ingest_and_read(&app, "https://www.gutenberg.org/ebooks/11.epub.noimages", "alice.epub").await;
	let text = collect_text_with_marks(&data);
	let full: String = text.iter().map(|(t, _)| t.as_str()).collect::<Vec<_>>().join(" ");
	assert!(full.contains("Alice"));
	assert!(full.contains("Wonderland"));
}

#[tokio::test]
async fn book_dracula() {
	let app = TestApp::new().await;
	let (_, _, _) = app.register_and_login("book_drac").await;
	let data = ingest_and_read(&app, "https://www.gutenberg.org/ebooks/345.epub.noimages", "dracula.epub").await;
	let text = collect_text_with_marks(&data);
	let full: String = text.iter().map(|(t, _)| t.as_str()).collect::<Vec<_>>().join(" ");
	assert!(full.contains("Dracula"));
}

#[tokio::test]
async fn book_great_gatsby() {
	let app = TestApp::new().await;
	let (_, _, _) = app.register_and_login("book_gg").await;
	let data = ingest_and_read(&app, "https://www.gutenberg.org/ebooks/64317.epub.noimages", "gatsby.epub").await;
	let text = collect_text_with_marks(&data);
	let full: String = text.iter().map(|(t, _)| t.as_str()).collect::<Vec<_>>().join(" ");
	assert!(full.contains("Gatsby"));
	assert!(full.contains("Nick"));
}

#[tokio::test]
async fn book_moby_dick() {
	let app = TestApp::new().await;
	let (_, _, _) = app.register_and_login("book_md").await;
	let data = ingest_and_read(&app, "https://www.gutenberg.org/ebooks/2701.epub.noimages", "moby.epub").await;
	let text = collect_text_with_marks(&data);
	let full: String = text.iter().map(|(t, _)| t.as_str()).collect::<Vec<_>>().join(" ");
	assert!(full.contains("Moby"));
	assert!(full.contains("Dick"));
	assert!(full.contains("Ahab"));
}

#[tokio::test]
async fn book_war_of_worlds() {
	let app = TestApp::new().await;
	let (_, _, _) = app.register_and_login("book_wow").await;
	let data = ingest_and_read(&app, "https://www.gutenberg.org/ebooks/36.epub.noimages", "war-of-worlds.epub").await;
	let text = collect_text_with_marks(&data);
	let full: String = text.iter().map(|(t, _)| t.as_str()).collect::<Vec<_>>().join(" ");
	assert!(full.contains("Martian"));
}

#[tokio::test]
async fn span_formatting_bold_italic() {
	let app = TestApp::new().await;
	let (_, _, _) = app.register_and_login("fmt_bold").await;
	let data = ingest_and_read(&app, "https://www.gutenberg.org/ebooks/1513.epub.noimages", "r&j-fmt.epub").await;
	let spans = collect_text_with_marks(&data);
	let italic = spans.iter().filter(|(_, m)| *m & 2 != 0).count();
	let bold = spans.iter().filter(|(_, m)| *m & 1 != 0).count();
	println!("Italic spans: {italic}, Bold spans: {bold}");
	assert!((italic + bold) > 0);
}

#[tokio::test]
async fn span_href_links() {
	let app = TestApp::new().await;
	let (_, _, _) = app.register_and_login("fmt_href").await;
	let data = ingest_and_read(&app, "https://www.gutenberg.org/ebooks/1513.epub.noimages", "r&j-href.epub").await;
	let links: Vec<_> = data["book"]["spine"]
		.as_array().into_iter().flatten()
		.flat_map(|s| s["blocks"].as_array().into_iter().flatten())
		.flat_map(|b| {
			b.get("Paragraph").or_else(|| b.get("Heading").and_then(|h| h.get("spans")))
				.and_then(|p| p.as_array()).into_iter().flatten()
		})
		.filter_map(|s| Some((s.get("text")?.as_str()?, s.get("href")?.as_str()?)))
		.collect();
	println!("Linked spans: {}", links.len());
}

#[tokio::test]
async fn span_image_assets() {
	let app = TestApp::new().await;
	let (_, _, _) = app.register_and_login("fmt_img").await;
	let data = ingest_and_read(&app, "https://www.gutenberg.org/ebooks/1513.epub.images", "r&j-img-fmt.epub").await;
	let images: Vec<_> = data["book"]["spine"]
		.as_array().into_iter().flatten()
		.flat_map(|s| s["blocks"].as_array().into_iter().flatten())
		.filter_map(|b| Some(b.get("Image")?.get("asset_ref")?.as_str()?))
		.collect();
	println!("Image blocks in IR: {}", images.len());
	assert!(images.len() >= 1);
	assert!(images.iter().all(|u| u.len() == 36));
}

#[tokio::test]
async fn structure_has_multiple_sections() {
	let app = TestApp::new().await;
	let (_, _, _) = app.register_and_login("struct_sec").await;
	let data = ingest_and_read(&app, "https://www.gutenberg.org/ebooks/1513.epub.noimages", "r&j-struct.epub").await;
	let spine = data["book"]["spine"].as_array().expect("spine");
	assert!(spine.len() >= 5, "sections: {}", spine.len());
}

#[tokio::test]
async fn structure_heading_hierarchy() {
	let app = TestApp::new().await;
	let (_, _, _) = app.register_and_login("struct_hdr").await;
	let data = ingest_and_read(&app, "https://www.gutenberg.org/ebooks/1513.epub.noimages", "r&j-hdr.epub").await;
	let types = collect_block_types(&data);
	let h = types.iter().filter(|t| *t == "Heading").count();
	let p = types.iter().filter(|t| *t == "Paragraph").count();
	println!("Headings: {h}, Paragraphs: {p}");
	assert!(h >= 10);
	assert!(p >= 50);
}

#[tokio::test]
async fn ingest_rejects_unknown_format() {
	let app = TestApp::new().await;
	let (_, _, _) = app.register_and_login("unknown").await;
	let garbage = b"This is not a valid ebook file at all!!!!";
	let resp = app.client.post(app.url("/api/v1/books/upload")).multipart(
		reqwest::multipart::Form::new().part(
			"file",
			reqwest::multipart::Part::bytes(garbage.to_vec())
				.file_name("random.txt").mime_str("text/plain").expect("mime"),
		),
	).send().await.expect("upload");
	assert_eq!(resp.status(), 400);
	let body: serde_json::Value = resp.json().await.expect("body");
	let err = body["error"].as_str().unwrap_or("");
	assert!(err.contains("Unsupported") || err.contains("format"));
}

#[tokio::test]
async fn metadata_title_and_format() {
	let app = TestApp::new().await;
	let (_, _, _) = app.register_and_login("meta_tf").await;
	let data = fetch("https://www.gutenberg.org/ebooks/1342.epub.noimages").await;
	let result = app.upload_file("pride.epub", &data).await;
	let book_id = result["book_id"].as_str().unwrap().to_string();
	let job_id = result["job_id"].as_str().unwrap().to_string();
	let job = app.wait_for_job(&job_id, JOB_TIMEOUT).await;
	assert_eq!(job["status"], "completed");
	let (_status, meta) = app.raw_get(&format!("/api/v1/books/{book_id}")).await;
	assert_eq!(meta["title"], "Pride and Prejudice", "title should be from EPUB metadata, got {:?}", meta["title"]);
	assert_eq!(meta["format"], "epub");
}
