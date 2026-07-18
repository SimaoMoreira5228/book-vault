

use crate::common::TestApp;

#[tokio::test]
async fn create_book_minimal() {
	let app = TestApp::new().await;
	app.register_and_login("books").await;

	let book = app.create_book("Test Title", None).await;
	assert_eq!(book["title"], "Test Title");
	assert_eq!(book["read_status"], "unread");
	assert!(book["id"].as_str().unwrap().len() > 0);
}

#[tokio::test]
async fn create_book_with_author() {
	let app = TestApp::new().await;
	app.register_and_login("books").await;

	let book = app.create_book("My Book", Some("John Doe")).await;
	assert_eq!(book["title"], "My Book");
	assert_eq!(book["author"], "John Doe");
}

#[tokio::test]
async fn list_books_returns_only_own() {
	let app = TestApp::new().await;
	let user = app.register_and_login("books").await;
	let _email = user.0;

	app.create_book("Alice's Book", None).await;
	app.create_book("Another Book", None).await;

	let list = app.list_books().await;
	let arr = list["books"].as_array().unwrap();
	assert!(arr.len() >= 2, "should have at least 2 books");
	let titles: Vec<&str> = arr.iter().map(|b| b["title"].as_str().unwrap()).collect();
	assert!(titles.contains(&"Alice's Book"));
}

#[tokio::test]
async fn update_book_fields() {
	let app = TestApp::new().await;
	app.register_and_login("books").await;

	let book = app.create_book("Original", None).await;
	let id = book["id"].as_str().unwrap().to_string();

	let (status, updated) = app
		.raw_put(
			&format!("/api/v1/books/{}", id),
			&serde_json::json!({ "title": "Updated Title", "read_status": "reading" }),
		)
		.await;
	assert_eq!(status, 200, "update should succeed");
	assert_eq!(updated["title"], "Updated Title");
	assert_eq!(updated["read_status"], "reading");
}

#[tokio::test]
async fn delete_book() {
	let app = TestApp::new().await;
	app.register_and_login("books").await;

	let book = app.create_book("To Delete", None).await;
	let id = book["id"].as_str().unwrap().to_string();

	let (status, _) = app.raw_delete(&format!("/api/v1/books/{}", id)).await;
	assert_eq!(status, 200, "delete should succeed");

	let (status, _) = app.raw_get(&format!("/api/v1/books/{}", id)).await;
	assert_eq!(status, 404, "deleted book should not be found");
}

#[tokio::test]
async fn get_nonexistent_book() {
	let app = TestApp::new().await;
	app.register_and_login("books").await;

	let (status, _) = app.raw_get("/api/v1/books/00000000-0000-0000-0000-000000000000").await;
	assert_eq!(status, 404, "nonexistent book should 404");
}

#[tokio::test]
async fn library_isolation() {
	let app = TestApp::new().await;

	app.register("alice@iso.test", "password1!", "Alice").await.unwrap();
	app.login("alice@iso.test", "password1!").await.unwrap();
	let book = app.create_book("Alice's Private Book", None).await;
	let book_id = book["id"].as_str().unwrap().to_string();

	app.register("bob@iso.test", "password2!", "Bob").await.unwrap();
	app.login("bob@iso.test", "password2!").await.unwrap();
	let (status, _) = app.raw_get(&format!("/api/v1/books/{}", book_id)).await;
	assert_eq!(status, 403, "Bob should not see Alice's book");
}

#[tokio::test]
async fn upload_file_detects_format() {
	let app = TestApp::new().await;
	app.register_and_login("books").await;

	let epub_bytes = b"PK\x03\x04...mock content for epub detection...";
	let resp = app
		.client
		.post(app.url("/api/v1/books/upload"))
		.multipart(reqwest::multipart::Form::new().part(
			"file",
			reqwest::multipart::Part::bytes(epub_bytes.to_vec()).file_name("test.epub"),
		))
		.send()
		.await
		.expect("upload");

	let status = resp.status();
	let json: serde_json::Value = resp.json().await.unwrap_or_default();
	assert_eq!(status.as_u16(), 202, "upload should return 202");
	assert!(json.get("job_id").is_some(), "should return a job_id");
}

#[tokio::test]
async fn book_count_increases() {
	let app = TestApp::new().await;
	app.register_and_login("books").await;

	let list0 = app.list_books().await;
	let count0 = list0["books"].as_array().unwrap().len();

	app.create_book("Count Test 1", None).await;
	app.create_book("Count Test 2", None).await;

	let list2 = app.list_books().await;
	let count2 = list2["books"].as_array().unwrap().len();
	assert_eq!(count2, count0 + 2, "book count should increase by 2");
}
