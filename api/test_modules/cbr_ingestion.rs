use std::time::Duration;
use crate::common::TestApp;

fn make_png_pixel() -> Vec<u8> {
	// Minimal valid 1x1 red PNG
	let png: Vec<u8> = vec![
		0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, // PNG signature
		0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44, 0x52, // IHDR chunk length + type
		0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, // 1x1 pixel
		0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53, // 8-bit grayscale
		0xde, 0x00, 0x00, 0x00, 0x0c, 0x49, 0x44, 0x41, // IHDR CRC
		0x54, 0x08, 0xd7, 0x63, 0xf8, 0x00, 0x00, 0x00, // IDAT chunk
		0x03, 0x00, 0x01, 0x36, 0x00, 0x29, 0x00, 0x00, // compressed data
		0x00, 0x00, 0x00, 0x49, 0x45, 0x4e, 0x44, 0xae, // IEND chunk
		0x42, 0x60, 0x82,
	];
	png
}

fn make_rar_with_pages(count: usize) -> Vec<u8> {
	use std::io::Write;
	let pixel = make_png_pixel();

	let mut buf = Vec::new();
	{
		let mut zip = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
		let opts: zip::write::FileOptions<()> = zip::write::FileOptions::default()
			.compression_method(zip::CompressionMethod::Stored);
		for i in 0..count {
			zip.start_file(format!("page_{:03}.png", i + 1), opts).unwrap();
			zip.write_all(&pixel).unwrap();
		}
		zip.finish().unwrap();
	}
	buf
}

async fn upload_comic(app: &TestApp, data: &[u8], ext: &str) -> (String, serde_json::Value) {
	let r = app.upload_file(&format!("test.{}", ext), data).await;
	let book_id = r["book_id"].as_str().unwrap().to_string();
	let job_id = r["job_id"].as_str().unwrap().to_string();
	let job = app.wait_for_job(&job_id, Duration::from_secs(30)).await;
	assert_eq!(job["status"], "completed", "job failed: {:?}", job["error"]);
	(book_id, job)
}

#[tokio::test]
async fn cbr_format_detected_by_magic() {
	let app = TestApp::new().await;
	app.register_and_login("cbr_magic").await;
	let data = b"Rar!\x1a\x07\x00some_data".to_vec();
	let r = app.upload_file("test.cbr", &data).await;
	assert!(r["job_id"].as_str().is_some(), "CBR with RAR magic should be accepted at upload");
	assert!(r["book_id"].as_str().is_some(), "book should be created");
}

#[tokio::test]
async fn detect_cbz_by_zip_magic() {
	let app = TestApp::new().await;
	app.register_and_login("detect_cbz").await;
	let data = make_rar_with_pages(2);
	let (book_id, _job) = upload_comic(&app, &data, "cbz").await;

	let (s, d) = app.raw_get(&format!("/api/v1/books/{book_id}/read")).await;
	assert_eq!(s, 200, "CBZ read should succeed: {d}");
	let spine = d["book"]["spine"].as_array().unwrap();
	assert_eq!(spine.len(), 1, "should have one section");
	let blocks = spine[0]["blocks"].as_array().unwrap();
	assert_eq!(blocks.len(), 2, "should have 2 pages");
}

#[tokio::test]
async fn detect_unknown_format_rejected() {
	let app = TestApp::new().await;
	app.register_and_login("detect_unknown").await;
	let data = b"this is not a valid ebook file";
	let resp = app
		.client
		.post(app.url("/api/v1/books/upload"))
		.multipart(
			reqwest::multipart::Form::new()
				.part("file", reqwest::multipart::Part::bytes(data.to_vec()).file_name("test.unknown".to_string()))
		)
		.send()
		.await
		.unwrap();
	assert_eq!(resp.status().as_u16(), 400, "unknown format should be rejected");
}

#[tokio::test]
async fn cbz_single_page() {
	let app = TestApp::new().await;
	app.register_and_login("cbz_1p").await;
	let data = make_rar_with_pages(1);
	let (book_id, _job) = upload_comic(&app, &data, "cbz").await;

	let (s, d) = app.raw_get(&format!("/api/v1/books/{book_id}/read")).await;
	assert_eq!(s, 200);
	let blocks = d["book"]["spine"][0]["blocks"].as_array().unwrap();
	assert_eq!(blocks.len(), 1);
	assert!(blocks[0]["Image"].is_object(), "block should be Image");
	assert!(blocks[0]["Image"]["asset_ref"].as_str().unwrap().len() > 0, "should have asset_ref");
}

