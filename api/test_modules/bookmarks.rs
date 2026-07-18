

use crate::common::TestApp;

#[tokio::test]
async fn bookmarks_list_empty() {
	let app = TestApp::new().await;
	app.register_and_login("bm").await;
	let book = app.create_book("Bookmark Test", None).await;
	let id = book["id"].as_str().unwrap();
	let (s, result) = app.raw_get(&format!("/api/v1/bookmarks/{}", id)).await;
	assert_eq!(s, 200);
	assert_eq!(result.as_array().unwrap().len(), 0, "no bookmarks initially");
}

#[tokio::test]
async fn bookmarks_create_and_list() {
	let app = TestApp::new().await;
	app.register_and_login("bm").await;
	let book = app.create_book("BM Book", None).await;
	let book_id = book["id"].as_str().unwrap().to_string();

	let (s, created) = app.raw_post(&format!("/api/v1/bookmarks/{}", book_id), &serde_json::json!({
		"book_id": book_id, "section_id": "00000000-0000-0000-0000-000000000001", "block_index": 0, "title": "My Bookmark"
	})).await;
	assert_eq!(s, 200, "create bookmark");
	assert_eq!(created["title"], "My Bookmark");

	let (s, list) = app.raw_get(&format!("/api/v1/bookmarks/{}", book_id)).await;
	assert_eq!(s, 200);
	assert_eq!(list.as_array().unwrap().len(), 1);
	assert_eq!(list[0]["title"], "My Bookmark");
}

#[tokio::test]
async fn bookmarks_delete() {
	let app = TestApp::new().await;
	app.register_and_login("bm").await;
	let book = app.create_book("BM Del", None).await;
	let book_id = book["id"].as_str().unwrap().to_string();

	let (s, created) = app.raw_post(&format!("/api/v1/bookmarks/{}", book_id), &serde_json::json!({
		"book_id": book_id, "section_id": "00000000-0000-0000-0000-000000000001", "block_index": 0
	})).await;
	assert_eq!(s, 200);
	let bm_id = created["id"].as_str().unwrap();

	let (s, _) = app.raw_delete(&format!("/api/v1/bookmarks/single/{}", bm_id)).await;
	assert_eq!(s, 200, "delete bookmark");

	let (_s, list) = app.raw_get(&format!("/api/v1/bookmarks/{}", book_id)).await;
	assert_eq!(list.as_array().unwrap().len(), 0, "should be empty after delete");
}

#[tokio::test]
async fn bookmarks_cross_user_isolation() {
	let app = TestApp::new().await;

	app.register("alice_bm@test.com", "pass1!", "Alice").await.unwrap();
	app.login("alice_bm@test.com", "pass1!").await.unwrap();
	let book = app.create_book("Alice BM", None).await;
	let book_id = book["id"].as_str().unwrap().to_string();
	app.raw_post(&format!("/api/v1/bookmarks/{}", book_id), &serde_json::json!({
		"book_id": book_id, "section_id": "s1", "block_index": 0
	})).await;
	app.logout().await.unwrap();

	app.register("bob_bm@test.com", "pass2!", "Bob").await.unwrap();
	app.login("bob_bm@test.com", "pass2!").await.unwrap();
	let (s, list) = app.raw_get(&format!("/api/v1/bookmarks/{}", book_id)).await;
	assert_eq!(s, 200);
	assert_eq!(list.as_array().unwrap().len(), 0, "Bob should see 0 bookmarks");
}

#[tokio::test]
async fn bookmarks_nonexistent_book() {
	let app = TestApp::new().await;
	app.register_and_login("bm").await;
	let (s, _) = app.raw_get("/api/v1/bookmarks/00000000-0000-0000-0000-000000000000").await;
	assert_eq!(s, 200, "empty list for non-existent book");
}

#[tokio::test]
async fn bookmarks_nonexistent_delete_returns_404() {
	let app = TestApp::new().await;
	app.register_and_login("bm").await;
	let (s, _) = app.raw_delete("/api/v1/bookmarks/single/00000000-0000-0000-0000-000000000000").await;
	assert_eq!(s, 404);
}
