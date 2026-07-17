mod common;

use common::TestApp;

#[tokio::test]
async fn authors_list_empty() {
	let app = TestApp::new().await;
	app.register_and_login("authors").await;
	let (s, result) = app.raw_get("/api/v1/authors").await;
	assert_eq!(s, 200);
	assert_eq!(result.as_array().unwrap().len(), 0, "no authors initially");
}

#[tokio::test]
async fn authors_create_and_list() {
	let app = TestApp::new().await;
	app.register_and_login("authors").await;

	let (s, _) = app.raw_post("/api/v1/authors", &serde_json::json!({
		"name": "Isaac Asimov", "bio": "Science fiction writer", "birth_date": "1920-01-02"
	})).await;
	assert_eq!(s, 200, "create author");

	let (s, result) = app.raw_get("/api/v1/authors").await;
	assert_eq!(s, 200);
	assert_eq!(result.as_array().unwrap().len(), 1);
	assert_eq!(result[0]["name"], "Isaac Asimov");
	assert_eq!(result[0]["bio"], "Science fiction writer");
	assert_eq!(result[0]["book_count"], 0);
}

#[tokio::test]
async fn authors_get_by_id() {
	let app = TestApp::new().await;
	app.register_and_login("authors").await;

	let (s, created) = app.raw_post("/api/v1/authors", &serde_json::json!({"name": "Arthur C. Clarke"})).await;
	assert_eq!(s, 200);
	let id = created["id"].as_str().unwrap();

	let (s, result) = app.raw_get(&format!("/api/v1/authors/{}", id)).await;
	assert_eq!(s, 200);
	assert_eq!(result["name"], "Arthur C. Clarke");
}

#[tokio::test]
async fn authors_update() {
	let app = TestApp::new().await;
	app.register_and_login("authors").await;

	let (s, created) = app.raw_post("/api/v1/authors", &serde_json::json!({"name": "Old Name"})).await;
	assert_eq!(s, 200);
	let id = created["id"].as_str().unwrap();

	let (s, result) = app.raw_put(&format!("/api/v1/authors/{}", id), &serde_json::json!({"name": "New Name", "birth_date": "1930-01-01"})).await;
	assert_eq!(s, 200);
	assert_eq!(result["name"], "New Name");
	assert_eq!(result["birth_date"], "1930-01-01");
}

#[tokio::test]
async fn authors_delete_unlinks_books() {
	let app = TestApp::new().await;
	app.register_and_login("authors").await;

	let (s, created) = app.raw_post("/api/v1/authors", &serde_json::json!({"name": "To Delete"})).await;
	assert_eq!(s, 200);
	let author_id = created["id"].as_str().unwrap().to_string();

	let book = app.create_book("Author's Book", Some("To Delete")).await;
	let book_id = book["id"].as_str().unwrap();

	let (s, _) = app.raw_put(&format!("/api/v1/books/{}/link-author", book_id), &serde_json::json!({"author_id": author_id})).await;
	assert_eq!(s, 200, "link author to book");

	let (s, _) = app.raw_delete(&format!("/api/v1/authors/{}", author_id)).await;
	assert_eq!(s, 200);
	assert_eq!(app.list_books().await["books"][0]["author_id"], serde_json::Value::Null, "book author_id should be null after deletion");
}

#[tokio::test]
async fn authors_cross_user_isolation() {
	let app = TestApp::new().await;

	app.register("alice_auth@test.com", "pass1!", "Alice").await.unwrap();
	app.login("alice_auth@test.com", "pass1!").await.unwrap();
	app.raw_post("/api/v1/authors", &serde_json::json!({"name": "Alice's Author"})).await;
	app.logout().await.unwrap();

	app.register("bob_auth@test.com", "pass2!", "Bob").await.unwrap();
	app.login("bob_auth@test.com", "pass2!").await.unwrap();
	let (s, result) = app.raw_get("/api/v1/authors").await;
	assert_eq!(s, 200);
	assert_eq!(result.as_array().unwrap().len(), 0, "Bob should see 0 authors");
}

#[tokio::test]
async fn authors_nonexistent_returns_404() {
	let app = TestApp::new().await;
	app.register_and_login("authors").await;
	let (s, _) = app.raw_get("/api/v1/authors/00000000-0000-0000-0000-000000000000").await;
	assert_eq!(s, 404);
}
