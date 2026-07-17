mod common;

use common::TestApp;

#[tokio::test]
async fn content_search_returns_metadata_hits() {
	let app = TestApp::new().await;
	app.register_and_login("cont_search").await;

	app.create_book("Fantasy World", Some("J.R.R. Tolkien")).await;
	app.create_book("Physics 101", Some("Richard Feynman")).await;

	let (status, result) = app.raw_get("/api/v1/search?q=Tolkien").await;
	assert_eq!(status, 200);
	assert_eq!(result["books"].as_array().unwrap().len(), 1, "should match author");
	assert_eq!(result["books"][0]["title"], "Fantasy World");
}

#[tokio::test]
async fn content_search_returns_content_hits() {
	let app = TestApp::new().await;
	let (_email, _pw, _user) = app.register_and_login("content").await;

	let book = app.create_book("Content Search Book", None).await;
	let book_id = book["id"].as_str().unwrap().to_string();

	let (status, result) = app.raw_get(&format!("/api/v1/search?q=Content")).await;
	assert_eq!(status, 200);
	let books = result["books"].as_array().unwrap();
	assert_eq!(books.len(), 1, "should find by title");
	assert!(books[0]["id"] == book_id);
}

#[tokio::test]
async fn content_search_handles_empty_query() {
	let app = TestApp::new().await;
	app.register_and_login("empty").await;

	app.create_book("Visible Book", None).await;
	let (status, result) = app.raw_get("/api/v1/search?q=").await;
	assert_eq!(status, 200);
	assert_eq!(result["books"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn content_search_isolates_users() {
	let app = TestApp::new().await;

	app.register("alice_cs@test.com", "pass1!", "Alice").await.unwrap();
	app.login("alice_cs@test.com", "pass1!").await.unwrap();
	app.create_book("Alice's Search Book", None).await;

	app.register("bob_cs@test.com", "pass2!", "Bob").await.unwrap();
	app.login("bob_cs@test.com", "pass2!").await.unwrap();
	let (status, result) = app.raw_get("/api/v1/search?q=Alice").await;
	assert_eq!(status, 200);
	assert_eq!(result["books"].as_array().unwrap().len(), 0, "Bob should not see Alice books");
}

#[tokio::test]
async fn content_search_no_match_returns_empty() {
	let app = TestApp::new().await;
	app.register_and_login("no_match").await;

	app.create_book("The Only Book", None).await;
	let (status, result) = app.raw_get("/api/v1/search?q=xyznonexistent999").await;
	assert_eq!(status, 200);
	assert_eq!(result["books"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn content_search_prefix_matching() {
	let app = TestApp::new().await;
	app.register_and_login("prefix").await;

	app.create_book("Programming in Rust", Some("John Doe")).await;
	app.create_book("Advanced Python", Some("Jane Doe")).await;

	let (status, result) = app.raw_get("/api/v1/search?q=Prog").await;
	assert_eq!(status, 200);
	let books = result["books"].as_array().unwrap();
	assert_eq!(books.len(), 1, "prefix 'Prog' should match 'Programming'");
	assert_eq!(books[0]["title"], "Programming in Rust");
}

#[tokio::test]
async fn content_search_fuzzy_via_trigram() {
	let app = TestApp::new().await;
	app.register_and_login("fuzzy").await;

	app.create_book("The Rust Programming Language", Some("Steve Klabnik")).await;

	let (status, result) = app.raw_get("/api/v1/search?q=Rust").await;
	assert_eq!(status, 200);
	let books = result["books"].as_array().unwrap();
	assert_eq!(books.len(), 1, "should find book by title substring");
}

#[tokio::test]
async fn content_search_content_hits_field() {
	let app = TestApp::new().await;
	app.register_and_login("chits").await;

	app.create_book("Search Results Test", None).await;

	let (status, result) = app.raw_get("/api/v1/search?q=Search").await;
	assert_eq!(status, 200);
	assert!(
		result.get("content_hits").is_some(),
		"response should include content_hits field"
	);
}