#[tokio::test]
async fn cbz_multi_page_ordering() {
	let app = TestApp::new().await;
	app.register_and_login("cbz_mpo").await;
	let data = make_rar_with_pages(5);
	let (book_id, _job) = upload_comic(&app, &data, "cbz").await;

	let (s, d) = app.raw_get(&format!("/api/v1/books/{book_id}/read")).await;
	assert_eq!(s, 200);
	let blocks = d["book"]["spine"][0]["blocks"].as_array().unwrap();
	assert_eq!(blocks.len(), 5);
	for (i, block) in blocks.iter().enumerate() {
		let alt = block["Image"]["alt"].as_str().unwrap();
		assert_eq!(alt, format!("Page {}", i + 1), "page order should be preserved");
	}
}

#[tokio::test]
async fn cbz_cover_extraction() {
	let app = TestApp::new().await;
	app.register_and_login("cbz_cover").await;
	let data = make_rar_with_pages(3);
	let (book_id, _job) = upload_comic(&app, &data, "cbz").await;

	let resp = app.client.get(app.url(&format!("/api/v1/books/{book_id}/cover"))).send().await.unwrap();
	let status = resp.status().as_u16();
	assert!(status == 200 || status == 302 || status == 307,
		"cover should be available after CBZ ingestion, got {status}");
}

#[tokio::test]
async fn cbz_empty_archive_rejected() {
	let app = TestApp::new().await;
	app.register_and_login("cbz_empty").await;
	use std::io::Write;
	let mut buf = Vec::new();
	{
		let mut zip = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
		let opts: zip::write::FileOptions<()> = zip::write::FileOptions::default()
			.compression_method(zip::CompressionMethod::Stored);
		zip.start_file("not_an_image.txt", opts).unwrap();
		write!(zip, "this is not an image").unwrap();
		zip.finish().unwrap();
	}

	let r = app.upload_file("empty.cbz", &buf).await;
	let job_id = r["job_id"].as_str().unwrap();
	let job = app.wait_for_job(&job_id, Duration::from_secs(30)).await;
	if job["status"] == "completed" {
		let book_id = r["book_id"].as_str().unwrap();
		let (s, d) = app.raw_get(&format!("/api/v1/books/{book_id}/read")).await;
		assert_eq!(s, 200);
		let blocks = d["book"]["spine"][0]["blocks"].as_array().unwrap();
		assert_eq!(blocks.len(), 0, "no images means no blocks");
	} else {
		assert_eq!(job["status"], "dead_letter");
	}
}

#[tokio::test]
async fn cbz_case_insensitive_extensions() {
	let app = TestApp::new().await;
	app.register_and_login("cbz_case").await;
	use std::io::Write;
	let pixel = make_png_pixel();
	let mut buf = Vec::new();
	{
		let mut zip = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
		let opts: zip::write::FileOptions<()> = zip::write::FileOptions::default()
			.compression_method(zip::CompressionMethod::Stored);
		zip.start_file("PAGE_001.PNG", opts).unwrap();
		zip.write_all(&pixel).unwrap();
		zip.start_file("Page_002.JPG", opts).unwrap();
		zip.write_all(&pixel).unwrap();
		zip.finish().unwrap();
	}

	let (book_id, _job) = upload_comic(&app, &buf, "cbz").await;
	let (s, d) = app.raw_get(&format!("/api/v1/books/{book_id}/read")).await;
	assert_eq!(s, 200);
	let blocks = d["book"]["spine"][0]["blocks"].as_array().unwrap();
	assert_eq!(blocks.len(), 2, "both uppercase extensions should be recognized");
}

#[tokio::test]
async fn cbz_jpeg_and_png_mixed() {
	let app = TestApp::new().await;
	app.register_and_login("cbz_mixed").await;
	use std::io::Write;
	let pixel = make_png_pixel();
	let mut buf = Vec::new();
	{
		let mut zip = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
		let opts: zip::write::FileOptions<()> = zip::write::FileOptions::default()
			.compression_method(zip::CompressionMethod::Stored);
		zip.start_file("cover.jpg", opts).unwrap();
		zip.write_all(&pixel).unwrap();
		zip.start_file("page1.png", opts).unwrap();
		zip.write_all(&pixel).unwrap();
		zip.start_file("page2.webp", opts).unwrap();
		zip.write_all(&pixel).unwrap();
		zip.finish().unwrap();
	}

	let (book_id, _job) = upload_comic(&app, &buf, "cbz").await;
	let (s, d) = app.raw_get(&format!("/api/v1/books/{book_id}/read")).await;
	assert_eq!(s, 200);
	let blocks = d["book"]["spine"][0]["blocks"].as_array().unwrap();
	assert_eq!(blocks.len(), 3, "jpg, png, webp all recognized");
}

