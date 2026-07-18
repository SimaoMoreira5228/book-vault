use std::time::Duration;

use crate::common::{FakeSmtpServer, TestApp};

#[tokio::test]
async fn email_status_disabled_by_default() {
	let app = TestApp::new().await;

	let resp = app.client.get(app.url("/api/v1/email/status")).send().await.unwrap();
	assert_eq!(resp.status().as_u16(), 200);
	let json: serde_json::Value = resp.json().await.unwrap();
	assert_eq!(json["enabled"], false);
	assert_eq!(json["configured"], false);
}

#[tokio::test]
async fn email_status_enabled() {
	let smtp = FakeSmtpServer::start().await;
	let app = TestApp::with_email_enabled(smtp.port).await;

	let resp = app.client.get(app.url("/api/v1/email/status")).send().await.unwrap();
	assert_eq!(resp.status().as_u16(), 200);
	let json: serde_json::Value = resp.json().await.unwrap();
	assert_eq!(json["enabled"], true);
	assert_eq!(json["configured"], true);
}

#[tokio::test]
async fn email_send_fails_when_disabled() {
	let app = TestApp::new().await;
	app.register_and_login("email_disabled").await;
	let book = app.create_book("Email Test", None).await;
	let book_id = book["id"].as_str().unwrap();

	let resp = app
		.client
		.post(app.url(&format!("/api/v1/books/{}/email", book_id)))
		.json(&serde_json::json!({ "to": "test@example.com" }))
		.send()
		.await
		.unwrap();
	assert_eq!(resp.status().as_u16(), 400);
}

#[tokio::test]
async fn email_send_fails_with_invalid_address() {
	let smtp = FakeSmtpServer::start().await;
	let app = TestApp::with_email_enabled(smtp.port).await;
	app.register_and_login("email_bad_addr").await;

	let epub = create_minimal_epub();
	let upload = app.upload_file("test.epub", &epub).await;
	let job_id = upload["job_id"].as_str().unwrap();
	app.wait_for_job(job_id, Duration::from_secs(10)).await;
	let book_id = upload["book_id"].as_str().unwrap();

	let resp = app
		.client
		.post(app.url(&format!("/api/v1/books/{}/email", book_id)))
		.json(&serde_json::json!({ "to": "not-an-email" }))
		.send()
		.await
		.unwrap();
	assert_eq!(resp.status().as_u16(), 400);
}

#[tokio::test]
async fn email_send_succeeds_through_fake_smtp() {
	let smtp = FakeSmtpServer::start().await;
	let app = TestApp::with_email_enabled(smtp.port).await;
	app.register_and_login("email_success").await;

	let epub = create_minimal_epub();
	let upload = app.upload_file("test.epub", &epub).await;
	let job_id = upload["job_id"].as_str().unwrap();
	app.wait_for_job(job_id, Duration::from_secs(10)).await;
	let book_id = upload["book_id"].as_str().unwrap();

	let resp = app
		.client
		.post(app.url(&format!("/api/v1/books/{}/email", book_id)))
		.json(&serde_json::json!({ "to": "kindle@test.com", "format": "epub" }))
		.send()
		.await
		.unwrap();
	assert_eq!(resp.status().as_u16(), 200, "email send failed: {:?}", resp.text().await);

	let json: serde_json::Value = resp.json().await.unwrap();
	assert_eq!(json["message"], "Email sent");
	assert_eq!(json["to"], "kindle@test.com");
	assert_eq!(json["format"], "epub");

	tokio::time::sleep(Duration::from_millis(200)).await;
	let conv = smtp.received.lock().unwrap();
	assert!(!conv.is_empty(), "fake smtp should have received a connection");
	let full = conv.join("\n");
	assert!(full.contains("kindle@test.com"), "should have recipient: {}", full);
}

fn create_minimal_epub() -> Vec<u8> {
	use std::io::Write;

	let mut buf = Vec::new();
	{
		let mut zip = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
		let options: zip::write::FileOptions<()> = zip::write::FileOptions::default()
			.compression_method(zip::CompressionMethod::Stored);

		zip.start_file("mimetype", options).unwrap();
		zip.write_all(b"application/epub+zip").unwrap();

		zip.start_file("META-INF/container.xml", options).unwrap();
		zip.write_all(br#"<?xml version="1.0"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
  <rootfiles>
    <rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/>
  </rootfiles>
</container>"#).unwrap();

		zip.start_file("OEBPS/content.opf", options).unwrap();
		zip.write_all(br#"<?xml version="1.0"?>
<package xmlns="http://www.idpf.org/2007/opf" version="3.0" unique-identifier="book-id">
  <metadata>
    <dc:title xmlns:dc="http://purl.org/dc/elements/1.1/">Test EPUB</dc:title>
    <dc:language xmlns:dc="http://purl.org/dc/elements/1.1/">en</dc:language>
  </metadata>
  <spine>
    <itemref idref="xhtml001"/>
  </spine>
  <manifest>
    <item id="xhtml001" href="content.xhtml" media-type="application/xhtml+xml"/>
  </manifest>
</package>"#).unwrap();

		zip.start_file("OEBPS/content.xhtml", options).unwrap();
		zip.write_all(br#"<?xml version="1.0"?>
<html xmlns="http://www.w3.org/1999/xhtml">
  <head><title>Test</title></head>
  <body><p>Hello from test EPUB.</p></body>
</html>"#).unwrap();
	}
	buf
}
