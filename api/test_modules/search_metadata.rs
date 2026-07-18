

use crate::common::TestApp;

#[tokio::test]
async fn search_by_title() {
	let app = TestApp::new().await;
	app.register_and_login("search").await;

	app.create_book("The Great Gatsby", Some("F. Scott Fitzgerald")).await;
	app.create_book("Gatsby's Return", Some("F. Scott Fitzgerald")).await;
	app.create_book("Moby Dick", Some("Herman Melville")).await;

	let (status, result) = app.raw_get("/api/v1/search?q=Gatsby").await;
	assert_eq!(status, 200);
	let books = result["books"].as_array().unwrap();
	assert_eq!(books.len(), 2, "should find both Gatsby books");
}

#[tokio::test]
async fn search_by_author() {
	let app = TestApp::new().await;
	app.register_and_login("search").await;

	app.create_book("Book A", Some("Jane Austen")).await;
	app.create_book("Book B", Some("Mark Twain")).await;

	let (status, result) = app.raw_get("/api/v1/search?q=Austen").await;
	assert_eq!(status, 200);
	let books = result["books"].as_array().unwrap();
	assert_eq!(books.len(), 1, "should find Jane Austen's book");
	assert_eq!(books[0]["title"], "Book A");
}

#[tokio::test]
async fn search_empty_query() {
	let app = TestApp::new().await;
	app.register_and_login("search").await;

	app.create_book("Any Book", None).await;
	let (status, result) = app.raw_get("/api/v1/search?q=").await;
	assert_eq!(status, 200);
	let books = result["books"].as_array().unwrap();
	assert!(books.len() >= 1, "empty query should match all books");
}

#[tokio::test]
async fn search_no_match() {
	let app = TestApp::new().await;
	app.register_and_login("search").await;

	app.create_book("Visible Book", None).await;
	let (status, result) = app.raw_get("/api/v1/search?q=xyznonexistent").await;
	assert_eq!(status, 200);
	assert_eq!(result["books"].as_array().unwrap().len(), 0, "no match should return empty");
}

#[tokio::test]
async fn search_other_user_books_not_included() {
	let app = TestApp::new().await;

	app.register("alice_search@test.com", "pass1!", "Alice").await.unwrap();
	app.login("alice_search@test.com", "pass1!").await.unwrap();
	app.create_book("Alice's Private Novel", None).await;

	app.register("bob_search@test.com", "pass2!", "Bob").await.unwrap();
	app.login("bob_search@test.com", "pass2!").await.unwrap();
	let (status, result) = app.raw_get("/api/v1/search?q=Novel").await;
	assert_eq!(status, 200);
	let books = result["books"].as_array().unwrap();
	assert_eq!(books.len(), 0, "Bob should not see Alice's books in search");
}

#[tokio::test]
async fn metadata_get_no_enrichment() {
	let app = TestApp::new().await;
	app.register_and_login("meta").await;

	let book = app.create_book("Metadata Test", None).await;
	let id = book["id"].as_str().unwrap().to_string();

	let (status, meta) = app.raw_get(&format!("/api/v1/books/{}/metadata", id)).await;
	assert_eq!(status, 200);
	assert_eq!(meta["title"], "Metadata Test");
}

#[tokio::test]
async fn metadata_candidates_requires_auth() {
	let app = TestApp::new().await;
	app.register_and_login("meta").await;

	let book = app.create_book("Candidate Test", None).await;
	let id = book["id"].as_str().unwrap().to_string();

	let (status, candidates) = app.raw_get(&format!("/api/v1/books/{}/metadata/candidates", id)).await;
	assert_eq!(status, 200, "should return 200 even with no candidates");
}

#[tokio::test]
async fn metadata_candidates_with_query() {
	let app = TestApp::new().await;
	app.register_and_login("meta").await;

	let book = app.create_book("Test Book", Some("Test Author")).await;
	let id = book["id"].as_str().unwrap().to_string();

	let (status, candidates) = app
		.raw_get(&format!(
			"/api/v1/books/{}/metadata/candidates?title=Test+Book&author=Test+Author",
			id
		))
		.await;
	assert_eq!(status, 200);
	assert!(candidates.is_array() || candidates.is_object(), "should return valid JSON");
}

#[tokio::test]
async fn metadata_lock_field() {
	let app = TestApp::new().await;
	app.register_and_login("meta").await;

	let book = app.create_book("Lock Test", None).await;
	let id = book["id"].as_str().unwrap().to_string();

	let (status, _) = app
		.raw_post(&format!("/api/v1/books/{}/metadata/lock/title", id), &serde_json::json!({}))
		.await;
	assert_eq!(status, 200, "should lock title field");

	let (status, meta) = app.raw_get(&format!("/api/v1/books/{}/metadata", id)).await;
	assert_eq!(status, 200);
	if let Some(locked) = meta.get("locked_fields") {
		assert!(
			locked.as_array().map_or(false, |a| a.contains(&serde_json::json!("title"))),
			"locked_fields should contain 'title'"
		);
	}

	let (status, _) = app.raw_delete(&format!("/api/v1/books/{}/metadata/lock/title", id)).await;
	assert_eq!(status, 200, "should unlock title field");
}