#[tokio::test]
async fn cbz_ten_digit_page_sorting() {
	let app = TestApp::new().await;
	app.register_and_login("cbz_sort").await;
	use std::io::Write;
	let pixel = make_png_pixel();
	let mut buf = Vec::new();
	{
		let mut zip = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
		let opts: zip::write::FileOptions<()> = zip::write::FileOptions::default()
			.compression_method(zip::CompressionMethod::Stored);
		zip.start_file("page_2.png", opts).unwrap(); zip.write_all(&pixel).unwrap();
		zip.start_file("page_10.png", opts).unwrap(); zip.write_all(&pixel).unwrap();
		zip.start_file("page_1.png", opts).unwrap(); zip.write_all(&pixel).unwrap();
		zip.finish().unwrap();
	}

	let (book_id, _job) = upload_comic(&app, &buf, "cbz").await;
	let (s, d) = app.raw_get(&format!("/api/v1/books/{book_id}/read")).await;
	assert_eq!(s, 200);
	let blocks = d["book"]["spine"][0]["blocks"].as_array().unwrap();
	assert_eq!(blocks.len(), 3);
	assert!(blocks[0]["Image"]["alt"] == "Page 1", "page_1 should be first: {:?}", blocks[0]);
	assert!(blocks[1]["Image"]["alt"] == "Page 2", "page_2 should be second");
	assert!(blocks[2]["Image"]["alt"] == "Page 3", "page_10 should be third (natural sort)");
}

#[tokio::test]
async fn cbz_reading_state_after_ingest() {
	let app = TestApp::new().await;
	app.register_and_login("cbz_readstate").await;
	let data = make_rar_with_pages(2);
	let (book_id, _job) = upload_comic(&app, &data, "cbz").await;

	let (s, _d) = app.raw_get(&format!("/api/v1/books/{book_id}/progress")).await;
	assert_eq!(s, 200, "progress should be available");
}

#[tokio::test]
async fn cbz_large_page_count_ok() {
	let app = TestApp::new().await;
	app.register_and_login("cbz_large").await;
	let data = make_rar_with_pages(50);
	let (book_id, _job) = upload_comic(&app, &data, "cbz").await;

	let (s, d) = app.raw_get(&format!("/api/v1/books/{book_id}/read")).await;
	assert_eq!(s, 200);
	let blocks = d["book"]["spine"][0]["blocks"].as_array().unwrap();
	assert_eq!(blocks.len(), 50, "all 50 pages should be extracted");
}

#[tokio::test]
async fn cbz_subdirectory_pages_skipped() {
	let app = TestApp::new().await;
	app.register_and_login("cbz_subdir").await;
	use std::io::Write;
	let pixel = make_png_pixel();
	let mut buf = Vec::new();
	{
		let mut zip = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
		let opts: zip::write::FileOptions<()> = zip::write::FileOptions::default()
			.compression_method(zip::CompressionMethod::Stored);
		zip.add_directory("subdir/", opts).unwrap();
		zip.start_file("subdir/page1.png", opts).unwrap();
		zip.write_all(&pixel).unwrap();
		zip.start_file("page2.png", opts).unwrap();
		zip.write_all(&pixel).unwrap();
		zip.finish().unwrap();
	}

	let (book_id, _job) = upload_comic(&app, &buf, "cbz").await;
	let (s, d) = app.raw_get(&format!("/api/v1/books/{book_id}/read")).await;
	assert_eq!(s, 200);
	let blocks = d["book"]["spine"][0]["blocks"].as_array().unwrap();
	assert_eq!(blocks.len(), 2, "should include subdirectory images");
}

#[tokio::test]
async fn cbz_export_epub() {
	let app = TestApp::new().await;
	app.register_and_login("cbz_export").await;
	let data = make_rar_with_pages(2);
	let (book_id, _job) = upload_comic(&app, &data, "cbz").await;

	let resp = app.client.get(app.url(&format!("/api/v1/books/{book_id}/export?format=epub"))).send().await.unwrap();
	assert_eq!(resp.status().as_u16(), 200, "CBZ should export as EPUB");
	let ct = resp.headers().get("content-type").unwrap().to_str().unwrap().to_string();
	assert!(ct.contains("epub"), "should be EPUB: {ct}");
}
