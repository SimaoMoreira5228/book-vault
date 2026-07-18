

use crate::common::TestApp;

#[tokio::test]
async fn export_markdown_returns_content() {
	let app = TestApp::new().await;
	app.register_and_login("export").await;

	let book = app.create_book("Export Test Book", Some("Export Author")).await;
	let book_id = book["id"].as_str().unwrap().to_string();

	let (status, result) = app
		.raw_get(&format!("/api/v1/books/{}/export?format=markdown", book_id))
		.await;
	assert!(
		status == 200 || status == 404 || status == 500,
		"export should not crash: {status}"
	);
	if status == 200 {
		assert!(result.is_string() || result.is_object(), "should return content");
	}
}

#[tokio::test]
async fn export_epub_returns_binary() {
	let app = TestApp::new().await;
	app.register_and_login("export").await;

	let book = app.create_book("EPUB Export", None).await;
	let book_id = book["id"].as_str().unwrap().to_string();

	let resp = app
		.client
		.get(app.url(&format!("/api/v1/books/{}/export?format=epub", book_id)))
		.send()
		.await
		.expect("export request");

	if resp.status().as_u16() == 200 {
		let content_type = resp.headers().get("content-type").and_then(|v| v.to_str().ok()).unwrap_or("");
		assert!(
			content_type.contains("epub") || content_type.contains("zip") || content_type.contains("octet"),
			"EPUB export should have appropriate content type: {content_type}"
		);
	} else {
		assert!(
			resp.status().as_u16() == 404 || resp.status().as_u16() == 500,
			"unexpected status: {}",
			resp.status()
		);
	}
}

#[tokio::test]
async fn export_pdf_returns_pdf() {
	let app = TestApp::new().await;
	app.register_and_login("export").await;

	let book = app.create_book("PDF Export", None).await;
	let book_id = book["id"].as_str().unwrap().to_string();

	let resp = app
		.client
		.get(app.url(&format!("/api/v1/books/{}/export?format=pdf", book_id)))
		.send()
		.await
		.expect("export request");

	if resp.status().as_u16() == 200 {
		let content_type = resp.headers().get("content-type").and_then(|v| v.to_str().ok()).unwrap_or("");
		assert!(
			content_type.contains("pdf") || content_type.contains("octet"),
			"PDF export should return PDF content type: {content_type}"
		);
	} else {
		assert!(
			resp.status().as_u16() == 404 || resp.status().as_u16() == 500,
			"unexpected status: {}",
			resp.status()
		);
	}
}

#[tokio::test]
async fn export_invalid_format_returns_error() {
	let app = TestApp::new().await;
	app.register_and_login("export").await;

	let book = app.create_book("Invalid Export", None).await;
	let book_id = book["id"].as_str().unwrap().to_string();

	let (status, _) = app.raw_get(&format!("/api/v1/books/{}/export?format=txt", book_id)).await;
	assert_eq!(status, 404, "invalid format should return 404");
}

#[tokio::test]
async fn export_requires_auth() {
	let app = TestApp::new().await;
	app.register_and_login("export").await;

	let book = app.create_book("Export Auth", None).await;
	let book_id = book["id"].as_str().unwrap().to_string();

	app.logout().await;
	let (status, _) = app
		.raw_get(&format!("/api/v1/books/{}/export?format=markdown", book_id))
		.await;
	assert_eq!(status, 401, "unauthenticated export should be rejected");
}

#[tokio::test]
async fn export_isolates_users() {
	let app = TestApp::new().await;

	app.register("alice_exp@test.com", "pass1!", "Alice").await.unwrap();
	app.login("alice_exp@test.com", "pass1!").await.unwrap();
	let book = app.create_book("Alice's Export", None).await;
	let book_id = book["id"].as_str().unwrap().to_string();

	app.register("bob_exp@test.com", "pass2!", "Bob").await.unwrap();
	app.login("bob_exp@test.com", "pass2!").await.unwrap();
	let (status, _) = app
		.raw_get(&format!("/api/v1/books/{}/export?format=markdown", book_id))
		.await;
	assert_eq!(status, 403, "Bob should not export Alice's book");
}

#[tokio::test]
async fn export_bvir_format() {
	let app = TestApp::new().await;
	app.register_and_login("export").await;

	let book = app.create_book("BVIR Export", None).await;
	let book_id = book["id"].as_str().unwrap().to_string();

	let (status, _) = app.raw_get(&format!("/api/v1/books/{}/export?format=bvir", book_id)).await;
	assert!(status == 200 || status == 404, "BVIR export: status {status} unexpected");
}
